//! Instrumented repositories with metrics support.
//!
//! This module provides wrappers around the standard repositories that automatically
//! record Prometheus metrics for all query operations.

use crate::error::StorageResult;
use crate::metrics::StorageMetrics;
use crate::models::{LogRecord, Metric, MetricDataPoint, Trace, TraceEvent, TraceSpan};
use crate::pool::StoragePool;
use crate::repositories::{
    log::{LogFilters, LogRepository},
    metric::{MetricFilters, MetricRepository},
    trace::{TraceFilters, TraceRepository, TraceStats},
};
use chrono::{DateTime, Utc};
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;

/// Instrumented trace repository with metrics.
pub struct InstrumentedTraceRepository {
    inner: TraceRepository,
    metrics: Arc<StorageMetrics>,
}

impl InstrumentedTraceRepository {
    /// Create a new instrumented trace repository.
    pub fn new(pool: StoragePool, metrics: Arc<StorageMetrics>) -> Self {
        Self {
            inner: TraceRepository::new(pool),
            metrics,
        }
    }

    /// Get a trace by its ID with metrics.
    pub async fn get_by_id(&self, id: Uuid) -> StorageResult<Trace> {
        let start = Instant::now();
        let result = self.inner.get_by_id(id).await;
        let duration = start.elapsed().as_secs_f64();

        self.metrics
            .record_query("trace_repository", "get_by_id", duration);
        if result.is_ok() {
            self.metrics
                .record_query_result_count("trace_repository", "get_by_id", 1);
        } else {
            self.metrics.record_error("query", Some("get_by_id"));
        }

        result
    }

    /// Get a trace by its trace ID with metrics.
    pub async fn get_by_trace_id(&self, trace_id: &str) -> StorageResult<Trace> {
        let start = Instant::now();
        let result = self.inner.get_by_trace_id(trace_id).await;
        let duration = start.elapsed().as_secs_f64();

        self.metrics
            .record_query("trace_repository", "get_by_trace_id", duration);
        if result.is_ok() {
            self.metrics
                .record_query_result_count("trace_repository", "get_by_trace_id", 1);
        } else {
            self.metrics.record_error("query", Some("get_by_trace_id"));
        }

        result
    }

    /// Get a trace with all its spans with metrics.
    pub async fn get_trace_by_id(&self, trace_id: &str) -> StorageResult<(Trace, Vec<TraceSpan>)> {
        let start = Instant::now();
        let result = self.inner.get_trace_by_id(trace_id).await;
        let duration = start.elapsed().as_secs_f64();

        self.metrics
            .record_query("trace_repository", "get_trace_by_id", duration);
        if let Ok((_, ref spans)) = result {
            self.metrics.record_query_result_count(
                "trace_repository",
                "get_trace_by_id",
                1 + spans.len(),
            );
        } else {
            self.metrics.record_error("query", Some("get_trace_by_id"));
        }

        result
    }

    /// List traces with filters with metrics.
    pub async fn list(&self, filters: TraceFilters) -> StorageResult<Vec<Trace>> {
        let start = Instant::now();
        let result = self.inner.list(filters).await;
        let duration = start.elapsed().as_secs_f64();

        self.metrics
            .record_query("trace_repository", "list", duration);
        if let Ok(ref traces) = result {
            self.metrics
                .record_query_result_count("trace_repository", "list", traces.len());
        } else {
            self.metrics.record_error("query", Some("list"));
        }

        result
    }

    /// Get traces for a time range with metrics.
    pub async fn get_traces(
        &self,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        limit: i64,
        filters: TraceFilters,
    ) -> StorageResult<Vec<Trace>> {
        let start = Instant::now();
        let result = self
            .inner
            .get_traces(start_time, end_time, limit, filters)
            .await;
        let duration = start.elapsed().as_secs_f64();

        self.metrics
            .record_query("trace_repository", "get_traces", duration);
        if let Ok(ref traces) = result {
            self.metrics
                .record_query_result_count("trace_repository", "get_traces", traces.len());
        } else {
            self.metrics.record_error("query", Some("get_traces"));
        }

        result
    }

