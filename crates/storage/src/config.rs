//! Database configuration module.
//!
//! This module handles configuration for database connections, including
//! PostgreSQL and Redis settings, connection pool parameters, and retry policies.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Main storage configuration structure.
///
/// Contains all necessary configuration for database connections,
/// including PostgreSQL and Redis settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// PostgreSQL database configuration
    pub postgres: PostgresConfig,

    /// Redis configuration (optional)
    pub redis: Option<RedisConfig>,

    /// Connection pool configuration
    pub pool: PoolConfig,

    /// Retry policy configuration
    pub retry: RetryConfig,
}

/// PostgreSQL database configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostgresConfig {
    /// Database host
    pub host: String,

    /// Database port
    pub port: u16,

    /// Database name
    pub database: String,

    /// Database username
    pub username: String,

    /// Database password
    pub password: String,

    /// SSL mode (disable, allow, prefer, require)
    #[serde(default = "default_ssl_mode")]
    pub ssl_mode: String,

    /// Application name for connection tracking
    #[serde(default = "default_app_name")]
    pub application_name: String,
}

/// Redis configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    /// Redis URL (redis://host:port/db)
    pub url: String,

    /// Connection pool size
    #[serde(default = "default_redis_pool_size")]
    pub pool_size: u32,

    /// Connection timeout in seconds
    #[serde(default = "default_redis_timeout")]
    pub timeout_secs: u64,
}

/// Connection pool configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConfig {
    /// Maximum number of connections in the pool
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,

    /// Minimum number of idle connections
    #[serde(default = "default_min_connections")]
    pub min_connections: u32,

    /// Connection timeout in seconds
    #[serde(default = "default_connect_timeout")]
    pub connect_timeout_secs: u64,

    /// Idle connection timeout in seconds
    #[serde(default = "default_idle_timeout")]
    pub idle_timeout_secs: u64,

    /// Maximum connection lifetime in seconds
    #[serde(default = "default_max_lifetime")]
    pub max_lifetime_secs: u64,
}

/// Retry policy configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,

    /// Initial retry delay in milliseconds
    #[serde(default = "default_initial_delay")]
    pub initial_delay_ms: u64,

    /// Maximum retry delay in milliseconds
    #[serde(default = "default_max_delay")]
    pub max_delay_ms: u64,

    /// Backoff multiplier
    #[serde(default = "default_backoff_multiplier")]
    pub backoff_multiplier: f64,
}

// Default value functions
fn default_ssl_mode() -> String {
    "prefer".to_string()
}

fn default_app_name() -> String {
    "llm-observatory".to_string()
}

fn default_redis_pool_size() -> u32 {
    10
}

fn default_redis_timeout() -> u64 {
    5
}

fn default_max_connections() -> u32 {
    50
}

fn default_min_connections() -> u32 {
    5
}

fn default_connect_timeout() -> u64 {
    10
}

fn default_idle_timeout() -> u64 {
    300
}

fn default_max_lifetime() -> u64 {
    1800
}

fn default_max_retries() -> u32 {
    3
}

fn default_initial_delay() -> u64 {
    100
}

fn default_max_delay() -> u64 {
    5000
}

fn default_backoff_multiplier() -> f64 {
    2.0
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_connections: default_max_connections(),
            min_connections: default_min_connections(),
            connect_timeout_secs: default_connect_timeout(),
            idle_timeout_secs: default_idle_timeout(),
            max_lifetime_secs: default_max_lifetime(),
        }
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: default_max_retries(),
            initial_delay_ms: default_initial_delay(),
            max_delay_ms: default_max_delay(),
            backoff_multiplier: default_backoff_multiplier(),
        }
    }
}

