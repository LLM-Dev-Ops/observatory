//! Mixed read/write workload benchmarks.
//!
//! This benchmark suite simulates realistic production scenarios with
//! concurrent reads and writes to measure overall system performance.
//!
//! ## Running the benchmark
//!
//! ```bash
//! cargo bench --bench mixed_workload
//! ```
//!
//! ## Target Metrics
//!
//! - Mixed workload throughput: >5,000 ops/sec
//! - Read latency under write load: P95 <100ms
//! - Write latency under read load: minimal degradation

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use llm_observatory_storage::{
    repositories::{LogRepository, TraceRepository},
    writers::{CopyWriter, LogWriter, TraceWriter},
};
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinSet;

mod common;
use common::{generate_logs, generate_spans, generate_traces, BenchmarkContext};

/// Setup benchmark data for mixed workload tests.
async fn setup_workload_data(ctx: &BenchmarkContext) -> Vec<uuid::Uuid> {
    // Insert 5,000 traces for querying
    let traces = generate_traces(5000);
    let trace_ids: Vec<uuid::Uuid> = traces.iter().map(|t| t.id).collect();

    let writer = TraceWriter::new(ctx.pool.clone());
    writer.write_traces(traces).await.unwrap();
    writer.flush().await.unwrap();

    // Insert 20,000 spans
    let spans = generate_spans(20000);
    writer.write_spans(spans).await.unwrap();
    writer.flush().await.unwrap();

    // Insert 10,000 logs
    let logs = generate_logs(10000);
    let log_writer = LogWriter::new(ctx.pool.clone());
    log_writer.write_logs(logs).await.unwrap();
    log_writer.flush().await.unwrap();

    trace_ids
}

/// Benchmark read-heavy workload (80% reads, 20% writes).
fn bench_read_heavy_workload(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let ctx = runtime.block_on(async { BenchmarkContext::new().await });
    let trace_ids = runtime.block_on(async { setup_workload_data(&ctx).await });

    let mut group = c.benchmark_group("read_heavy_workload");
    group.measurement_time(Duration::from_secs(20));
    group.sample_size(10);

    group.bench_function("80_read_20_write", |b| {
        b.to_async(&runtime).iter(|| async {
            let mut tasks = JoinSet::new();

            // 8 read tasks
            for _ in 0..8 {
                let pool = ctx.pool.clone();
                let trace_ids = trace_ids.clone();

                tasks.spawn(async move {
                    let repository = TraceRepository::new(pool);
                    let trace_id = trace_ids[rand::random::<usize>() % trace_ids.len()];
                    let _ = repository.get_by_id(trace_id).await;
                });
            }

            // 2 write tasks
            for _ in 0..2 {
                let pool = ctx.pool.clone();
                let spans = generate_spans(50);

                tasks.spawn(async move {
                    let (client, _handle) = pool.get_tokio_postgres_client().await.unwrap();
                    CopyWriter::write_spans(&client, spans).await.unwrap();
                });
            }

            while tasks.join_next().await.is_some() {}
            black_box(());
        });
    });

    group.finish();
}

/// Benchmark write-heavy workload (20% reads, 80% writes).
fn bench_write_heavy_workload(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let ctx = runtime.block_on(async { BenchmarkContext::new().await });
    let trace_ids = runtime.block_on(async { setup_workload_data(&ctx).await });

    let mut group = c.benchmark_group("write_heavy_workload");
    group.measurement_time(Duration::from_secs(20));
    group.sample_size(10);

    group.bench_function("20_read_80_write", |b| {
        b.to_async(&runtime).iter(|| async {
            let mut tasks = JoinSet::new();

            // 2 read tasks
            for _ in 0..2 {
                let pool = ctx.pool.clone();
                let trace_ids = trace_ids.clone();

                tasks.spawn(async move {
                    let repository = TraceRepository::new(pool);
                    let trace_id = trace_ids[rand::random::<usize>() % trace_ids.len()];
                    let _ = repository.get_by_id(trace_id).await;
                });
            }

            // 8 write tasks
            for _ in 0..8 {
                let pool = ctx.pool.clone();
                let spans = generate_spans(50);

                tasks.spawn(async move {
                    let (client, _handle) = pool.get_tokio_postgres_client().await.unwrap();
                    CopyWriter::write_spans(&client, spans).await.unwrap();
                });
            }

            while tasks.join_next().await.is_some() {}
            black_box(());
        });
    });

    group.finish();
}

/// Benchmark balanced workload (50% reads, 50% writes).
fn bench_balanced_workload(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let ctx = runtime.block_on(async { BenchmarkContext::new().await });
    let trace_ids = runtime.block_on(async { setup_workload_data(&ctx).await });

    let mut group = c.benchmark_group("balanced_workload");
    group.measurement_time(Duration::from_secs(20));
    group.sample_size(10);

    group.bench_function("50_read_50_write", |b| {
        b.to_async(&runtime).iter(|| async {
            let mut tasks = JoinSet::new();

            // 5 read tasks
            for _ in 0..5 {
                let pool = ctx.pool.clone();
                let trace_ids = trace_ids.clone();

                tasks.spawn(async move {
                    let repository = TraceRepository::new(pool);
                    let trace_id = trace_ids[rand::random::<usize>() % trace_ids.len()];
                    let _ = repository.get_by_id(trace_id).await;
                });
            }

            // 5 write tasks
            for _ in 0..5 {
                let pool = ctx.pool.clone();
                let spans = generate_spans(50);

                tasks.spawn(async move {
                    let (client, _handle) = pool.get_tokio_postgres_client().await.unwrap();
                    CopyWriter::write_spans(&client, spans).await.unwrap();
                });
            }

            while tasks.join_next().await.is_some() {}
            black_box(());
        });
    });

    group.finish();
}

