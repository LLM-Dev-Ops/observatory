//! Health check and metrics HTTP endpoints.
//!
//! This module provides HTTP endpoints for:
//! - `/health` - Health check for PostgreSQL and Redis
//! - `/metrics` - Prometheus metrics scraping endpoint
//!
//! # Usage
//!
//! ```no_run
//! use llm_observatory_storage::{StoragePool, StorageConfig};
//! use llm_observatory_storage::health::HealthServer;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = StorageConfig::from_env()?;
//! let pool = StoragePool::new(config).await?;
//!
//! // Start health and metrics server on port 9090
//! let server = HealthServer::new(pool);
//! server.serve("0.0.0.0:9090").await?;
//! # Ok(())
//! # }
//! ```

use crate::pool::{HealthCheckResult, PoolStats, StoragePool};
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

/// Health and metrics server.
///
/// Provides HTTP endpoints for health checks and Prometheus metrics.
pub struct HealthServer {
    pool: StoragePool,
    prometheus_handle: PrometheusHandle,
}

impl HealthServer {
    /// Create a new health server.
    ///
    /// This initializes the Prometheus metrics exporter.
    pub fn new(pool: StoragePool) -> Self {
        let prometheus_handle = PrometheusBuilder::new()
            .install_recorder()
            .expect("Failed to install Prometheus recorder");

        Self {
            pool,
            prometheus_handle,
        }
    }

    /// Start the health and metrics server.
    ///
    /// # Arguments
    ///
    /// * `addr` - Address to bind to (e.g., "0.0.0.0:9090")
    ///
    /// # Errors
    ///
    /// Returns an error if the server fails to start.
    pub async fn serve(self, addr: &str) -> Result<(), Box<dyn std::error::Error>> {
        let addr: SocketAddr = addr.parse()?;

        let app_state = Arc::new(AppState {
            pool: self.pool,
            prometheus_handle: self.prometheus_handle,
        });

        let app = Router::new()
            .route("/health", get(health_handler))
            .route("/health/live", get(liveness_handler))
            .route("/health/ready", get(readiness_handler))
            .route("/metrics", get(metrics_handler))
            .with_state(app_state);

        tracing::info!("Health and metrics server listening on {}", addr);

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }

    /// Get the router for embedding in another service.
    ///
    /// This is useful when you want to add health and metrics endpoints
    /// to an existing axum application.
    pub fn router(self) -> Router {
        let app_state = Arc::new(AppState {
            pool: self.pool,
            prometheus_handle: self.prometheus_handle,
        });

        Router::new()
            .route("/health", get(health_handler))
            .route("/health/live", get(liveness_handler))
            .route("/health/ready", get(readiness_handler))
            .route("/metrics", get(metrics_handler))
            .with_state(app_state)
    }
}

/// Shared application state.
struct AppState {
    pool: StoragePool,
    prometheus_handle: PrometheusHandle,
}

/// Health check response.
#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    /// Overall health status
    pub status: String,

    /// Timestamp of the health check
    pub timestamp: String,

    /// Database health details
    pub database: DatabaseHealth,

    /// Connection pool statistics
    pub pool_stats: PoolStatsResponse,

    /// Health check duration in milliseconds
    pub check_duration_ms: u64,
}

/// Database health details.
#[derive(Debug, Serialize, Deserialize)]
pub struct DatabaseHealth {
    /// PostgreSQL health status
    pub postgres: ServiceHealth,

    /// Redis health status (if configured)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redis: Option<ServiceHealth>,
}

/// Individual service health.
#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceHealth {
    /// Service status
    pub status: String,

    /// Latency in milliseconds
    pub latency_ms: f64,

    /// Error message if unhealthy
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Pool statistics response.
#[derive(Debug, Serialize, Deserialize)]
pub struct PoolStatsResponse {
    /// Total connections in pool
    pub size: u32,

    /// Active connections
    pub active: u32,

    /// Idle connections
    pub idle: u32,

    /// Maximum connections
    pub max_connections: u32,

    /// Minimum connections
    pub min_connections: u32,

    /// Utilization percentage
    pub utilization_percent: f64,

    /// Whether pool is near capacity
    pub near_capacity: bool,
}

impl From<PoolStats> for PoolStatsResponse {
    fn from(stats: PoolStats) -> Self {
        Self {
            size: stats.postgres_size,
            active: stats.postgres_active,
            idle: stats.postgres_idle,
            max_connections: stats.postgres_max_connections,
            min_connections: stats.postgres_min_connections,
            utilization_percent: stats.utilization_percent(),
            near_capacity: stats.is_near_capacity(),
        }
    }
}

