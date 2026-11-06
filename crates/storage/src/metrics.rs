//! Prometheus metrics for storage layer monitoring.
//!
//! This module provides comprehensive metrics for monitoring storage operations:
//! - Write performance (duration, throughput, errors)
//! - Query performance (duration by repository and method)
//! - Connection pool health (active/idle/max connections)
//! - Batch operations (sizes, COPY vs INSERT comparison)
//! - Error tracking (by error type)
//!
//! # Usage
//!
//! ```no_run
//! use llm_observatory_storage::metrics::StorageMetrics;
//!
//! // Initialize metrics (call once at startup)
//! let metrics = StorageMetrics::new();
//!
//! // Record a write operation
//! metrics.record_write("trace", "insert", true, 0.025);
//!
//! // Record a query operation
//! metrics.record_query("trace_repository", "get_by_id", 0.003);
//! ```

use metrics::{counter, describe_counter, describe_gauge, describe_histogram, gauge, histogram};
use std::sync::Arc;
use std::time::Instant;

/// Storage layer metrics collector.
///
/// This struct provides methods to record various storage metrics.
/// All metrics are registered with Prometheus and can be scraped via /metrics endpoint.
#[derive(Clone)]
pub struct StorageMetrics {
    _private: (),
}

impl StorageMetrics {
    /// Create a new metrics collector and register all metrics.
    ///
    /// This should be called once at application startup.
    pub fn new() -> Self {
        Self::register_metrics();
        Self { _private: () }
    }

    /// Register all Prometheus metrics with descriptions.
    fn register_metrics() {
        // Write duration histogram
        describe_histogram!(
            "storage_write_duration_seconds",
            "Duration of storage write operations in seconds"
        );

        // Write counter
        describe_counter!(
            "storage_writes_total",
            "Total number of storage write operations"
        );

        // Pool connection gauges
        describe_gauge!(
            "storage_pool_connections",
            "Number of database connections in various states"
        );

        // Query duration histogram
        describe_histogram!(
            "storage_query_duration_seconds",
            "Duration of storage query operations in seconds"
        );

        // Error counter
        describe_counter!(
            "storage_errors_total",
            "Total number of storage errors by type"
        );

        // Batch size histogram
        describe_histogram!(
            "storage_batch_size",
            "Size of batch operations (number of items)"
        );

        // Buffer size gauge
        describe_gauge!(
            "storage_buffer_size",
            "Current size of write buffers by writer type"
        );

        // Flush counter
        describe_counter!(
            "storage_flushes_total",
            "Total number of buffer flush operations"
        );

        // Retry counter
        describe_counter!("storage_retries_total", "Total number of retry attempts");

        // Items written counter
        describe_counter!(
            "storage_items_written_total",
            "Total number of items written to storage"
        );

        // Query result count histogram
        describe_histogram!(
            "storage_query_result_count",
            "Number of results returned by query operations"
        );

        // Connection acquisition duration
        describe_histogram!(
            "storage_connection_acquire_duration_seconds",
            "Time taken to acquire a connection from the pool"
        );
    }

    /// Record a write operation.
    ///
    /// # Arguments
    ///
    /// * `writer_type` - Type of writer (trace, metric, log)
    /// * `operation` - Operation type (insert, copy, flush)
    /// * `success` - Whether the operation succeeded
    /// * `duration_secs` - Duration in seconds
    pub fn record_write(
        &self,
        writer_type: &str,
        operation: &str,
        success: bool,
        duration_secs: f64,
    ) {
        let status = if success { "success" } else { "error" };

        histogram!(
            "storage_write_duration_seconds",
            "writer_type" => writer_type.to_string(),
            "operation" => operation.to_string()
        )
        .record(duration_secs);

        counter!(
            "storage_writes_total",
            "writer_type" => writer_type.to_string(),
            "operation" => operation.to_string(),
            "status" => status.to_string()
        )
        .increment(1);
    }

    /// Record a query operation.
    ///
    /// # Arguments
    ///
    /// * `repository` - Repository name (trace_repository, metric_repository, log_repository)
    /// * `method` - Method name (get_by_id, list, search, etc.)
    /// * `duration_secs` - Duration in seconds
    pub fn record_query(&self, repository: &str, method: &str, duration_secs: f64) {
        histogram!(
            "storage_query_duration_seconds",
            "repository" => repository.to_string(),
            "method" => method.to_string()
        )
        .record(duration_secs);
    }

    /// Record query results count.
    pub fn record_query_result_count(&self, repository: &str, method: &str, count: usize) {
        histogram!(
            "storage_query_result_count",
            "repository" => repository.to_string(),
            "method" => method.to_string()
        )
        .record(count as f64);
    }

    /// Update connection pool metrics.
    ///
    /// # Arguments
    ///
    /// * `active` - Number of active connections
    /// * `idle` - Number of idle connections
    /// * `max` - Maximum number of connections
    pub fn update_pool_connections(&self, active: u32, idle: u32, max: u32) {
        gauge!(
            "storage_pool_connections",
            "state" => "active"
        )
        .set(active as f64);

        gauge!(
            "storage_pool_connections",
            "state" => "idle"
        )
        .set(idle as f64);

        gauge!(
            "storage_pool_connections",
            "state" => "max"
        )
        .set(max as f64);
    }