    /// Get all spans for a trace with metrics.
    pub async fn get_spans(&self, trace_id: Uuid) -> StorageResult<Vec<TraceSpan>> {
        let start = Instant::now();
        let result = self.inner.get_spans(trace_id).await;
        let duration = start.elapsed().as_secs_f64();

        self.metrics
            .record_query("trace_repository", "get_spans", duration);
        if let Ok(ref spans) = result {
            self.metrics
                .record_query_result_count("trace_repository", "get_spans", spans.len());
        } else {
            self.metrics.record_error("query", Some("get_spans"));
        }

        result
    }

    /// Get a specific span by ID with metrics.
    pub async fn get_span_by_id(&self, span_id: Uuid) -> StorageResult<TraceSpan> {
        let start = Instant::now();
        let result = self.inner.get_span_by_id(span_id).await;
        let duration = start.elapsed().as_secs_f64();

        self.metrics
            .record_query("trace_repository", "get_span_by_id", duration);
        if result.is_ok() {
            self.metrics
                .record_query_result_count("trace_repository", "get_span_by_id", 1);
        } else {
            self.metrics.record_error("query", Some("get_span_by_id"));
        }

        result
    }

    /// Get all events for a span with metrics.
    pub async fn get_events(&self, span_id: Uuid) -> StorageResult<Vec<TraceEvent>> {
        let start = Instant::now();
        let result = self.inner.get_events(span_id).await;
        let duration = start.elapsed().as_secs_f64();

        self.metrics
            .record_query("trace_repository", "get_events", duration);
        if let Ok(ref events) = result {
            self.metrics
                .record_query_result_count("trace_repository", "get_events", events.len());
        } else {
            self.metrics.record_error("query", Some("get_events"));
        }

        result
    }

