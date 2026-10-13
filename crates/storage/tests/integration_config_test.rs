//! Integration tests for storage configuration.
//!
//! This test suite validates configuration loading from various sources,
//! including environment variables, files, and direct construction.

mod common;

use common::*;
use llm_observatory_storage::config::{
    PoolConfig, PostgresConfig, RedisConfig, RetryConfig, StorageConfig,
};
use std::env;
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn test_config_from_individual_components() {
    let config = StorageConfig {
        postgres: PostgresConfig {
            host: "localhost".to_string(),
            port: 5432,
            database: "testdb".to_string(),
            username: "testuser".to_string(),
            password: "testpass".to_string(),
            ssl_mode: "disable".to_string(),
            application_name: "test-app".to_string(),
        },
        redis: None,
        pool: PoolConfig::default(),
        retry: RetryConfig::default(),
    };

    assert_eq!(config.postgres.host, "localhost");
    assert_eq!(config.postgres.port, 5432);
    assert_eq!(config.postgres.database, "testdb");
    assert!(config.redis.is_none());
}

#[test]
fn test_config_postgres_url_generation() {
    let config = StorageConfig {
        postgres: PostgresConfig {
            host: "db.example.com".to_string(),
            port: 5433,
            database: "mydb".to_string(),
            username: "user".to_string(),
            password: "pass123".to_string(),
            ssl_mode: "require".to_string(),
            application_name: "myapp".to_string(),
        },
        redis: None,
        pool: PoolConfig::default(),
        retry: RetryConfig::default(),
    };

    let url = config.postgres_url();
    assert!(url.contains("db.example.com"));
    assert!(url.contains("5433"));
    assert!(url.contains("mydb"));
    assert!(url.contains("user"));
    assert!(url.contains("pass123"));
    assert!(url.contains("sslmode=require"));
    assert!(url.contains("application_name=myapp"));
}

#[test]
fn test_config_with_redis() {
    let config = StorageConfig {
        postgres: PostgresConfig {
            host: "localhost".to_string(),
            port: 5432,
            database: "testdb".to_string(),
            username: "user".to_string(),
            password: "pass".to_string(),
            ssl_mode: "disable".to_string(),
            application_name: "test".to_string(),
        },
        redis: Some(RedisConfig {
            url: "redis://localhost:6379/0".to_string(),
            pool_size: 10,
            timeout_secs: 5,
        }),
        pool: PoolConfig::default(),
        retry: RetryConfig::default(),
    };

    assert!(config.redis.is_some());
    assert_eq!(config.redis_url(), Some("redis://localhost:6379/0"));
}

#[test]
fn test_pool_config_defaults() {
    let config = PoolConfig::default();
    assert_eq!(config.max_connections, 50);
    assert_eq!(config.min_connections, 5);
    assert_eq!(config.connect_timeout_secs, 10);
    assert_eq!(config.idle_timeout_secs, 300);
    assert_eq!(config.max_lifetime_secs, 1800);
}

#[test]
fn test_retry_config_defaults() {
    let config = RetryConfig::default();
    assert_eq!(config.max_retries, 3);
    assert_eq!(config.initial_delay_ms, 100);
    assert_eq!(config.max_delay_ms, 5000);
    assert_eq!(config.backoff_multiplier, 2.0);
}

#[test]
fn test_retry_config_delay_calculation() {
    let config = RetryConfig::default();

    let delay0 = config.delay_for_attempt(0);
    assert_eq!(delay0.as_millis(), 100);

    let delay1 = config.delay_for_attempt(1);
    assert_eq!(delay1.as_millis(), 200);

    let delay2 = config.delay_for_attempt(2);
    assert_eq!(delay2.as_millis(), 400);

    let delay3 = config.delay_for_attempt(3);
    assert_eq!(delay3.as_millis(), 800);

    // Test max delay cap
    let delay10 = config.delay_for_attempt(10);
    assert_eq!(delay10.as_millis(), 5000);
}

#[test]
fn test_postgres_config_validation_success() {
    let config = PostgresConfig {
        host: "localhost".to_string(),
        port: 5432,
        database: "test".to_string(),
        username: "user".to_string(),
        password: "pass".to_string(),
        ssl_mode: "prefer".to_string(),
        application_name: "app".to_string(),
    };

    assert!(config.validate().is_ok());
}

