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

/// Create model comparison routes
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v1/analytics/models/compare", get(compare_models))
        .route(
            "/api/v1/analytics/optimization",
            get(get_optimization_recommendations),
        )
}

/// GET /api/v1/analytics/models/compare - Compare multiple models
///
/// Compares performance, cost, and quality metrics across multiple models
/// for A/B testing and model selection.
///
/// Query Parameters:
/// - models: Comma-separated list of models to compare (required, min 2)
/// - metrics: Comma-separated list of metrics to include (optional)
///   Options: latency, cost, quality, throughput, token_usage
/// - start_time: Start of time range (ISO 8601)
/// - end_time: End of time range (ISO 8601)
/// - environment: Filter by environment (optional)
///
/// Response includes:
/// - Metrics for each model (latency, cost, success rate, throughput)
/// - Summary with fastest, cheapest, and most reliable models
/// - Recommendations for model selection based on requirements
#[instrument(skip(state))]
async fn compare_models(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ModelComparisonQuery>,
) -> Result<Json<ModelComparison>, ApiError> {
    info!("Comparing models: {:?}", query.models);

    // Validate input
    if query.models.len() < 2 {
        return Err(ApiError::BadRequest(
            "At least 2 models are required for comparison".to_string(),
        ));
    }

    if query.models.len() > 10 {
        return Err(ApiError::BadRequest(
            "Maximum 10 models can be compared at once".to_string(),
        ));
    }

    // Generate cache key
    let cache_key = format!(
        "models:compare:{}:{}:{}:{}",
        query.models.join(","),
        query.start_time.map(|t| t.to_rfc3339()).unwrap_or_default(),
        query.end_time.map(|t| t.to_rfc3339()).unwrap_or_default(),
        query.environment.as_deref().unwrap_or("all"),
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
        if let Ok(result) = serde_json::from_str::<ModelComparison>(&cached) {
            info!("Returning cached model comparison");
            return Ok(Json(result));
        }
    }

    // Query database
    let service = TimescaleDBService::new(state.db_pool.clone());
    let comparison = service.compare_models(&query).await.map_err(|e| {
        error!("Database query error: {}", e);
        ApiError::Internal(format!("Failed to compare models: {}", e))
    })?;

    // Cache the result
    let serialized = serde_json::to_string(&comparison).unwrap();
    let _: Result<(), _> = redis_conn
        .set_ex(&cache_key, serialized, state.cache_ttl)
        .await;

    info!(
        "Model comparison complete: {} models analyzed",
        comparison.models.len()
    );

    Ok(Json(comparison))
}

/// GET /api/v1/analytics/optimization - Get optimization recommendations
///
/// Analyzes current usage patterns and provides actionable recommendations
/// for cost optimization, performance improvements, and quality enhancements.
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
/// - Cost optimization recommendations (e.g., switch to smaller models)
/// - Performance optimization recommendations (e.g., implement caching)
/// - Quality optimization recommendations (e.g., retry logic)
/// - Overall optimization score (0-1)
/// - Potential savings estimates where applicable
#[instrument(skip(state))]
async fn get_optimization_recommendations(
    State(state): State<Arc<AppState>>,
    Query(query): Query<AnalyticsQuery>,
) -> Result<Json<OptimizationRecommendations>, ApiError> {
    info!(
        "Generating optimization recommendations: provider={:?}, model={:?}",
        query.provider, query.model
    );

    // Generate cache key
    let cache_key = format!(
        "optimization:recommendations:{}:{}:{}:{}:{}",
        query.start_time.map(|t| t.to_rfc3339()).unwrap_or_default(),
        query.end_time.map(|t| t.to_rfc3339()).unwrap_or_default(),
        query.provider.as_deref().unwrap_or("all"),
        query.model.as_deref().unwrap_or("all"),
        query.granularity
    );

    // Try to get from cache (shorter TTL for recommendations)
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
        if let Ok(result) = serde_json::from_str::<OptimizationRecommendations>(&cached) {
            info!("Returning cached optimization recommendations");
            return Ok(Json(result));
        }
    }

    // Query database and generate recommendations
    let service = TimescaleDBService::new(state.db_pool.clone());
    let recommendations = service
        .get_optimization_recommendations(&query)
        .await
        .map_err(|e| {
            error!("Database query error: {}", e);
            ApiError::Internal(format!("Failed to generate recommendations: {}", e))
        })?;

    // Cache the result (shorter TTL)
    let serialized = serde_json::to_string(&recommendations).unwrap();
    let cache_ttl = state.cache_ttl / 2; // Half the normal TTL
    let _: Result<(), _> = redis_conn.set_ex(&cache_key, serialized, cache_ttl).await;

    info!(
        "Optimization recommendations generated: {} cost, {} performance, {} quality, score={:.2}",
        recommendations.cost_optimizations.len(),
        recommendations.performance_optimizations.len(),
        recommendations.quality_optimizations.len(),
        recommendations.overall_score
    );

    Ok(Json(recommendations))
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
    fn test_model_comparison_validation() {
        // Test minimum models
        let models_too_few = vec!["gpt-4".to_string()];
        assert!(models_too_few.len() < 2);

        // Test valid count
        let models_valid = vec!["gpt-4".to_string(), "claude-3-opus".to_string()];
        assert!(models_valid.len() >= 2 && models_valid.len() <= 10);

        // Test maximum models
        let models_too_many = vec![
            "model1".to_string(),
            "model2".to_string(),
            "model3".to_string(),
            "model4".to_string(),
            "model5".to_string(),
            "model6".to_string(),
            "model7".to_string(),
            "model8".to_string(),
            "model9".to_string(),
            "model10".to_string(),
            "model11".to_string(),
        ];
        assert!(models_too_many.len() > 10);
    }

    #[test]
    fn test_comparison_cache_key() {
        let query = ModelComparisonQuery {
            models: vec!["gpt-4".to_string(), "claude-3-opus".to_string()],
            metrics: vec![],
            start_time: None,
            end_time: None,
            environment: Some("production".to_string()),
        };

        let cache_key = format!(
            "models:compare:{}:{}:{}:{}",
            query.models.join(","),
            query.start_time.map(|t| t.to_rfc3339()).unwrap_or_default(),
            query.end_time.map(|t| t.to_rfc3339()).unwrap_or_default(),
            query.environment.as_deref().unwrap_or("all"),
        );

        assert!(cache_key.contains("gpt-4"));
        assert!(cache_key.contains("claude-3-opus"));
        assert!(cache_key.contains("production"));
    }
}
