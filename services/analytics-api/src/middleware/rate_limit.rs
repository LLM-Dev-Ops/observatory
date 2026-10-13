///! Rate limiting middleware using Token Bucket algorithm
///!
///! This module provides distributed rate limiting with Redis backend.
///! It implements the token bucket algorithm which allows for burst traffic
///! while maintaining average rate limits.
///!
///! # Features
///! - Token bucket algorithm for smooth rate limiting
///! - Redis-backed for distributed rate limiting across API instances
///! - Tiered rate limits based on user role
///! - Per-user and per-API-key rate limiting
///! - Rate limit headers in responses (X-RateLimit-*)
///!
///! # Usage
///! ```rust,no_run
///! use axum::Router;
///! use analytics_api::middleware::RateLimitLayer;
///!
///! let app = Router::new()
///!     .route("/api/v1/traces", get(list_traces))
///!     .layer(RateLimitLayer::new(redis_client));
///! ```
use axum::{
    body::Body,
    extract::{Request, State},
    http::{HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use redis::{aio::MultiplexedConnection, AsyncCommands};
use serde_json::json;
use std::{sync::Arc, time::Duration};
use tracing::{error, info, warn};

use super::auth::{AuthContext, Role};

/// Rate limit configuration for different tiers
#[derive(Debug, Clone, Copy)]
pub struct RateLimitConfig {
    /// Requests per minute
    pub requests_per_minute: u32,
    /// Burst capacity (allows temporary burst above rate)
    pub burst_capacity: u32,
}

impl RateLimitConfig {
    /// Get configuration for a role
    pub fn for_role(role: &Role) -> Self {
        match role {
            Role::Admin => Self {
                requests_per_minute: 100_000,
                burst_capacity: 120_000,
            },
            Role::Developer => Self {
                requests_per_minute: 10_000,
                burst_capacity: 12_000,
            },
            Role::Viewer => Self {
                requests_per_minute: 1_000,
                burst_capacity: 1_200,
            },
            Role::Billing => Self {
                requests_per_minute: 1_000,
                burst_capacity: 1_200,
            },
        }
    }

    /// Get refill rate (tokens per second)
    fn refill_rate(&self) -> f64 {
        self.requests_per_minute as f64 / 60.0
    }
}

/// Rate limiter using token bucket algorithm with Redis backend
pub struct RateLimiter {
    redis: MultiplexedConnection,
    window_seconds: u64,
}

impl RateLimiter {
    /// Create new rate limiter
    pub fn new(redis: MultiplexedConnection) -> Self {
        Self {
            redis,
            window_seconds: 60,
        }
    }

    /// Check rate limit and consume tokens
    pub async fn check_rate_limit(
        &mut self,
        key: &str,
        config: RateLimitConfig,
    ) -> Result<RateLimitState, RateLimitError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Lua script for atomic token bucket implementation
        let lua_script = r#"
            local key = KEYS[1]
            local capacity = tonumber(ARGV[1])
            local refill_rate = tonumber(ARGV[2])
            local requested = tonumber(ARGV[3])
            local now = tonumber(ARGV[4])
            local window = tonumber(ARGV[5])

            -- Get current bucket state
            local bucket = redis.call('HMGET', key, 'tokens', 'last_refill')
            local tokens = tonumber(bucket[1])
            local last_refill = tonumber(bucket[2])

            -- Initialize if doesn't exist
            if tokens == nil then
                tokens = capacity
                last_refill = now
            end

            -- Calculate tokens to add based on time elapsed
            local time_elapsed = now - last_refill
            local tokens_to_add = time_elapsed * refill_rate
            tokens = math.min(capacity, tokens + tokens_to_add)

            -- Check if we have enough tokens
            if tokens >= requested then
                tokens = tokens - requested
                redis.call('HMSET', key, 'tokens', tokens, 'last_refill', now)
                redis.call('EXPIRE', key, window)
                return {1, tokens, capacity, now + window}
            else
                -- Not enough tokens, return current state without consuming
                return {0, tokens, capacity, now + window}
            end
        "#;

        let result: Vec<i64> = redis::Script::new(lua_script)
            .key(key)
            .arg(config.burst_capacity)
            .arg(config.refill_rate())
            .arg(1) // Request 1 token
            .arg(now)
            .arg(self.window_seconds)
            .invoke_async(&mut self.redis)
            .await
            .map_err(|e| {
                error!("Redis rate limit error: {}", e);
                RateLimitError::Internal(format!("Rate limit check failed: {}", e))
            })?;

        let allowed = result[0] == 1;
        let remaining = result[1] as u32;
        let limit = result[2] as u32;
        let reset_at = result[3] as u64;

        if allowed {
            Ok(RateLimitState {
                allowed: true,
                limit,
                remaining,
                reset_at,
            })
        } else {
            warn!("Rate limit exceeded for key: {}", key);
            Ok(RateLimitState {
                allowed: false,
                limit,
                remaining: 0,
                reset_at,
            })
        }
    }

    /// Get rate limit key for a user
    fn rate_limit_key(user_id: &str, endpoint: &str) -> String {
        format!("ratelimit:{}:{}", user_id, endpoint)
    }
}

/// Rate limit state
#[derive(Debug, Clone)]
pub struct RateLimitState {
    /// Whether the request is allowed
    pub allowed: bool,
    /// Total limit
    pub limit: u32,
    /// Remaining requests
    pub remaining: u32,
    /// Unix timestamp when the limit resets
    pub reset_at: u64,
}

impl RateLimitState {
    /// Add rate limit headers to response
    pub fn add_headers(&self, response: &mut Response<Body>) {
        let headers = response.headers_mut();

        headers.insert(
            "X-RateLimit-Limit",
            HeaderValue::from_str(&self.limit.to_string()).unwrap(),
        );

        headers.insert(
            "X-RateLimit-Remaining",
            HeaderValue::from_str(&self.remaining.to_string()).unwrap(),
        );

        headers.insert(
            "X-RateLimit-Reset",
            HeaderValue::from_str(&self.reset_at.to_string()).unwrap(),
        );

        // Add Retry-After header if rate limited
        if !self.allowed {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let retry_after = self.reset_at.saturating_sub(now);

            headers.insert(
                "Retry-After",
                HeaderValue::from_str(&retry_after.to_string()).unwrap(),
            );
        }
    }
}

/// Rate limit errors
#[derive(Debug, thiserror::Error)]
pub enum RateLimitError {
    #[error("Rate limit exceeded")]
    Exceeded,

    #[error("Internal rate limit error: {0}")]
    Internal(String),
}

impl IntoResponse for RateLimitError {
    fn into_response(self) -> Response {
        let (status, error_code, message) = match self {
            RateLimitError::Exceeded => (
                StatusCode::TOO_MANY_REQUESTS,
                "RATE_LIMIT_EXCEEDED",
                "Too many requests. Please slow down.",
            ),
            RateLimitError::Internal(ref msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                msg.as_str(),
            ),
        };

        let body = Json(json!({
            "error": {
                "code": error_code,
                "message": message,
            }
        }));

        (status, body).into_response()
    }
}

/// Rate limiting middleware
pub async fn rate_limit_middleware(
    auth: AuthContext,
    State(redis_client): State<Arc<redis::Client>>,
    mut req: Request,
    next: Next,
) -> Result<Response, RateLimitError> {
    // Get rate limit configuration for user's role
    let config = RateLimitConfig::for_role(&auth.role);

    // Extract endpoint path for rate limit key
    let endpoint = req.uri().path().to_string();

    // Create rate limit key
    let key = RateLimiter::rate_limit_key(&auth.user_id, &endpoint);

    // Get Redis connection
    let redis_conn: MultiplexedConnection = redis_client
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| {
        error!("Failed to get Redis connection: {}", e);
        RateLimitError::Internal("Rate limit service unavailable".to_string())
    })?;

    // Create rate limiter
    let mut limiter = RateLimiter::new(redis_conn);

    // Check rate limit
    let state = limiter.check_rate_limit(&key, config).await?;

    if !state.allowed {
        info!(
            user_id = %auth.user_id,
            role = ?auth.role,
            endpoint = %endpoint,
            "Rate limit exceeded"
        );

        // Return rate limit error with headers
        let mut response = RateLimitError::Exceeded.into_response();
        state.add_headers(&mut response);
        return Ok(response);
    }

    info!(
        user_id = %auth.user_id,
        remaining = state.remaining,
        limit = state.limit,
        "Rate limit check passed"
    );

    // Continue with request
    let mut response = next.run(req).await;

    // Add rate limit headers to successful response
    state.add_headers(&mut response);

    Ok(response)
}

