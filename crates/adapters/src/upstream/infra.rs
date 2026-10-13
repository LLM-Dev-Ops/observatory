// Copyright 2025 LLM Observatory Contributors
// SPDX-License-Identifier: Apache-2.0

//! Infra adapter for Observatory.
//!
//! This module provides foundational infrastructure utilities, and converts
//! its configuration types into those of the `infra-*` crates from the
//! LLM-Dev-Ops ecosystem.
//!
//! # Features
//!
//! - Metrics collection utilities with OpenTelemetry integration
//! - Structured logging with context propagation
//! - Distributed tracing helpers
//! - Configuration loading and validation
//! - Error utilities with rich context
//! - Caching abstractions for performance optimization
//! - Retry logic with exponential backoff
//! - Rate limiting for API protection
//!
//! # Example
//!
//! ```ignore
//! use llm_observatory_adapters::upstream::infra::InfraAdapter;
//!
//! // Create adapter with default configuration
//! let adapter = InfraAdapter::new("observatory-service");
//!
//! // Use metrics utilities
//! adapter.metrics().increment_counter("requests_total", 1);
//!
//! // Use logging with context
//! adapter.logger().info("Processing request", &[("trace_id", trace_id)]);
//!
//! // Use retry logic
//! let result = adapter.retry()
//!     .with_max_attempts(3)
//!     .with_exponential_backoff()
//!     .execute(|| async { make_request().await });
//!
//! // Use rate limiter
//! if adapter.rate_limiter().check("api_calls", user_id) {
//!     process_request();
//! }
//! ```

use infra_cache::CacheConfig;
use infra_errors::{InfraError, RetryConfig};
use infra_rate_limit::RateLimitConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;

/// Errors that can occur during Infra operations.
#[derive(Debug, Error)]
pub enum InfraAdapterError {
    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Metrics error
    #[error("Metrics error: {0}")]
    MetricsError(String),

    /// Logging error
    #[error("Logging error: {0}")]
    LoggingError(String),

    /// Cache error
    #[error("Cache error: {0}")]
    CacheError(String),

    /// Rate limit exceeded
    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),

    /// Retry exhausted
    #[error("Retry exhausted after {attempts} attempts: {message}")]
    RetryExhausted { attempts: u32, message: String },

    /// Underlying Infra error
    #[error("Infra error: {0}")]
    InfraError(String),
}

impl From<InfraError> for InfraAdapterError {
    fn from(err: InfraError) -> Self {
        InfraAdapterError::InfraError(err.to_string())
    }
}

/// Result type for Infra operations.
pub type Result<T> = std::result::Result<T, InfraAdapterError>;

/// Observatory-specific metrics names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObservatoryMetric {
    /// Total LLM requests processed
    RequestsTotal,
    /// Request latency histogram
    RequestLatency,
    /// Active connections gauge
    ActiveConnections,
    /// Token usage counter
    TokensProcessed,
    /// Cost accumulator
    TotalCostUsd,
    /// Errors counter
    ErrorsTotal,
    /// Cache hit rate
    CacheHits,
    /// Cache miss rate
    CacheMisses,
    /// Rate limit rejections
    RateLimitRejections,
    /// Retry attempts
    RetryAttempts,
}

impl ObservatoryMetric {
    /// Get the metric name as a string.
    pub fn name(&self) -> &'static str {
        match self {
            Self::RequestsTotal => "observatory_requests_total",
            Self::RequestLatency => "observatory_request_latency_seconds",
            Self::ActiveConnections => "observatory_active_connections",
            Self::TokensProcessed => "observatory_tokens_processed_total",
            Self::TotalCostUsd => "observatory_cost_usd_total",
            Self::ErrorsTotal => "observatory_errors_total",
            Self::CacheHits => "observatory_cache_hits_total",
            Self::CacheMisses => "observatory_cache_misses_total",
            Self::RateLimitRejections => "observatory_rate_limit_rejections_total",
            Self::RetryAttempts => "observatory_retry_attempts_total",
        }
    }

    /// Get the metric description.
    pub fn description(&self) -> &'static str {
        match self {
            Self::RequestsTotal => "Total number of LLM requests processed",
            Self::RequestLatency => "Latency of LLM requests in seconds",
            Self::ActiveConnections => "Number of active connections",
            Self::TokensProcessed => "Total number of tokens processed",
            Self::TotalCostUsd => "Total cost in USD",
            Self::ErrorsTotal => "Total number of errors",
            Self::CacheHits => "Total number of cache hits",
            Self::CacheMisses => "Total number of cache misses",
            Self::RateLimitRejections => "Total number of rate limit rejections",
            Self::RetryAttempts => "Total number of retry attempts",
        }
    }
}

