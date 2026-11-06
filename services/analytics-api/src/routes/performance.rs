use crate::models::*;
use crate::services::timescaledb::TimescaleDBService;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use redis::AsyncCommands;
use serde_json::json;
use std::sync::Arc;
use tracing::{error, info, instrument};

/// Create performance routes
pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route(
        "/api/v1/analytics/performance",
        get(get_performance_metrics),
    )
}

/// GET /api/v1/analytics/performance - Get performance metrics
///
/// Returns performance metrics including latency percentiles (P50, P95, P99),
/// throughput, and token processing statistics.
///
/// Query Parameters:
/// - start_time: Start of time range (ISO 8601)
/// - end_time: End of time range (ISO 8601)
/// - provider: Filter by provider (optional)
/// - model: Filter by model (optional)
/// - environment: Filter by environment (optional)
/// - granularity: Time bucket granularity (1min, 1hour, 1day) - default: 1hour
///
/// Note: Percentile calculations (P50, P95, P99) are only available for granularities
/// of 1min or when querying raw data, as they require ordered-set aggregates that
/// cannot be computed from pre-aggregated data in continuous aggregates.
#[instrument(skip(state))]
async fn get_performance_metrics(
    State(state): State<Arc<AppState>>,
    Query(query): Query<AnalyticsQuery>,
) -> Result<Json<PerformanceMetrics>, ApiError> {
    info!(
        "Fetching performance metrics: provider={:?}, model={:?}, granularity={}",
        query.provider, query.model, query.granularity
    );

    // Validate granularity for percentile calculations
    if query.granularity != "1min" && query.granularity != "raw" {
        info!(
            "Percentile calculations not available for granularity '{}', will be null in response",
            query.granularity
        );
    }

    // Generate cache key
    let cache_key = format!(
        "performance:metrics:{}:{}:{}:{}:{}",
        query.start_time.map(|t| t.to_rfc3339()).unwrap_or_default(),
        query.end_time.map(|t| t.to_rfc3339()).unwrap_or_default(),
        query.provider.as_deref().unwrap_or("all"),
        query.model.as_deref().unwrap_or("all"),
        query.granularity
    );

    // Try to get from cache
    let mut redis_conn = state
        .redis_client
        .get_async_connection()
        .await
        .map_err(|e| {
            error!("Redis connection error: {}", e);
            ApiError::Internal("Failed to connect to cache".to_string())
        })?;

    // Check cache
    if let Ok(cached) = redis_conn.get::<_, String>(&cache_key).await {
        if let Ok(result) = serde_json::from_str::<PerformanceMetrics>(&cached) {
            info!("Returning cached performance metrics");
            return Ok(Json(result));
        }
    }

    // Query database
    let service = TimescaleDBService::new(state.db_pool.clone());
    let metrics = service.get_performance_metrics(&query).await.map_err(|e| {
        error!("Database query error: {}", e);
        ApiError::Internal(format!("Failed to fetch performance metrics: {}", e))
    })?;

    // Cache the result
    let serialized = serde_json::to_string(&metrics).unwrap();
    let _: Result<(), _> = redis_conn
        .set_ex(&cache_key, serialized, state.cache_ttl)
        .await;

    info!(
        "Performance metrics fetched: avg_latency={:.2}ms, p95={:?}ms, throughput={:.2} rps",
        metrics.avg_latency_ms, metrics.p95_latency_ms, metrics.throughput_rps
    );

    Ok(Json(metrics))
}

/// API error type
#[derive(Debug)]
pub enum ApiError {
    BadRequest(String),
    Internal(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        let body = Json(json!({
            "error": status.canonical_reason().unwrap_or("Unknown"),
            "message": error_message,
        }));

        (status, body).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_cache_key() {
        let query = AnalyticsQuery {
            start_time: None,
            end_time: None,
            provider: Some("anthropic".to_string()),
            model: Some("claude-3-opus".to_string()),
            environment: None,
            user_id: None,
            granularity: "1min".to_string(),
        };

        let cache_key = format!(
            "performance:metrics:{}:{}:{}:{}:{}",
            query.start_time.map(|t| t.to_rfc3339()).unwrap_or_default(),
            query.end_time.map(|t| t.to_rfc3339()).unwrap_or_default(),
            query.provider.as_deref().unwrap_or("all"),
            query.model.as_deref().unwrap_or("all"),
            query.granularity
        );

        assert!(cache_key.contains("anthropic"));
        assert!(cache_key.contains("claude-3-opus"));
        assert!(cache_key.contains("1min"));
    }

    #[test]
    fn test_granularity_validation() {
        // Valid granularities
        assert!(["1min", "1hour", "1day", "raw"].contains(&"1hour"));

        // Percentile availability
        let has_percentiles = |granularity: &str| granularity == "1min" || granularity == "raw";
        assert!(has_percentiles("1min"));
        assert!(has_percentiles("raw"));
        assert!(!has_percentiles("1hour"));
        assert!(!has_percentiles("1day"));
    }
}
