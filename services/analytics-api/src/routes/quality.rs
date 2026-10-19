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

/// Create quality routes
pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/api/v1/analytics/quality", get(get_quality_metrics))
}

/// GET /api/v1/analytics/quality - Get quality metrics
///
/// Returns quality metrics including success/error rates, error breakdowns,
/// and feedback scores.
///
/// Query Parameters:
/// - start_time: Start of time range (ISO 8601)
/// - end_time: End of time range (ISO 8601)
/// - provider: Filter by provider (optional)
/// - model: Filter by model (optional)
/// - environment: Filter by environment (optional)
/// - granularity: Time bucket granularity (1min, 1hour, 1day) - default: 1hour
///
/// Response includes:
/// - Total requests, successful and failed counts
/// - Success and error rates
/// - Error breakdown by type with sample messages
/// - Time series showing quality trends
#[instrument(skip(state))]
async fn get_quality_metrics(
    State(state): State<Arc<AppState>>,
    Query(query): Query<AnalyticsQuery>,
) -> Result<Json<QualityMetrics>, ApiError> {
    info!(
        "Fetching quality metrics: provider={:?}, model={:?}, granularity={}",
        query.provider, query.model, query.granularity
    );

    // Generate cache key
    let cache_key = format!(
        "quality:metrics:{}:{}:{}:{}:{}:{}",
        query.start_time.map(|t| t.to_rfc3339()).unwrap_or_default(),
        query.end_time.map(|t| t.to_rfc3339()).unwrap_or_default(),
        query.provider.as_deref().unwrap_or("all"),
        query.model.as_deref().unwrap_or("all"),
        query.environment.as_deref().unwrap_or("all"),
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
        if let Ok(result) = serde_json::from_str::<QualityMetrics>(&cached) {
            info!("Returning cached quality metrics");
            return Ok(Json(result));
        }
    }

    // Query database
    let service = TimescaleDBService::new(state.db_pool.clone());
    let metrics = service.get_quality_metrics(&query).await.map_err(|e| {
        error!("Database query error: {}", e);
        ApiError::Internal(format!("Failed to fetch quality metrics: {}", e))
    })?;

    // Cache the result
    let serialized = serde_json::to_string(&metrics).unwrap();
    let _: Result<(), _> = redis_conn
        .set_ex(&cache_key, serialized, state.cache_ttl)
        .await;

    info!(
        "Quality metrics fetched: success_rate={:.2}%, error_rate={:.2}%, total_requests={}",
        metrics.success_rate * 100.0,
        metrics.error_rate * 100.0,
        metrics.total_requests
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
    fn test_quality_cache_key() {
        let query = AnalyticsQuery {
            start_time: None,
            end_time: None,
            provider: Some("openai".to_string()),
            model: Some("gpt-4".to_string()),
            environment: Some("production".to_string()),
            user_id: None,
            granularity: "1hour".to_string(),
        };

        let cache_key = format!(
            "quality:metrics:{}:{}:{}:{}:{}:{}",
            query.start_time.map(|t| t.to_rfc3339()).unwrap_or_default(),
            query.end_time.map(|t| t.to_rfc3339()).unwrap_or_default(),
            query.provider.as_deref().unwrap_or("all"),
            query.model.as_deref().unwrap_or("all"),
            query.environment.as_deref().unwrap_or("all"),
            query.granularity
        );

        assert!(cache_key.contains("openai"));
        assert!(cache_key.contains("gpt-4"));
        assert!(cache_key.contains("production"));
    }

    #[test]
    fn test_error_rate_calculation() {
        // Test cases for error rate calculation
        let total = 100;
        let errors = 5;
        let error_rate = errors as f64 / total as f64;
        assert_eq!(error_rate, 0.05);

        let success_rate = (total - errors) as f64 / total as f64;
        assert_eq!(success_rate, 0.95);
    }
}