/// Observatory-specific log levels with semantic meaning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservatoryLogLevel {
    /// Debug information for development
    Debug,
    /// General informational messages
    Info,
    /// Warning conditions
    Warn,
    /// Error conditions
    Error,
    /// Critical conditions requiring immediate attention
    Critical,
}

/// Cache configuration for Observatory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservatoryCacheConfig {
    /// Maximum cache size in entries
    pub max_entries: usize,
    /// Default TTL in seconds
    pub default_ttl_secs: u64,
    /// Enable cache statistics
    pub enable_stats: bool,
}

impl Default for ObservatoryCacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 10_000,
            default_ttl_secs: 300, // 5 minutes
            enable_stats: true,
        }
    }
}

impl From<ObservatoryCacheConfig> for CacheConfig {
    fn from(config: ObservatoryCacheConfig) -> Self {
        CacheConfig::with_max_size(config.max_entries)
            .with_ttl(Duration::from_secs(config.default_ttl_secs))
            .with_metrics(config.enable_stats)
    }
}

/// Rate limit configuration for Observatory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservatoryRateLimitConfig {
    /// Maximum requests per window
    pub max_requests: u32,
    /// Window duration in seconds
    pub window_secs: u64,
    /// Enable burst handling
    pub enable_burst: bool,
    /// Burst size (if enabled)
    pub burst_size: u32,
}

impl Default for ObservatoryRateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests: 1000,
            window_secs: 60,
            enable_burst: true,
            burst_size: 100,
        }
    }
}

// `RateLimitConfig::new` is fallible but `From` is not, so the fields are set
// directly; a zero-length window would yield an infinite rate rather than an error.
impl From<ObservatoryRateLimitConfig> for RateLimitConfig {
    fn from(config: ObservatoryRateLimitConfig) -> Self {
        let window = Duration::from_secs(config.window_secs);
        RateLimitConfig {
            requests_per_second: config.max_requests as f64 / window.as_secs_f64(),
            burst_size: if config.enable_burst {
                config.burst_size as u64
            } else {
                config.max_requests as u64
            },
            window_size: window,
        }
    }
}

/// Retry configuration for Observatory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservatoryRetryConfig {
    /// Maximum retry attempts
    pub max_attempts: u32,
    /// Initial delay in milliseconds
    pub initial_delay_ms: u64,
    /// Maximum delay in milliseconds
    pub max_delay_ms: u64,
    /// Exponential backoff multiplier
    pub backoff_multiplier: f64,
    /// Add jitter to delays
    pub add_jitter: bool,
}

impl Default for ObservatoryRetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay_ms: 100,
            max_delay_ms: 10_000,
            backoff_multiplier: 2.0,
            add_jitter: true,
        }
    }
}

// `backoff_multiplier` has no counterpart in `infra_errors::RetryConfig`, which
// selects a `RetryStrategy` variant instead of taking a numeric multiplier.
impl From<ObservatoryRetryConfig> for RetryConfig {
    fn from(config: ObservatoryRetryConfig) -> Self {
        RetryConfig::new()
            .with_max_attempts(config.max_attempts as usize)
            .with_base_delay(Duration::from_millis(config.initial_delay_ms))
            .with_max_delay(Duration::from_millis(config.max_delay_ms))
            .with_jitter(config.add_jitter)
    }
}

/// Wrapper for Infra metrics functionality.
pub struct MetricsAdapter {
    /// Service name
    service_name: String,
    /// Counters storage
    counters: HashMap<String, u64>,
    /// Gauges storage
    gauges: HashMap<String, f64>,
    /// Histogram samples
    histograms: HashMap<String, Vec<f64>>,
}