impl StorageConfig {
    /// Load configuration from environment variables.
    ///
    /// # Environment Variables
    ///
    /// **PostgreSQL (required):**
    /// - `DATABASE_URL` - Full connection string (takes precedence), OR:
    /// - `DB_HOST` - Database host (default: "localhost")
    /// - `DB_PORT` - Database port (default: 5432)
    /// - `DB_NAME` - Database name (default: "llm_observatory")
    /// - `DB_USER` - Database username (default: "postgres")
    /// - `DB_PASSWORD` - Database password (required)
    /// - `DB_SSL_MODE` - SSL mode: disable, prefer, require (default: "prefer")
    /// - `DB_APP_NAME` - Application name (default: "llm-observatory")
    ///
    /// **Redis (optional):**
    /// - `REDIS_URL` - Redis connection URL (redis://host:port/db)
    /// - `REDIS_POOL_SIZE` - Redis pool size (default: 10)
    /// - `REDIS_TIMEOUT_SECS` - Redis timeout in seconds (default: 5)
    ///
    /// **Pool Configuration:**
    /// - `DB_POOL_MAX_CONNECTIONS` - Max connections (default: 50)
    /// - `DB_POOL_MIN_CONNECTIONS` - Min connections (default: 5)
    /// - `DB_POOL_CONNECT_TIMEOUT` - Connect timeout in seconds (default: 10)
    /// - `DB_POOL_IDLE_TIMEOUT` - Idle timeout in seconds (default: 300)
    /// - `DB_POOL_MAX_LIFETIME` - Max lifetime in seconds (default: 1800)
    ///
    /// **Retry Configuration:**
    /// - `DB_RETRY_MAX_ATTEMPTS` - Max retry attempts (default: 3)
    /// - `DB_RETRY_INITIAL_DELAY_MS` - Initial delay in ms (default: 100)
    /// - `DB_RETRY_MAX_DELAY_MS` - Max delay in ms (default: 5000)
    /// - `DB_RETRY_BACKOFF_MULTIPLIER` - Backoff multiplier (default: 2.0)
    ///
    /// # Errors
    ///
    /// Returns an error if required environment variables are missing or invalid.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use llm_observatory_storage::StorageConfig;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// // Load .env file (optional)
    /// let _ = dotenvy::dotenv();
    ///
    /// // Load configuration from environment
    /// let config = StorageConfig::from_env()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_env() -> Result<Self, crate::error::StorageError> {
        use crate::error::StorageError;

        // Try to load .env file if it exists (ignore errors)
        let _ = dotenvy::dotenv();

        tracing::debug!("Loading storage configuration from environment variables");

        // Try DATABASE_URL first, otherwise build from individual components
        let postgres = if let Ok(database_url) = std::env::var("DATABASE_URL") {
            Self::parse_postgres_url(&database_url)?
        } else {
            PostgresConfig {
                host: std::env::var("DB_HOST").unwrap_or_else(|_| "localhost".to_string()),
                port: std::env::var("DB_PORT")
                    .unwrap_or_else(|_| "5432".to_string())
                    .parse()
                    .map_err(|e| StorageError::ConfigError(format!("Invalid DB_PORT: {}", e)))?,
                database: std::env::var("DB_NAME")
                    .unwrap_or_else(|_| "llm_observatory".to_string()),
                username: std::env::var("DB_USER").unwrap_or_else(|_| "postgres".to_string()),
                password: std::env::var("DB_PASSWORD").map_err(|_| {
                    StorageError::ConfigError(
                        "DB_PASSWORD environment variable is required".to_string(),
                    )
                })?,
                ssl_mode: std::env::var("DB_SSL_MODE").unwrap_or_else(|_| "prefer".to_string()),
                application_name: std::env::var("DB_APP_NAME")
                    .unwrap_or_else(|_| "llm-observatory".to_string()),
            }
        };

        // Validate PostgreSQL config
        postgres.validate()?;

        // Redis configuration (optional)
        let redis = if let Ok(redis_url) = std::env::var("REDIS_URL") {
            Some(RedisConfig {
                url: redis_url,
                pool_size: std::env::var("REDIS_POOL_SIZE")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or_else(default_redis_pool_size),
                timeout_secs: std::env::var("REDIS_TIMEOUT_SECS")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or_else(default_redis_timeout),
            })
        } else {
            None
        };