/// Rate limit layer for Axum
#[derive(Clone)]
pub struct RateLimitLayer {
    redis_client: Arc<redis::Client>,
}

impl RateLimitLayer {
    /// Create new rate limit layer
    pub fn new(redis_client: redis::Client) -> Self {
        Self {
            redis_client: Arc::new(redis_client),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_config_for_roles() {
        let admin_config = RateLimitConfig::for_role(&Role::Admin);
        assert_eq!(admin_config.requests_per_minute, 100_000);
        assert_eq!(admin_config.burst_capacity, 120_000);

        let dev_config = RateLimitConfig::for_role(&Role::Developer);
        assert_eq!(dev_config.requests_per_minute, 10_000);
        assert_eq!(dev_config.burst_capacity, 12_000);

        let viewer_config = RateLimitConfig::for_role(&Role::Viewer);
        assert_eq!(viewer_config.requests_per_minute, 1_000);
        assert_eq!(viewer_config.burst_capacity, 1_200);
    }

    #[test]
    fn test_rate_limit_config_refill_rate() {
        let config = RateLimitConfig {
            requests_per_minute: 60,
            burst_capacity: 72,
        };

        // Should be 1 token per second (60/60)
        assert_eq!(config.refill_rate(), 1.0);

        let fast_config = RateLimitConfig {
            requests_per_minute: 600,
            burst_capacity: 720,
        };

        // Should be 10 tokens per second (600/60)
        assert_eq!(fast_config.refill_rate(), 10.0);
    }

    #[test]
    fn test_rate_limit_key_generation() {
        let key = RateLimiter::rate_limit_key("user123", "/api/v1/traces");
        assert_eq!(key, "ratelimit:user123:/api/v1/traces");
    }

    #[test]
    fn test_rate_limit_state() {
        let state = RateLimitState {
            allowed: true,
            limit: 100,
            remaining: 95,
            reset_at: 1699200000,
        };

        assert!(state.allowed);
        assert_eq!(state.limit, 100);
        assert_eq!(state.remaining, 95);
    }
}