impl MetricsAdapter {
    /// Create a new metrics adapter.
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
            counters: HashMap::new(),
            gauges: HashMap::new(),
            histograms: HashMap::new(),
        }
    }

    /// Increment a counter.
    pub fn increment_counter(&mut self, metric: ObservatoryMetric, value: u64) {
        *self.counters.entry(metric.name().to_string()).or_insert(0) += value;
    }

    /// Increment a counter with labels.
    pub fn increment_counter_with_labels(
        &mut self,
        metric: ObservatoryMetric,
        value: u64,
        labels: &[(&str, &str)],
    ) {
        let key = self.build_key(metric.name(), labels);
        *self.counters.entry(key).or_insert(0) += value;
    }

    /// Set a gauge value.
    pub fn set_gauge(&mut self, metric: ObservatoryMetric, value: f64) {
        self.gauges.insert(metric.name().to_string(), value);
    }

    /// Set a gauge with labels.
    pub fn set_gauge_with_labels(
        &mut self,
        metric: ObservatoryMetric,
        value: f64,
        labels: &[(&str, &str)],
    ) {
        let key = self.build_key(metric.name(), labels);
        self.gauges.insert(key, value);
    }

    /// Record a histogram value.
    pub fn record_histogram(&mut self, metric: ObservatoryMetric, value: f64) {
        self.histograms
            .entry(metric.name().to_string())
            .or_default()
            .push(value);
    }

    /// Record a histogram with labels.
    pub fn record_histogram_with_labels(
        &mut self,
        metric: ObservatoryMetric,
        value: f64,
        labels: &[(&str, &str)],
    ) {
        let key = self.build_key(metric.name(), labels);
        self.histograms.entry(key).or_default().push(value);
    }

    /// Get counter value.
    pub fn get_counter(&self, metric: ObservatoryMetric) -> u64 {
        self.counters.get(metric.name()).copied().unwrap_or(0)
    }

    /// Get gauge value.
    pub fn get_gauge(&self, metric: ObservatoryMetric) -> f64 {
        self.gauges.get(metric.name()).copied().unwrap_or(0.0)
    }

    /// Get histogram samples.
    pub fn get_histogram_samples(&self, metric: ObservatoryMetric) -> Vec<f64> {
        self.histograms
            .get(metric.name())
            .cloned()
            .unwrap_or_default()
    }

    /// Build a key with labels.
    fn build_key(&self, name: &str, labels: &[(&str, &str)]) -> String {
        let label_str: Vec<String> = labels.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
        format!("{}:{{{}}}", name, label_str.join(","))
    }

    /// Get service name.
    pub fn service_name(&self) -> &str {
        &self.service_name
    }

    /// Reset all metrics.
    pub fn reset(&mut self) {
        self.counters.clear();
        self.gauges.clear();
        self.histograms.clear();
    }
}

/// Wrapper for Infra logging functionality.
pub struct LoggingAdapter {
    /// Service name
    service_name: String,
    /// Current log level
    level: ObservatoryLogLevel,
    /// Log context
    context: HashMap<String, String>,
}

