//! Integration tests for trace, metric, and log writers.
//!
//! This test suite validates batch writing functionality for all data types,
//! including buffering, flushing, error handling, and performance.

mod common;

use common::*;
use llm_observatory_storage::writers::{LogWriter, MetricWriter, TraceWriter};
use uuid::Uuid;

// ============================================================================
// Trace Writer Tests
// ============================================================================

#[tokio::test]
async fn test_trace_writer_single_trace() {
    let (pool, _guard) = setup_test_pool().await;
    cleanup_test_data(&pool).await;

    let writer = TraceWriter::new(pool.clone());
    let trace = create_test_trace("trace-001", "test-service");

    writer.write_trace(trace.clone()).await.unwrap();
    writer.flush().await.unwrap();

    // Verify trace was written
    let result: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM traces WHERE trace_id = $1")
        .bind(&trace.trace_id)
        .fetch_one(pool.postgres())
        .await
        .unwrap();

    assert_eq!(result.0, 1);
}

#[tokio::test]
async fn test_trace_writer_multiple_traces() {
    let (pool, _guard) = setup_test_pool().await;
    cleanup_test_data(&pool).await;

    let writer = TraceWriter::new(pool.clone());
    let traces = create_test_traces(10, "test-service");

    writer.write_traces(traces).await.unwrap();
    writer.flush().await.unwrap();

    // Verify all traces were written
    let result: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM traces")
        .fetch_one(pool.postgres())
        .await
        .unwrap();

    assert_eq!(result.0, 10);
}

#[tokio::test]
async fn test_trace_writer_batch_flush() {
    let (pool, _guard) = setup_test_pool().await;
    cleanup_test_data(&pool).await;

    let mut config = llm_observatory_storage::writers::trace::WriterConfig::default();
    config.batch_size = 5;

    let writer = TraceWriter::with_config(pool.clone(), config);

    // Write 4 traces - should not auto-flush
    for i in 0..4 {
        let trace = create_test_trace(&format!("trace-{}", i), "test-service");
        writer.write_trace(trace).await.unwrap();
    }

    // Check buffer stats
    let stats = writer.buffer_stats().await;
    assert_eq!(stats.traces_buffered, 4);

    // Write 5th trace - should trigger auto-flush
    let trace = create_test_trace("trace-4", "test-service");
    writer.write_trace(trace).await.unwrap();

    // Buffer should be empty after auto-flush
    let stats = writer.buffer_stats().await;
    assert_eq!(stats.traces_buffered, 0);
}

#[tokio::test]
async fn test_trace_writer_span_insertion() {
    let (pool, _guard) = setup_test_pool().await;
    cleanup_test_data(&pool).await;

    let writer = TraceWriter::new(pool.clone());
    let trace_id = Uuid::new_v4();
    let span = create_test_span(trace_id, "span-001", "test-operation", "test-service");

    writer.write_span(span.clone()).await.unwrap();
    writer.flush().await.unwrap();

    // Verify span was written
    let result: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM trace_spans WHERE span_id = $1")
        .bind(&span.span_id)
        .fetch_one(pool.postgres())
        .await
        .unwrap();

    assert_eq!(result.0, 1);
}

#[tokio::test]
async fn test_trace_writer_multiple_spans() {
    let (pool, _guard) = setup_test_pool().await;
    cleanup_test_data(&pool).await;

    let writer = TraceWriter::new(pool.clone());
    let trace_id = Uuid::new_v4();
    let spans = create_test_spans(5, trace_id, "test-service");

    writer.write_spans(spans).await.unwrap();
    writer.flush().await.unwrap();

    // Verify all spans were written
    let result: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM trace_spans WHERE trace_id = $1")
        .bind(trace_id)
        .fetch_one(pool.postgres())
        .await
        .unwrap();

    assert_eq!(result.0, 5);
}

#[tokio::test]
async fn test_trace_writer_event_insertion() {
    let (pool, _guard) = setup_test_pool().await;
    cleanup_test_data(&pool).await;

    let writer = TraceWriter::new(pool.clone());
    let span_id = Uuid::new_v4();
    let event = create_test_event(span_id, "test-event");

    writer.write_event(event.clone()).await.unwrap();
    writer.flush().await.unwrap();

    // Verify event was written
    let result: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM trace_events WHERE span_id = $1")
        .bind(span_id)
        .fetch_one(pool.postgres())
        .await
        .unwrap();

    assert_eq!(result.0, 1);
}

