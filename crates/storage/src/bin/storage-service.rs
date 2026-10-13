//! # Storage Service Binary
//!
//! This binary runs the LLM Observatory Storage Service, which provides:
//! - Health check endpoints
//! - Prometheus metrics endpoints
//! - Database connection pool management
//! - Background tasks for data retention and cleanup
//!
//! The service acts as a managed database layer that other services can connect to.

use llm_observatory_storage::{
    HealthServer, StorageConfig, StorageMetrics, StoragePool, StorageResult,
};
use std::sync::Arc;
use tokio::signal;
use tokio::sync::Mutex;
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> StorageResult<()> {
    // Initialize tracing
    init_tracing();

    info!("Starting LLM Observatory Storage Service");
    info!("Version: {}", llm_observatory_storage::VERSION);

    // Load configuration
    let config = match StorageConfig::from_env() {
        Ok(cfg) => {
            info!("Configuration loaded successfully");
            cfg
        }
        Err(e) => {
            error!("Failed to load configuration: {}", e);
            return Err(e);
        }
    };

    // Log configuration (with sensitive data masked)
    log_configuration(&config);

    // Initialize storage pool
    info!("Initializing storage pool...");
    let pool = match StoragePool::new(config.clone()).await {
        Ok(p) => {
            info!("Storage pool initialized successfully");
            p
        }
        Err(e) => {
            error!("Failed to initialize storage pool: {}", e);
            return Err(e);
        }
    };

    // Verify database connectivity
    info!("Verifying database connectivity...");
    match pool.health_check().await {
        Ok(health) => {
            info!("Database health check passed");
            info!("  Connected: {}", health.is_healthy);
            info!("  Latency: {:?}", health.latency);
            if let Some(stats) = health.pool_stats {
                info!("  Pool connections: {}/{}", stats.active, stats.max_size);
            }
        }
        Err(e) => {
            error!("Database health check failed: {}", e);
            return Err(e);
        }
    }

    // Initialize metrics
    let metrics = Arc::new(Mutex::new(StorageMetrics::new()));
    info!("Metrics initialized");

    // Start health/metrics server
    let health_port = config
        .health_check_port
        .unwrap_or(std::env::var("APP_PORT").unwrap_or_else(|_| "8080".to_string()));
    let metrics_port = config
        .metrics_port
        .unwrap_or(std::env::var("METRICS_PORT").unwrap_or_else(|_| "9090".to_string()));

    info!("Starting health and metrics servers...");
    info!("  Health endpoint: http://0.0.0.0:{}/health", health_port);
    info!(
        "  Metrics endpoint: http://0.0.0.0:{}/metrics",
        metrics_port
    );

    let health_server = HealthServer::new(pool.clone(), metrics.clone());

    // Start health server in background
    let health_handle = {
        let health_server = health_server.clone();
        let health_port = health_port.clone();
        tokio::spawn(async move {
            if let Err(e) = health_server.run(&health_port).await {
                error!("Health server error: {}", e);
            }
        })
    };

    // Start metrics server in background
    let metrics_handle = {
        let health_server = health_server.clone();
        tokio::spawn(async move {
            if let Err(e) = health_server.run_metrics(&metrics_port).await {
                error!("Metrics server error: {}", e);
            }
        })
    };

    info!("Storage service is ready");
    info!("Press Ctrl+C to shutdown");

    // Wait for shutdown signal
    match signal::ctrl_c().await {
        Ok(()) => {
            info!("Shutdown signal received");
        }
        Err(err) => {
            error!("Failed to listen for shutdown signal: {}", err);
        }
    }

    // Graceful shutdown
    info!("Shutting down gracefully...");

    // Abort background tasks
    health_handle.abort();
    metrics_handle.abort();

    // Close pool
    info!("Closing storage pool...");
    pool.close().await;
    info!("Storage pool closed");

    info!("Storage service shutdown complete");
    Ok(())
}

/// Initialize tracing subscriber
fn init_tracing() {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new("info,llm_observatory_storage=debug,sqlx=warn,tokio_postgres=warn")
    });

    let log_format = std::env::var("LOG_FORMAT").unwrap_or_else(|_| "text".to_string());

    if log_format == "json" {
        // JSON format for production
        tracing_subscriber::registry()
            .with(env_filter)
            .with(tracing_subscriber::fmt::layer().json())
            .init();
    } else {
        // Text format for development
        tracing_subscriber::registry()
            .with(env_filter)
            .with(tracing_subscriber::fmt::layer().pretty())
            .init();
    }
}

/// Log configuration with sensitive data masked
fn log_configuration(config: &StorageConfig) {
    info!("Configuration:");
    info!("  Environment: {:?}", config.environment);

    // Mask database URL password
    if let Some(db_url) = &config.database_url {
        let masked_url = mask_password(db_url);
        info!("  Database URL: {}", masked_url);
    }

    // Mask Redis URL password
    if let Some(redis_url) = &config.redis_url {
        let masked_url = mask_password(redis_url);
        info!("  Redis URL: {}", masked_url);
    }

    // Pool settings
    if let Some(pool_config) = &config.pool {
        info!("  Connection Pool:");
        info!("    Min size: {}", pool_config.min_size);
        info!("    Max size: {}", pool_config.max_size);
        info!("    Timeout: {}s", pool_config.timeout_seconds);
        info!("    Idle timeout: {}s", pool_config.idle_timeout_seconds);
        info!("    Max lifetime: {}s", pool_config.max_lifetime_seconds);
    }

    // Writer settings
    if let Some(writer_config) = &config.writer {
        info!("  COPY Protocol:");
        info!("    Batch size: {}", writer_config.batch_size);
        info!("    Flush interval: {}ms", writer_config.flush_interval_ms);
        info!("    Buffer size: {} bytes", writer_config.buffer_size);
        info!(
            "    Max retries: {}",
            writer_config.max_retries.unwrap_or(3)
        );
    }

    // Retention settings
    if let Some(retention) = &config.retention {
        info!("  Data Retention:");
        info!("    Traces: {} days", retention.traces_days.unwrap_or(30));
        info!("    Metrics: {} days", retention.metrics_days.unwrap_or(90));
        info!("    Logs: {} days", retention.logs_days.unwrap_or(7));
    }
}

/// Mask password in connection URLs
fn mask_password(url: &str) -> String {
    if let Some(at_pos) = url.rfind('@') {
        if let Some(colon_pos) = url[..at_pos].rfind(':') {
            let mut masked = url.to_string();
            masked.replace_range(colon_pos + 1..at_pos, "***");
            return masked;
        }
    }
    url.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_password() {
        let url = "postgresql://user:password@localhost:5432/db";
        let masked = mask_password(url);
        assert_eq!(masked, "postgresql://user:***@localhost:5432/db");

        let url_no_pass = "postgresql://user@localhost:5432/db";
        let masked = mask_password(url_no_pass);
        assert_eq!(masked, "postgresql://user@localhost:5432/db");
    }
}