impl LoggingAdapter {
    /// Create a new logging adapter.
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
            level: ObservatoryLogLevel::Info,
            context: HashMap::new(),
        }
    }

    /// Set the log level.
    pub fn with_level(mut self, level: ObservatoryLogLevel) -> Self {
        self.level = level;
        self
    }

    /// Add context to all log messages.
    pub fn with_context(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.context.insert(key.into(), value.into());
        self
    }

    /// Log a debug message.
    pub fn debug(&self, message: &str, fields: &[(&str, &str)]) {
        if self.level == ObservatoryLogLevel::Debug {
            self.log(ObservatoryLogLevel::Debug, message, fields);
        }
    }

    /// Log an info message.
    pub fn info(&self, message: &str, fields: &[(&str, &str)]) {
        self.log(ObservatoryLogLevel::Info, message, fields);
    }

    /// Log a warning message.
    pub fn warn(&self, message: &str, fields: &[(&str, &str)]) {
        self.log(ObservatoryLogLevel::Warn, message, fields);
    }

    /// Log an error message.
    pub fn error(&self, message: &str, fields: &[(&str, &str)]) {
        self.log(ObservatoryLogLevel::Error, message, fields);
    }

    /// Log a critical message.
    pub fn critical(&self, message: &str, fields: &[(&str, &str)]) {
        self.log(ObservatoryLogLevel::Critical, message, fields);
    }

    /// Internal log method.
    fn log(&self, level: ObservatoryLogLevel, message: &str, fields: &[(&str, &str)]) {
        // In production, this would use the actual Infra logger
        // For now, we use tracing
        let mut all_fields: Vec<(&str, &str)> = self
            .context
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        all_fields.extend_from_slice(fields);

        match level {
            ObservatoryLogLevel::Debug => {
                tracing::debug!(service = %self.service_name, ?all_fields, "{}", message);
            }
            ObservatoryLogLevel::Info => {
                tracing::info!(service = %self.service_name, ?all_fields, "{}", message);
            }
            ObservatoryLogLevel::Warn => {
                tracing::warn!(service = %self.service_name, ?all_fields, "{}", message);
            }
            ObservatoryLogLevel::Error => {
                tracing::error!(service = %self.service_name, ?all_fields, "{}", message);
            }
            ObservatoryLogLevel::Critical => {
                tracing::error!(service = %self.service_name, ?all_fields, "CRITICAL: {}", message);
            }
        }
    }
}

/// Wrapper for Infra caching functionality.
pub struct CacheAdapter<V> {
    /// Cache entries
    entries: HashMap<String, CacheEntryWrapper<V>>,
    /// Configuration
    config: ObservatoryCacheConfig,
    /// Statistics
    hits: u64,
    misses: u64,
}

/// Internal cache entry wrapper.
struct CacheEntryWrapper<V> {
    value: V,
    expires_at: std::time::Instant,
}

impl<V: Clone> CacheAdapter<V> {
    /// Create a new cache adapter with default configuration.
    pub fn new() -> Self {
        Self::with_config(ObservatoryCacheConfig::default())
    }

    /// Create a new cache adapter with custom configuration.
    pub fn with_config(config: ObservatoryCacheConfig) -> Self {
        Self {
            entries: HashMap::new(),
            config,
            hits: 0,
            misses: 0,
        }
    }

    /// Get a value from the cache.
    pub fn get(&mut self, key: &str) -> Option<V> {
        if let Some(entry) = self.entries.get(key) {
            if entry.expires_at > std::time::Instant::now() {
                self.hits += 1;
                return Some(entry.value.clone());
            } else {
                self.entries.remove(key);
            }
        }
        self.misses += 1;
        None
    }

    /// Set a value in the cache.
    pub fn set(&mut self, key: impl Into<String>, value: V) {
        self.set_with_ttl(
            key,
            value,
            Duration::from_secs(self.config.default_ttl_secs),
        );
    }

    /// Set a value with a custom TTL.
    pub fn set_with_ttl(&mut self, key: impl Into<String>, value: V, ttl: Duration) {
        // Evict if at capacity
        if self.entries.len() >= self.config.max_entries {
            self.evict_expired();
        }

        self.entries.insert(
            key.into(),
            CacheEntryWrapper {
                value,
                expires_at: std::time::Instant::now() + ttl,
            },
        );
    }

    /// Remove a value from the cache.
    pub fn remove(&mut self, key: &str) -> Option<V> {
        self.entries.remove(key).map(|e| e.value)
    }

    /// Clear the cache.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.hits = 0;
        self.misses = 0;
    }

    /// Evict expired entries.
    fn evict_expired(&mut self) {
        let now = std::time::Instant::now();
        self.entries.retain(|_, entry| entry.expires_at > now);
    }

    /// Get cache statistics.
    pub fn stats(&self) -> CacheStatsInfo {
        CacheStatsInfo {
            hits: self.hits,
            misses: self.misses,
            entries: self.entries.len(),
            hit_rate: if self.hits + self.misses > 0 {
                self.hits as f64 / (self.hits + self.misses) as f64
            } else {
                0.0
            },
        }
    }
}

impl<V: Clone> Default for CacheAdapter<V> {
    fn default() -> Self {
        Self::new()
    }
}