#[tokio::test]
async fn test_trace_writer_write_stats() {
    let (pool, _guard) = setup_test_pool().await;
    cleanup_test_data(&pool).await;

    let writer = TraceWriter::new(pool.clone());
    let traces = create_test_traces(3, "test-service");
    let trace_id = Uuid::new_v4();
    let spans = create_test_spans(2, trace_id, "test-service");

    writer.write_traces(traces).await.unwrap();
    writer.write_spans(spans).await.unwrap();
    writer.flush().await.unwrap();

    let stats = writer.write_stats().await;
    assert_eq!(stats.traces_written, 3);
    assert_eq!(stats.spans_written, 2);
}

#[tokio::test]
async fn test_trace_writer_duplicate_handling() {
    let (pool, _guard) = setup_test_pool().await;
    cleanup_test_data(&pool).await;

    let writer = TraceWriter::new(pool.clone());
    let trace = create_test_trace("duplicate-trace", "test-service");

    // Write the same trace twice
    writer.write_trace(trace.clone()).await.unwrap();
    writer.flush().await.unwrap();

    let mut trace2 = trace.clone();
    trace2.status = "error".to_string();
    writer.write_trace(trace2).await.unwrap();
    writer.flush().await.unwrap();

    // Should have only one trace (updated)
    let result: (i64, String) =
        sqlx::query_as("SELECT COUNT(*), status FROM traces WHERE trace_id = $1 GROUP BY status")
            .bind("duplicate-trace")
            .fetch_one(pool.postgres())
            .await
            .unwrap();

    assert_eq!(result.0, 1);
    assert_eq!(result.1, "error");
}

// ============================================================================
// Metric Writer Tests
// ============================================================================

#[tokio::test]
async fn test_metric_writer_single_metric() {
    let (pool, _guard) = setup_test_pool().await;
    cleanup_test_data(&pool).await;

    let writer = MetricWriter::new(pool.clone());
    let metric = create_test_metric("test.counter", "counter", "test-service");

    writer.write_metric(metric.clone()).await.unwrap();
    writer.flush().await.unwrap();

    // Verify metric was written
    let result: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM metrics WHERE name = $1")
        .bind(&metric.name)
        .fetch_one(pool.postgres())
        .await
        .unwrap();

    assert_eq!(result.0, 1);
}

#[tokio::test]
async fn test_metric_writer_multiple_metrics() {
    let (pool, _guard) = setup_test_pool().await;
    cleanup_test_data(&pool).await;

    let writer = MetricWriter::new(pool.clone());
    let metrics = create_test_metrics(10, "test-service");

    writer.write_metrics(metrics).await.unwrap();
    writer.flush().await.unwrap();

    // Verify all metrics were written
    let result: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM metrics")
        .fetch_one(pool.postgres())
        .await
        .unwrap();

    assert_eq!(result.0, 10);
}

#[tokio::test]
async fn test_metric_writer_data_point() {
    let (pool, _guard) = setup_test_pool().await;
    cleanup_test_data(&pool).await;

    let writer = MetricWriter::new(pool.clone());
    let metric = create_test_metric("test.gauge", "gauge", "test-service");

    writer.write_metric(metric.clone()).await.unwrap();
    writer.flush().await.unwrap();

    let data_point = create_test_metric_data_point(metric.id, 42.5);
    writer.write_data_point(data_point).await.unwrap();
    writer.flush().await.unwrap();

    // Verify data point was written
    let result: (i64, f64) =
        sqlx::query_as("SELECT COUNT(*), SUM(value) FROM metric_data_points WHERE metric_id = $1")
            .bind(metric.id)
            .fetch_one(pool.postgres())
            .await
            .unwrap();

    assert_eq!(result.0, 1);
    assert_eq!(result.1, 42.5);
}

#[tokio::test]
async fn test_metric_writer_multiple_data_points() {
    let (pool, _guard) = setup_test_pool().await;
    cleanup_test_data(&pool).await;

    let writer = MetricWriter::new(pool.clone());
    let metric = create_test_metric("test.counter", "counter", "test-service");

    writer.write_metric(metric.clone()).await.unwrap();
    writer.flush().await.unwrap();

    let data_points: Vec<_> = (0..5)
        .map(|i| create_test_metric_data_point(metric.id, i as f64 * 10.0))
        .collect();

    writer.write_data_points(data_points).await.unwrap();
    writer.flush().await.unwrap();

    // Verify all data points were written
    let result: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM metric_data_points WHERE metric_id = $1")
            .bind(metric.id)
            .fetch_one(pool.postgres())
            .await
            .unwrap();

    assert_eq!(result.0, 5);
}