#[test]
fn test_postgres_config_validation_empty_host() {
    let config = PostgresConfig {
        host: "".to_string(),
        port: 5432,
        database: "test".to_string(),
        username: "user".to_string(),
        password: "pass".to_string(),
        ssl_mode: "disable".to_string(),
        application_name: "app".to_string(),
    };

    assert!(config.validate().is_err());
}

#[test]
fn test_postgres_config_validation_zero_port() {
    let config = PostgresConfig {
        host: "localhost".to_string(),
        port: 0,
        database: "test".to_string(),
        username: "user".to_string(),
        password: "pass".to_string(),
        ssl_mode: "disable".to_string(),
        application_name: "app".to_string(),
    };

    assert!(config.validate().is_err());
}

#[test]
fn test_postgres_config_validation_empty_database() {
    let config = PostgresConfig {
        host: "localhost".to_string(),
        port: 5432,
        database: "".to_string(),
        username: "user".to_string(),
        password: "pass".to_string(),
        ssl_mode: "disable".to_string(),
        application_name: "app".to_string(),
    };

    assert!(config.validate().is_err());
}

#[test]
fn test_postgres_config_validation_invalid_ssl_mode() {
    let config = PostgresConfig {
        host: "localhost".to_string(),
        port: 5432,
        database: "test".to_string(),
        username: "user".to_string(),
        password: "pass".to_string(),
        ssl_mode: "invalid".to_string(),
        application_name: "app".to_string(),
    };

    assert!(config.validate().is_err());
}

#[test]
fn test_postgres_config_validation_valid_ssl_modes() {
    let ssl_modes = vec![
        "disable",
        "allow",
        "prefer",
        "require",
        "verify-ca",
        "verify-full",
    ];

    for ssl_mode in ssl_modes {
        let config = PostgresConfig {
            host: "localhost".to_string(),
            port: 5432,
            database: "test".to_string(),
            username: "user".to_string(),
            password: "pass".to_string(),
            ssl_mode: ssl_mode.to_string(),
            application_name: "app".to_string(),
        };

        assert!(
            config.validate().is_ok(),
            "SSL mode {} should be valid",
            ssl_mode
        );
    }
}

#[test]
fn test_redis_config_validation_success() {
    let config = RedisConfig {
        url: "redis://localhost:6379".to_string(),
        pool_size: 10,
        timeout_secs: 5,
    };

    assert!(config.validate().is_ok());
}

#[test]
fn test_redis_config_validation_empty_url() {
    let config = RedisConfig {
        url: "".to_string(),
        pool_size: 10,
        timeout_secs: 5,
    };

    assert!(config.validate().is_err());
}

#[test]
fn test_redis_config_validation_invalid_url() {
    let config = RedisConfig {
        url: "http://localhost:6379".to_string(),
        pool_size: 10,
        timeout_secs: 5,
    };

    assert!(config.validate().is_err());
}

#[test]
fn test_redis_config_validation_zero_pool_size() {
    let config = RedisConfig {
        url: "redis://localhost:6379".to_string(),
        pool_size: 0,
        timeout_secs: 5,
    };

    assert!(config.validate().is_err());
}

#[test]
fn test_pool_config_validation_success() {
    let config = PoolConfig {
        max_connections: 10,
        min_connections: 2,
        connect_timeout_secs: 5,
        idle_timeout_secs: 60,
        max_lifetime_secs: 300,
    };

    assert!(config.validate().is_ok());
}

#[test]
fn test_pool_config_validation_zero_max_connections() {
    let config = PoolConfig {
        max_connections: 0,
        min_connections: 0,
        connect_timeout_secs: 5,
        idle_timeout_secs: 60,
        max_lifetime_secs: 300,
    };

    assert!(config.validate().is_err());
}

#[test]
fn test_pool_config_validation_min_greater_than_max() {
    let config = PoolConfig {
        max_connections: 5,
        min_connections: 10,
        connect_timeout_secs: 5,
        idle_timeout_secs: 60,
        max_lifetime_secs: 300,
    };

    assert!(config.validate().is_err());
}

#[test]
fn test_pool_config_validation_zero_timeout() {
    let config = PoolConfig {
        max_connections: 10,
        min_connections: 2,
        connect_timeout_secs: 0,
        idle_timeout_secs: 60,
        max_lifetime_secs: 300,
    };

    assert!(config.validate().is_err());
}

