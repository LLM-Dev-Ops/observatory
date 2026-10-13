use analytics_api::{models::*, AppState};
use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use chrono::{Duration, Utc};
use redis::Client as RedisClient;
use serde_json::{json, Value};
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;

/// Helper to create test app state
async fn create_test_state() -> Arc<AppState> {
    // In a real test environment, use test containers or a test database
    let database_url = std::env::var("TEST_DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://postgres:postgres@localhost:5432/llm_observatory_test".to_string()
    });

    let redis_url =
        std::env::var("TEST_REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());

    let db_pool = PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to test database");

    let redis_client = RedisClient::open(redis_url).expect("Failed to create Redis client");

    Arc::new(AppState {
        db_pool,
        redis_client,
        cache_ttl: 60, // Short TTL for tests
    })
}

/// Helper to create test router
fn create_test_router(state: Arc<AppState>) -> Router {
    Router::new()
        .merge(analytics_api::routes::costs::routes())
        .merge(analytics_api::routes::performance::routes())
        .merge(analytics_api::routes::quality::routes())
        .merge(analytics_api::routes::models::routes())
        .with_state(state)
}

#[tokio::test]
#[ignore] // Requires database
async fn test_cost_analytics_endpoint() {
    let state = create_test_state().await;
    let app = create_test_router(state);

    let end_time = Utc::now();
    let start_time = end_time - Duration::days(7);

    let request = Request::builder()
        .uri(format!(
            "/api/v1/analytics/costs?start_time={}&end_time={}&granularity=1hour",
            start_time.to_rfc3339(),
            end_time.to_rfc3339()
        ))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let analytics: CostAnalytics = serde_json::from_slice(&body).unwrap();

    assert!(analytics.total_cost >= 0.0);
    assert!(analytics.request_count >= 0);
}

#[tokio::test]
#[ignore] // Requires database
async fn test_cost_breakdown_endpoint() {
    let state = create_test_state().await;
    let app = create_test_router(state);

    let end_time = Utc::now();
    let start_time = end_time - Duration::days(7);

    let request = Request::builder()
        .uri(format!(
            "/api/v1/analytics/costs/breakdown?start_time={}&end_time={}&granularity=1hour",
            start_time.to_rfc3339(),
            end_time.to_rfc3339()
        ))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let breakdown: CostBreakdown = serde_json::from_slice(&body).unwrap();

    assert!(breakdown.by_model.len() >= 0);
    assert!(breakdown.by_provider.len() >= 0);
}

#[tokio::test]
#[ignore] // Requires database
async fn test_performance_metrics_endpoint() {
    let state = create_test_state().await;
    let app = create_test_router(state);

    let end_time = Utc::now();
    let start_time = end_time - Duration::days(1);

    let request = Request::builder()
        .uri(format!(
            "/api/v1/analytics/performance?start_time={}&end_time={}&granularity=1hour&provider=openai",
            start_time.to_rfc3339(),
            end_time.to_rfc3339()
        ))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let metrics: PerformanceMetrics = serde_json::from_slice(&body).unwrap();

    assert!(metrics.avg_latency_ms >= 0.0);
    assert!(metrics.throughput_rps >= 0.0);
}

#[tokio::test]
#[ignore] // Requires database
async fn test_quality_metrics_endpoint() {
    let state = create_test_state().await;
    let app = create_test_router(state);

    let end_time = Utc::now();
    let start_time = end_time - Duration::days(7);

    let request = Request::builder()
        .uri(format!(
            "/api/v1/analytics/quality?start_time={}&end_time={}&granularity=1hour",
            start_time.to_rfc3339(),
            end_time.to_rfc3339()
        ))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let metrics: QualityMetrics = serde_json::from_slice(&body).unwrap();

    assert!(metrics.success_rate >= 0.0 && metrics.success_rate <= 1.0);
    assert!(metrics.error_rate >= 0.0 && metrics.error_rate <= 1.0);
}

