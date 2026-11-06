///! HTTP caching middleware
///!
///! This module provides HTTP caching support with ETag and Last-Modified headers.
///! It implements conditional requests (304 Not Modified) to reduce bandwidth usage.
///!
///! # Features
///! - ETag generation based on response body hashing
///! - Last-Modified header support
///! - Conditional request handling (If-None-Match, If-Modified-Since)
///! - Cache-Control header management
///! - Automatic 304 Not Modified responses
///!
///! # Usage
///! ```rust,no_run
///! use axum::Router;
///! use analytics_api::middleware::CacheMiddleware;
///!
///! let app = Router::new()
///!     .route("/api/v1/traces", get(list_traces))
///!     .layer(CacheMiddleware::new(60)); // 60 second cache TTL
///! ```
use axum::{
    body::Body,
    extract::Request,
    http::{header, HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use bytes::Bytes;
use http_body_util::BodyExt;
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, info};

/// Cache configuration
#[derive(Debug, Clone, Copy)]
pub struct CacheConfig {
    /// Time-to-live in seconds
    pub ttl_seconds: u64,
    /// Whether to enable ETag generation
    pub enable_etag: bool,
    /// Whether to enable Last-Modified headers
    pub enable_last_modified: bool,
}

impl CacheConfig {
    /// Create new cache configuration
    pub fn new(ttl_seconds: u64) -> Self {
        Self {
            ttl_seconds,
            enable_etag: true,
            enable_last_modified: true,
        }
    }

    /// Create configuration with ETags only
    pub fn etag_only(ttl_seconds: u64) -> Self {
        Self {
            ttl_seconds,
            enable_etag: true,
            enable_last_modified: false,
        }
    }

    /// Create configuration with Last-Modified only
    pub fn last_modified_only(ttl_seconds: u64) -> Self {
        Self {
            ttl_seconds,
            enable_etag: false,
            enable_last_modified: true,
        }
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self::new(60) // Default to 60 seconds
    }
}

/// HTTP caching middleware
pub async fn cache_middleware(
    config: CacheConfig,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract conditional request headers
    let if_none_match = req
        .headers()
        .get(header::IF_NONE_MATCH)
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    let if_modified_since = req
        .headers()
        .get(header::IF_MODIFIED_SINCE)
        .and_then(|h| h.to_str().ok())
        .and_then(|s| httpdate::parse_http_date(s).ok());

    // Execute the request
    let response = next.run(req).await;

    // Only cache successful responses
    if response.status() != StatusCode::OK {
        return Ok(response);
    }

    // Extract response parts
    let (mut parts, body) = response.into_parts();

    // Collect the body bytes
    let body_bytes = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };

    // Generate ETag if enabled
    let etag = if config.enable_etag {
        Some(generate_etag(&body_bytes))
    } else {
        None
    };

    // Get current timestamp for Last-Modified
    let now = SystemTime::now();
    let last_modified = if config.enable_last_modified {
        Some(now)
    } else {
        None
    };

    // Check if client has a valid cached version
    let not_modified =
        check_not_modified(&if_none_match, &if_modified_since, &etag, &last_modified);

    if not_modified {
        info!("Cache hit: returning 304 Not Modified");

        // Build 304 response with minimal headers
        let mut not_modified_response = Response::builder()
            .status(StatusCode::NOT_MODIFIED)
            .body(Body::empty())
            .unwrap();

        // Copy cache headers
        if let Some(etag_value) = etag {
            not_modified_response
                .headers_mut()
                .insert(header::ETAG, HeaderValue::from_str(&etag_value).unwrap());
        }

        if let Some(modified_time) = last_modified {
            if let Some(http_date) = format_http_date(modified_time) {
                not_modified_response.headers_mut().insert(
                    header::LAST_MODIFIED,
                    HeaderValue::from_str(&http_date).unwrap(),
                );
            }
        }

        // Add Cache-Control header
        not_modified_response.headers_mut().insert(
            header::CACHE_CONTROL,
            HeaderValue::from_str(&format!("private, max-age={}", config.ttl_seconds)).unwrap(),
        );

        return Ok(not_modified_response);
    }

    // Cache miss: add caching headers to the full response
    debug!("Cache miss: returning full response with cache headers");

    if let Some(etag_value) = etag {
        parts
            .headers
            .insert(header::ETAG, HeaderValue::from_str(&etag_value).unwrap());
    }

    if let Some(modified_time) = last_modified {
        if let Some(http_date) = format_http_date(modified_time) {
            parts.headers.insert(
                header::LAST_MODIFIED,
                HeaderValue::from_str(&http_date).unwrap(),
            );
        }
    }

    // Add Cache-Control header
    parts.headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_str(&format!("private, max-age={}", config.ttl_seconds)).unwrap(),
    );

    // Rebuild response with caching headers
    let cached_response = Response::from_parts(parts, Body::from(body_bytes));

    Ok(cached_response)
}

/// Generate ETag from response body
fn generate_etag(body: &Bytes) -> String {
    let mut hasher = Sha256::new();
    hasher.update(body);
    let hash = hasher.finalize();
    format!("\"{}\"", hex::encode(&hash[..16])) // Use first 16 bytes (32 hex chars)
}