/// Benchmark complex queries under write load.
fn bench_complex_queries_under_write_load(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let ctx = runtime.block_on(async { BenchmarkContext::new().await });
    runtime.block_on(async { setup_workload_data(&ctx).await });

    let mut group = c.benchmark_group("complex_queries_under_write_load");
    group.measurement_time(Duration::from_secs(20));
    group.sample_size(10);

    group.bench_function("list_traces_with_concurrent_writes", |b| {
        b.to_async(&runtime).iter(|| async {
            let mut tasks = JoinSet::new();

            // 1 complex query task
            {
                let pool = ctx.pool.clone();
                tasks.spawn(async move {
                    let repository = TraceRepository::new(pool);
                    let filters = llm_observatory_storage::repositories::TraceFilters {
                        service_name: Some("benchmark-service".to_string()),
                        status: None,
                        start_time: None,
                        end_time: None,
                        min_duration_us: Some(50000),
                        max_duration_us: None,
                        limit: Some(100),
                        offset: None,
                    };
                    let _ = repository.list(filters).await;
                });
            }

            // 3 concurrent write tasks
            for _ in 0..3 {
                let pool = ctx.pool.clone();
                let spans = generate_spans(100);

                tasks.spawn(async move {
                    let (client, _handle) = pool.get_tokio_postgres_client().await.unwrap();
                    CopyWriter::write_spans(&client, spans).await.unwrap();
                });
            }

            while tasks.join_next().await.is_some() {}
            black_box(());
        });
    });

    group.finish();
}

/// Benchmark realistic application workload.
fn bench_realistic_application_workload(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let ctx = runtime.block_on(async { BenchmarkContext::new().await });
    let trace_ids = runtime.block_on(async { setup_workload_data(&ctx).await });

    let mut group = c.benchmark_group("realistic_application_workload");
    group.measurement_time(Duration::from_secs(25));
    group.sample_size(10);

    // Simulate realistic mix: frequent writes, occasional reads, rare complex queries
    group.bench_function("realistic_mix", |b| {
        b.to_async(&runtime).iter(|| async {
            let mut tasks = JoinSet::new();

            // 6 write tasks (traces, spans, logs)
            for i in 0..6 {
                let pool = ctx.pool.clone();

                tasks.spawn(async move {
                    let (client, _handle) = pool.get_tokio_postgres_client().await.unwrap();

                    match i % 3 {
                        0 => {
                            let traces = generate_traces(20);
                            CopyWriter::write_traces(&client, traces).await.unwrap();
                        }
                        1 => {
                            let spans = generate_spans(100);
                            CopyWriter::write_spans(&client, spans).await.unwrap();
                        }
                        _ => {
                            let logs = generate_logs(50);
                            CopyWriter::write_logs(&client, logs).await.unwrap();
                        }
                    }
                });
            }

            // 3 simple read tasks
            for _ in 0..3 {
                let pool = ctx.pool.clone();
                let trace_ids = trace_ids.clone();

                tasks.spawn(async move {
                    let repository = TraceRepository::new(pool);
                    let trace_id = trace_ids[rand::random::<usize>() % trace_ids.len()];
                    let _ = repository.get_by_id(trace_id).await;
                });
            }

            // 1 complex query task
            {
                let pool = ctx.pool.clone();
                tasks.spawn(async move {
                    let repository = LogRepository::new(pool);
                    let filters = llm_observatory_storage::repositories::LogFilters {
                        service_name: Some("benchmark-service".to_string()),
                        severity_min: Some(13), // WARN
                        severity_max: None,
                        start_time: None,
                        end_time: None,
                        trace_id: None,
                        search_body: None,
                        limit: Some(100),
                        offset: None,
                    };
                    let _ = repository.list(filters).await;
                });
            }

            while tasks.join_next().await.is_some() {}
            black_box(());
        });
    });

    group.finish();
}

/// Benchmark sustained mixed workload over time.
fn bench_sustained_mixed_workload(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let ctx = runtime.block_on(async { BenchmarkContext::new().await });
    let trace_ids = runtime.block_on(async { setup_workload_data(&ctx).await });

    let mut group = c.benchmark_group("sustained_mixed_workload");
    group.measurement_time(Duration::from_secs(30));
    group.sample_size(10);

    // Run 100 operations in parallel to simulate sustained load
    group.bench_function("100_concurrent_operations", |b| {
        b.to_async(&runtime).iter(|| async {
            let mut tasks = JoinSet::new();

            for i in 0..100 {
                let pool = ctx.pool.clone();
                let trace_ids = trace_ids.clone();

                tasks.spawn(async move {
                    // 60% writes, 40% reads
                    if i < 60 {
                        let (client, _handle) = pool.get_tokio_postgres_client().await.unwrap();
                        let spans = generate_spans(10);
                        CopyWriter::write_spans(&client, spans).await.unwrap();
                    } else {
                        let repository = TraceRepository::new(pool);
                        let trace_id = trace_ids[rand::random::<usize>() % trace_ids.len()];
                        let _ = repository.get_by_id(trace_id).await;
                    }
                });
            }

            while tasks.join_next().await.is_some() {}
            black_box(());
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_read_heavy_workload,
    bench_write_heavy_workload,
    bench_balanced_workload,
    bench_complex_queries_under_write_load,
    bench_realistic_application_workload,
    bench_sustained_mixed_workload,
);

criterion_main!(benches);