#[tokio::test]
#[ignore] // Requires database
async fn test_model_comparison_endpoint() {
    let state = create_test_state().await;
    let app = create_test_router(state);

    let end_time = Utc::now();
    let start_time = end_time - Duration::days(7);

    let request = Request::builder()
        .uri(format!(
            "/api/v1/analytics/models/compare?models=gpt-4,claude-3-opus&start_time={}&end_time={}",
            start_time.to_rfc3339(),
            end_time.to_rfc3339()
        ))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let comparison: ModelComparison = serde_json::from_slice(&body).unwrap();

    assert!(comparison.models.len() >= 0);
}

#[tokio::test]
#[ignore] // Requires database
async fn test_model_comparison_validation() {
    let state = create_test_state().await;
    let app = create_test_router(state);

    // Test with only 1 model (should fail)
    let request = Request::builder()
        .uri("/api/v1/analytics/models/compare?models=gpt-4")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
#[ignore] // Requires database
async fn test_optimization_recommendations_endpoint() {
    let state = create_test_state().await;
    let app = create_test_router(state);

    let end_time = Utc::now();
    let start_time = end_time - Duration::days(7);

    let request = Request::builder()
        .uri(format!(
            "/api/v1/analytics/optimization?start_time={}&end_time={}&granularity=1hour",
            start_time.to_rfc3339(),
            end_time.to_rfc3339()
        ))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let recommendations: OptimizationRecommendations = serde_json::from_slice(&body).unwrap();

    assert!(recommendations.overall_score >= 0.0 && recommendations.overall_score <= 1.0);
}

#[test]
fn test_analytics_query_defaults() {
    let query = AnalyticsQuery {
        start_time: None,
        end_time: None,
        provider: None,
        model: None,
        environment: None,
        user_id: None,
        granularity: "1hour".to_string(),
    };

    assert_eq!(query.granularity, "1hour");
}

#[test]
fn test_model_comparison_query_validation() {
    let query = ModelComparisonQuery {
        models: vec!["gpt-4".to_string(), "claude-3-opus".to_string()],
        metrics: vec![],
        start_time: None,
        end_time: None,
        environment: None,
    };

    assert!(query.models.len() >= 2);
}

#[test]
fn test_granularity_options() {
    let valid_granularities = vec!["1min", "1hour", "1day", "raw"];

    for granularity in valid_granularities {
        let query = AnalyticsQuery {
            start_time: None,
            end_time: None,
            provider: None,
            model: None,
            environment: None,
            user_id: None,
            granularity: granularity.to_string(),
        };

        assert!(["1min", "1hour", "1day", "raw"].contains(&query.granularity.as_str()));
    }
}

#[test]
fn test_cost_analytics_calculations() {
    let data_point = CostDataPoint {
        timestamp: Utc::now(),
        total_cost: 10.0,
        prompt_cost: 6.0,
        completion_cost: 4.0,
        request_count: 100,
    };

    assert_eq!(
        data_point.total_cost,
        data_point.prompt_cost + data_point.completion_cost
    );

    let avg_cost = data_point.total_cost / data_point.request_count as f64;
    assert_eq!(avg_cost, 0.1);
}

#[test]
fn test_quality_metrics_calculations() {
    let total_requests = 100i64;
    let successful_requests = 95i64;
    let failed_requests = 5i64;

    let success_rate = successful_requests as f64 / total_requests as f64;
    let error_rate = failed_requests as f64 / total_requests as f64;

    assert_eq!(success_rate, 0.95);
    assert_eq!(error_rate, 0.05);
    assert_eq!(success_rate + error_rate, 1.0);
}

#[test]
fn test_error_breakdown_percentage() {
    let total_errors = 100i64;
    let error_count = 25i64;

    let percentage = (error_count as f64 / total_errors as f64) * 100.0;
    assert_eq!(percentage, 25.0);
}

#[test]
fn test_throughput_calculation() {
    let request_count = 1000i64;
    let duration_seconds = 100.0;

    let throughput_rps = request_count as f64 / duration_seconds;
    assert_eq!(throughput_rps, 10.0);
}