#[tokio::test]
async fn test_metric_writer_histogram_data_point() {
    let (pool, _guard) = setup_test_pool().await;
    cleanup_test_data(&pool).await;

    let writer = MetricWriter::new(pool.clone());
    let metric = create_test_metric("test.histogram", "histogram", "test-service");

    writer.write_metric(metric.clone()).await.unwrap();
    writer.flush().await.unwrap();

    let histogram = create_histogram_data_point(metric.id);
    writer.write_data_point(histogram).await.unwrap();
    writer.flush().await.unwrap();

    // Verify histogram data point was written
    let result: (i64, i64, f64) =
        sqlx::query_as("SELECT COUNT(*), count, sum FROM metric_data_points WHERE metric_id = $1")
            .bind(metric.id)
            .fetch_one(pool.postgres())
            .await
            .unwrap();

    assert_eq!(result.0, 1);
    assert_eq!(result.1, 100);
    assert_eq!(result.2, 5000.0);
}

#[tokio::test]
async fn test_metric_writer_buffer_stats() {
    let (pool, _guard) = setup_test_pool().await;
    cleanup_test_data(&pool).await;

    let mut config = llm_observatory_storage::writers::metric::WriterConfig::default();
    config.batch_size = 100;

    let writer = MetricWriter::with_config(pool.clone(), config);
    let metrics = create_test_metrics(5, "test-service");

    writer.write_metrics(metrics).await.unwrap();

    let stats = writer.buffer_stats().await;
    assert_eq!(stats.metrics_buffered, 5);

    writer.flush().await.unwrap();

    let stats = writer.buffer_stats().await;
    assert_eq!(stats.metrics_buffered, 0);
}

#[tokio::test]
async fn test_metric_writer_upsert() {
    let (pool, _guard) = setup_test_pool().await;
    cleanup_test_data(&pool).await;

    let writer = MetricWriter::new(pool.clone());
    let mut metric = create_test_metric("upsert.metric", "counter", "test-service");

    writer.write_metric(metric.clone()).await.unwrap();
    writer.flush().await.unwrap();

    // Update description
    metric.description = Some("Updated description".to_string());
    writer.write_metric(metric.clone()).await.unwrap();
    writer.flush().await.unwrap();

    // Should have only one metric with updated description
    let result: (i64, String) = sqlx::query_as(
        "SELECT COUNT(*), description FROM metrics WHERE name = $1 GROUP BY description",
    )
    .bind("upsert.metric")
    .fetch_one(pool.postgres())
    .await
    .unwrap();

    assert_eq!(result.0, 1);
    assert_eq!(result.1, "Updated description");
}

// ============================================================================
// Log Writer Tests
// ============================================================================

#[tokio::test]
async fn test_log_writer_single_log() {
    let (pool, _guard) = setup_test_pool().await;
    cleanup_test_data(&pool).await;

    let writer = LogWriter::new(pool.clone());
    let log = create_test_log("test-service", "INFO", "Test log message", None);

    writer.write_log(log.clone()).await.unwrap();
    writer.flush().await.unwrap();

    // Verify log was written
    let result: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM logs WHERE body = $1")
        .bind(&log.body)
        .fetch_one(pool.postgres())
        .await
        .unwrap();

    assert_eq!(result.0, 1);
}

#[tokio::test]
async fn test_log_writer_multiple_logs() {
    let (pool, _guard) = setup_test_pool().await;
    cleanup_test_data(&pool).await;

    let writer = LogWriter::new(pool.clone());
    let logs = create_test_logs(20, "test-service");

    writer.write_logs(logs).await.unwrap();
    writer.flush().await.unwrap();

    // Verify all logs were written
    let result: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM logs")
        .fetch_one(pool.postgres())
        .await
        .unwrap();

    assert_eq!(result.0, 20);
}

#[tokio::test]
async fn test_log_writer_different_severities() {
    let (pool, _guard) = setup_test_pool().await;
    cleanup_test_data(&pool).await;

    let writer = LogWriter::new(pool.clone());

    let severities = vec!["TRACE", "DEBUG", "INFO", "WARN", "ERROR", "FATAL"];
    for severity in &severities {
        let log = create_test_log(
            "test-service",
            severity,
            &format!("{} message", severity),
            None,
        );
        writer.write_log(log).await.unwrap();
    }

    writer.flush().await.unwrap();

    // Verify all severity levels were written
    let result: (i64,) = sqlx::query_as("SELECT COUNT(DISTINCT severity_text) FROM logs")
        .fetch_one(pool.postgres())
        .await
        .unwrap();

    assert_eq!(result.0, 6);
}