    /// Search traces by service with metrics.
    pub async fn search_by_service(
        &self,
        service_name: &str,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> StorageResult<Vec<Trace>> {
        let start = Instant::now();
        let result = self
            .inner
            .search_by_service(service_name, start_time, end_time)
            .await;
        let duration = start.elapsed().as_secs_f64();

        self.metrics
            .record_query("trace_repository", "search_by_service", duration);
        if let Ok(ref traces) = result {
            self.metrics.record_query_result_count(
                "trace_repository",
                "search_by_service",
                traces.len(),
            );
        } else {
            self.metrics
                .record_error("query", Some("search_by_service"));
        }

        result
    }

    /// Search traces with errors with metrics.
    pub async fn search_errors(&self, filters: TraceFilters) -> StorageResult<Vec<Trace>> {
        let start = Instant::now();
        let result = self.inner.search_errors(filters).await;
        let duration = start.elapsed().as_secs_f64();

        self.metrics
            .record_query("trace_repository", "search_errors", duration);
        if let Ok(ref traces) = result {
            self.metrics.record_query_result_count(
                "trace_repository",
                "search_errors",
                traces.len(),
            );
        } else {
            self.metrics.record_error("query", Some("search_errors"));
        }

        result
    }

    /// Get trace statistics with metrics.
    pub async fn get_trace_statistics(&self, trace_id: &str) -> StorageResult<TraceStats> {
        let start = Instant::now();
        let result = self.inner.get_trace_statistics(trace_id).await;
        let duration = start.elapsed().as_secs_f64();

        self.metrics
            .record_query("trace_repository", "get_trace_statistics", duration);
        if result.is_err() {
            self.metrics
                .record_error("query", Some("get_trace_statistics"));
        }

        result
    }

    /// Get stats for a time range with metrics.
    pub async fn get_stats(
        &self,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> StorageResult<TraceStats> {
        let start = Instant::now();
        let result = self.inner.get_stats(start_time, end_time).await;
        let duration = start.elapsed().as_secs_f64();

        self.metrics
            .record_query("trace_repository", "get_stats", duration);
        if result.is_err() {
            self.metrics.record_error("query", Some("get_stats"));
        }

        result
    }

    /// Delete old traces with metrics.
    pub async fn delete_before(&self, before: DateTime<Utc>) -> StorageResult<u64> {
        let start = Instant::now();
        let result = self.inner.delete_before(before).await;
        let duration = start.elapsed().as_secs_f64();

        self.metrics
            .record_query("trace_repository", "delete_before", duration);
        if result.is_err() {
            self.metrics.record_error("query", Some("delete_before"));
        }

        result
    }
}

/// Instrumented metric repository with metrics.
pub struct InstrumentedMetricRepository {
    inner: MetricRepository,
    metrics: Arc<StorageMetrics>,
}

impl InstrumentedMetricRepository {
    /// Create a new instrumented metric repository.
    pub fn new(pool: StoragePool, metrics: Arc<StorageMetrics>) -> Self {
        Self {
            inner: MetricRepository::new(pool),
            metrics,
        }
    }

    /// Get a metric by ID with metrics.
    pub async fn get_by_id(&self, id: Uuid) -> StorageResult<Metric> {
        let start = Instant::now();
        let result = self.inner.get_by_id(id).await;
        let duration = start.elapsed().as_secs_f64();

        self.metrics
            .record_query("metric_repository", "get_by_id", duration);
        if result.is_ok() {
            self.metrics
                .record_query_result_count("metric_repository", "get_by_id", 1);
        } else {
            self.metrics.record_error("query", Some("get_by_id"));
        }

        result
    }

    /// List metrics with filters with metrics.
    pub async fn list(&self, filters: MetricFilters) -> StorageResult<Vec<Metric>> {
        let start = Instant::now();
        let result = self.inner.list(filters).await;
        let duration = start.elapsed().as_secs_f64();

        self.metrics
            .record_query("metric_repository", "list", duration);
        if let Ok(ref metrics_list) = result {
            self.metrics
                .record_query_result_count("metric_repository", "list", metrics_list.len());
        } else {
            self.metrics.record_error("query", Some("list"));
        }

        result
    }

    /// Get data points with metrics.
    pub async fn get_data_points(
        &self,
        metric_id: Uuid,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> StorageResult<Vec<MetricDataPoint>> {
        let start = Instant::now();
        let result = self
            .inner
            .get_data_points(metric_id, start_time, end_time)
            .await;
        let duration = start.elapsed().as_secs_f64();

        self.metrics
            .record_query("metric_repository", "get_data_points", duration);
        if let Ok(ref data_points) = result {
            self.metrics.record_query_result_count(
                "metric_repository",
                "get_data_points",
                data_points.len(),
            );
        } else {
            self.metrics.record_error("query", Some("get_data_points"));
        }

        result
    }
}

/// Instrumented log repository with metrics.
pub struct InstrumentedLogRepository {
    inner: LogRepository,
    metrics: Arc<StorageMetrics>,
}

impl InstrumentedLogRepository {
    /// Create a new instrumented log repository.
    pub fn new(pool: StoragePool, metrics: Arc<StorageMetrics>) -> Self {
        Self {
            inner: LogRepository::new(pool),
            metrics,
        }
    }

    /// Get a log by ID with metrics.
    pub async fn get_by_id(&self, id: Uuid) -> StorageResult<LogRecord> {
        let start = Instant::now();
        let result = self.inner.get_by_id(id).await;
        let duration = start.elapsed().as_secs_f64();

        self.metrics
            .record_query("log_repository", "get_by_id", duration);
        if result.is_ok() {
            self.metrics
                .record_query_result_count("log_repository", "get_by_id", 1);
        } else {
            self.metrics.record_error("query", Some("get_by_id"));
        }

        result
    }

    /// List logs with filters with metrics.
    pub async fn list(&self, filters: LogFilters) -> StorageResult<Vec<LogRecord>> {
        let start = Instant::now();
        let result = self.inner.list(filters).await;
        let duration = start.elapsed().as_secs_f64();

        self.metrics
            .record_query("log_repository", "list", duration);
        if let Ok(ref logs) = result {
            self.metrics
                .record_query_result_count("log_repository", "list", logs.len());
        } else {
            self.metrics.record_error("query", Some("list"));
        }

        result
    }

    /// Get logs for a time range with metrics.
    pub async fn get_logs(
        &self,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        limit: i64,
        filters: LogFilters,
    ) -> StorageResult<Vec<LogRecord>> {
        let start = Instant::now();
        let result = self
            .inner
            .get_logs(start_time, end_time, limit, filters)
            .await;
        let duration = start.elapsed().as_secs_f64();

        self.metrics
            .record_query("log_repository", "get_logs", duration);
        if let Ok(ref logs) = result {
            self.metrics
                .record_query_result_count("log_repository", "get_logs", logs.len());
        } else {
            self.metrics.record_error("query", Some("get_logs"));
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Integration tests would go here, requiring a test database
}