/// Check if the cached version is still valid
fn check_not_modified(
    if_none_match: &Option<String>,
    if_modified_since: &Option<SystemTime>,
    etag: &Option<String>,
    last_modified: &Option<SystemTime>,
) -> bool {
    // Check ETag first (stronger validator)
    if let (Some(client_etag), Some(server_etag)) = (if_none_match, etag) {
        // Handle multiple ETags in If-None-Match
        let client_etags: Vec<&str> = client_etag.split(',').map(|s| s.trim()).collect();
        if client_etags.contains(&server_etag.as_str()) || client_etags.contains(&"*") {
            return true;
        }
    }

    // Check Last-Modified (weaker validator)
    if let (Some(client_time), Some(server_time)) = (if_modified_since, last_modified) {
        // Resource not modified if client's cached version is >= server's version
        if client_time >= server_time {
            return true;
        }
    }

    false
}

/// Format SystemTime as HTTP date string
fn format_http_date(time: SystemTime) -> Option<String> {
    let duration_since_epoch = time.duration_since(UNIX_EPOCH).ok()?;
    let secs = duration_since_epoch.as_secs();

    // Convert to HTTP date format (RFC 7231)
    let datetime = chrono::DateTime::from_timestamp(secs as i64, 0)?;
    Some(datetime.format("%a, %d %b %Y %H:%M:%S GMT").to_string())
}

/// Cache middleware layer builder
#[derive(Clone)]
pub struct CacheMiddleware {
    config: CacheConfig,
}

impl CacheMiddleware {
    /// Create new cache middleware with default config
    pub fn new(ttl_seconds: u64) -> Self {
        Self {
            config: CacheConfig::new(ttl_seconds),
        }
    }

    /// Create with custom configuration
    pub fn with_config(config: CacheConfig) -> Self {
        Self { config }
    }

    /// Get the configuration
    pub fn config(&self) -> CacheConfig {
        self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_config_creation() {
        let config = CacheConfig::new(300);
        assert_eq!(config.ttl_seconds, 300);
        assert!(config.enable_etag);
        assert!(config.enable_last_modified);

        let etag_config = CacheConfig::etag_only(60);
        assert_eq!(etag_config.ttl_seconds, 60);
        assert!(etag_config.enable_etag);
        assert!(!etag_config.enable_last_modified);

        let lm_config = CacheConfig::last_modified_only(120);
        assert_eq!(lm_config.ttl_seconds, 120);
        assert!(!lm_config.enable_etag);
        assert!(lm_config.enable_last_modified);
    }

    #[test]
    fn test_etag_generation() {
        let body1 = Bytes::from("Hello, World!");
        let etag1 = generate_etag(&body1);

        // ETag should be consistent for same content
        let etag1_again = generate_etag(&body1);
        assert_eq!(etag1, etag1_again);

        // Different content should produce different ETag
        let body2 = Bytes::from("Different content");
        let etag2 = generate_etag(&body2);
        assert_ne!(etag1, etag2);

        // ETag should be quoted and hex-encoded
        assert!(etag1.starts_with('"'));
        assert!(etag1.ends_with('"'));
        assert!(etag1.len() > 10); // Should have reasonable length
    }

    #[test]
    fn test_check_not_modified_with_etag() {
        let etag = Some("\"abc123\"".to_string());
        let if_none_match = Some("\"abc123\"".to_string());

        // Matching ETag should indicate not modified
        assert!(check_not_modified(&if_none_match, &None, &etag, &None));

        // Different ETag should indicate modified
        let different_etag = Some("\"xyz789\"".to_string());
        assert!(!check_not_modified(
            &if_none_match,
            &None,
            &different_etag,
            &None
        ));

        // Wildcard should match
        let wildcard = Some("*".to_string());
        assert!(check_not_modified(&wildcard, &None, &etag, &None));
    }

    #[test]
    fn test_check_not_modified_with_multiple_etags() {
        let etag = Some("\"abc123\"".to_string());
        let if_none_match = Some("\"xyz789\", \"abc123\", \"def456\"".to_string());

        // Should find matching ETag in list
        assert!(check_not_modified(&if_none_match, &None, &etag, &None));
    }

    #[test]
    fn test_check_not_modified_with_last_modified() {
        let now = SystemTime::now();
        let past = now - std::time::Duration::from_secs(3600); // 1 hour ago

        let last_modified = Some(past);
        let if_modified_since = Some(now);

        // Client's cached version is newer, so not modified
        assert!(check_not_modified(
            &None,
            &if_modified_since,
            &None,
            &last_modified
        ));

        // Client's cached version is older, so modified
        let older_cache = Some(past - std::time::Duration::from_secs(3600));
        assert!(!check_not_modified(
            &None,
            &older_cache,
            &None,
            &last_modified
        ));
    }

    #[test]
    fn test_format_http_date() {
        let time = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1234567890);
        let formatted = format_http_date(time).unwrap();

        // Should be in HTTP date format
        assert!(formatted.ends_with(" GMT"));
        assert!(formatted.len() > 20);
    }

    #[test]
    fn test_etag_with_empty_body() {
        let empty = Bytes::new();
        let etag = generate_etag(&empty);

        // Should generate valid ETag even for empty body
        assert!(etag.starts_with('"'));
        assert!(etag.ends_with('"'));
    }

    #[test]
    fn test_etag_with_large_body() {
        let large_body = Bytes::from(vec![0u8; 1_000_000]); // 1MB of zeros
        let etag = generate_etag(&large_body);

        // Should handle large bodies
        assert!(etag.starts_with('"'));
        assert!(etag.ends_with('"'));
    }
}
