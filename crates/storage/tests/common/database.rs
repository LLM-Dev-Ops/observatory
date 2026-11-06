//! Test database setup and management using testcontainers.
//!
//! This module provides utilities for setting up isolated PostgreSQL databases
//! for integration testing.

use std::sync::Arc;
use tokio::sync::Mutex;

/// Test database instance using testcontainers
#[derive(Clone)]
pub struct TestDatabase {
    /// The database URL for connecting
    pub database_url: String,
    /// Container reference (kept alive)
    container: Arc<Mutex<Option<TestContainer>>>,
}

/// Internal container wrapper
struct TestContainer {
    #[allow(dead_code)]
    container: testcontainers::ContainerAsync<testcontainers::GenericImage>,
    database_url: String,
}

impl TestDatabase {
    /// Create a new test database instance with a PostgreSQL container
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        use testcontainers::*;

        tracing::info!("Starting PostgreSQL test container...");

        // Create a PostgreSQL container with TimescaleDB support
        let image = GenericImage::new("postgres", "16-alpine")
            .with_env_var("POSTGRES_USER", "test")
            .with_env_var("POSTGRES_PASSWORD", "test")
            .with_env_var("POSTGRES_DB", "test")
            .with_wait_for(WaitFor::message_on_stderr(
                "database system is ready to accept connections",
            ))
            .with_wait_for(WaitFor::seconds(2));

        let container = image.start().await?;

        // Get the mapped port
        let port = container.get_host_port_ipv4(5432).await?;
        let database_url = format!("postgres://test:test@localhost:{}/test", port);

        tracing::info!("PostgreSQL test container started at {}", database_url);

        // Wait for the database to be fully ready
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        let container_wrapper = TestContainer {
            container,
            database_url: database_url.clone(),
        };

        Ok(Self {
            database_url: database_url.clone(),
            container: Arc::new(Mutex::new(Some(container_wrapper))),
        })
    }

    /// Get a connection guard for this database
    pub async fn get_connection(&self) -> TestDatabaseGuard {
        TestDatabaseGuard {
            database_url: self.database_url.clone(),
            _container: self.container.clone(),
        }
    }

    /// Get the database URL
    pub fn url(&self) -> &str {
        &self.database_url
    }
}

/// Guard that keeps the database container alive
pub struct TestDatabaseGuard {
    pub database_url: String,
    _container: Arc<Mutex<Option<TestContainer>>>,
}

impl TestDatabaseGuard {
    /// Get the database URL
    pub fn url(&self) -> &str {
        &self.database_url
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_database_creation() {
        let db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        assert!(!db.database_url.is_empty());
        assert!(db.database_url.starts_with("postgres://"));
    }

    #[tokio::test]
    async fn test_database_connection() {
        let db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let guard = db.get_connection().await;

        // Try to connect to the database
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect(&guard.database_url)
            .await
            .expect("Failed to connect to test database");

        // Run a simple query
        let result: (i32,) = sqlx::query_as("SELECT 1")
            .fetch_one(&pool)
            .await
            .expect("Failed to execute query");

        assert_eq!(result.0, 1);
    }
}
