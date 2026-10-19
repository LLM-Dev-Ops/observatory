//! Query performance benchmarks for repository operations.
//!
//! This benchmark suite measures query latency across common repository
//! operations to ensure P95 latency stays below 100ms.
//!
//! ## Running the benchmark
//!
//! ```bash
//! cargo bench --bench query_performance
//! ```
//!
//! ## Target Metrics
//!
//! - Query latency: P95 <100ms
//! - Simple queries: P50 <10ms
//! - Complex queries: P95 <100ms

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use llm_observatory_storage::{
    repositories::{LogRepository, MetricRepository, TraceRepository},
    writers::{LogWriter, MetricWriter, TraceWriter},
};
use std::time::Duration;

mod common;
use common::{
    generate_logs, generate_metric_data_points, generate_metrics, generate_spans, generate_traces,
    BenchmarkContext,
};

/// Setup benchmark data for query tests.
async fn setup_query_data(ctx: &BenchmarkContext) -> (Vec<String>, Vec<uuid::Uuid>) {
    // Insert 10,000 traces for realistic query scenarios
    let traces = generate_traces(10000);
    let trace_ids: Vec<String> = traces.iter().map(|t| t.trace_id.clone()).collect();
    let trace_uuids: Vec<uuid::Uuid> = traces.iter().map(|t| t.id).collect();

    let writer = TraceWriter::new(ctx.pool.clone());
    writer.write_traces(traces).await.unwrap();
    writer.flush().await.unwrap();

    // Insert 50,000 spans
    let spans = generate_spans(50000);
    writer.write_spans(spans).await.unwrap();
    writer.flush().await.unwrap();

    // Insert 10,000 logs
    let logs = generate_logs(10000);
    let log_writer = LogWriter::new(ctx.pool.clone());
    log_writer.write_logs(logs).await.unwrap();
    log_writer.flush().await.unwrap();

    // Insert 1,000 metrics with 10,000 data points
    let metrics = generate_metrics(1000);
    let metric_ids: Vec<uuid::Uuid> = metrics.iter().map(|m| m.id).collect();
    let metric_writer = MetricWriter::new(ctx.pool.clone());
    metric_writer.write_metrics(metrics).await.unwrap();
    metric_writer.flush().await.unwrap();

    for metric_id in &metric_ids[..100] {
        let data_points = generate_metric_data_points(100, *metric_id);
        metric_writer.write_data_points(data_points).await.unwrap();
    }
    metric_writer.flush().await.unwrap();

    (trace_ids, trace_uuids)
}

/// Benchmark trace lookup by ID.
fn bench_trace_get_by_id(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let ctx = runtime.block_on(async { BenchmarkContext::new().await });
    let (_, trace_uuids) = runtime.block_on(async { setup_query_data(&ctx).await });

    let repository = TraceRepository::new(ctx.pool.clone());
    let mut group = c.benchmark_group("trace_get_by_id");
    group.measurement_time(Duration::from_secs(10));

    group.bench_function("single_lookup", |b| {
        b.to_async(&runtime).iter(|| async {
            let id = trace_uuids[rand::random::<usize>() % trace_uuids.len()];
            black_box(repository.get_by_id(id).await.ok());
        });
    });

    group.finish();
}

/// Benchmark trace lookup by trace_id string.
fn bench_trace_get_by_trace_id(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let ctx = runtime.block_on(async { BenchmarkContext::new().await });
    let (trace_ids, _) = runtime.block_on(async { setup_query_data(&ctx).await });

    let repository = TraceRepository::new(ctx.pool.clone());
    let mut group = c.benchmark_group("trace_get_by_trace_id");
    group.measurement_time(Duration::from_secs(10));

    group.bench_function("single_lookup", |b| {
        b.to_async(&runtime).iter(|| async {
            let trace_id = &trace_ids[rand::random::<usize>() % trace_ids.len()];
            black_box(repository.get_by_trace_id(trace_id).await.ok());
        });
    });

    group.finish();
}

/// Benchmark trace list queries with different filters.
fn bench_trace_list(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let ctx = runtime.block_on(async { BenchmarkContext::new().await });
    runtime.block_on(async { setup_query_data(&ctx).await });

    let repository = TraceRepository::new(ctx.pool.clone());
    let mut group = c.benchmark_group("trace_list");
    group.measurement_time(Duration::from_secs(10));

    // Benchmark list with limit only
    group.bench_function("limit_100", |b| {
        b.to_async(&runtime).iter(|| async {
            let filters = llm_observatory_storage::repositories::TraceFilters {
                service_name: None,
                status: None,
                start_time: None,
                end_time: None,
                min_duration_us: None,
                max_duration_us: None,
                limit: Some(100),
                offset: None,
            };
            black_box(repository.list(filters).await.unwrap());
        });
    });

    // Benchmark list with service name filter
    group.bench_function("filter_service_limit_100", |b| {
        b.to_async(&runtime).iter(|| async {
            let filters = llm_observatory_storage::repositories::TraceFilters {
                service_name: Some("benchmark-service".to_string()),
                status: None,
                start_time: None,
                end_time: None,
                min_duration_us: None,
                max_duration_us: None,
                limit: Some(100),
                offset: None,
            };
            black_box(repository.list(filters).await.unwrap());
        });
    });

    // Benchmark list with time range filter
    group.bench_function("filter_time_range_limit_100", |b| {
        b.to_async(&runtime).iter(|| async {
            let now = chrono::Utc::now();
            let filters = llm_observatory_storage::repositories::TraceFilters {
                service_name: None,
                status: None,
                start_time: Some(now - chrono::Duration::hours(1)),
                end_time: Some(now),
                min_duration_us: None,
                max_duration_us: None,
                limit: Some(100),
                offset: None,
            };
            black_box(repository.list(filters).await.unwrap());
        });
    });

    group.finish();
}