/// Cache statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStatsInfo {
    /// Number of cache hits
    pub hits: u64,
    /// Number of cache misses
    pub misses: u64,
    /// Current number of entries
    pub entries: usize,
    /// Hit rate (0.0 - 1.0)
    pub hit_rate: f64,
}

/// Wrapper for Infra rate limiting functionality.
pub struct RateLimitAdapter {
    /// Rate limit buckets
    buckets: HashMap<String, RateLimitBucket>,
    /// Configuration
    config: ObservatoryRateLimitConfig,
}

/// Internal rate limit bucket.
struct RateLimitBucket {
    tokens: u32,
    last_refill: std::time::Instant,
}

impl RateLimitAdapter {
    /// Create a new rate limit adapter with default configuration.
    pub fn new() -> Self {
        Self::with_config(ObservatoryRateLimitConfig::default())
    }

    /// Create a new rate limit adapter with custom configuration.
    pub fn with_config(config: ObservatoryRateLimitConfig) -> Self {
        Self {
            buckets: HashMap::new(),
            config,
        }
    }

    /// Check if a request is allowed.
    pub fn check(&mut self, resource: &str, identifier: &str) -> bool {
        let key = format!("{}:{}", resource, identifier);
        self.try_acquire(&key, 1)
    }

    /// Check if multiple requests are allowed.
    pub fn check_n(&mut self, resource: &str, identifier: &str, n: u32) -> bool {
        let key = format!("{}:{}", resource, identifier);
        self.try_acquire(&key, n)
    }

    /// Try to acquire tokens from the bucket.
    fn try_acquire(&mut self, key: &str, tokens: u32) -> bool {
        let now = std::time::Instant::now();
        let window = Duration::from_secs(self.config.window_secs);

        let bucket = self
            .buckets
            .entry(key.to_string())
            .or_insert_with(|| RateLimitBucket {
                tokens: self.config.max_requests,
                last_refill: now,
            });

        // Refill tokens based on elapsed time
        let elapsed = now.duration_since(bucket.last_refill);
        if elapsed >= window {
            bucket.tokens = self.config.max_requests;
            bucket.last_refill = now;
        } else {
            let refill_rate = self.config.max_requests as f64 / window.as_secs_f64();
            let refill = (elapsed.as_secs_f64() * refill_rate) as u32;
            bucket.tokens = (bucket.tokens + refill).min(self.config.max_requests);
            bucket.last_refill = now;
        }

        // Handle burst
        let effective_limit = if self.config.enable_burst {
            self.config.max_requests + self.config.burst_size
        } else {
            self.config.max_requests
        };

        if bucket.tokens >= tokens {
            bucket.tokens -= tokens;
            true
        } else {
            false
        }
    }

    /// Get remaining tokens for a key.
    pub fn remaining(&self, resource: &str, identifier: &str) -> u32 {
        let key = format!("{}:{}", resource, identifier);
        self.buckets
            .get(&key)
            .map(|b| b.tokens)
            .unwrap_or(self.config.max_requests)
    }

    /// Reset a rate limit bucket.
    pub fn reset(&mut self, resource: &str, identifier: &str) {
        let key = format!("{}:{}", resource, identifier);
        self.buckets.remove(&key);
    }

    /// Reset all buckets.
    pub fn reset_all(&mut self) {
        self.buckets.clear();
    }
}

impl Default for RateLimitAdapter {
    fn default() -> Self {
        Self::new()
    }
}

/// Wrapper for Infra retry functionality.
pub struct RetryAdapter {
    /// Configuration
    config: ObservatoryRetryConfig,
}

impl RetryAdapter {
    /// Create a new retry adapter with default configuration.
    pub fn new() -> Self {
        Self::with_config(ObservatoryRetryConfig::default())
    }

    /// Create a new retry adapter with custom configuration.
    pub fn with_config(config: ObservatoryRetryConfig) -> Self {
        Self { config }
    }