#[test]
fn test_retry_config_validation_success() {
    let config = RetryConfig {
        max_retries: 3,
        initial_delay_ms: 100,
        max_delay_ms: 5000,
        backoff_multiplier: 2.0,
    };

    assert!(config.validate().is_ok());
}

#[test]
fn test_retry_config_validation_zero_retries() {
    let config = RetryConfig {
        max_retries: 0,
        initial_delay_ms: 100,
        max_delay_ms: 5000,
        backoff_multiplier: 2.0,
    };

    assert!(config.validate().is_err());
}

#[test]
fn test_retry_config_validation_zero_initial_delay() {
    let config = RetryConfig {
        max_retries: 3,
        initial_delay_ms: 0,
        max_delay_ms: 5000,
        backoff_multiplier: 2.0,
    };

    assert!(config.validate().is_err());
}

#[test]
fn test_retry_config_validation_max_less_than_initial() {
    let config = RetryConfig {
        max_retries: 3,
        initial_delay_ms: 5000,
        max_delay_ms: 100,
        backoff_multiplier: 2.0,
    };

    assert!(config.validate().is_err());
}

#[test]
fn test_retry_config_validation_invalid_multiplier() {
    let config = RetryConfig {
        max_retries: 3,
        initial_delay_ms: 100,
        max_delay_ms: 5000,
        backoff_multiplier: 1.0,
    };

    assert!(config.validate().is_err());
}

#[test]
fn test_config_from_yaml_file() {
    let yaml_content = r#"
postgres:
  host: "db.example.com"
  port: 5432
  database: "testdb"
  username: "testuser"
  password: "testpass"
  ssl_mode: "require"
  application_name: "test-app"

pool:
  max_connections: 20
  min_connections: 5
  connect_timeout_secs: 10
  idle_timeout_secs: 300
  max_lifetime_secs: 1800

retry:
  max_retries: 5
  initial_delay_ms: 200
  max_delay_ms: 10000
  backoff_multiplier: 2.5
"#;

    let mut file = NamedTempFile::new().unwrap();
    file.write_all(yaml_content.as_bytes()).unwrap();
    let path = file.path().with_extension("yaml");
    std::fs::copy(file.path(), &path).unwrap();

    let config = StorageConfig::from_file(path.to_str().unwrap()).unwrap();

    assert_eq!(config.postgres.host, "db.example.com");
    assert_eq!(config.postgres.port, 5432);
    assert_eq!(config.pool.max_connections, 20);
    assert_eq!(config.retry.max_retries, 5);

    // Cleanup
    std::fs::remove_file(path).ok();
}

#[test]
fn test_config_duration_conversions() {
    let config = StorageConfig {
        postgres: PostgresConfig {
            host: "localhost".to_string(),
            port: 5432,
            database: "test".to_string(),
            username: "user".to_string(),
            password: "pass".to_string(),
            ssl_mode: "disable".to_string(),
            application_name: "app".to_string(),
        },
        redis: None,
        pool: PoolConfig {
            max_connections: 10,
            min_connections: 1,
            connect_timeout_secs: 15,
            idle_timeout_secs: 120,
            max_lifetime_secs: 600,
        },
        retry: RetryConfig {
            max_retries: 3,
            initial_delay_ms: 250,
            max_delay_ms: 7500,
            backoff_multiplier: 2.0,
        },
    };

    assert_eq!(config.connect_timeout().as_secs(), 15);
    assert_eq!(config.idle_timeout().as_secs(), 120);
    assert_eq!(config.max_lifetime().as_secs(), 600);
    assert_eq!(config.initial_retry_delay().as_millis(), 250);
    assert_eq!(config.max_retry_delay().as_millis(), 7500);
}

#[test]
fn test_full_config_validation() {
    let config = StorageConfig {
        postgres: PostgresConfig {
            host: "localhost".to_string(),
            port: 5432,
            database: "test".to_string(),
            username: "user".to_string(),
            password: "pass".to_string(),
            ssl_mode: "disable".to_string(),
            application_name: "app".to_string(),
        },
        redis: Some(RedisConfig {
            url: "redis://localhost:6379".to_string(),
            pool_size: 10,
            timeout_secs: 5,
        }),
        pool: PoolConfig::default(),
        retry: RetryConfig::default(),
    };

    assert!(config.validate().is_ok());
}