/// Benchmark span queries.
fn bench_span_queries(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let ctx = runtime.block_on(async { BenchmarkContext::new().await });
    let (_, trace_uuids) = runtime.block_on(async { setup_query_data(&ctx).await });

    let repository = TraceRepository::new(ctx.pool.clone());
    let mut group = c.benchmark_group("span_queries");
    group.measurement_time(Duration::from_secs(10));

    // Benchmark getting spans for a trace
    group.bench_function("get_spans_for_trace", |b| {
        b.to_async(&runtime).iter(|| async {
            let trace_id = trace_uuids[rand::random::<usize>() % trace_uuids.len()];
            black_box(repository.get_spans(trace_id).await.unwrap());
        });
    });

    group.finish();
}

/// Benchmark log queries with different filters.
fn bench_log_queries(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let ctx = runtime.block_on(async { BenchmarkContext::new().await });
    runtime.block_on(async { setup_query_data(&ctx).await });

    let repository = LogRepository::new(ctx.pool.clone());
    let mut group = c.benchmark_group("log_queries");
    group.measurement_time(Duration::from_secs(10));

    // Benchmark list with limit only
    group.bench_function("limit_100", |b| {
        b.to_async(&runtime).iter(|| async {
            let filters = llm_observatory_storage::repositories::LogFilters {
                service_name: None,
                severity_min: None,
                severity_max: None,
                start_time: None,
                end_time: None,
                trace_id: None,
                search_body: None,
                limit: Some(100),
                offset: None,
            };
            black_box(repository.list(filters).await.unwrap());
        });
    });

    // Benchmark list with severity filter
    group.bench_function("filter_severity_limit_100", |b| {
        b.to_async(&runtime).iter(|| async {
            let filters = llm_observatory_storage::repositories::LogFilters {
                service_name: None,
                severity_min: Some(13), // WARN
                severity_max: None,
                start_time: None,
                end_time: None,
                trace_id: None,
                search_body: None,
                limit: Some(100),
                offset: None,
            };
            black_box(repository.list(filters).await.unwrap());
        });
    });

    // Benchmark list with service name filter
    group.bench_function("filter_service_limit_100", |b| {
        b.to_async(&runtime).iter(|| async {
            let filters = llm_observatory_storage::repositories::LogFilters {
                service_name: Some("benchmark-service".to_string()),
                severity_min: None,
                severity_max: None,
                start_time: None,
                end_time: None,
                trace_id: None,
                search_body: None,
                limit: Some(100),
                offset: None,
            };
            black_box(repository.list(filters).await.unwrap());
        });
    });

    group.finish();
}

/// Benchmark metric queries.
fn bench_metric_queries(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let ctx = runtime.block_on(async { BenchmarkContext::new().await });
    runtime.block_on(async { setup_query_data(&ctx).await });

    let repository = MetricRepository::new(ctx.pool.clone());
    let mut group = c.benchmark_group("metric_queries");
    group.measurement_time(Duration::from_secs(10));

    // Benchmark list metrics
    group.bench_function("list_limit_100", |b| {
        b.to_async(&runtime).iter(|| async {
            let filters = llm_observatory_storage::repositories::MetricFilters {
                service_name: None,
                metric_type: None,
                name_pattern: None,
                limit: Some(100),
                offset: None,
            };
            black_box(repository.list(filters).await.unwrap());
        });
    });

    // Benchmark list with service filter
    group.bench_function("filter_service_limit_100", |b| {
        b.to_async(&runtime).iter(|| async {
            let filters = llm_observatory_storage::repositories::MetricFilters {
                service_name: Some("benchmark-service".to_string()),
                metric_type: None,
                name_pattern: None,
                limit: Some(100),
                offset: None,
            };
            black_box(repository.list(filters).await.unwrap());
        });
    });

    group.finish();
}

/// Benchmark metric data point queries.
fn bench_metric_data_point_queries(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let ctx = runtime.block_on(async { BenchmarkContext::new().await });
    runtime.block_on(async { setup_query_data(&ctx).await });

    // Get a metric ID to query
    let metric_id = runtime.block_on(async {
        let repository = MetricRepository::new(ctx.pool.clone());
        let filters = llm_observatory_storage::repositories::MetricFilters {
            service_name: Some("benchmark-service".to_string()),
            metric_type: None,
            name_pattern: None,
            limit: Some(1),
            offset: None,
        };
        let metrics = repository.list(filters).await.unwrap();
        metrics[0].id
    });

    let repository = MetricRepository::new(ctx.pool.clone());
    let mut group = c.benchmark_group("metric_data_point_queries");
    group.measurement_time(Duration::from_secs(10));

    // Benchmark get data points with time range
    group.bench_function("get_data_points_time_range", |b| {
        b.to_async(&runtime).iter(|| async {
            let now = chrono::Utc::now();
            let start_time = now - chrono::Duration::hours(1);
            let end_time = now;
            black_box(
                repository
                    .get_data_points(metric_id, start_time, end_time)
                    .await
                    .unwrap(),
            );
        });
    });

    // Benchmark get latest data point
    group.bench_function("get_latest_data_point", |b| {
        b.to_async(&runtime).iter(|| async {
            black_box(repository.get_latest_data_point(metric_id).await.unwrap());
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_trace_get_by_id,
    bench_trace_get_by_trace_id,
    bench_trace_list,
    bench_span_queries,
    bench_log_queries,
    bench_metric_queries,
    bench_metric_data_point_queries,
);

criterion_main!(benches);