/// Health check handler.
///
/// Returns comprehensive health information including database status,
/// pool statistics, and latency measurements.
async fn health_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<HealthResponse>, AppError> {
    let start = Instant::now();

    // Check PostgreSQL
    let pg_start = Instant::now();
    let pg_result = state.pool.health_check_postgres().await;
    let pg_latency = pg_start.elapsed().as_secs_f64() * 1000.0;

    let postgres = match pg_result {
        Ok(_) => ServiceHealth {
            status: "healthy".to_string(),
            latency_ms: pg_latency,
            error: None,
        },
        Err(e) => ServiceHealth {
            status: "unhealthy".to_string(),
            latency_ms: pg_latency,
            error: Some(e.to_string()),
        },
    };

    // Check Redis if configured
    let redis = if state.pool.redis().is_some() {
        let redis_start = Instant::now();
        let redis_result = state.pool.health_check_redis().await;
        let redis_latency = redis_start.elapsed().as_secs_f64() * 1000.0;

        Some(match redis_result {
            Ok(_) => ServiceHealth {
                status: "healthy".to_string(),
                latency_ms: redis_latency,
                error: None,
            },
            Err(e) => ServiceHealth {
                status: "unhealthy".to_string(),
                latency_ms: redis_latency,
                error: Some(e.to_string()),
            },
        })
    } else {
        None
    };

    // Get pool statistics
    let pool_stats = state.pool.stats();

    // Determine overall status
    let overall_healthy = postgres.status == "healthy"
        && redis
            .as_ref()
            .map(|r| r.status == "healthy")
            .unwrap_or(true);

    let status = if overall_healthy {
        "healthy".to_string()
    } else {
        "unhealthy".to_string()
    };

    let check_duration_ms = start.elapsed().as_millis() as u64;

    let response = HealthResponse {
        status,
        timestamp: chrono::Utc::now().to_rfc3339(),
        database: DatabaseHealth { postgres, redis },
        pool_stats: pool_stats.into(),
        check_duration_ms,
    };

    // Return 503 if unhealthy
    if response.status == "unhealthy" {
        return Err(AppError::Unhealthy(response));
    }

    Ok(Json(response))
}

/// Liveness probe handler.
///
/// Returns 200 OK if the service is running. This doesn't check external dependencies.
async fn liveness_handler() -> impl IntoResponse {
    (StatusCode::OK, "alive")
}

/// Readiness probe handler.
///
/// Returns 200 OK if the service is ready to accept traffic.
/// Checks database connectivity.
async fn readiness_handler(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    // Quick health check
    state
        .pool
        .health_check_postgres()
        .await
        .map_err(|_| AppError::NotReady)?;

    Ok((StatusCode::OK, "ready"))
}

/// Metrics handler for Prometheus scraping.
async fn metrics_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    // Update pool metrics before rendering
    let stats = state.pool.stats();
    let metrics = crate::metrics::StorageMetrics::new();
    metrics.update_pool_connections(
        stats.postgres_active,
        stats.postgres_idle,
        stats.postgres_max_connections,
    );

    // Render Prometheus metrics
    state.prometheus_handle.render()
}

/// Application error types.
#[derive(Debug)]
enum AppError {
    Unhealthy(HealthResponse),
    NotReady,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::Unhealthy(health) => {
                (StatusCode::SERVICE_UNAVAILABLE, Json(health)).into_response()
            }
            AppError::NotReady => (StatusCode::SERVICE_UNAVAILABLE, "not ready").into_response(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_stats_conversion() {
        let stats = PoolStats {
            postgres_size: 10,
            postgres_idle: 5,
            postgres_active: 5,
            redis_connected: true,
            postgres_max_connections: 20,
            postgres_min_connections: 2,
        };

        let response: PoolStatsResponse = stats.into();
        assert_eq!(response.size, 10);
        assert_eq!(response.active, 5);
        assert_eq!(response.idle, 5);
        assert_eq!(response.max_connections, 20);
        assert!(!response.near_capacity);
    }

    #[test]
    fn test_service_health_serialization() {
        let health = ServiceHealth {
            status: "healthy".to_string(),
            latency_ms: 5.2,
            error: None,
        };

        let json = serde_json::to_string(&health).unwrap();
        assert!(json.contains("healthy"));
        assert!(json.contains("5.2"));
    }
}
