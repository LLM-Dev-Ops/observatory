//! Common test utilities and helpers for storage integration tests.
//!
//! This module provides shared functionality across all integration tests,
//! including database setup, fixtures, and utility functions.

pub mod database;
pub mod fixtures;

// Re-exports for convenience
pub use database::{TestDatabase, TestDatabaseGuard};
pub use fixtures::{
    create_test_log, create_test_logs, create_test_metric, create_test_metric_data_point,
    create_test_metrics, create_test_span, create_test_spans, create_test_trace,
    create_test_traces,
};

use llm_observatory_storage::{StorageConfig, StoragePool};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Global test database instance for sharing across tests
static TEST_DB: once_cell::sync::Lazy<Arc<Mutex<Option<TestDatabase>>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(None)));

/// Initialize the test database (called once per test suite)
pub async fn init_test_db() -> TestDatabase {
    let mut db = TEST_DB.lock().await;
    if db.is_none() {
        *db = Some(
            TestDatabase::new()
                .await
                .expect("Failed to create test database"),
        );
    }
    db.as_ref().unwrap().clone()
}

/// Get a test storage configuration for a specific database
pub fn get_test_config(database_url: &str) -> StorageConfig {
    StorageConfig {
        postgres: llm_observatory_storage::config::PostgresConfig {
            host: "localhost".to_string(),
            port: 5432,
            database: "test".to_string(),
            username: "test".to_string(),
            password: "test".to_string(),
            ssl_mode: "disable".to_string(),
            application_name: "llm-observatory-test".to_string(),
        },
        redis: None,
        pool: llm_observatory_storage::config::PoolConfig {
            max_connections: 10,
            min_connections: 1,
            connect_timeout_secs: 5,
            idle_timeout_secs: 60,
            max_lifetime_secs: 300,
        },
        retry: llm_observatory_storage::config::RetryConfig {
            max_retries: 3,
            initial_delay_ms: 100,
            max_delay_ms: 1000,
            backoff_multiplier: 2.0,
        },
    }
}

/// Parse database URL to get components
pub fn parse_database_url(url: &str) -> (String, u16, String, String, String) {
    // Simple parsing for test URLs
    // Format: postgres://user:pass@host:port/db
    let url = url.strip_prefix("postgres://").unwrap_or(url);
    let parts: Vec<&str> = url.split('@').collect();
    let creds: Vec<&str> = parts[0].split(':').collect();
    let host_parts: Vec<&str> = parts[1].split('/').collect();
    let host_port: Vec<&str> = host_parts[0].split(':').collect();

    (
        host_port[0].to_string(),
        host_port.get(1).unwrap_or(&"5432").parse().unwrap_or(5432),
        host_parts[1].to_string(),
        creds[0].to_string(),
        creds[1].to_string(),
    )
}

/// Create a storage config from a database URL
pub fn config_from_url(database_url: &str) -> StorageConfig {
    let (host, port, database, username, password) = parse_database_url(database_url);

    StorageConfig {
        postgres: llm_observatory_storage::config::PostgresConfig {
            host,
            port,
            database,
            username,
            password,
            ssl_mode: "disable".to_string(),
            application_name: "llm-observatory-test".to_string(),
        },
        redis: None,
        pool: llm_observatory_storage::config::PoolConfig {
            max_connections: 10,
            min_connections: 1,
            connect_timeout_secs: 5,
            idle_timeout_secs: 60,
            max_lifetime_secs: 300,
        },
        retry: llm_observatory_storage::config::RetryConfig {
            max_retries: 3,
            initial_delay_ms: 100,
            max_delay_ms: 1000,
            backoff_multiplier: 2.0,
        },
    }
}

/// Setup a test pool with migrations
pub async fn setup_test_pool() -> (StoragePool, TestDatabaseGuard) {
    let db = init_test_db().await;
    let guard = db.get_connection().await;
    let config = config_from_url(&guard.database_url);
    let pool = StoragePool::new(config)
        .await
        .expect("Failed to create storage pool");

    // Run migrations
    run_test_migrations(&pool).await;

    (pool, guard)
}

