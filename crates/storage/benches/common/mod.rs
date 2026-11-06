//! Common benchmark utilities and test data generators.
//!
//! This module provides shared functionality for all benchmarks including:
//! - Test data generation
//! - Database setup using testcontainers
//! - Benchmark context management

use chrono::Utc;
use llm_observatory_storage::{
    models::{LogRecord, Metric, MetricDataPoint, Trace, TraceSpan},
    StorageConfig, StoragePool,
};
use once_cell::sync::Lazy;
use std::sync::Mutex;
use testcontainers::{clients::Cli, Container, RunnableImage};
use testcontainers_modules::postgres::Postgres;
use uuid::Uuid;

/// Global Docker client for testcontainers.
static DOCKER: Lazy<Cli> = Lazy::new(Cli::default);

/// Global container instance to ensure it lives for the entire benchmark run.
static CONTAINER: Lazy<Mutex<Option<Container<'static, Postgres>>>> =
    Lazy::new(|| Mutex::new(None));

/// Benchmark context with database connection.
pub struct BenchmarkContext {
    pub pool: StoragePool,
}

impl BenchmarkContext {
    /// Create a new benchmark context with test database.
    pub async fn new() -> Self {
        let pool = setup_test_database().await;
        Self { pool }
    }
}

/// Setup test database using testcontainers or environment variable.
async fn setup_test_database() -> StoragePool {
    // Check if DATABASE_URL is set (for faster iteration during development)
    if let Ok(database_url) = std::env::var("DATABASE_URL") {
        eprintln!("Using DATABASE_URL: {}", database_url);
        let config = StorageConfig::from_env().expect("Failed to load config from environment");
        let pool = StoragePool::new(config)
            .await
            .expect("Failed to create pool");

        // Run migrations
        sqlx::migrate!("./migrations")
            .run(pool.postgres())
            .await
            .expect("Failed to run migrations");

        return pool;
    }

    // Use testcontainers for isolated testing
    eprintln!("Starting PostgreSQL container for benchmarks...");

    let postgres_image = Postgres::default().with_tag("16-alpine");
    let image = RunnableImage::from(postgres_image)
        .with_env_var(("POSTGRES_DB", "llm_observatory_bench"))
        .with_env_var(("POSTGRES_USER", "postgres"))
        .with_env_var(("POSTGRES_PASSWORD", "password"));

    // Start container and store it globally
    let mut container_guard = CONTAINER.lock().unwrap();
    if container_guard.is_none() {
        let container = DOCKER.run(image);
        *container_guard = Some(container);
    }
    let container = container_guard.as_ref().unwrap();

    // Build database URL
    let port = container.get_host_port_ipv4(5432);
    let database_url = format!(
        "postgres://postgres:password@localhost:{}/llm_observatory_bench",
        port
    );

    eprintln!("Container started on port {}", port);

    // Create config and pool
    let mut config = StorageConfig::default();
    config.postgres.url = database_url;
    config.postgres.max_connections = 20;
    config.postgres.min_connections = 5;
    config.postgres.acquire_timeout_secs = 30;
    config.redis = None; // Disable Redis for benchmarks

    let pool = StoragePool::new(config)
        .await
        .expect("Failed to create pool");

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(pool.postgres())
        .await
        .expect("Failed to run migrations");

    eprintln!("Database ready for benchmarks");

    pool
}

/// Generate realistic test traces.
pub fn generate_traces(count: usize) -> Vec<Trace> {
    (0..count)
        .map(|i| {
            let now = Utc::now();
            Trace {
                id: Uuid::new_v4(),
                trace_id: format!("trace_{:016x}", i),
                service_name: "benchmark-service".to_string(),
                start_time: now,
                end_time: Some(now + chrono::Duration::milliseconds(100 + (i % 200) as i64)),
                duration_us: Some(100_000 + (i % 200_000) as i64),
                status: if i % 10 == 0 { "error" } else { "ok" }.to_string(),
                status_message: if i % 10 == 0 {
                    Some("Error occurred".to_string())
                } else {
                    None
                },
                root_span_name: Some(format!("operation_{}", i % 20)),
                attributes: serde_json::json!({
                    "test": "benchmark",
                    "index": i,
                    "batch": i / 100,
                }),
                resource_attributes: serde_json::json!({
                    "service.name": "benchmark-service",
                    "service.version": "1.0.0",
                }),
                span_count: 1 + (i % 10) as i32,
                created_at: now,
                updated_at: now,
            }
        })
        .collect()
}