        // Pool configuration
        let pool = PoolConfig {
            max_connections: std::env::var("DB_POOL_MAX_CONNECTIONS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(default_max_connections),
            min_connections: std::env::var("DB_POOL_MIN_CONNECTIONS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(default_min_connections),
            connect_timeout_secs: std::env::var("DB_POOL_CONNECT_TIMEOUT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(default_connect_timeout),
            idle_timeout_secs: std::env::var("DB_POOL_IDLE_TIMEOUT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(default_idle_timeout),
            max_lifetime_secs: std::env::var("DB_POOL_MAX_LIFETIME")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(default_max_lifetime),
        };

        // Retry configuration
        let retry = RetryConfig {
            max_retries: std::env::var("DB_RETRY_MAX_ATTEMPTS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(default_max_retries),
            initial_delay_ms: std::env::var("DB_RETRY_INITIAL_DELAY_MS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(default_initial_delay),
            max_delay_ms: std::env::var("DB_RETRY_MAX_DELAY_MS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(default_max_delay),
            backoff_multiplier: std::env::var("DB_RETRY_BACKOFF_MULTIPLIER")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(default_backoff_multiplier),
        };

        tracing::info!(
            "Storage configuration loaded: postgres={}:{}, redis={}, pool_max={}",
            postgres.host,
            postgres.port,
            redis.is_some(),
            pool.max_connections
        );

        Ok(Self {
            postgres,
            redis,
            pool,
            retry,
        })
    }

    /// Parse a PostgreSQL connection URL into a PostgresConfig.
    ///
    /// Supports formats like:
    /// - postgres://user:pass@host:port/db
    /// - postgresql://user:pass@host:port/db?sslmode=require&application_name=app
    fn parse_postgres_url(url: &str) -> Result<PostgresConfig, crate::error::StorageError> {
        use crate::error::StorageError;

        // Simple URL parsing (for production, consider using url crate)
        let url = url.trim();

        // Remove protocol
        let url = url
            .strip_prefix("postgres://")
            .or_else(|| url.strip_prefix("postgresql://"))
            .ok_or_else(|| {
                StorageError::ConfigError("Invalid DATABASE_URL: missing protocol".to_string())
            })?;

        // Split at @ to get credentials and host parts
        let parts: Vec<&str> = url.split('@').collect();
        if parts.len() != 2 {
            return Err(StorageError::ConfigError(
                "Invalid DATABASE_URL: missing @ separator".to_string(),
            ));
        }

        // Parse credentials
        let creds: Vec<&str> = parts[0].split(':').collect();
        if creds.len() != 2 {
            return Err(StorageError::ConfigError(
                "Invalid DATABASE_URL: missing credentials".to_string(),
            ));
        }
        let username = creds[0].to_string();
        let password = creds[1].to_string();

        // Parse host, port, database, and query params
        let rest = parts[1];
        let (host_port_db, query_params) = if let Some(pos) = rest.find('?') {
            (&rest[..pos], Some(&rest[pos + 1..]))
        } else {
            (rest, None)
        };

        let host_port_db_parts: Vec<&str> = host_port_db.split('/').collect();
        if host_port_db_parts.len() != 2 {
            return Err(StorageError::ConfigError(
                "Invalid DATABASE_URL: missing database name".to_string(),
            ));
        }

        let host_port: Vec<&str> = host_port_db_parts[0].split(':').collect();
        let host = host_port[0].to_string();
        let port = if host_port.len() > 1 {
            host_port[1].parse().map_err(|e| {
                StorageError::ConfigError(format!("Invalid port in DATABASE_URL: {}", e))
            })?
        } else {
            5432
        };

        let database = host_port_db_parts[1].to_string();

        // Parse query parameters
        let mut ssl_mode = default_ssl_mode();
        let mut application_name = default_app_name();

        if let Some(params) = query_params {
            for param in params.split('&') {
                let kv: Vec<&str> = param.split('=').collect();
                if kv.len() == 2 {
                    match kv[0] {
                        "sslmode" => ssl_mode = kv[1].to_string(),
                        "application_name" => application_name = kv[1].to_string(),
                        _ => {} // Ignore unknown params
                    }
                }
            }
        }

        Ok(PostgresConfig {
            host,
            port,
            database,
            username,
            password,
            ssl_mode,
            application_name,
        })
    }

    /// Load configuration from a file.
    ///
    /// Supports YAML, TOML, and JSON formats.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed.
    pub fn from_file(path: &str) -> Result<Self, crate::error::StorageError> {
        use config::{Config, File, FileFormat};

        tracing::debug!("Loading storage configuration from file: {}", path);

        // Determine format from file extension
        let format = if path.ends_with(".yaml") || path.ends_with(".yml") {
            FileFormat::Yaml
        } else if path.ends_with(".toml") {
            FileFormat::Toml
        } else if path.ends_with(".json") {
            FileFormat::Json
        } else {
            return Err(crate::error::StorageError::ConfigError(
                "Unsupported file format. Use .yaml, .toml, or .json".to_string(),
            ));
        };

        let config = Config::builder()
            .add_source(File::new(path, format))
            .build()
            .map_err(|e| crate::error::StorageError::ConfigError(e.to_string()))?;

        let storage_config: StorageConfig = config
            .try_deserialize()
            .map_err(|e| crate::error::StorageError::ConfigError(e.to_string()))?;

        storage_config.validate()?;

        tracing::info!("Storage configuration loaded from file: {}", path);

        Ok(storage_config)
    }

    /// Validate the configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if any configuration values are invalid.
    pub fn validate(&self) -> Result<(), crate::error::StorageError> {
        self.postgres.validate()?;

        if let Some(ref redis) = self.redis {
            redis.validate()?;
        }

        self.pool.validate()?;
        self.retry.validate()?;

        Ok(())
    }

    /// Build a PostgreSQL connection string from the configuration.
    pub fn postgres_url(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}?sslmode={}&application_name={}",
            self.postgres.username,
            self.postgres.password,
            self.postgres.host,
            self.postgres.port,
            self.postgres.database,
            self.postgres.ssl_mode,
            self.postgres.application_name
        )
    }

    /// Get connection timeout as Duration.
    pub fn connect_timeout(&self) -> Duration {
        Duration::from_secs(self.pool.connect_timeout_secs)
    }

    /// Get idle timeout as Duration.
    pub fn idle_timeout(&self) -> Duration {
        Duration::from_secs(self.pool.idle_timeout_secs)
    }

    /// Get max lifetime as Duration.
    pub fn max_lifetime(&self) -> Duration {
        Duration::from_secs(self.pool.max_lifetime_secs)
    }

    /// Get the Redis connection URL if configured.
    pub fn redis_url(&self) -> Option<&str> {
        self.redis.as_ref().map(|r| r.url.as_str())
    }

    /// Get initial retry delay as Duration.
    pub fn initial_retry_delay(&self) -> Duration {
        Duration::from_millis(self.retry.initial_delay_ms)
    }

    /// Get max retry delay as Duration.
    pub fn max_retry_delay(&self) -> Duration {
        Duration::from_millis(self.retry.max_delay_ms)
    }
}

impl PostgresConfig {
    /// Validate PostgreSQL configuration.
    pub fn validate(&self) -> Result<(), crate::error::StorageError> {
        use crate::error::StorageError;

        if self.host.is_empty() {
            return Err(StorageError::ConfigError(
                "Host cannot be empty".to_string(),
            ));
        }

        if self.port == 0 {
            return Err(StorageError::ConfigError("Port cannot be 0".to_string()));
        }

        if self.database.is_empty() {
            return Err(StorageError::ConfigError(
                "Database name cannot be empty".to_string(),
            ));
        }

        if self.username.is_empty() {
            return Err(StorageError::ConfigError(
                "Username cannot be empty".to_string(),
            ));
        }

        if self.password.is_empty() {
            return Err(StorageError::ConfigError(
                "Password cannot be empty".to_string(),
            ));
        }

        // Validate SSL mode
        match self.ssl_mode.as_str() {
            "disable" | "allow" | "prefer" | "require" | "verify-ca" | "verify-full" => {}
            _ => {
                return Err(StorageError::ConfigError(format!(
                    "Invalid SSL mode: {}. Must be one of: disable, allow, prefer, require, verify-ca, verify-full",
                    self.ssl_mode
                )));
            }
        }

        Ok(())
    }
}

impl RedisConfig {
    /// Validate Redis configuration.
    pub fn validate(&self) -> Result<(), crate::error::StorageError> {
        use crate::error::StorageError;

        if self.url.is_empty() {
            return Err(StorageError::ConfigError(
                "Redis URL cannot be empty".to_string(),
            ));
        }

        if !self.url.starts_with("redis://") && !self.url.starts_with("rediss://") {
            return Err(StorageError::ConfigError(
                "Redis URL must start with redis:// or rediss://".to_string(),
            ));
        }

        if self.pool_size == 0 {
            return Err(StorageError::ConfigError(
                "Redis pool size must be greater than 0".to_string(),
            ));
        }

        Ok(())
    }
}

impl PoolConfig {
    /// Validate pool configuration.
    pub fn validate(&self) -> Result<(), crate::error::StorageError> {
        use crate::error::StorageError;

        if self.max_connections == 0 {
            return Err(StorageError::ConfigError(
                "Max connections must be greater than 0".to_string(),
            ));
        }

        if self.min_connections > self.max_connections {
            return Err(StorageError::ConfigError(format!(
                "Min connections ({}) cannot be greater than max connections ({})",
                self.min_connections, self.max_connections
            )));
        }

        if self.connect_timeout_secs == 0 {
            return Err(StorageError::ConfigError(
                "Connect timeout must be greater than 0".to_string(),
            ));
        }

        Ok(())
    }
}

impl RetryConfig {
    /// Validate retry configuration.
    pub fn validate(&self) -> Result<(), crate::error::StorageError> {
        use crate::error::StorageError;

        if self.max_retries == 0 {
            return Err(StorageError::ConfigError(
                "Max retries must be greater than 0".to_string(),
            ));
        }

        if self.initial_delay_ms == 0 {
            return Err(StorageError::ConfigError(
                "Initial delay must be greater than 0".to_string(),
            ));
        }

        if self.max_delay_ms < self.initial_delay_ms {
            return Err(StorageError::ConfigError(format!(
                "Max delay ({} ms) cannot be less than initial delay ({} ms)",
                self.max_delay_ms, self.initial_delay_ms
            )));
        }

        if self.backoff_multiplier <= 1.0 {
            return Err(StorageError::ConfigError(
                "Backoff multiplier must be greater than 1.0".to_string(),
            ));
        }

        Ok(())
    }

    /// Calculate the delay for a given retry attempt.
    ///
    /// Uses exponential backoff with the configured multiplier.
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        if attempt == 0 {
            return Duration::from_millis(self.initial_delay_ms);
        }

        let delay_ms = (self.initial_delay_ms as f64 * self.backoff_multiplier.powi(attempt as i32))
            .min(self.max_delay_ms as f64) as u64;

        Duration::from_millis(delay_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_pool_config() {
        let config = PoolConfig::default();
        assert_eq!(config.max_connections, 50);
        assert_eq!(config.min_connections, 5);
    }

    #[test]
    fn test_default_retry_config() {
        let config = RetryConfig::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.backoff_multiplier, 2.0);
    }

    #[test]
    fn test_postgres_url() {
        let config = StorageConfig {
            postgres: PostgresConfig {
                host: "localhost".to_string(),
                port: 5432,
                database: "testdb".to_string(),
                username: "user".to_string(),
                password: "pass".to_string(),
                ssl_mode: "disable".to_string(),
                application_name: "test-app".to_string(),
            },
            redis: None,
            pool: PoolConfig::default(),
            retry: RetryConfig::default(),
        };

        let url = config.postgres_url();
        assert!(url.contains("localhost"));
        assert!(url.contains("testdb"));
        assert!(url.contains("user"));
    }
}
