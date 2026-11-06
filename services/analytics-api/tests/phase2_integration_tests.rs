///! Phase 2 Integration Tests
///!
///! Tests for advanced filtering, search endpoint, and full-text search functionality.
///!
///! Run with: cargo test --test phase2_integration_tests -- --ignored
///! Requires: PostgreSQL, Redis, and test data
use analytics_api::{
    middleware::auth::{AuthContext, Role},
    models::*,
    routes,
};
use axum::{
    body::Body,
    http::{header, Request, StatusCode},
    Router,
};
use chrono::Utc;
use redis::Client as RedisClient;
use serde_json::json;
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;

/// Helper to create test app state
async fn create_test_state() -> Arc<AppState> {
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

/// Helper to create test router with trace routes
fn create_test_router(state: Arc<AppState>) -> Router {
    Router::new()
        .merge(routes::traces::routes())
        .with_state(state)
}

/// Helper to create a mock JWT token (for testing only)
fn create_mock_jwt() -> String {
    // In real tests, use a proper JWT library to generate valid tokens
    // This is a placeholder that assumes auth middleware is mocked
    "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJ0ZXN0X3VzZXIiLCJyb2xlIjoiZGV2ZWxvcGVyIiwib3JnX2lkIjoidGVzdF9vcmciLCJleHAiOjk5OTk5OTk5OTl9.test".to_string()
}

/// Helper to insert test trace data
async fn insert_test_trace(
    pool: &PgPool,
    provider: &str,
    model: &str,
    input: &str,
    output: &str,
    duration_ms: i32,
    cost_usd: f64,
) -> String {
    let trace_id = Uuid::new_v4().to_string();
    let span_id = Uuid::new_v4().to_string();

    sqlx::query(
        r#"
        INSERT INTO llm_traces (
            ts, trace_id, span_id, provider, model, input_text, output_text,
            duration_ms, total_cost_usd, status_code, environment
        ) VALUES (
            NOW(), $1, $2, $3, $4, $5, $6, $7, $8, 'SUCCESS', 'test'
        )
        "#,
    )
    .bind(&trace_id)
    .bind(&span_id)
    .bind(provider)
    .bind(model)
    .bind(input)
    .bind(output)
    .bind(duration_ms)
    .bind(cost_usd)
    .execute(pool)
    .await
    .expect("Failed to insert test trace");

    trace_id
}

// ============================================================================
// Advanced Search Endpoint Tests
// ============================================================================

#[tokio::test]
#[ignore] // Requires database and auth setup
async fn test_advanced_search_simple_filter() {
    let state = create_test_state().await;
    let app = create_test_router(state.clone());

    // Insert test data
    insert_test_trace(
        &state.db_pool,
        "openai",
        "gpt-4",
        "test input",
        "test output",
        1000,
        0.01,
    )
    .await;

    let search_request = json!({
        "filter": {
            "field": "provider",
            "operator": "eq",
            "value": "openai"
        },
        "limit": 10
    });

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/traces/search")
        .header(header::CONTENT_TYPE, "application/json")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", create_mock_jwt()),
        )
        .body(Body::from(serde_json::to_string(&search_request).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let result: PaginatedTraceResponse = serde_json::from_slice(&body).unwrap();

    assert_eq!(result.status, ResponseStatus::Success);
    assert!(result.data.len() > 0);
}

#[tokio::test]
#[ignore]
async fn test_advanced_search_comparison_operators() {
    let state = create_test_state().await;
    let app = create_test_router(state.clone());

    // Insert traces with different durations
    insert_test_trace(
        &state.db_pool,
        "openai",
        "gpt-4",
        "test",
        "output",
        500,
        0.01,
    )
    .await;
    insert_test_trace(
        &state.db_pool,
        "openai",
        "gpt-4",
        "test",
        "output",
        1500,
        0.02,
    )
    .await;
    insert_test_trace(
        &state.db_pool,
        "openai",
        "gpt-4",
        "test",
        "output",
        2500,
        0.03,
    )
    .await;

    // Test GTE operator
    let search_request = json!({
        "filter": {
            "field": "duration_ms",
            "operator": "gte",
            "value": 1000
        },
        "limit": 100
    });

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/traces/search")
        .header(header::CONTENT_TYPE, "application/json")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", create_mock_jwt()),
        )
        .body(Body::from(serde_json::to_string(&search_request).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let result: PaginatedTraceResponse = serde_json::from_slice(&body).unwrap();

    // Should return only traces with duration >= 1000ms
    for trace in &result.data {
        if let Some(duration) = trace.duration_ms {
            assert!(duration >= 1000);
        }
    }
}

#[tokio::test]
#[ignore]
async fn test_advanced_search_in_operator() {
    let state = create_test_state().await;
    let app = create_test_router(state.clone());

    insert_test_trace(
        &state.db_pool,
        "openai",
        "gpt-4",
        "test",
        "output",
        1000,
        0.01,
    )
    .await;
    insert_test_trace(
        &state.db_pool,
        "anthropic",
        "claude-3-opus",
        "test",
        "output",
        1000,
        0.02,
    )
    .await;
    insert_test_trace(
        &state.db_pool,
        "google",
        "gemini-pro",
        "test",
        "output",
        1000,
        0.01,
    )
    .await;

    let search_request = json!({
        "filter": {
            "field": "provider",
            "operator": "in",
            "value": ["openai", "anthropic"]
        },
        "limit": 100
    });

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/traces/search")
        .header(header::CONTENT_TYPE, "application/json")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", create_mock_jwt()),
        )
        .body(Body::from(serde_json::to_string(&search_request).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let result: PaginatedTraceResponse = serde_json::from_slice(&body).unwrap();

    // All results should be from openai or anthropic
    for trace in &result.data {
        assert!(trace.provider == "openai" || trace.provider == "anthropic");
    }
}

#[tokio::test]
#[ignore]
async fn test_advanced_search_complex_nested_filter() {
    let state = create_test_state().await;
    let app = create_test_router(state.clone());

    insert_test_trace(
        &state.db_pool,
        "openai",
        "gpt-4",
        "test",
        "output",
        1500,
        0.05,
    )
    .await;
    insert_test_trace(
        &state.db_pool,
        "openai",
        "gpt-3.5-turbo",
        "test",
        "output",
        500,
        0.001,
    )
    .await;
    insert_test_trace(
        &state.db_pool,
        "anthropic",
        "claude-3-opus",
        "test",
        "output",
        2000,
        0.08,
    )
    .await;

    // Complex filter: (provider = openai) AND (duration > 1000 OR cost > 0.03)
    let search_request = json!({
        "filter": {
            "operator": "AND",
            "filters": [
                {
                    "field": "provider",
                    "operator": "eq",
                    "value": "openai"
                },
                {
                    "operator": "OR",
                    "filters": [
                        {
                            "field": "duration_ms",
                            "operator": "gt",
                            "value": 1000
                        },
                        {
                            "field": "total_cost_usd",
                            "operator": "gt",
                            "value": 0.03
                        }
                    ]
                }
            ]
        },
        "limit": 100
    });

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/traces/search")
        .header(header::CONTENT_TYPE, "application/json")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", create_mock_jwt()),
        )
        .body(Body::from(serde_json::to_string(&search_request).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let result: PaginatedTraceResponse = serde_json::from_slice(&body).unwrap();

    // Should return only OpenAI traces that match the OR condition
    for trace in &result.data {
        assert_eq!(trace.provider, "openai");
        let matches_or = trace.duration_ms.map(|d| d > 1000).unwrap_or(false)
            || trace.total_cost_usd.map(|c| c > 0.03).unwrap_or(false);
        assert!(matches_or);
    }
}

// ============================================================================
// Full-Text Search Tests
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_fulltext_search_input_text() {
    let state = create_test_state().await;
    let app = create_test_router(state.clone());

    // Insert traces with specific text
    insert_test_trace(
        &state.db_pool,
        "openai",
        "gpt-4",
        "authentication error occurred",
        "resolved",
        1000,
        0.01,
    )
    .await;
    insert_test_trace(
        &state.db_pool,
        "openai",
        "gpt-4",
        "user login successful",
        "confirmed",
        1000,
        0.01,
    )
    .await;
    insert_test_trace(
        &state.db_pool,
        "openai",
        "gpt-4",
        "database connection failed",
        "error",
        1000,
        0.01,
    )
    .await;

    // Search for "authentication error"
    let search_request = json!({
        "filter": {
            "field": "input_text",
            "operator": "search",
            "value": "authentication error"
        },
        "limit": 100
    });

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/traces/search")
        .header(header::CONTENT_TYPE, "application/json")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", create_mock_jwt()),
        )
        .body(Body::from(serde_json::to_string(&search_request).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let result: PaginatedTraceResponse = serde_json::from_slice(&body).unwrap();

    // Should find the trace with "authentication error" text
    assert!(result.data.len() > 0);
    assert!(result.data.iter().any(|t| t
        .input_text
        .as_ref()
        .map(|s| s.contains("authentication"))
        .unwrap_or(false)));
}

#[tokio::test]
#[ignore]
async fn test_fulltext_search_combined_with_filters() {
    let state = create_test_state().await;
    let app = create_test_router(state.clone());

    insert_test_trace(
        &state.db_pool,
        "openai",
        "gpt-4",
        "error in authentication",
        "failed",
        2000,
        0.05,
    )
    .await;
    insert_test_trace(
        &state.db_pool,
        "anthropic",
        "claude-3-opus",
        "error in authentication",
        "failed",
        500,
        0.01,
    )
    .await;
    insert_test_trace(
        &state.db_pool,
        "openai",
        "gpt-4",
        "successful login",
        "ok",
        1500,
        0.03,
    )
    .await;

    // Search: provider=openai AND full-text search for "authentication" AND duration > 1000
    let search_request = json!({
        "filter": {
            "operator": "AND",
            "filters": [
                {
                    "field": "provider",
                    "operator": "eq",
                    "value": "openai"
                },
                {
                    "field": "input_text",
                    "operator": "search",
                    "value": "authentication"
                },
                {
                    "field": "duration_ms",
                    "operator": "gt",
                    "value": 1000
                }
            ]
        },
        "limit": 100
    });

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/traces/search")
        .header(header::CONTENT_TYPE, "application/json")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", create_mock_jwt()),
        )
        .body(Body::from(serde_json::to_string(&search_request).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let result: PaginatedTraceResponse = serde_json::from_slice(&body).unwrap();

    // Should match all three conditions
    for trace in &result.data {
        assert_eq!(trace.provider, "openai");
        assert!(trace.duration_ms.unwrap() > 1000);
        assert!(trace
            .input_text
            .as_ref()
            .unwrap()
            .to_lowercase()
            .contains("authentication"));
    }
}

// ============================================================================
// String Operator Tests
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_contains_operator() {
    let state = create_test_state().await;
    let app = create_test_router(state.clone());

    insert_test_trace(
        &state.db_pool,
        "openai",
        "gpt-4-turbo",
        "test",
        "output",
        1000,
        0.01,
    )
    .await;
    insert_test_trace(
        &state.db_pool,
        "openai",
        "gpt-3.5-turbo",
        "test",
        "output",
        1000,
        0.01,
    )
    .await;
    insert_test_trace(
        &state.db_pool,
        "anthropic",
        "claude-3-opus",
        "test",
        "output",
        1000,
        0.01,
    )
    .await;

    let search_request = json!({
        "filter": {
            "field": "model",
            "operator": "contains",
            "value": "turbo"
        },
        "limit": 100
    });

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/traces/search")
        .header(header::CONTENT_TYPE, "application/json")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", create_mock_jwt()),
        )
        .body(Body::from(serde_json::to_string(&search_request).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let result: PaginatedTraceResponse = serde_json::from_slice(&body).unwrap();

    // All results should contain "turbo"
    for trace in &result.data {
        assert!(trace.model.to_lowercase().contains("turbo"));
    }
}

#[tokio::test]
#[ignore]
async fn test_starts_with_operator() {
    let state = create_test_state().await;
    let app = create_test_router(state.clone());

    insert_test_trace(
        &state.db_pool,
        "openai",
        "gpt-4-turbo",
        "test",
        "output",
        1000,
        0.01,
    )
    .await;
    insert_test_trace(
        &state.db_pool,
        "openai",
        "gpt-3.5-turbo",
        "test",
        "output",
        1000,
        0.01,
    )
    .await;
    insert_test_trace(
        &state.db_pool,
        "anthropic",
        "claude-3-opus",
        "test",
        "output",
        1000,
        0.01,
    )
    .await;

    let search_request = json!({
        "filter": {
            "field": "model",
            "operator": "starts_with",
            "value": "gpt"
        },
        "limit": 100
    });

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/traces/search")
        .header(header::CONTENT_TYPE, "application/json")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", create_mock_jwt()),
        )
        .body(Body::from(serde_json::to_string(&search_request).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let result: PaginatedTraceResponse = serde_json::from_slice(&body).unwrap();

    // All results should start with "gpt"
    for trace in &result.data {
        assert!(trace.model.to_lowercase().starts_with("gpt"));
    }
}

// ============================================================================
// Error and Validation Tests
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_invalid_field_name_rejected() {
    let state = create_test_state().await;
    let app = create_test_router(state.clone());

    let search_request = json!({
        "filter": {
            "field": "DROP TABLE llm_traces; --",
            "operator": "eq",
            "value": "test"
        },
        "limit": 10
    });

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/traces/search")
        .header(header::CONTENT_TYPE, "application/json")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", create_mock_jwt()),
        )
        .body(Body::from(serde_json::to_string(&search_request).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should reject with BAD_REQUEST
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let error: ErrorResponse = serde_json::from_slice(&body).unwrap();
    assert!(error.error.message.contains("not allowed") || error.error.message.contains("invalid"));
}

#[tokio::test]
#[ignore]
async fn test_invalid_limit_rejected() {
    let state = create_test_state().await;
    let app = create_test_router(state.clone());

    let search_request = json!({
        "filter": {
            "field": "provider",
            "operator": "eq",
            "value": "openai"
        },
        "limit": 10000 // Exceeds maximum
    });

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/traces/search")
        .header(header::CONTENT_TYPE, "application/json")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", create_mock_jwt()),
        )
        .body(Body::from(serde_json::to_string(&search_request).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
#[ignore]
async fn test_invalid_sort_field_rejected() {
    let state = create_test_state().await;
    let app = create_test_router(state.clone());

    let search_request = json!({
        "filter": {
            "field": "provider",
            "operator": "eq",
            "value": "openai"
        },
        "sort_by": "invalid_field_name",
        "limit": 10
    });

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/traces/search")
        .header(header::CONTENT_TYPE, "application/json")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", create_mock_jwt()),
        )
        .body(Body::from(serde_json::to_string(&search_request).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// ============================================================================
// Pagination and Sorting Tests
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_field_selection() {
    let state = create_test_state().await;
    let app = create_test_router(state.clone());

    insert_test_trace(
        &state.db_pool,
        "openai",
        "gpt-4",
        "test input",
        "test output",
        1000,
        0.01,
    )
    .await;

    let search_request = json!({
        "filter": {
            "field": "provider",
            "operator": "eq",
            "value": "openai"
        },
        "fields": ["trace_id", "provider", "model"],
        "limit": 10
    });

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/traces/search")
        .header(header::CONTENT_TYPE, "application/json")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", create_mock_jwt()),
        )
        .body(Body::from(serde_json::to_string(&search_request).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let result: PaginatedTraceResponse = serde_json::from_slice(&body).unwrap();

    // Should return results (field selection is a hint, may return all fields)
    assert!(result.data.len() > 0);
}

#[tokio::test]
#[ignore]
async fn test_sorting() {
    let state = create_test_state().await;
    let app = create_test_router(state.clone());

    // Insert traces with different costs
    insert_test_trace(
        &state.db_pool,
        "openai",
        "gpt-4",
        "test",
        "output",
        1000,
        0.01,
    )
    .await;
    insert_test_trace(
        &state.db_pool,
        "openai",
        "gpt-4",
        "test",
        "output",
        1000,
        0.05,
    )
    .await;
    insert_test_trace(
        &state.db_pool,
        "openai",
        "gpt-4",
        "test",
        "output",
        1000,
        0.03,
    )
    .await;

    let search_request = json!({
        "filter": {
            "field": "provider",
            "operator": "eq",
            "value": "openai"
        },
        "sort_by": "total_cost_usd",
        "sort_desc": true,
        "limit": 100
    });

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/traces/search")
        .header(header::CONTENT_TYPE, "application/json")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", create_mock_jwt()),
        )
        .body(Body::from(serde_json::to_string(&search_request).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let result: PaginatedTraceResponse = serde_json::from_slice(&body).unwrap();

    // Verify descending order
    let costs: Vec<f64> = result
        .data
        .iter()
        .filter_map(|t| t.total_cost_usd)
        .collect();

    for i in 1..costs.len() {
        assert!(
            costs[i - 1] >= costs[i],
            "Results should be sorted in descending order by cost"
        );
    }
}

// ============================================================================
// Performance and Cache Tests
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_response_metadata() {
    let state = create_test_state().await;
    let app = create_test_router(state.clone());

    insert_test_trace(
        &state.db_pool,
        "openai",
        "gpt-4",
        "test",
        "output",
        1000,
        0.01,
    )
    .await;

    let search_request = json!({
        "filter": {
            "field": "provider",
            "operator": "eq",
            "value": "openai"
        },
        "limit": 10
    });

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/traces/search")
        .header(header::CONTENT_TYPE, "application/json")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", create_mock_jwt()),
        )
        .body(Body::from(serde_json::to_string(&search_request).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let result: PaginatedTraceResponse = serde_json::from_slice(&body).unwrap();

    // Verify metadata
    assert_eq!(result.status, ResponseStatus::Success);
    assert!(result.meta.execution_time_ms > 0);
    assert_eq!(result.meta.version, "1.0");
    assert!(result.meta.request_id.is_some());
}
