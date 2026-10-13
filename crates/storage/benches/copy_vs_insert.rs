//! Benchmark comparing INSERT vs COPY protocol performance.
//!
//! This benchmark measures the throughput difference between standard batch INSERT
//! statements and PostgreSQL's COPY protocol for bulk data insertion.
//!
//! ## Running the benchmark
//!
//! ```bash
//! # Set up test database
//! export DATABASE_URL="postgres://postgres:password@localhost/llm_observatory_bench"
//!
//! # Run benchmarks
//! cargo bench --bench copy_vs_insert
//! ```
//!
//! ## Expected Results
//!
//! - INSERT: 5,000-10,000 rows/sec
//! - COPY: 50,000-100,000 rows/sec
//! - Speedup: 10-100x depending on data complexity

use chrono::Utc;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use llm_observatory_storage::{
    models::{LogRecord, Metric, MetricDataPoint, Trace, TraceEvent, TraceSpan},
    writers::CopyWriter,
    StorageConfig, StoragePool,
};
use std::time::Duration;
use uuid::Uuid;

/// Generate test traces for benchmarking.
fn generate_traces(count: usize) -> Vec<Trace> {
    (0..count)
        .map(|i| {
            let now = Utc::now();
            Trace {
                id: Uuid::new_v4(),
                trace_id: format!("trace_{}", i),
                service_name: "benchmark-service".to_string(),
                start_time: now,
                end_time: Some(now + chrono::Duration::milliseconds(100)),
                duration_us: Some(100_000),
                status: "ok".to_string(),
                status_message: None,
                root_span_name: Some("root_span".to_string()),
                attributes: serde_json::json!({
                    "test": "benchmark",
                    "index": i,
                }),
                resource_attributes: serde_json::json!({
                    "service.name": "benchmark",
                }),
                span_count: 1,
                created_at: now,
                updated_at: now,
            }
        })
        .collect()
}

/// Generate test spans for benchmarking.
fn generate_spans(count: usize) -> Vec<TraceSpan> {
    (0..count)
        .map(|i| {
            let now = Utc::now();
            TraceSpan {
                id: Uuid::new_v4(),
                trace_id: Uuid::new_v4(),
                span_id: format!("span_{}", i),
                parent_span_id: None,
                name: format!("operation_{}", i % 10),
                kind: "internal".to_string(),
                service_name: "benchmark-service".to_string(),
                start_time: now,
                end_time: Some(now + chrono::Duration::milliseconds(50)),
                duration_us: Some(50_000),
                status: "ok".to_string(),
                status_message: None,
                attributes: serde_json::json!({
                    "span.kind": "internal",
                    "index": i,
                }),
                events: None,
                links: None,
                created_at: now,
            }
        })
        .collect()
}

/// Generate test logs for benchmarking.
fn generate_logs(count: usize) -> Vec<LogRecord> {
    (0..count)
        .map(|i| {
            let now = Utc::now();
            LogRecord {
                id: Uuid::new_v4(),
                timestamp: now,
                observed_timestamp: now,
                severity_number: 9, // INFO
                severity_text: "INFO".to_string(),
                body: format!("Benchmark log message {}", i),
                service_name: "benchmark-service".to_string(),
                trace_id: None,
                span_id: None,
                trace_flags: None,
                attributes: serde_json::json!({
                    "log.index": i,
                }),
                resource_attributes: serde_json::json!({
                    "service.name": "benchmark",
                }),
                scope_name: Some("benchmark".to_string()),
                scope_version: Some("1.0.0".to_string()),
                scope_attributes: None,
                created_at: now,
            }
        })
        .collect()
}

/// Benchmark trace insertion using COPY protocol.
fn bench_copy_traces(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    // Try to load config from environment, skip benchmark if not available
    let config = match StorageConfig::from_env() {
        Ok(cfg) => cfg,
        Err(_) => {
            eprintln!("Skipping benchmark: DATABASE_URL not set");
            return;
        }
    };

    let pool = runtime.block_on(async {
        StoragePool::new(config)
            .await
            .expect("Failed to create pool")
    });

    let batch_sizes = vec![100, 1000, 5000, 10000];

    let mut group = c.benchmark_group("traces_copy");

    for &size in &batch_sizes {
        group.throughput(Throughput::Elements(size as u64));
        group.measurement_time(Duration::from_secs(20));

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.to_async(&runtime).iter(|| async {
                let traces = generate_traces(size);
                let (client, _handle) = pool.get_tokio_postgres_client().await.unwrap();

                black_box(CopyWriter::write_traces(&client, traces).await.unwrap());
            });
        });
    }

    group.finish();
}