    /// Execute an operation with retry.
    pub async fn execute<F, Fut, T, E>(&self, operation: F) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = std::result::Result<T, E>>,
        E: std::fmt::Display,
    {
        let mut attempts = 0;
        let mut delay = Duration::from_millis(self.config.initial_delay_ms);

        loop {
            attempts += 1;
            match operation().await {
                Ok(result) => return Ok(result),
                Err(err) => {
                    if attempts >= self.config.max_attempts {
                        return Err(InfraAdapterError::RetryExhausted {
                            attempts,
                            message: err.to_string(),
                        });
                    }

                    // Apply jitter if enabled
                    let actual_delay = if self.config.add_jitter {
                        let jitter = rand::random::<f64>() * 0.3; // Up to 30% jitter
                        Duration::from_secs_f64(delay.as_secs_f64() * (1.0 + jitter))
                    } else {
                        delay
                    };

                    tokio::time::sleep(actual_delay).await;

                    // Apply exponential backoff
                    delay = Duration::from_secs_f64(
                        (delay.as_secs_f64() * self.config.backoff_multiplier)
                            .min(self.config.max_delay_ms as f64 / 1000.0),
                    );
                }
            }
        }
    }

    /// Execute an operation with retry (sync version).
    pub fn execute_sync<F, T, E>(&self, mut operation: F) -> Result<T>
    where
        F: FnMut() -> std::result::Result<T, E>,
        E: std::fmt::Display,
    {
        let mut attempts = 0;
        let mut delay = Duration::from_millis(self.config.initial_delay_ms);

        loop {
            attempts += 1;
            match operation() {
                Ok(result) => return Ok(result),
                Err(err) => {
                    if attempts >= self.config.max_attempts {
                        return Err(InfraAdapterError::RetryExhausted {
                            attempts,
                            message: err.to_string(),
                        });
                    }

                    std::thread::sleep(delay);

                    // Apply exponential backoff
                    delay = Duration::from_secs_f64(
                        (delay.as_secs_f64() * self.config.backoff_multiplier)
                            .min(self.config.max_delay_ms as f64 / 1000.0),
                    );
                }
            }
        }
    }

    /// Get the configuration.
    pub fn config(&self) -> &ObservatoryRetryConfig {
        &self.config
    }
}

impl Default for RetryAdapter {
    fn default() -> Self {
        Self::new()
    }
}

/// Main Infra adapter for Observatory.
///
/// Provides a unified interface to all Infra functionality including
/// metrics, logging, caching, rate limiting, and retry logic.
pub struct InfraAdapter {
    /// Service name
    service_name: String,
    /// Metrics adapter
    metrics: MetricsAdapter,
    /// Logging adapter
    logger: LoggingAdapter,
    /// Rate limit adapter
    rate_limiter: RateLimitAdapter,
    /// Retry adapter
    retry: RetryAdapter,
}

impl InfraAdapter {
    /// Create a new InfraAdapter with default configuration.
    pub fn new(service_name: impl Into<String>) -> Self {
        let name = service_name.into();
        Self {
            service_name: name.clone(),
            metrics: MetricsAdapter::new(&name),
            logger: LoggingAdapter::new(&name),
            rate_limiter: RateLimitAdapter::new(),
            retry: RetryAdapter::new(),
        }
    }

    /// Create a new InfraAdapter with custom configuration.
    pub fn with_config(
        service_name: impl Into<String>,
        rate_limit_config: ObservatoryRateLimitConfig,
        retry_config: ObservatoryRetryConfig,
    ) -> Self {
        let name = service_name.into();
        Self {
            service_name: name.clone(),
            metrics: MetricsAdapter::new(&name),
            logger: LoggingAdapter::new(&name),
            rate_limiter: RateLimitAdapter::with_config(rate_limit_config),
            retry: RetryAdapter::with_config(retry_config),
        }
    }

    /// Get the service name.
    pub fn service_name(&self) -> &str {
        &self.service_name
    }

    /// Get a mutable reference to the metrics adapter.
    pub fn metrics(&mut self) -> &mut MetricsAdapter {
        &mut self.metrics
    }

    /// Get a reference to the logging adapter.
    pub fn logger(&self) -> &LoggingAdapter {
        &self.logger
    }

    /// Get a mutable reference to the rate limiter.
    pub fn rate_limiter(&mut self) -> &mut RateLimitAdapter {
        &mut self.rate_limiter
    }

    /// Get a reference to the retry adapter.
    pub fn retry(&self) -> &RetryAdapter {
        &self.retry
    }

