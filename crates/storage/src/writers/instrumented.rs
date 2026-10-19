//! Instrumented writers with metrics support.
//!
//! This module provides wrappers around the standard writers that automatically
//! record Prometheus metrics for all operations.

use crate::error::StorageResult;
use crate::metrics::StorageMetrics;
use crate::models::{LogRecord, Metric, MetricDataPoint, Trace, TraceEvent, TraceSpan};
use crate::pool::StoragePool;
use std::sync::Arc;
use std::time::Instant;

/// Instrumented trace writer with metrics.
pub struct InstrumentedTraceWriter {
    inner: super::TraceWriter,
    metrics: Arc<StorageMetrics>,
}

impl InstrumentedTraceWriter {
    /// Create a new instrumented trace writer.
    pub fn new(pool: StoragePool, metrics: Arc<StorageMetrics>) -> Self {
        Self {
            inner: super::TraceWriter::new(pool),
            metrics,
        }
    }

    /// Create with custom configuration.
    pub fn with_config(
        pool: StoragePool,
        config: super::trace::WriterConfig,
        metrics: Arc<StorageMetrics>,
    ) -> Self {
        Self {
            inner: super::TraceWriter::with_config(pool, config),
            metrics,
        }
    }

    /// Write a single trace with metrics.
    pub async fn write_trace(&self, trace: Trace) -> StorageResult<()> {
        let start = Instant::now();
        let result = self.inner.write_trace(trace).await;
        let duration = start.elapsed().as_secs_f64();

        self.metrics
            .record_write("trace", "write_trace", result.is_ok(), duration);
        if result.is_ok() {
            self.metrics.record_items_written("trace", "traces", 1);
        } else {
            self.metrics.record_error("write", Some("write_trace"));
        }

        // Update buffer metrics
        let stats = self.inner.buffer_stats().await;
        self.metrics
            .update_buffer_size("trace", "traces", stats.traces_buffered);
        self.metrics
            .update_buffer_size("trace", "spans", stats.spans_buffered);
        self.metrics
            .update_buffer_size("trace", "events", stats.events_buffered);

        result
    }

    /// Write multiple traces with metrics.
    pub async fn write_traces(&self, traces: Vec<Trace>) -> StorageResult<()> {
        let count = traces.len();
        let start = Instant::now();
        let result = self.inner.write_traces(traces).await;
        let duration = start.elapsed().as_secs_f64();

        self.metrics
            .record_write("trace", "write_traces", result.is_ok(), duration);
        if result.is_ok() {
            self.metrics
                .record_items_written("trace", "traces", count as u64);
        } else {
            self.metrics.record_error("write", Some("write_traces"));
        }

        // Update buffer metrics
        let stats = self.inner.buffer_stats().await;
        self.metrics
            .update_buffer_size("trace", "traces", stats.traces_buffered);

        result
    }

    /// Write a single span with metrics.
    pub async fn write_span(&self, span: TraceSpan) -> StorageResult<()> {
        let start = Instant::now();
        let result = self.inner.write_span(span).await;
        let duration = start.elapsed().as_secs_f64();

        self.metrics
            .record_write("trace", "write_span", result.is_ok(), duration);
        if result.is_ok() {
            self.metrics.record_items_written("trace", "spans", 1);
        } else {
            self.metrics.record_error("write", Some("write_span"));
        }

        let stats = self.inner.buffer_stats().await;
        self.metrics
            .update_buffer_size("trace", "spans", stats.spans_buffered);

        result
    }

    /// Write multiple spans with metrics.
    pub async fn write_spans(&self, spans: Vec<TraceSpan>) -> StorageResult<()> {
        let count = spans.len();
        let start = Instant::now();
        let result = self.inner.write_spans(spans).await;
        let duration = start.elapsed().as_secs_f64();

        self.metrics
            .record_write("trace", "write_spans", result.is_ok(), duration);
        if result.is_ok() {
            self.metrics
                .record_items_written("trace", "spans", count as u64);
        } else {
            self.metrics.record_error("write", Some("write_spans"));
        }

        let stats = self.inner.buffer_stats().await;
        self.metrics
            .update_buffer_size("trace", "spans", stats.spans_buffered);

        result
    }

    /// Write a single event with metrics.
    pub async fn write_event(&self, event: TraceEvent) -> StorageResult<()> {
        let start = Instant::now();
        let result = self.inner.write_event(event).await;
        let duration = start.elapsed().as_secs_f64();

        self.metrics
            .record_write("trace", "write_event", result.is_ok(), duration);
        if result.is_ok() {
            self.metrics.record_items_written("trace", "events", 1);
        } else {
            self.metrics.record_error("write", Some("write_event"));
        }

        let stats = self.inner.buffer_stats().await;
        self.metrics
            .update_buffer_size("trace", "events", stats.events_buffered);

        result
    }