/// Benchmark span insertion using COPY protocol.
fn bench_copy_spans(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    let config = match StorageConfig::from_env() {
        Ok(cfg) => cfg,
        Err(_) => {
            eprintln!("Skipping benchmark: DATABASE_URL not set");
            return;
        }
    };

    let pool = runtime.block_on(async {
        StoragePool::new(config)
            .await
            .expect("Failed to create pool")
    });

    let batch_sizes = vec![100, 1000, 5000, 10000];

    let mut group = c.benchmark_group("spans_copy");

    for &size in &batch_sizes {
        group.throughput(Throughput::Elements(size as u64));
        group.measurement_time(Duration::from_secs(20));

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.to_async(&runtime).iter(|| async {
                let spans = generate_spans(size);
                let (client, _handle) = pool.get_tokio_postgres_client().await.unwrap();

                black_box(CopyWriter::write_spans(&client, spans).await.unwrap());
            });
        });
    }

    group.finish();
}

/// Benchmark log insertion using COPY protocol.
fn bench_copy_logs(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    let config = match StorageConfig::from_env() {
        Ok(cfg) => cfg,
        Err(_) => {
            eprintln!("Skipping benchmark: DATABASE_URL not set");
            return;
        }
    };

    let pool = runtime.block_on(async {
        StoragePool::new(config)
            .await
            .expect("Failed to create pool")
    });

    let batch_sizes = vec![100, 1000, 5000, 10000];

    let mut group = c.benchmark_group("logs_copy");

    for &size in &batch_sizes {
        group.throughput(Throughput::Elements(size as u64));
        group.measurement_time(Duration::from_secs(20));

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.to_async(&runtime).iter(|| async {
                let logs = generate_logs(size);
                let (client, _handle) = pool.get_tokio_postgres_client().await.unwrap();

                black_box(CopyWriter::write_logs(&client, logs).await.unwrap());
            });
        });
    }

    group.finish();
}

/// Benchmark trace insertion using INSERT (QueryBuilder).
fn bench_insert_traces(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    let config = match StorageConfig::from_env() {
        Ok(cfg) => cfg,
        Err(_) => {
            eprintln!("Skipping benchmark: DATABASE_URL not set");
            return;
        }
    };

    let pool = runtime.block_on(async {
        StoragePool::new(config)
            .await
            .expect("Failed to create pool")
    });

    let batch_sizes = vec![100, 1000, 5000];

    let mut group = c.benchmark_group("traces_insert");

    for &size in &batch_sizes {
        group.throughput(Throughput::Elements(size as u64));
        group.measurement_time(Duration::from_secs(20));

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.to_async(&runtime).iter(|| async {
                let traces = generate_traces(size);

                // Use QueryBuilder for batch insert
                let mut query_builder = sqlx::QueryBuilder::new(
                    "INSERT INTO traces (id, trace_id, service_name, start_time, end_time, duration_us, \
                     status, status_message, root_span_name, attributes, resource_attributes, span_count, \
                     created_at, updated_at) "
                );

                query_builder.push_values(traces, |mut b, trace| {
                    b.push_bind(trace.id)
                        .push_bind(trace.trace_id)
                        .push_bind(trace.service_name)
                        .push_bind(trace.start_time)
                        .push_bind(trace.end_time)
                        .push_bind(trace.duration_us)
                        .push_bind(trace.status)
                        .push_bind(trace.status_message)
                        .push_bind(trace.root_span_name)
                        .push_bind(trace.attributes)
                        .push_bind(trace.resource_attributes)
                        .push_bind(trace.span_count)
                        .push_bind(trace.created_at)
                        .push_bind(trace.updated_at);
                });

                black_box(query_builder.build().execute(pool.postgres()).await.unwrap());
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_copy_traces,
    bench_copy_spans,
    bench_copy_logs,
    bench_insert_traces,
);

criterion_main!(benches);