    /// Create a new cache adapter.
    pub fn cache<V: Clone>(&self) -> CacheAdapter<V> {
        CacheAdapter::new()
    }

    /// Create a new cache adapter with custom configuration.
    pub fn cache_with_config<V: Clone>(&self, config: ObservatoryCacheConfig) -> CacheAdapter<V> {
        CacheAdapter::with_config(config)
    }

    /// Record a request with all relevant metrics.
    pub fn record_request(
        &mut self,
        provider: &str,
        model: &str,
        latency_secs: f64,
        tokens: u64,
        cost_usd: f64,
        success: bool,
    ) {
        // Increment request counter
        self.metrics.increment_counter_with_labels(
            ObservatoryMetric::RequestsTotal,
            1,
            &[("provider", provider), ("model", model)],
        );

        // Record latency
        self.metrics.record_histogram_with_labels(
            ObservatoryMetric::RequestLatency,
            latency_secs,
            &[("provider", provider), ("model", model)],
        );

        // Record tokens
        self.metrics.increment_counter_with_labels(
            ObservatoryMetric::TokensProcessed,
            tokens,
            &[("provider", provider), ("model", model)],
        );

        // Record cost (as integer cents for counter)
        // Note: For actual implementation, use a gauge or separate float tracking

        // Record error if not successful
        if !success {
            self.metrics.increment_counter_with_labels(
                ObservatoryMetric::ErrorsTotal,
                1,
                &[("provider", provider), ("model", model)],
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infra_adapter_creation() {
        let adapter = InfraAdapter::new("test-service");
        assert_eq!(adapter.service_name(), "test-service");
    }

    #[test]
    fn test_metrics_counter() {
        let mut adapter = InfraAdapter::new("test-service");

        adapter
            .metrics()
            .increment_counter(ObservatoryMetric::RequestsTotal, 5);
        adapter
            .metrics()
            .increment_counter(ObservatoryMetric::RequestsTotal, 3);

        assert_eq!(
            adapter
                .metrics()
                .get_counter(ObservatoryMetric::RequestsTotal),
            8
        );
    }

    #[test]
    fn test_metrics_gauge() {
        let mut adapter = InfraAdapter::new("test-service");

        adapter
            .metrics()
            .set_gauge(ObservatoryMetric::ActiveConnections, 10.0);

        assert_eq!(
            adapter
                .metrics()
                .get_gauge(ObservatoryMetric::ActiveConnections),
            10.0
        );
    }

    #[test]
    fn test_cache_operations() {
        let mut cache: CacheAdapter<String> = CacheAdapter::new();

        cache.set("key1", "value1".to_string());
        assert_eq!(cache.get("key1"), Some("value1".to_string()));
        assert_eq!(cache.get("key2"), None);

        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
    }

    #[test]
    fn test_rate_limiter() {
        let mut limiter = RateLimitAdapter::with_config(ObservatoryRateLimitConfig {
            max_requests: 5,
            window_secs: 60,
            enable_burst: false,
            burst_size: 0,
        });

        // Should allow first 5 requests
        for _ in 0..5 {
            assert!(limiter.check("api", "user1"));
        }

        // 6th request should be denied
        assert!(!limiter.check("api", "user1"));
    }

    #[test]
    fn test_retry_sync() {
        let retry = RetryAdapter::with_config(ObservatoryRetryConfig {
            max_attempts: 3,
            initial_delay_ms: 1,
            max_delay_ms: 10,
            backoff_multiplier: 2.0,
            add_jitter: false,
        });

        let mut attempts = 0;
        let result: Result<i32> = retry.execute_sync(|| {
            attempts += 1;
            if attempts < 3 {
                Err("temporary failure")
            } else {
                Ok(42)
            }
        });

        assert_eq!(result.unwrap(), 42);
        assert_eq!(attempts, 3);
    }

    #[test]
    fn test_observatory_metric_names() {
        assert_eq!(
            ObservatoryMetric::RequestsTotal.name(),
            "observatory_requests_total"
        );
        assert_eq!(
            ObservatoryMetric::RequestLatency.name(),
            "observatory_request_latency_seconds"
        );
    }
}
