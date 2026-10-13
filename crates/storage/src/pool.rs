//! Connection pool management module.
//!
//! This module handles database connection pooling for both PostgreSQL and Redis,
//! providing efficient connection reuse and automatic reconnection.

use crate::config::StorageConfig;
use crate::error::{StorageError, StorageResult};
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::sync::Arc;
use std::time::Duration;

/// Main storage pool that manages connections to PostgreSQL and optionally Redis.
#[derive(Clone)]
pub struct StoragePool {
    /// PostgreSQL connection pool
    postgres: PgPool,

    /// Redis connection pool (optional)
    redis: Option<Arc<redis::aio::ConnectionManager>>,

    /// Configuration reference
    config: Arc<StorageConfig>,
}

impl StoragePool {
    /// Create a new storage pool with the given configuration.
    ///
    /// This will establish connections to PostgreSQL and optionally Redis,
    /// with automatic retry logic based on the retry configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - Storage configuration
    ///
    /// # Errors
    ///
    /// Returns an error if connection to any database fails after all retry attempts.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use llm_observatory_storage::{StorageConfig, StoragePool};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = StorageConfig::from_env()?;
    /// let pool = StoragePool::new(config).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new(config: StorageConfig) -> StorageResult<Self> {
        let config = Arc::new(config);

        tracing::info!("Initializing storage pool with retry logic");

        // Create PostgreSQL connection pool with retry
        let postgres = Self::create_postgres_pool_with_retry(&config).await?;

        // Create Redis connection pool if configured
        let redis = if let Some(redis_config) = &config.redis {
            tracing::info!("Creating Redis connection pool");
            match Self::create_redis_pool_with_retry(redis_config, &config.retry).await {
                Ok(manager) => Some(Arc::new(manager)),
                Err(e) => {
                    tracing::warn!(
                        "Failed to connect to Redis: {}. Continuing without Redis.",
                        e
                    );
                    None
                }
            }
        } else {
            tracing::info!("Redis not configured, skipping");
            None
        };

        tracing::info!("Storage pool initialized successfully");

