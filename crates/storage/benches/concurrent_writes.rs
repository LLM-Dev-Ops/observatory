//! Concurrent write performance benchmarks.
//!
//! This benchmark suite measures write performance under concurrent load,
//! simulating real-world scenarios with multiple writers.
//!
//! ## Running the benchmark
//!
//! ```bash
//! cargo bench --bench concurrent_writes
//! ```
//!
//! ## Target Metrics
//!
//! - Concurrent throughput: scales linearly up to CPU cores
//! - Lock contention: minimal under concurrent load
//! - Write amplification: <2x baseline

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use llm_observatory_storage::writers::{CopyWriter, LogWriter, MetricWriter, TraceWriter};
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinSet;

mod common;
use common::{generate_logs, generate_metrics, generate_spans, generate_traces, BenchmarkContext};

/// Benchmark concurrent trace writes using COPY protocol.
fn bench_concurrent_trace_writes_copy(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let ctx = runtime.block_on(async { BenchmarkContext::new().await });

    let configs = vec![
        (2, 500),  // 2 writers, 500 traces each
        (4, 500),  // 4 writers, 500 traces each
        (8, 250),  // 8 writers, 250 traces each
        (16, 125), // 16 writers, 125 traces each
    ];

    let mut group = c.benchmark_group("concurrent_trace_writes_copy");
    group.measurement_time(Duration::from_secs(20));
    group.sample_size(10);

    for (writers, batch_size) in configs {
        let total = writers * batch_size;
        group.throughput(Throughput::Elements(total as u64));

        group.bench_with_input(
            BenchmarkId::new("writers", format!("{}_x_{}", writers, batch_size)),
            &(writers, batch_size),
            |b, &(writers, batch_size)| {
                b.to_async(&runtime).iter(|| async {
                    let mut tasks = JoinSet::new();

                    for _ in 0..writers {
                        let pool = ctx.pool.clone();
                        let traces = generate_traces(batch_size);

                        tasks.spawn(async move {
                            let (client, _handle) = pool.get_tokio_postgres_client().await.unwrap();
                            CopyWriter::write_traces(&client, traces).await.unwrap();
                        });
                    }

                    while tasks.join_next().await.is_some() {}
                    black_box(());
                });
            },
        );
    }

    group.finish();
}

/// Benchmark concurrent span writes using COPY protocol.
fn bench_concurrent_span_writes_copy(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let ctx = runtime.block_on(async { BenchmarkContext::new().await });

    let configs = vec![
        (2, 1000), // 2 writers, 1000 spans each
        (4, 1000), // 4 writers, 1000 spans each
        (8, 500),  // 8 writers, 500 spans each
        (16, 250), // 16 writers, 250 spans each
    ];

    let mut group = c.benchmark_group("concurrent_span_writes_copy");
    group.measurement_time(Duration::from_secs(20));
    group.sample_size(10);

    for (writers, batch_size) in configs {
        let total = writers * batch_size;
        group.throughput(Throughput::Elements(total as u64));

        group.bench_with_input(
            BenchmarkId::new("writers", format!("{}_x_{}", writers, batch_size)),
            &(writers, batch_size),
            |b, &(writers, batch_size)| {
                b.to_async(&runtime).iter(|| async {
                    let mut tasks = JoinSet::new();

                    for _ in 0..writers {
                        let pool = ctx.pool.clone();
                        let spans = generate_spans(batch_size);

                        tasks.spawn(async move {
                            let (client, _handle) = pool.get_tokio_postgres_client().await.unwrap();
                            CopyWriter::write_spans(&client, spans).await.unwrap();
                        });
                    }

                    while tasks.join_next().await.is_some() {}
                    black_box(());
                });
            },
        );
    }

    group.finish();
}

/// Benchmark concurrent log writes using COPY protocol.
fn bench_concurrent_log_writes_copy(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let ctx = runtime.block_on(async { BenchmarkContext::new().await });

    let configs = vec![
        (2, 2000), // 2 writers, 2000 logs each
        (4, 2000), // 4 writers, 2000 logs each
        (8, 1000), // 8 writers, 1000 logs each
        (16, 500), // 16 writers, 500 logs each
    ];

    let mut group = c.benchmark_group("concurrent_log_writes_copy");
    group.measurement_time(Duration::from_secs(20));
    group.sample_size(10);

    for (writers, batch_size) in configs {
        let total = writers * batch_size;
        group.throughput(Throughput::Elements(total as u64));

        group.bench_with_input(
            BenchmarkId::new("writers", format!("{}_x_{}", writers, batch_size)),
            &(writers, batch_size),
            |b, &(writers, batch_size)| {
                b.to_async(&runtime).iter(|| async {
                    let mut tasks = JoinSet::new();

                    for _ in 0..writers {
                        let pool = ctx.pool.clone();
                        let logs = generate_logs(batch_size);

                        tasks.spawn(async move {
                            let (client, _handle) = pool.get_tokio_postgres_client().await.unwrap();
                            CopyWriter::write_logs(&client, logs).await.unwrap();
                        });
                    }

                    while tasks.join_next().await.is_some() {}
                    black_box(());
                });
            },
        );
    }

    group.finish();
}

