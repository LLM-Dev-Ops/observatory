//! Writer throughput benchmarks for traces, metrics, and logs.
//!
//! This benchmark suite measures write throughput across different batch sizes
//! and data types to establish baseline performance expectations.
//!
//! ## Running the benchmark
//!
//! ```bash
//! # Using testcontainers (recommended - isolated testing)
//! cargo bench --bench writer_throughput
//!
//! # Using existing database (faster startup)
//! export DATABASE_URL="postgres://postgres:password@localhost/llm_observatory_bench"
//! cargo bench --bench writer_throughput -- --skip-containers
//! ```
//!
//! ## Target Metrics
//!
//! - Write throughput: >10,000 spans/sec
//! - Batch processing: 100-10,000 records per batch
//! - COPY protocol: 50,000-100,000 rows/sec

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use llm_observatory_storage::{
    models::{LogRecord, Metric, MetricDataPoint, Trace, TraceSpan},
    writers::{CopyWriter, LogWriter, MetricWriter, TraceWriter},
    StorageConfig, StoragePool,
};
use std::time::Duration;

mod common;
use common::{
    generate_logs, generate_metric_data_points, generate_metrics, generate_spans, generate_traces,
    setup_test_container, BenchmarkContext,
};

/// Benchmark trace writer throughput using COPY protocol.
fn bench_trace_writer_copy(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let ctx = runtime.block_on(async { BenchmarkContext::new().await });

    let batch_sizes = vec![100, 500, 1000, 5000, 10000];
    let mut group = c.benchmark_group("trace_writer_copy");
    group.sample_size(20);

    for &size in &batch_sizes {
        group.throughput(Throughput::Elements(size as u64));
        group.measurement_time(Duration::from_secs(15));

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.to_async(&runtime).iter(|| async {
                let traces = generate_traces(size);
                let (client, _handle) = ctx.pool.get_tokio_postgres_client().await.unwrap();

                black_box(CopyWriter::write_traces(&client, traces).await.unwrap());
            });
        });
    }

    group.finish();
}

/// Benchmark span writer throughput using COPY protocol.
fn bench_span_writer_copy(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let ctx = runtime.block_on(async { BenchmarkContext::new().await });

    let batch_sizes = vec![100, 500, 1000, 5000, 10000];
    let mut group = c.benchmark_group("span_writer_copy");
    group.sample_size(20);

    for &size in &batch_sizes {
        group.throughput(Throughput::Elements(size as u64));
        group.measurement_time(Duration::from_secs(15));

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.to_async(&runtime).iter(|| async {
                let spans = generate_spans(size);
                let (client, _handle) = ctx.pool.get_tokio_postgres_client().await.unwrap();

                black_box(CopyWriter::write_spans(&client, spans).await.unwrap());
            });
        });
    }

    group.finish();
}

/// Benchmark log writer throughput using COPY protocol.
fn bench_log_writer_copy(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let ctx = runtime.block_on(async { BenchmarkContext::new().await });

    let batch_sizes = vec![100, 500, 1000, 5000, 10000];
    let mut group = c.benchmark_group("log_writer_copy");
    group.sample_size(20);

    for &size in &batch_sizes {
        group.throughput(Throughput::Elements(size as u64));
        group.measurement_time(Duration::from_secs(15));

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.to_async(&runtime).iter(|| async {
                let logs = generate_logs(size);
                let (client, _handle) = ctx.pool.get_tokio_postgres_client().await.unwrap();

                black_box(CopyWriter::write_logs(&client, logs).await.unwrap());
            });
        });
    }

    group.finish();
}

/// Benchmark trace writer with buffering (INSERT-based).
fn bench_trace_writer_buffered(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let ctx = runtime.block_on(async { BenchmarkContext::new().await });

    let batch_sizes = vec![100, 500, 1000, 2000];
    let mut group = c.benchmark_group("trace_writer_buffered");
    group.sample_size(15);

    for &size in &batch_sizes {
        group.throughput(Throughput::Elements(size as u64));
        group.measurement_time(Duration::from_secs(15));

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.to_async(&runtime).iter(|| async {
                let traces = generate_traces(size);
                let writer = TraceWriter::new(ctx.pool.clone());

                writer.write_traces(traces).await.unwrap();
                black_box(writer.flush().await.unwrap());
            });
        });
    }

    group.finish();
}