/// Run database migrations for tests
pub async fn run_test_migrations(pool: &StoragePool) {
    // Create tables for testing
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS traces (
            id UUID PRIMARY KEY,
            trace_id VARCHAR(32) UNIQUE NOT NULL,
            service_name VARCHAR(255) NOT NULL,
            start_time TIMESTAMPTZ NOT NULL,
            end_time TIMESTAMPTZ,
            duration_us BIGINT,
            status VARCHAR(50) NOT NULL,
            status_message TEXT,
            root_span_name VARCHAR(255),
            attributes JSONB NOT NULL DEFAULT '{}',
            resource_attributes JSONB NOT NULL DEFAULT '{}',
            span_count INTEGER NOT NULL DEFAULT 0,
            created_at TIMESTAMPTZ NOT NULL,
            updated_at TIMESTAMPTZ NOT NULL
        )
        "#,
    )
    .execute(pool.postgres())
    .await
    .expect("Failed to create traces table");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS trace_spans (
            id UUID PRIMARY KEY,
            trace_id UUID NOT NULL,
            span_id VARCHAR(32) UNIQUE NOT NULL,
            parent_span_id VARCHAR(32),
            name VARCHAR(255) NOT NULL,
            kind VARCHAR(50) NOT NULL,
            service_name VARCHAR(255) NOT NULL,
            start_time TIMESTAMPTZ NOT NULL,
            end_time TIMESTAMPTZ,
            duration_us BIGINT,
            status VARCHAR(50) NOT NULL,
            status_message TEXT,
            attributes JSONB NOT NULL DEFAULT '{}',
            events JSONB,
            links JSONB,
            created_at TIMESTAMPTZ NOT NULL
        )
        "#,
    )
    .execute(pool.postgres())
    .await
    .expect("Failed to create trace_spans table");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS trace_events (
            id UUID PRIMARY KEY,
            span_id UUID NOT NULL,
            name VARCHAR(255) NOT NULL,
            timestamp TIMESTAMPTZ NOT NULL,
            attributes JSONB NOT NULL DEFAULT '{}',
            created_at TIMESTAMPTZ NOT NULL
        )
        "#,
    )
    .execute(pool.postgres())
    .await
    .expect("Failed to create trace_events table");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS metrics (
            id UUID PRIMARY KEY,
            name VARCHAR(255) NOT NULL,
            description TEXT,
            unit VARCHAR(50),
            metric_type VARCHAR(50) NOT NULL,
            service_name VARCHAR(255) NOT NULL,
            attributes JSONB NOT NULL DEFAULT '{}',
            resource_attributes JSONB NOT NULL DEFAULT '{}',
            created_at TIMESTAMPTZ NOT NULL,
            updated_at TIMESTAMPTZ NOT NULL,
            UNIQUE(name, service_name)
        )
        "#,
    )
    .execute(pool.postgres())
    .await
    .expect("Failed to create metrics table");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS metric_data_points (
            id UUID PRIMARY KEY,
            metric_id UUID NOT NULL,
            timestamp TIMESTAMPTZ NOT NULL,
            value DOUBLE PRECISION,
            count BIGINT,
            sum DOUBLE PRECISION,
            min DOUBLE PRECISION,
            max DOUBLE PRECISION,
            buckets JSONB,
            quantiles JSONB,
            exemplars JSONB,
            attributes JSONB NOT NULL DEFAULT '{}',
            created_at TIMESTAMPTZ NOT NULL
        )
        "#,
    )
    .execute(pool.postgres())
    .await
    .expect("Failed to create metric_data_points table");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS logs (
            id UUID PRIMARY KEY,
            timestamp TIMESTAMPTZ NOT NULL,
            observed_timestamp TIMESTAMPTZ NOT NULL,
            severity_number INTEGER NOT NULL,
            severity_text VARCHAR(50) NOT NULL,
            body TEXT NOT NULL,
            service_name VARCHAR(255) NOT NULL,
            trace_id VARCHAR(32),
            span_id VARCHAR(32),
            trace_flags INTEGER,
            attributes JSONB NOT NULL DEFAULT '{}',
            resource_attributes JSONB NOT NULL DEFAULT '{}',
            scope_name VARCHAR(255),
            scope_version VARCHAR(50),
            scope_attributes JSONB,
            created_at TIMESTAMPTZ NOT NULL
        )
        "#,
    )
    .execute(pool.postgres())
    .await
    .expect("Failed to create logs table");

    // Create indexes
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_traces_trace_id ON traces(trace_id)")
        .execute(pool.postgres())
        .await;
    let _ =
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_traces_service_name ON traces(service_name)")
            .execute(pool.postgres())
            .await;
    let _ =
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_trace_spans_trace_id ON trace_spans(trace_id)")
            .execute(pool.postgres())
            .await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_logs_service_name ON logs(service_name)")
        .execute(pool.postgres())
        .await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_logs_severity ON logs(severity_number)")
        .execute(pool.postgres())
        .await;
}

/// Clean up test data from a pool
pub async fn cleanup_test_data(pool: &StoragePool) {
    let _ = sqlx::query("TRUNCATE TABLE trace_events CASCADE")
        .execute(pool.postgres())
        .await;
    let _ = sqlx::query("TRUNCATE TABLE trace_spans CASCADE")
        .execute(pool.postgres())
        .await;
    let _ = sqlx::query("TRUNCATE TABLE traces CASCADE")
        .execute(pool.postgres())
        .await;
    let _ = sqlx::query("TRUNCATE TABLE metric_data_points CASCADE")
        .execute(pool.postgres())
        .await;
    let _ = sqlx::query("TRUNCATE TABLE metrics CASCADE")
        .execute(pool.postgres())
        .await;
    let _ = sqlx::query("TRUNCATE TABLE logs CASCADE")
        .execute(pool.postgres())
        .await;
}
