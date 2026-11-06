//! Connection pool performance benchmarks.
//!
//! This benchmark suite measures connection pool behavior under various
//! load conditions to ensure efficient connection reuse and minimal contention.
//!
//! ## Running the benchmark
//!
//! ```bash
//! cargo bench --bench pool_performance
//! ```
//!
//! ## Target Metrics
//!
//! - Connection acquisition: <1ms P95
//! - Pool saturation handling: graceful degradation
//! - Concurrent access: minimal contention

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use llm_observatory_storage::{writers::CopyWriter, StorageConfig, StoragePool};
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinSet;

mod common;
use common::{generate_spans, BenchmarkContext};

/// Benchmark connection acquisition latency.
fn bench_connection_acquisition(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let ctx = runtime.block_on(async { BenchmarkContext::new().await });

    let mut group = c.benchmark_group("connection_acquisition");
    group.measurement_time(Duration::from_secs(10));

    group.bench_function("single_connection", |b| {
        b.to_async(&runtime).iter(|| async {
            black_box(ctx.pool.get_tokio_postgres_client().await.unwrap());
        });
    });

    group.finish();
}

/// Benchmark pool behavior under concurrent load.
fn bench_concurrent_connections(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let ctx = runtime.block_on(async { BenchmarkContext::new().await });

    let concurrency_levels = vec![2, 5, 10, 20];
    let mut group = c.benchmark_group("concurrent_connections");
    group.measurement_time(Duration::from_secs(15));

    for &concurrency in &concurrency_levels {
        group.throughput(Throughput::Elements(concurrency as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(concurrency),
            &concurrency,
            |b, &concurrency| {
                b.to_async(&runtime).iter(|| async {
                    let mut tasks = JoinSet::new();

                    for _ in 0..concurrency {
                        let pool = ctx.pool.clone();
                        tasks.spawn(async move {
                            let (client, _handle) = pool.get_tokio_postgres_client().await.unwrap();
                            // Simulate a simple query
                            let _ = client.query("SELECT 1", &[]).await;
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

/// Benchmark pool with concurrent writes.
fn bench_concurrent_writes(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let ctx = runtime.block_on(async { BenchmarkContext::new().await });

    let concurrency_levels = vec![2, 5, 10];
    let batch_size = 100;

    let mut group = c.benchmark_group("concurrent_writes");
    group.measurement_time(Duration::from_secs(20));
    group.sample_size(10);

    for &concurrency in &concurrency_levels {
        group.throughput(Throughput::Elements((concurrency * batch_size) as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(concurrency),
            &concurrency,
            |b, &concurrency| {
                b.to_async(&runtime).iter(|| async {
                    let mut tasks = JoinSet::new();

                    for _ in 0..concurrency {
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

/// Benchmark pool recovery from saturation.
fn bench_pool_saturation_recovery(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let ctx = runtime.block_on(async { BenchmarkContext::new().await });

    let mut group = c.benchmark_group("pool_saturation_recovery");
    group.measurement_time(Duration::from_secs(15));
    group.sample_size(10);

    // Saturate the pool (assuming max_connections = 10)
    group.bench_function("saturate_and_recover", |b| {
        b.to_async(&runtime).iter(|| async {
            // Saturate with 20 concurrent requests (2x pool size)
            let mut tasks = JoinSet::new();

            for _ in 0..20 {
                let pool = ctx.pool.clone();
                tasks.spawn(async move {
                    let (client, _handle) = pool.get_tokio_postgres_client().await.unwrap();
                    // Hold connection briefly
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    let _ = client.query("SELECT 1", &[]).await;
                });
            }

            while tasks.join_next().await.is_some() {}
            black_box(());
        });
    });

    group.finish();
}

/// Benchmark pool with mixed read/write workload.
fn bench_mixed_workload(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let ctx = runtime.block_on(async { BenchmarkContext::new().await });

    let mut group = c.benchmark_group("mixed_workload");
    group.measurement_time(Duration::from_secs(20));
    group.sample_size(10);

    group.bench_function("70_read_30_write", |b| {
        b.to_async(&runtime).iter(|| async {
            let mut tasks = JoinSet::new();

            // 70% reads
            for _ in 0..7 {
                let pool = ctx.pool.clone();
                tasks.spawn(async move {
                    let (client, _handle) = pool.get_tokio_postgres_client().await.unwrap();
                    let _ = client.query("SELECT * FROM traces LIMIT 10", &[]).await;
                });
            }

            // 30% writes
            for _ in 0..3 {
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

/// Benchmark connection reuse efficiency.
fn bench_connection_reuse(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let ctx = runtime.block_on(async { BenchmarkContext::new().await });

    let mut group = c.benchmark_group("connection_reuse");
    group.measurement_time(Duration::from_secs(10));

    // Sequential operations to test connection reuse
    group.bench_function("sequential_10_operations", |b| {
        b.to_async(&runtime).iter(|| async {
            for _ in 0..10 {
                let (client, _handle) = ctx.pool.get_tokio_postgres_client().await.unwrap();
                black_box(client.query("SELECT 1", &[]).await.unwrap());
            }
        });
    });

    group.finish();
}

/// Benchmark pool initialization time.
fn bench_pool_initialization(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    // Get config from existing context
    let config = runtime.block_on(async {
        let ctx = BenchmarkContext::new().await;
        (*ctx.pool.config()).clone()
    });

    let mut group = c.benchmark_group("pool_initialization");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(10);

    group.bench_function("create_pool", |b| {
        b.to_async(&runtime).iter(|| async {
            black_box(StoragePool::new(config.clone()).await.unwrap());
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_connection_acquisition,
    bench_concurrent_connections,
    bench_concurrent_writes,
    bench_pool_saturation_recovery,
    bench_mixed_workload,
    bench_connection_reuse,
    bench_pool_initialization,
);

criterion_main!(benches);