/// Benchmark concurrent writes with buffered writers.
fn bench_concurrent_buffered_writes(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let ctx = runtime.block_on(async { BenchmarkContext::new().await });

    let configs = vec![(2, 500), (4, 500), (8, 250)];

    let mut group = c.benchmark_group("concurrent_buffered_writes");
    group.measurement_time(Duration::from_secs(20));
    group.sample_size(10);

    for (writers, batch_size) in configs {
        let total = writers * batch_size;
        group.throughput(Throughput::Elements(total as u64));

        group.bench_with_input(
            BenchmarkId::new("writers", format!("{}_x_{}", writers, batch_size)),
            &(writers, batch_size),
            |b, &(writers, batch_size)| {
                b.to_async(&runtime).iter(|| async {
                    let mut tasks = JoinSet::new();

                    for _ in 0..writers {
                        let pool = ctx.pool.clone();
                        let traces = generate_traces(batch_size);

                        tasks.spawn(async move {
                            let writer = TraceWriter::new(pool);
                            writer.write_traces(traces).await.unwrap();
                            writer.flush().await.unwrap();
                        });
                    }

                    while tasks.join_next().await.is_some() {}
                    black_box(());
                });
            },
        );
    }

    group.finish();
}

/// Benchmark concurrent mixed writes (traces + spans + logs).
fn bench_concurrent_mixed_writes(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let ctx = runtime.block_on(async { BenchmarkContext::new().await });

    let mut group = c.benchmark_group("concurrent_mixed_writes");
    group.measurement_time(Duration::from_secs(20));
    group.sample_size(10);

    group.bench_function("4_writers_mixed", |b| {
        b.to_async(&runtime).iter(|| async {
            let mut tasks = JoinSet::new();

            // 2 trace writers
            for _ in 0..2 {
                let pool = ctx.pool.clone();
                let traces = generate_traces(200);

                tasks.spawn(async move {
                    let (client, _handle) = pool.get_tokio_postgres_client().await.unwrap();
                    CopyWriter::write_traces(&client, traces).await.unwrap();
                });
            }

            // 1 span writer
            {
                let pool = ctx.pool.clone();
                let spans = generate_spans(500);

                tasks.spawn(async move {
                    let (client, _handle) = pool.get_tokio_postgres_client().await.unwrap();
                    CopyWriter::write_spans(&client, spans).await.unwrap();
                });
            }

            // 1 log writer
            {
                let pool = ctx.pool.clone();
                let logs = generate_logs(500);

                tasks.spawn(async move {
                    let (client, _handle) = pool.get_tokio_postgres_client().await.unwrap();
                    CopyWriter::write_logs(&client, logs).await.unwrap();
                });
            }

            while tasks.join_next().await.is_some() {}
            black_box(());
        });
    });

    group.finish();
}

/// Benchmark write contention with shared writer instances.
fn bench_shared_writer_contention(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let ctx = runtime.block_on(async { BenchmarkContext::new().await });

    let mut group = c.benchmark_group("shared_writer_contention");
    group.measurement_time(Duration::from_secs(15));
    group.sample_size(10);

    // Shared writer scenario - multiple tasks using same writer
    group.bench_function("shared_writer_4_tasks", |b| {
        b.to_async(&runtime).iter(|| async {
            let writer = Arc::new(TraceWriter::new(ctx.pool.clone()));
            let mut tasks = JoinSet::new();

            for _ in 0..4 {
                let writer = writer.clone();
                let traces = generate_traces(100);

                tasks.spawn(async move {
                    writer.write_traces(traces).await.unwrap();
                });
            }

            while tasks.join_next().await.is_some() {}

            writer.flush().await.unwrap();
            black_box(());
        });
    });

    // Independent writer scenario - each task has its own writer
    group.bench_function("independent_writers_4_tasks", |b| {
        b.to_async(&runtime).iter(|| async {
            let mut tasks = JoinSet::new();

            for _ in 0..4 {
                let pool = ctx.pool.clone();
                let traces = generate_traces(100);

                tasks.spawn(async move {
                    let writer = TraceWriter::new(pool);
                    writer.write_traces(traces).await.unwrap();
                    writer.flush().await.unwrap();
                });
            }

            while tasks.join_next().await.is_some() {}
            black_box(());
        });
    });

    group.finish();
}

/// Benchmark scaling efficiency across different concurrency levels.
fn bench_concurrent_scaling(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let ctx = runtime.block_on(async { BenchmarkContext::new().await });

    // Fixed total workload (4000 spans), distributed across different numbers of workers
    let configs = vec![
        (1, 4000), // Baseline
        (2, 2000),
        (4, 1000),
        (8, 500),
        (16, 250),
    ];

    let mut group = c.benchmark_group("concurrent_scaling");
    group.measurement_time(Duration::from_secs(20));
    group.sample_size(10);

    for (workers, batch_size) in configs {
        group.throughput(Throughput::Elements(4000)); // Total is always 4000

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_workers", workers)),
            &(workers, batch_size),
            |b, &(workers, batch_size)| {
                b.to_async(&runtime).iter(|| async {
                    let mut tasks = JoinSet::new();

                    for _ in 0..workers {
                        let pool = ctx.pool.clone();
                        let spans = generate_spans(batch_size);

                        tasks.spawn(async move {
                            let (client, _handle) = pool.get_tokio_postgres_client().await.unwrap();
                            CopyWriter::write_spans(&client, spans).await.unwrap();
                        });
                    }

                    while tasks.join_next().await.is_some() {}
                    black_box(());
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_concurrent_trace_writes_copy,
    bench_concurrent_span_writes_copy,
    bench_concurrent_log_writes_copy,
    bench_concurrent_buffered_writes,
    bench_concurrent_mixed_writes,
    bench_shared_writer_contention,
    bench_concurrent_scaling,
);

criterion_main!(benches);