#[tokio::test]
async fn test_log_writer_with_trace_context() {
    let (pool, _guard) = setup_test_pool().await;
    cleanup_test_data(&pool).await;

    let writer = LogWriter::new(pool.clone());
    let log = create_test_log(
        "test-service",
        "ERROR",
        "Error with trace",
        Some("trace-123"),
    );

    writer.write_log(log).await.unwrap();
    writer.flush().await.unwrap();

    // Verify log with trace context was written
    let result: (i64, String) = sqlx::query_as(
        "SELECT COUNT(*), trace_id FROM logs WHERE trace_id IS NOT NULL GROUP BY trace_id",
    )
    .fetch_one(pool.postgres())
    .await
    .unwrap();

    assert_eq!(result.0, 1);
    assert_eq!(result.1, "trace-123");
}

#[tokio::test]
async fn test_log_writer_buffer_stats() {
    let (pool, _guard) = setup_test_pool().await;
    cleanup_test_data(&pool).await;

    let mut config = llm_observatory_storage::writers::log::WriterConfig::default();
    config.batch_size = 100;

    let writer = LogWriter::with_config(pool.clone(), config);
    let logs = create_test_logs(10, "test-service");

    writer.write_logs(logs).await.unwrap();

    let stats = writer.buffer_stats().await;
    assert_eq!(stats.logs_buffered, 10);

    writer.flush().await.unwrap();

    let stats = writer.buffer_stats().await;
    assert_eq!(stats.logs_buffered, 0);
}

#[tokio::test]
async fn test_log_writer_batch_auto_flush() {
    let (pool, _guard) = setup_test_pool().await;
    cleanup_test_data(&pool).await;

    let mut config = llm_observatory_storage::writers::log::WriterConfig::default();
    config.batch_size = 5;

    let writer = LogWriter::with_config(pool.clone(), config);

    // Write 4 logs - should not auto-flush
    for i in 0..4 {
        let log = create_test_log("test-service", "INFO", &format!("Message {}", i), None);
        writer.write_log(log).await.unwrap();
    }

    let stats = writer.buffer_stats().await;
    assert_eq!(stats.logs_buffered, 4);

    // Write 5th log - should trigger auto-flush
    let log = create_test_log("test-service", "INFO", "Message 4", None);
    writer.write_log(log).await.unwrap();

    // Buffer should be empty after auto-flush
    let stats = writer.buffer_stats().await;
    assert_eq!(stats.logs_buffered, 0);
}

#[tokio::test]
async fn test_log_writer_custom_attributes() {
    let (pool, _guard) = setup_test_pool().await;
    cleanup_test_data(&pool).await;

    let writer = LogWriter::new(pool.clone());
    let custom_attrs = serde_json::json!({
        "user_id": "user123",
        "request_id": "req456",
        "action": "purchase"
    });

    let log = create_custom_log("test-service", 9, "User action logged", custom_attrs);
    writer.write_log(log.clone()).await.unwrap();
    writer.flush().await.unwrap();

    // Verify custom attributes were stored
    let result: (serde_json::Value,) = sqlx::query_as("SELECT attributes FROM logs WHERE id = $1")
        .bind(log.id)
        .fetch_one(pool.postgres())
        .await
        .unwrap();

    assert_eq!(result.0["user_id"], "user123");
    assert_eq!(result.0["request_id"], "req456");
}

// ============================================================================
// Performance and Stress Tests
// ============================================================================

#[tokio::test]
async fn test_writer_concurrent_writes() {
    let (pool, _guard) = setup_test_pool().await;
    cleanup_test_data(&pool).await;

    let writer = LogWriter::new(pool.clone());

    // Spawn multiple concurrent write tasks
    let handles: Vec<_> = (0..10)
        .map(|i| {
            let writer_clone = writer.clone();
            tokio::spawn(async move {
                let log = create_test_log(
                    "test-service",
                    "INFO",
                    &format!("Concurrent log {}", i),
                    None,
                );
                writer_clone.write_log(log).await.unwrap();
            })
        })
        .collect();

    // Wait for all tasks to complete
    for handle in handles {
        handle.await.unwrap();
    }

    writer.flush().await.unwrap();

    // Verify all logs were written
    let result: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM logs")
        .fetch_one(pool.postgres())
        .await
        .unwrap();

    assert_eq!(result.0, 10);
}

#[tokio::test]
async fn test_writer_large_batch() {
    let (pool, _guard) = setup_test_pool().await;
    cleanup_test_data(&pool).await;

    let writer = TraceWriter::new(pool.clone());
    let traces = create_test_traces(100, "test-service");

    let start = std::time::Instant::now();
    writer.write_traces(traces).await.unwrap();
    writer.flush().await.unwrap();
    let elapsed = start.elapsed();

    println!("Wrote 100 traces in {:?}", elapsed);

    // Verify all traces were written
    let result: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM traces")
        .fetch_one(pool.postgres())
        .await
        .unwrap();

    assert_eq!(result.0, 100);
    assert!(elapsed.as_secs() < 5, "Batch write should complete quickly");
}