    /// Flush with metrics.
    pub async fn flush(&self) -> StorageResult<()> {
        let stats_before = self.inner.buffer_stats().await;
        let total_items = stats_before.traces_buffered
            + stats_before.spans_buffered
            + stats_before.events_buffered;

        let start = Instant::now();
        let result = self.inner.flush().await;
        let duration = start.elapsed().as_secs_f64();

        self.metrics
            .record_write("trace", "flush", result.is_ok(), duration);
        self.metrics.record_flush("trace", result.is_ok());

        if total_items > 0 {
            self.metrics
                .record_batch_size("trace", "flush", total_items);
        }

        if result.is_err() {
            self.metrics.record_error("flush", Some("trace_flush"));
        }

        // Update buffer metrics after flush
        let stats_after = self.inner.buffer_stats().await;
        self.metrics
            .update_buffer_size("trace", "traces", stats_after.traces_buffered);
        self.metrics
            .update_buffer_size("trace", "spans", stats_after.spans_buffered);
        self.metrics
            .update_buffer_size("trace", "events", stats_after.events_buffered);

        result
    }
}

/// Instrumented metric writer with metrics.
pub struct InstrumentedMetricWriter {
    inner: super::MetricWriter,
    metrics: Arc<StorageMetrics>,
}

impl InstrumentedMetricWriter {
    /// Create a new instrumented metric writer.
    pub fn new(pool: StoragePool, metrics: Arc<StorageMetrics>) -> Self {
        Self {
            inner: super::MetricWriter::new(pool),
            metrics,
        }
    }

    /// Create with custom configuration.
    pub fn with_config(
        pool: StoragePool,
        config: super::metric::WriterConfig,
        metrics: Arc<StorageMetrics>,
    ) -> Self {
        Self {
            inner: super::MetricWriter::with_config(pool, config),
            metrics,
        }
    }

    /// Write a single metric with metrics.
    pub async fn write_metric(&self, metric: Metric) -> StorageResult<()> {
        let start = Instant::now();
        let result = self.inner.write_metric(metric).await;
        let duration = start.elapsed().as_secs_f64();

        self.metrics
            .record_write("metric", "write_metric", result.is_ok(), duration);
        if result.is_ok() {
            self.metrics.record_items_written("metric", "metrics", 1);
        } else {
            self.metrics.record_error("write", Some("write_metric"));
        }

        let stats = self.inner.buffer_stats().await;
        self.metrics
            .update_buffer_size("metric", "metrics", stats.metrics_buffered);

        result
    }

    /// Write multiple metrics with metrics.
    pub async fn write_metrics(&self, metrics_list: Vec<Metric>) -> StorageResult<()> {
        let count = metrics_list.len();
        let start = Instant::now();
        let result = self.inner.write_metrics(metrics_list).await;
        let duration = start.elapsed().as_secs_f64();

        self.metrics
            .record_write("metric", "write_metrics", result.is_ok(), duration);
        if result.is_ok() {
            self.metrics
                .record_items_written("metric", "metrics", count as u64);
        } else {
            self.metrics.record_error("write", Some("write_metrics"));
        }

        let stats = self.inner.buffer_stats().await;
        self.metrics
            .update_buffer_size("metric", "metrics", stats.metrics_buffered);

        result
    }

    /// Write a single data point with metrics.
    pub async fn write_data_point(&self, data_point: MetricDataPoint) -> StorageResult<()> {
        let start = Instant::now();
        let result = self.inner.write_data_point(data_point).await;
        let duration = start.elapsed().as_secs_f64();

        self.metrics
            .record_write("metric", "write_data_point", result.is_ok(), duration);
        if result.is_ok() {
            self.metrics
                .record_items_written("metric", "data_points", 1);
        } else {
            self.metrics.record_error("write", Some("write_data_point"));
        }

        let stats = self.inner.buffer_stats().await;
        self.metrics
            .update_buffer_size("metric", "data_points", stats.data_points_buffered);

        result
    }