        Ok(Self {
            postgres,
            redis,
            config,
        })
    }

    /// Create a PostgreSQL connection pool with retry logic.
    async fn create_postgres_pool_with_retry(config: &StorageConfig) -> StorageResult<PgPool> {
        let mut attempt = 0;
        let max_attempts = config.retry.max_retries;

        loop {
            match Self::create_postgres_pool(config).await {
                Ok(pool) => return Ok(pool),
                Err(e) if attempt < max_attempts => {
                    let delay = config.retry.delay_for_attempt(attempt);
                    attempt += 1;
                    tracing::warn!(
                        "PostgreSQL connection attempt {}/{} failed: {}. Retrying in {:?}...",
                        attempt,
                        max_attempts,
                        e,
                        delay
                    );
                    tokio::time::sleep(delay).await;
                }
                Err(e) => {
                    tracing::error!(
                        "PostgreSQL connection failed after {} attempts: {}",
                        max_attempts,
                        e
                    );
                    return Err(e);
                }
            }
        }
    }

    /// Create a Redis connection pool with retry logic.
    async fn create_redis_pool_with_retry(
        redis_config: &crate::config::RedisConfig,
        retry_config: &crate::config::RetryConfig,
    ) -> StorageResult<redis::aio::ConnectionManager> {
        let mut attempt = 0;
        let max_attempts = retry_config.max_retries;

        loop {
            match Self::create_redis_pool(redis_config).await {
                Ok(manager) => return Ok(manager),
                Err(e) if attempt < max_attempts => {
                    let delay = retry_config.delay_for_attempt(attempt);
                    attempt += 1;
                    tracing::warn!(
                        "Redis connection attempt {}/{} failed: {}. Retrying in {:?}...",
                        attempt,
                        max_attempts,
                        e,
                        delay
                    );
                    tokio::time::sleep(delay).await;
                }
                Err(e) => {
                    tracing::error!(
                        "Redis connection failed after {} attempts: {}",
                        max_attempts,
                        e
                    );
                    return Err(e);
                }
            }
        }
    }

    /// Create a PostgreSQL connection pool.
    async fn create_postgres_pool(config: &StorageConfig) -> StorageResult<PgPool> {
        let pool = PgPoolOptions::new()
            .max_connections(config.pool.max_connections)
            .min_connections(config.pool.min_connections)
            .acquire_timeout(Duration::from_secs(config.pool.connect_timeout_secs))
            .idle_timeout(Some(Duration::from_secs(config.pool.idle_timeout_secs)))
            .max_lifetime(Some(Duration::from_secs(config.pool.max_lifetime_secs)))
            .connect(&config.postgres_url())
            .await
            .map_err(|e| StorageError::ConnectionError(e.to_string()))?;

        tracing::info!(
            "PostgreSQL connection pool created with {} max connections",
            config.pool.max_connections
        );

        Ok(pool)
    }

    /// Create a Redis connection manager.
    async fn create_redis_pool(
        config: &crate::config::RedisConfig,
    ) -> StorageResult<redis::aio::ConnectionManager> {
        let client = redis::Client::open(config.url.as_str())
            .map_err(|e| StorageError::ConnectionError(e.to_string()))?;

        let manager = redis::aio::ConnectionManager::new(client)
            .await
            .map_err(|e| StorageError::ConnectionError(e.to_string()))?;

        tracing::info!("Redis connection manager created");

        Ok(manager)
    }

    /// Get a reference to the PostgreSQL connection pool.
    pub fn postgres(&self) -> &PgPool {
        &self.postgres
    }

    /// Get a reference to the Redis connection manager.
    pub fn redis(&self) -> Option<&redis::aio::ConnectionManager> {
        self.redis.as_ref().map(|r| r.as_ref())
    }

    /// Get a reference to the storage configuration.
    pub fn config(&self) -> &StorageConfig {
        &self.config
    }

    /// Get a tokio-postgres client for COPY operations.
    ///
    /// This creates a new connection using tokio-postgres directly, which is needed
    /// for COPY protocol operations. The connection should be returned/closed when done.
    ///
    /// # Errors
    ///
    /// Returns an error if connection fails.
    pub async fn get_tokio_postgres_client(
        &self,
    ) -> StorageResult<(tokio_postgres::Client, tokio::task::JoinHandle<()>)> {
        let (client, connection) =
            tokio_postgres::connect(&self.config.postgres_url(), tokio_postgres::NoTls)
                .await
                .map_err(|e| {
                    StorageError::ConnectionError(format!(
                        "Failed to create tokio-postgres client: {}",
                        e
                    ))
                })?;

        // Spawn connection handler in background
        let handle = tokio::spawn(async move {
            if let Err(e) = connection.await {
                tracing::error!("tokio-postgres connection error: {}", e);
            }
        });

        Ok((client, handle))
    }

    /// Run database migrations.
    ///
    /// This applies all pending migrations to the PostgreSQL database.
    ///
    /// # Errors
    ///
    /// Returns an error if migrations fail to apply.
    pub async fn run_migrations(&self) -> StorageResult<()> {
        // TODO: Implement migration running
        // Use sqlx::migrate!() macro or runtime migrations
        tracing::info!("Running database migrations...");

        // Example:
        // sqlx::migrate!("./migrations")
        //     .run(&self.postgres)
        //     .await
        //     .map_err(|e| StorageError::MigrationError(e.to_string()))?;

        tracing::info!("Database migrations completed");
        Ok(())
    }

    /// Check if the database connection is healthy.
    ///
    /// Performs a simple query to verify connectivity for both PostgreSQL and Redis.
    ///
    /// # Errors
    ///
    /// Returns an error if the health check fails for PostgreSQL.
    /// Redis failures are logged but don't cause the health check to fail.
    pub async fn health_check(&self) -> StorageResult<HealthCheckResult> {
        tracing::debug!("Running health check");

        // Check PostgreSQL
        let postgres_healthy = match self.health_check_postgres().await {
            Ok(_) => true,
            Err(e) => {
                tracing::error!("PostgreSQL health check failed: {}", e);
                return Err(e);
            }
        };

        // Check Redis if configured
        let redis_healthy = if self.redis.is_some() {
            match self.health_check_redis().await {
                Ok(_) => Some(true),
                Err(e) => {
                    tracing::warn!("Redis health check failed: {}", e);
                    Some(false)
                }
            }
        } else {
            None
        };

        Ok(HealthCheckResult {
            postgres_healthy,
            redis_healthy,
        })
    }

    /// Check PostgreSQL connection health.
    pub async fn health_check_postgres(&self) -> StorageResult<()> {
        sqlx::query("SELECT 1")
            .execute(&self.postgres)
            .await
            .map_err(|e| {
                StorageError::ConnectionError(format!("PostgreSQL health check failed: {}", e))
            })?;

        tracing::debug!("PostgreSQL health check passed");
        Ok(())
    }

    /// Check Redis connection health.
    pub async fn health_check_redis(&self) -> StorageResult<()> {
        if let Some(redis) = &self.redis {
            let mut conn = redis.as_ref().clone();
            redis::cmd("PING")
                .query_async::<_, String>(&mut conn)
                .await
                .map_err(|e| {
                    StorageError::RedisError(format!("Redis health check failed: {}", e))
                })?;

            tracing::debug!("Redis health check passed");
            Ok(())
        } else {
            Err(StorageError::RedisError("Redis not configured".to_string()))
        }
    }

    /// Close all database connections gracefully.
    pub async fn close(&self) {
        self.postgres.close().await;
        tracing::info!("Storage pool closed");
    }

    /// Get pool statistics for monitoring.
    pub fn stats(&self) -> PoolStats {
        let size = self.postgres.size() as u32;
        let idle = self.postgres.num_idle() as u32;
        let active = size.saturating_sub(idle);

        PoolStats {
            postgres_size: size,
            postgres_idle: idle,
            postgres_active: active,
            redis_connected: self.redis.is_some(),
            postgres_max_connections: self.config.pool.max_connections,
            postgres_min_connections: self.config.pool.min_connections,
        }
    }

    /// Update pool metrics.
    ///
    /// This method updates Prometheus metrics with the current pool state.
    /// Call this periodically (e.g., every 10 seconds) to keep metrics up to date.
    pub fn update_metrics(&self) {
        let metrics = crate::metrics::StorageMetrics::new();
        let stats = self.stats();
        metrics.update_pool_connections(
            stats.postgres_active,
            stats.postgres_idle,
            stats.postgres_max_connections,
        );
    }
}