/// Benchmark span writer with buffering (INSERT-based).
fn bench_span_writer_buffered(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let ctx = runtime.block_on(async { BenchmarkContext::new().await });

    let batch_sizes = vec![100, 500, 1000, 2000];
    let mut group = c.benchmark_group("span_writer_buffered");
    group.sample_size(15);

    for &size in &batch_sizes {
        group.throughput(Throughput::Elements(size as u64));
        group.measurement_time(Duration::from_secs(15));

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.to_async(&runtime).iter(|| async {
                let spans = generate_spans(size);
                let writer = TraceWriter::new(ctx.pool.clone());

                writer.write_spans(spans).await.unwrap();
                black_box(writer.flush().await.unwrap());
            });
        });
    }

    group.finish();
}

/// Benchmark metric writer throughput.
fn bench_metric_writer(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let ctx = runtime.block_on(async { BenchmarkContext::new().await });

    let batch_sizes = vec![100, 500, 1000, 5000];
    let mut group = c.benchmark_group("metric_writer");
    group.sample_size(15);

    for &size in &batch_sizes {
        group.throughput(Throughput::Elements(size as u64));
        group.measurement_time(Duration::from_secs(15));

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.to_async(&runtime).iter(|| async {
                let metrics = generate_metrics(size);
                let writer = MetricWriter::new(ctx.pool.clone());

                writer.write_metrics(metrics).await.unwrap();
                black_box(writer.flush().await.unwrap());
            });
        });
    }

    group.finish();
}

/// Benchmark metric data point writer throughput.
fn bench_metric_data_point_writer(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let ctx = runtime.block_on(async { BenchmarkContext::new().await });

    // Setup: create a metric to reference
    let metric_id = runtime.block_on(async {
        let metrics = generate_metrics(1);
        let writer = MetricWriter::new(ctx.pool.clone());
        writer.write_metrics(metrics.clone()).await.unwrap();
        writer.flush().await.unwrap();
        metrics[0].id
    });

    let batch_sizes = vec![100, 500, 1000, 5000, 10000];
    let mut group = c.benchmark_group("metric_data_point_writer");
    group.sample_size(15);

    for &size in &batch_sizes {
        group.throughput(Throughput::Elements(size as u64));
        group.measurement_time(Duration::from_secs(15));

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.to_async(&runtime).iter(|| async {
                let data_points = generate_metric_data_points(size, metric_id);
                let writer = MetricWriter::new(ctx.pool.clone());

                writer.write_data_points(data_points).await.unwrap();
                black_box(writer.flush().await.unwrap());
            });
        });
    }

    group.finish();
}

/// Benchmark log writer throughput.
fn bench_log_writer_buffered(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let ctx = runtime.block_on(async { BenchmarkContext::new().await });

    let batch_sizes = vec![100, 500, 1000, 5000, 10000];
    let mut group = c.benchmark_group("log_writer_buffered");
    group.sample_size(15);

    for &size in &batch_sizes {
        group.throughput(Throughput::Elements(size as u64));
        group.measurement_time(Duration::from_secs(15));

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.to_async(&runtime).iter(|| async {
                let logs = generate_logs(size);
                let writer = LogWriter::new(ctx.pool.clone());

                writer.write_logs(logs).await.unwrap();
                black_box(writer.flush().await.unwrap());
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_trace_writer_copy,
    bench_span_writer_copy,
    bench_log_writer_copy,
    bench_trace_writer_buffered,
    bench_span_writer_buffered,
    bench_metric_writer,
    bench_metric_data_point_writer,
    bench_log_writer_buffered,
);

criterion_main!(benches);