    /// Record an error.
    ///
    /// # Arguments
    ///
    /// * `error_type` - Type of error (connection, query, timeout, etc.)
    /// * `operation` - Operation that failed (optional)
    pub fn record_error(&self, error_type: &str, operation: Option<&str>) {
        if let Some(op) = operation {
            counter!(
                "storage_errors_total",
                "error_type" => error_type.to_string(),
                "operation" => op.to_string()
            )
            .increment(1);
        } else {
            counter!(
                "storage_errors_total",
                "error_type" => error_type.to_string()
            )
            .increment(1);
        }
    }

    /// Record a batch size.
    ///
    /// # Arguments
    ///
    /// * `writer_type` - Type of writer (trace, metric, log)
    /// * `operation` - Operation type (insert, copy)
    /// * `size` - Batch size
    pub fn record_batch_size(&self, writer_type: &str, operation: &str, size: usize) {
        histogram!(
            "storage_batch_size",
            "writer_type" => writer_type.to_string(),
            "operation" => operation.to_string()
        )
        .record(size as f64);
    }

    /// Update buffer size gauge.
    ///
    /// # Arguments
    ///
    /// * `writer_type` - Type of writer (trace, metric, log)
    /// * `buffer_type` - Type of buffer (traces, spans, events, metrics, data_points, logs)
    /// * `size` - Current buffer size
    pub fn update_buffer_size(&self, writer_type: &str, buffer_type: &str, size: usize) {
        gauge!(
            "storage_buffer_size",
            "writer_type" => writer_type.to_string(),
            "buffer_type" => buffer_type.to_string()
        )
        .set(size as f64);
    }

    /// Record a buffer flush operation.
    ///
    /// # Arguments
    ///
    /// * `writer_type` - Type of writer (trace, metric, log)
    /// * `success` - Whether the flush succeeded
    pub fn record_flush(&self, writer_type: &str, success: bool) {
        let status = if success { "success" } else { "error" };
        counter!(
            "storage_flushes_total",
            "writer_type" => writer_type.to_string(),
            "status" => status.to_string()
        )
        .increment(1);
    }

    /// Record a retry attempt.
    ///
    /// # Arguments
    ///
    /// * `operation` - Operation being retried
    pub fn record_retry(&self, operation: &str) {
        counter!(
            "storage_retries_total",
            "operation" => operation.to_string()
        )
        .increment(1);
    }

    /// Record items written to storage.
    ///
    /// # Arguments
    ///
    /// * `writer_type` - Type of writer (trace, metric, log)
    /// * `item_type` - Type of item (traces, spans, events, metrics, data_points, logs)
    /// * `count` - Number of items written
    pub fn record_items_written(&self, writer_type: &str, item_type: &str, count: u64) {
        counter!(
            "storage_items_written_total",
            "writer_type" => writer_type.to_string(),
            "item_type" => item_type.to_string()
        )
        .increment(count);
    }

    /// Record connection acquisition duration.
    pub fn record_connection_acquire(&self, duration_secs: f64) {
        histogram!("storage_connection_acquire_duration_seconds").record(duration_secs);
    }
}

impl Default for StorageMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// RAII guard for timing operations.
///
/// This automatically records the duration when dropped.
pub struct TimingGuard {
    start: Instant,
    metrics: Arc<StorageMetrics>,
    recorder: Box<dyn FnOnce(&StorageMetrics, f64) + Send>,
}

impl TimingGuard {
    /// Create a new timing guard.
    pub fn new<F>(metrics: Arc<StorageMetrics>, recorder: F) -> Self
    where
        F: FnOnce(&StorageMetrics, f64) + Send + 'static,
    {
        Self {
            start: Instant::now(),
            metrics,
            recorder: Box::new(recorder),
        }
    }
}

impl Drop for TimingGuard {
    fn drop(&mut self) {
        let duration = self.start.elapsed().as_secs_f64();
        // We need to extract the recorder since we can't call a FnOnce in a &mut method
        // This is a bit of a hack but necessary due to Drop's signature
        let recorder = std::mem::replace(&mut self.recorder, Box::new(|_, _| {}));
        recorder(&self.metrics, duration);
    }
}

/// Helper to create a timing guard for write operations.
pub fn time_write(
    metrics: Arc<StorageMetrics>,
    writer_type: &'static str,
    operation: &'static str,
) -> TimingGuard {
    TimingGuard::new(metrics, move |m, duration| {
        m.record_write(writer_type, operation, true, duration);
    })
}

/// Helper to create a timing guard for query operations.
pub fn time_query(
    metrics: Arc<StorageMetrics>,
    repository: &'static str,
    method: &'static str,
) -> TimingGuard {
    TimingGuard::new(metrics, move |m, duration| {
        m.record_query(repository, method, duration);
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_creation() {
        let metrics = StorageMetrics::new();
        // Should not panic
        metrics.record_write("trace", "insert", true, 0.1);
    }

    #[test]
    fn test_metrics_record_query() {
        let metrics = StorageMetrics::new();
        metrics.record_query("trace_repository", "get_by_id", 0.005);
        // Verify it doesn't panic
    }

    #[test]
    fn test_metrics_update_pool() {
        let metrics = StorageMetrics::new();
        metrics.update_pool_connections(5, 3, 10);
        // Verify it doesn't panic
    }

    #[test]
    fn test_metrics_record_error() {
        let metrics = StorageMetrics::new();
        metrics.record_error("connection", Some("connect"));
        metrics.record_error("timeout", None);
        // Verify it doesn't panic
    }

    #[test]
    fn test_timing_guard() {
        let metrics = Arc::new(StorageMetrics::new());
        {
            let _guard = time_write(metrics.clone(), "trace", "insert");
            // Guard should record duration when dropped
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        // Should not panic
    }
}