/// Generate realistic test spans.
pub fn generate_spans(count: usize) -> Vec<TraceSpan> {
    (0..count)
        .map(|i| {
            let now = Utc::now();
            TraceSpan {
                id: Uuid::new_v4(),
                trace_id: Uuid::new_v4(),
                span_id: format!("span_{:016x}", i),
                parent_span_id: if i % 3 == 0 {
                    None
                } else {
                    Some(format!("span_{:016x}", i / 3))
                },
                name: format!("operation_{}", i % 50),
                kind: match i % 5 {
                    0 => "client",
                    1 => "server",
                    2 => "producer",
                    3 => "consumer",
                    _ => "internal",
                }
                .to_string(),
                service_name: "benchmark-service".to_string(),
                start_time: now,
                end_time: Some(now + chrono::Duration::milliseconds(10 + (i % 100) as i64)),
                duration_us: Some(10_000 + (i % 100_000) as i64),
                status: if i % 20 == 0 { "error" } else { "ok" }.to_string(),
                status_message: if i % 20 == 0 {
                    Some("Span error".to_string())
                } else {
                    None
                },
                attributes: serde_json::json!({
                    "span.kind": "internal",
                    "index": i,
                    "db.system": if i % 2 == 0 { "postgresql" } else { "redis" },
                }),
                events: if i % 10 == 0 {
                    Some(serde_json::json!([
                        {
                            "name": "event_1",
                            "timestamp": now.to_rfc3339(),
                            "attributes": {},
                        }
                    ]))
                } else {
                    None
                },
                links: None,
                created_at: now,
            }
        })
        .collect()
}

/// Generate realistic test logs.
pub fn generate_logs(count: usize) -> Vec<LogRecord> {
    let severities = [
        (1, "TRACE"),
        (5, "DEBUG"),
        (9, "INFO"),
        (13, "WARN"),
        (17, "ERROR"),
    ];

    (0..count)
        .map(|i| {
            let now = Utc::now();
            let (severity_number, severity_text) = severities[i % severities.len()];

            LogRecord {
                id: Uuid::new_v4(),
                timestamp: now,
                observed_timestamp: now,
                severity_number,
                severity_text: severity_text.to_string(),
                body: format!(
                    "[{}] Benchmark log message {} - {}",
                    severity_text, i, "Lorem ipsum dolor sit amet, consectetur adipiscing elit"
                ),
                service_name: "benchmark-service".to_string(),
                trace_id: if i % 5 == 0 {
                    Some(format!("trace_{:016x}", i / 5))
                } else {
                    None
                },
                span_id: if i % 5 == 0 {
                    Some(format!("span_{:016x}", i / 5))
                } else {
                    None
                },
                trace_flags: None,
                attributes: serde_json::json!({
                    "log.index": i,
                    "component": format!("component_{}", i % 10),
                    "user.id": format!("user_{}", i % 100),
                }),
                resource_attributes: serde_json::json!({
                    "service.name": "benchmark-service",
                    "service.version": "1.0.0",
                    "deployment.environment": "benchmark",
                }),
                scope_name: Some("benchmark-scope".to_string()),
                scope_version: Some("1.0.0".to_string()),
                scope_attributes: Some(serde_json::json!({})),
                created_at: now,
            }
        })
        .collect()
}

/// Generate realistic test metrics.
pub fn generate_metrics(count: usize) -> Vec<Metric> {
    let metric_types = ["counter", "gauge", "histogram", "summary"];

    (0..count)
        .map(|i| {
            let now = Utc::now();
            let metric_type = metric_types[i % metric_types.len()];

            Metric {
                id: Uuid::new_v4(),
                name: format!("benchmark.metric.{}.{}", metric_type, i),
                description: Some(format!("Benchmark {} metric {}", metric_type, i)),
                unit: Some(
                    match metric_type {
                        "counter" => "count",
                        "gauge" => "percent",
                        "histogram" => "milliseconds",
                        "summary" => "bytes",
                        _ => "unit",
                    }
                    .to_string(),
                ),
                metric_type: metric_type.to_string(),
                service_name: "benchmark-service".to_string(),
                attributes: serde_json::json!({
                    "environment": "benchmark",
                    "index": i,
                }),
                resource_attributes: serde_json::json!({
                    "service.name": "benchmark-service",
                    "service.version": "1.0.0",
                }),
                created_at: now,
                updated_at: now,
            }
        })
        .collect()
}

/// Generate realistic test metric data points.
pub fn generate_metric_data_points(count: usize, metric_id: Uuid) -> Vec<MetricDataPoint> {
    (0..count)
        .map(|i| {
            let now = Utc::now();

            MetricDataPoint {
                id: Uuid::new_v4(),
                metric_id,
                timestamp: now - chrono::Duration::seconds((count - i) as i64),
                value: Some((i as f64 * 1.5).sin() * 100.0 + 100.0), // Sinusoidal pattern
                count: if i % 3 == 0 { Some(i as i64) } else { None },
                sum: if i % 3 == 0 {
                    Some(i as f64 * 10.0)
                } else {
                    None
                },
                min: if i % 3 == 0 {
                    Some((i as f64).min(50.0))
                } else {
                    None
                },
                max: if i % 3 == 0 {
                    Some((i as f64).max(200.0))
                } else {
                    None
                },
                buckets: None,
                quantiles: None,
                exemplars: None,
                attributes: serde_json::json!({
                    "index": i,
                }),
                created_at: now,
            }
        })
        .collect()
}

/// Setup test container for isolated benchmarking.
pub fn setup_test_container() -> Container<'static, Postgres> {
    let postgres_image = Postgres::default().with_tag("16-alpine");
    let image = RunnableImage::from(postgres_image)
        .with_env_var(("POSTGRES_DB", "llm_observatory_bench"))
        .with_env_var(("POSTGRES_USER", "postgres"))
        .with_env_var(("POSTGRES_PASSWORD", "password"));

    DOCKER.run(image)
}