    /// Write multiple data points with metrics.
    pub async fn write_data_points(&self, data_points: Vec<MetricDataPoint>) -> StorageResult<()> {
        let count = data_points.len();
        let start = Instant::now();
        let result = self.inner.write_data_points(data_points).await;
        let duration = start.elapsed().as_secs_f64();

        self.metrics
            .record_write("metric", "write_data_points", result.is_ok(), duration);
        if result.is_ok() {
            self.metrics
                .record_items_written("metric", "data_points", count as u64);
        } else {
            self.metrics
                .record_error("write", Some("write_data_points"));
        }

        let stats = self.inner.buffer_stats().await;
        self.metrics
            .update_buffer_size("metric", "data_points", stats.data_points_buffered);

        result
    }

    /// Flush with metrics.
    pub async fn flush(&self) -> StorageResult<()> {
        let stats_before = self.inner.buffer_stats().await;
        let total_items = stats_before.metrics_buffered + stats_before.data_points_buffered;

        let start = Instant::now();
        let result = self.inner.flush().await;
        let duration = start.elapsed().as_secs_f64();

        self.metrics
            .record_write("metric", "flush", result.is_ok(), duration);
        self.metrics.record_flush("metric", result.is_ok());

        if total_items > 0 {
            self.metrics
                .record_batch_size("metric", "flush", total_items);
        }

        if result.is_err() {
            self.metrics.record_error("flush", Some("metric_flush"));
        }

        let stats_after = self.inner.buffer_stats().await;
        self.metrics
            .update_buffer_size("metric", "metrics", stats_after.metrics_buffered);
        self.metrics
            .update_buffer_size("metric", "data_points", stats_after.data_points_buffered);

        result
    }
}

/// Instrumented log writer with metrics.
pub struct InstrumentedLogWriter {
    inner: super::LogWriter,
    metrics: Arc<StorageMetrics>,
}

impl InstrumentedLogWriter {
    /// Create a new instrumented log writer.
    pub fn new(pool: StoragePool, metrics: Arc<StorageMetrics>) -> Self {
        Self {
            inner: super::LogWriter::new(pool),
            metrics,
        }
    }

    /// Create with custom configuration.
    pub fn with_config(
        pool: StoragePool,
        config: super::log::WriterConfig,
        metrics: Arc<StorageMetrics>,
    ) -> Self {
        Self {
            inner: super::LogWriter::with_config(pool, config),
            metrics,
        }
    }

    /// Write a single log with metrics.
    pub async fn write_log(&self, log: LogRecord) -> StorageResult<()> {
        let start = Instant::now();
        let result = self.inner.write_log(log).await;
        let duration = start.elapsed().as_secs_f64();

        self.metrics
            .record_write("log", "write_log", result.is_ok(), duration);
        if result.is_ok() {
            self.metrics.record_items_written("log", "logs", 1);
        } else {
            self.metrics.record_error("write", Some("write_log"));
        }

        let stats = self.inner.buffer_stats().await;
        self.metrics
            .update_buffer_size("log", "logs", stats.logs_buffered);

        result
    }

    /// Write multiple logs with metrics.
    pub async fn write_logs(&self, logs: Vec<LogRecord>) -> StorageResult<()> {
        let count = logs.len();
        let start = Instant::now();
        let result = self.inner.write_logs(logs).await;
        let duration = start.elapsed().as_secs_f64();

        self.metrics
            .record_write("log", "write_logs", result.is_ok(), duration);
        if result.is_ok() {
            self.metrics
                .record_items_written("log", "logs", count as u64);
        } else {
            self.metrics.record_error("write", Some("write_logs"));
        }

        let stats = self.inner.buffer_stats().await;
        self.metrics
            .update_buffer_size("log", "logs", stats.logs_buffered);

        result
    }

    /// Flush with metrics.
    pub async fn flush(&self) -> StorageResult<()> {
        let stats_before = self.inner.buffer_stats().await;
        let total_items = stats_before.logs_buffered;

        let start = Instant::now();
        let result = self.inner.flush().await;
        let duration = start.elapsed().as_secs_f64();

        self.metrics
            .record_write("log", "flush", result.is_ok(), duration);
        self.metrics.record_flush("log", result.is_ok());

        if total_items > 0 {
            self.metrics.record_batch_size("log", "flush", total_items);
        }

        if result.is_err() {
            self.metrics.record_error("flush", Some("log_flush"));
        }

        let stats_after = self.inner.buffer_stats().await;
        self.metrics
            .update_buffer_size("log", "logs", stats_after.logs_buffered);

        result
    }

    /// Start auto-flush with the inner writer.
    pub fn start_auto_flush(&self) -> tokio::task::JoinHandle<()> {
        self.inner.start_auto_flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Integration tests would go here, requiring a test database
}