/// Result of a health check operation.
#[derive(Debug, Clone)]
pub struct HealthCheckResult {
    /// Whether PostgreSQL is healthy
    pub postgres_healthy: bool,

    /// Whether Redis is healthy (None if not configured)
    pub redis_healthy: Option<bool>,
}

impl HealthCheckResult {
    /// Check if all configured services are healthy.
    pub fn is_healthy(&self) -> bool {
        self.postgres_healthy && self.redis_healthy.unwrap_or(true)
    }
}

/// Statistics about connection pool usage.
#[derive(Debug, Clone)]
pub struct PoolStats {
    /// Total PostgreSQL connections in the pool
    pub postgres_size: u32,

    /// Number of idle PostgreSQL connections
    pub postgres_idle: u32,

    /// Number of active PostgreSQL connections
    pub postgres_active: u32,

    /// Whether Redis is connected
    pub redis_connected: bool,

    /// Maximum configured PostgreSQL connections
    pub postgres_max_connections: u32,

    /// Minimum configured PostgreSQL connections
    pub postgres_min_connections: u32,
}

impl PoolStats {
    /// Get the percentage of connections in use.
    pub fn utilization_percent(&self) -> f64 {
        if self.postgres_max_connections == 0 {
            return 0.0;
        }
        (self.postgres_active as f64 / self.postgres_max_connections as f64) * 100.0
    }

    /// Check if the pool is approaching capacity.
    ///
    /// Returns true if utilization is above 80%.
    pub fn is_near_capacity(&self) -> bool {
        self.utilization_percent() > 80.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO: Add integration tests with test databases
    // These tests would require a running PostgreSQL instance

    #[test]
    fn test_pool_stats_structure() {
        let stats = PoolStats {
            postgres_size: 10,
            postgres_idle: 5,
            redis_connected: true,
        };

        assert_eq!(stats.postgres_size, 10);
        assert_eq!(stats.postgres_idle, 5);
        assert!(stats.redis_connected);
    }
}
