//! Instrumented COPY writer with metrics support.

use crate::error::StorageResult;
use crate::metrics::StorageMetrics;
use crate::models::{LogRecord, Metric, MetricDataPoint, Trace, TraceEvent, TraceSpan};
use crate::writers::CopyWriter;
use std::sync::Arc;
use std::time::Instant;
use tokio_postgres::Client;

/// Instrumented COPY writer with metrics.
pub struct InstrumentedCopyWriter {
    metrics: Arc<StorageMetrics>,
}

impl InstrumentedCopyWriter {
    /// Create a new instrumented COPY writer.
    pub fn new(metrics: Arc<StorageMetrics>) -> Self {
        Self { metrics }
    }

    /// Write traces using COPY protocol with metrics.
    pub async fn write_traces(&self, client: &Client, traces: Vec<Trace>) -> StorageResult<u64> {
        let count = traces.len();
        let start = Instant::now();

        let result = CopyWriter::write_traces(client, traces).await;
        let duration = start.elapsed().as_secs_f64();

        self.metrics
            .record_write("copy", "write_traces", result.is_ok(), duration);
        self.metrics.record_batch_size("trace", "copy", count);

        if let Ok(rows) = result {
            self.metrics.record_items_written("copy", "traces", rows);
            Ok(rows)
        } else {
            self.metrics.record_error("copy", Some("write_traces"));
            result
        }
    }

    /// Write spans using COPY protocol with metrics.
    pub async fn write_spans(&self, client: &Client, spans: Vec<TraceSpan>) -> StorageResult<u64> {
        let count = spans.len();
        let start = Instant::now();

        let result = CopyWriter::write_spans(client, spans).await;
        let duration = start.elapsed().as_secs_f64();

        self.metrics
            .record_write("copy", "write_spans", result.is_ok(), duration);
        self.metrics.record_batch_size("trace", "copy", count);

        if let Ok(rows) = result {
            self.metrics.record_items_written("copy", "spans", rows);
            Ok(rows)
        } else {
            self.metrics.record_error("copy", Some("write_spans"));
            result
        }
    }

    /// Write events using COPY protocol with metrics.
    pub async fn write_events(
        &self,
        client: &Client,
        events: Vec<TraceEvent>,
    ) -> StorageResult<u64> {
        let count = events.len();
        let start = Instant::now();

        let result = CopyWriter::write_events(client, events).await;
        let duration = start.elapsed().as_secs_f64();

        self.metrics
            .record_write("copy", "write_events", result.is_ok(), duration);
        self.metrics.record_batch_size("trace", "copy", count);

        if let Ok(rows) = result {
            self.metrics.record_items_written("copy", "events", rows);
            Ok(rows)
        } else {
            self.metrics.record_error("copy", Some("write_events"));
            result
        }
    }

    /// Write metrics using COPY protocol with metrics.
    pub async fn write_metrics(
        &self,
        client: &Client,
        metrics_list: Vec<Metric>,
    ) -> StorageResult<u64> {
        let count = metrics_list.len();
        let start = Instant::now();

        let result = CopyWriter::write_metrics(client, metrics_list).await;
        let duration = start.elapsed().as_secs_f64();

        self.metrics
            .record_write("copy", "write_metrics", result.is_ok(), duration);
        self.metrics.record_batch_size("metric", "copy", count);

        if let Ok(rows) = result {
            self.metrics.record_items_written("copy", "metrics", rows);
            Ok(rows)
        } else {
            self.metrics.record_error("copy", Some("write_metrics"));
            result
        }
    }

    /// Write data points using COPY protocol with metrics.
    pub async fn write_data_points(
        &self,
        client: &Client,
        data_points: Vec<MetricDataPoint>,
    ) -> StorageResult<u64> {
        let count = data_points.len();
        let start = Instant::now();

        let result = CopyWriter::write_data_points(client, data_points).await;
        let duration = start.elapsed().as_secs_f64();

        self.metrics
            .record_write("copy", "write_data_points", result.is_ok(), duration);
        self.metrics.record_batch_size("metric", "copy", count);

        if let Ok(rows) = result {
            self.metrics
                .record_items_written("copy", "data_points", rows);
            Ok(rows)
        } else {
            self.metrics.record_error("copy", Some("write_data_points"));
            result
        }
    }

    /// Write logs using COPY protocol with metrics.
    pub async fn write_logs(&self, client: &Client, logs: Vec<LogRecord>) -> StorageResult<u64> {
        let count = logs.len();
        let start = Instant::now();

        let result = CopyWriter::write_logs(client, logs).await;
        let duration = start.elapsed().as_secs_f64();

        self.metrics
            .record_write("copy", "write_logs", result.is_ok(), duration);
        self.metrics.record_batch_size("log", "copy", count);

        if let Ok(rows) = result {
            self.metrics.record_items_written("copy", "logs", rows);
            Ok(rows)
        } else {
            self.metrics.record_error("copy", Some("write_logs"));
            result
        }
    }
}
