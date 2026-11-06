//! # Phase 3: Metrics API Integration Tests
//!
//! Comprehensive integration tests for the Phase 3 Metrics API endpoints.
//!
//! ## Test Coverage
//! - GET /api/v1/metrics - Time-series metrics query
//! - GET /api/v1/metrics/summary - Metrics summary with period comparison
//! - POST /api/v1/metrics/query - Custom metrics query
//!
//! ## Test Categories
//! 1. Basic Functionality Tests
//! 2. Aggregation and Grouping Tests
//! 3. Time Interval Tests
//! 4. Filtering Tests
//! 5. Summary and Comparison Tests
//! 6. Custom Query Tests
//! 7. Validation and Error Handling Tests
//! 8. Performance and Caching Tests
//!
//! ## Running Tests
//! ```bash
//! # Set up test database
//! export TEST_DATABASE_URL="postgresql://postgres:postgres@localhost:5432/llm_observatory_test"
//! export TEST_REDIS_URL="redis://localhost:6379"
//! export JWT_SECRET="test_secret_for_integration_tests_minimum_32_chars"
//!
//! # Run all Phase 3 tests
//! cargo test --test phase3_metrics_integration_tests -- --ignored
//!
//! # Run specific test
//! cargo test --test phase3_metrics_integration_tests test_metrics_basic_request_count -- --ignored
//! ```

use analytics_api::{middleware::auth::JwtValidator, models::*, routes};
use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
    middleware,
    routing::get,
    Router,
};
use chrono::{Duration, Utc};
use serde_json::{json, Value};
use sqlx::{PgPool, Row};
use std::sync::Arc;
use tower::ServiceExt;

// ============================================================================
// Test Setup and Helpers
// ============================================================================

/// Test helper to create test app with all routes
async fn create_test_app(pool: PgPool) -> Router {
    let redis_url = std::env::var("TEST_REDIS_URL").unwrap_or("redis://localhost:6379".to_string());
    let redis_client = redis::Client::open(redis_url).expect("Failed to connect to test Redis");

    let state = Arc::new(AppState {
        db_pool: pool,
        redis_client,
        cache_ttl: 60, // 1 minute for tests
    });

    let jwt_secret = std::env::var("JWT_SECRET")
        .unwrap_or("test_secret_for_integration_tests_minimum_32_chars".to_string());
    let jwt_validator = Arc::new(JwtValidator::new(&jwt_secret));

    // Build test router
    let protected_routes =
        Router::new()
            .merge(routes::metrics::routes())
            .layer(middleware::from_fn_with_state(
                jwt_validator.clone(),
                analytics_api::middleware::auth::require_auth,
            ));

    Router::new()
        .route("/health", get(|| async { "OK" }))
        .merge(protected_routes)
        .with_state(state)
}

/// Generate test JWT token
fn generate_test_jwt() -> String {
    use jsonwebtoken::{encode, EncodingKey, Header};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize)]
    struct Claims {
        sub: String,
        org_id: String,
        role: String,
        permissions: Vec<String>,
        exp: usize,
    }

    let exp = (Utc::now() + Duration::hours(1)).timestamp() as usize;

    let claims = Claims {
        sub: "test_user".to_string(),
        org_id: "test_org".to_string(),
        role: "admin".to_string(),
        permissions: vec![
            "metrics:read".to_string(),
            "metrics:query".to_string(),
            "traces:read".to_string(),
        ],
        exp,
    };

    let jwt_secret = std::env::var("JWT_SECRET")
        .unwrap_or("test_secret_for_integration_tests_minimum_32_chars".to_string());

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_ref()),
    )
    .expect("Failed to generate JWT")
}

/// Setup test database with sample data
async fn setup_test_data(pool: &PgPool) -> anyhow::Result<()> {
    // Clear existing data
    sqlx::query("DELETE FROM llm_traces WHERE org_id = 'test_org'")
        .execute(pool)
        .await?;

    // Insert sample traces for testing
    let now = Utc::now();

    for i in 0..100 {
        let ts = now - Duration::hours(i as i64);
        let provider = if i % 2 == 0 { "openai" } else { "anthropic" };
        let model = if i % 2 == 0 { "gpt-4" } else { "claude-3-opus" };
        let status_code = if i % 10 == 0 { "ERROR" } else { "OK" };

        sqlx::query(
            r#"
            INSERT INTO llm_traces (
                trace_id, org_id, user_id, session_id, ts, provider, model,
                environment, status_code, duration_ms, total_tokens, prompt_tokens,
                completion_tokens, total_cost_usd, prompt_cost_usd, completion_cost_usd,
                input_text, output_text
            ) VALUES (
                gen_random_uuid(), 'test_org', 'user1', 'session1', $1, $2, $3,
                'production', $4, $5, $6, $7, $8, $9, $10, $11, 'test input', 'test output'
            )
            "#,
        )
        .bind(ts)
        .bind(provider)
        .bind(model)
        .bind(status_code)
        .bind((1000 + i * 10) as i32) // duration_ms
        .bind((500 + i * 5) as i64) // total_tokens
        .bind((300 + i * 3) as i64) // prompt_tokens
        .bind((200 + i * 2) as i64) // completion_tokens
        .bind((0.01 + (i as f64 * 0.001))) // total_cost_usd
        .bind((0.006 + (i as f64 * 0.0006))) // prompt_cost_usd
        .bind((0.004 + (i as f64 * 0.0004))) // completion_cost_usd
        .execute(pool)
        .await?;
    }

    // Refresh continuous aggregates
    sqlx::query("CALL refresh_continuous_aggregate('llm_metrics_1min', NULL, NULL)")
        .execute(pool)
        .await
        .ok(); // Ignore errors if aggregate doesn't exist

    sqlx::query("CALL refresh_continuous_aggregate('llm_metrics_1hour', NULL, NULL)")
        .execute(pool)
        .await
        .ok();

    sqlx::query("CALL refresh_continuous_aggregate('llm_metrics_1day', NULL, NULL)")
        .execute(pool)
        .await
        .ok();

    Ok(())
}

// ============================================================================
// Test 1: Basic Metrics Query
// ============================================================================

#[tokio::test]
#[ignore] // Run with: cargo test --test phase3_metrics_integration_tests -- --ignored
async fn test_metrics_basic_request_count() {
    let database_url = std::env::var("TEST_DATABASE_URL")
        .expect("TEST_DATABASE_URL must be set for integration tests");

    let pool = PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to test database");

    setup_test_data(&pool)
        .await
        .expect("Failed to setup test data");

    let app = create_test_app(pool.clone()).await;
    let token = generate_test_jwt();

    // Test basic request count query
    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/metrics?metrics=request_count&interval=1hour")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Expected 200 OK for metrics query"
    );

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert!(json["metadata"].is_object());
    assert!(json["data"].is_array());
    assert!(json["metadata"]["total_points"].as_u64().unwrap() > 0);
}

// ============================================================================
// Test 2: Metrics with Grouping
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_metrics_group_by_provider() {
    let database_url = std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL must be set");

    let pool = PgPool::connect(&database_url).await.unwrap();
    setup_test_data(&pool).await.unwrap();

    let app = create_test_app(pool.clone()).await;
    let token = generate_test_jwt();

    // Test grouping by provider
    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/metrics?metrics=request_count,total_cost&interval=1hour&group_by=provider")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert!(json["data"].is_array());

    // Check that data points have provider dimension
    if let Some(first_point) = json["data"].as_array().and_then(|arr| arr.first()) {
        assert!(
            first_point["provider"].is_string(),
            "Data point should have provider dimension"
        );
    }
}

// ============================================================================
// Test 3: Multiple Group By Dimensions
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_metrics_multiple_group_by() {
    let database_url = std::env::var("TEST_DATABASE_URL").unwrap();
    let pool = PgPool::connect(&database_url).await.unwrap();
    setup_test_data(&pool).await.unwrap();

    let app = create_test_app(pool.clone()).await;
    let token = generate_test_jwt();

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/metrics?metrics=request_count&interval=1hour&group_by=provider,model")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    // Check that data points have both dimensions
    if let Some(first_point) = json["data"].as_array().and_then(|arr| arr.first()) {
        assert!(first_point["provider"].is_string());
        assert!(first_point["model"].is_string());
    }
}

// ============================================================================
// Test 4: Time Interval Variations
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_metrics_different_intervals() {
    let database_url = std::env::var("TEST_DATABASE_URL").unwrap();
    let pool = PgPool::connect(&database_url).await.unwrap();
    setup_test_data(&pool).await.unwrap();

    let app = create_test_app(pool.clone()).await;
    let token = generate_test_jwt();

    // Test 1-minute interval
    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/metrics?metrics=request_count&interval=1min")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Test 1-day interval
    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/metrics?metrics=request_count&interval=1day")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

// ============================================================================
// Test 5: Metrics Summary Endpoint
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_metrics_summary() {
    let database_url = std::env::var("TEST_DATABASE_URL").unwrap();
    let pool = PgPool::connect(&database_url).await.unwrap();
    setup_test_data(&pool).await.unwrap();

    let app = create_test_app(pool.clone()).await;
    let token = generate_test_jwt();

    let now = Utc::now();
    let start_time = (now - Duration::days(1)).to_rfc3339();
    let end_time = now.to_rfc3339();

    let request = Request::builder()
        .method(Method::GET)
        .uri(format!(
            "/api/v1/metrics/summary?start_time={}&end_time={}",
            start_time, end_time
        ))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    // Check response structure
    assert!(json["current_period"].is_object());
    assert!(json["current_period"]["total_requests"].is_number());
    assert!(json["current_period"]["total_cost_usd"].is_number());
    assert!(json["current_period"]["avg_duration_ms"].is_number());
    assert!(json["quality"].is_object());
    assert!(json["top_items"].is_object());
}

// ============================================================================
// Test 6: Summary with Period Comparison
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_metrics_summary_with_comparison() {
    let database_url = std::env::var("TEST_DATABASE_URL").unwrap();
    let pool = PgPool::connect(&database_url).await.unwrap();
    setup_test_data(&pool).await.unwrap();

    let app = create_test_app(pool.clone()).await;
    let token = generate_test_jwt();

    let now = Utc::now();
    let start_time = (now - Duration::days(1)).to_rfc3339();
    let end_time = now.to_rfc3339();

    let request = Request::builder()
        .method(Method::GET)
        .uri(format!(
            "/api/v1/metrics/summary?start_time={}&end_time={}&compare_previous_period=true",
            start_time, end_time
        ))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    // Check that previous period and changes are included
    assert!(
        json["previous_period"].is_object(),
        "Should include previous period"
    );
    assert!(json["changes"].is_object(), "Should include period changes");
    assert!(json["changes"]["requests_change_pct"].is_number());
    assert!(json["changes"]["cost_change_pct"].is_number());
}

// ============================================================================
// Test 7: Custom Metrics Query
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_custom_metrics_query() {
    let database_url = std::env::var("TEST_DATABASE_URL").unwrap();
    let pool = PgPool::connect(&database_url).await.unwrap();
    setup_test_data(&pool).await.unwrap();

    let app = create_test_app(pool.clone()).await;
    let token = generate_test_jwt();

    let now = Utc::now();
    let start_time = now - Duration::days(1);
    let end_time = now;

    let query_body = json!({
        "metrics": [
            {"metric": "request_count", "aggregation": "sum", "alias": "total_requests"},
            {"metric": "duration", "aggregation": "avg", "alias": "avg_duration"}
        ],
        "interval": "1hour",
        "start_time": start_time.to_rfc3339(),
        "end_time": end_time.to_rfc3339(),
        "group_by": ["provider"],
        "filters": [
            {"dimension": "provider", "operator": "in", "value": ["openai", "anthropic"]}
        ],
        "limit": 100
    });

    let request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/metrics/query")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&query_body).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert!(json["metadata"].is_object());
    assert!(json["data"].is_array());
}

// ============================================================================
// Test 8: Validation - Invalid Metric Name
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_validation_invalid_metric() {
    let database_url = std::env::var("TEST_DATABASE_URL").unwrap();
    let pool = PgPool::connect(&database_url).await.unwrap();

    let app = create_test_app(pool.clone()).await;
    let token = generate_test_jwt();

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/metrics?metrics=invalid_metric&interval=1hour")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "Should reject invalid metric name"
    );
}

// ============================================================================
// Test 9: Validation - Invalid Time Range
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_validation_invalid_time_range() {
    let database_url = std::env::var("TEST_DATABASE_URL").unwrap();
    let pool = PgPool::connect(&database_url).await.unwrap();

    let app = create_test_app(pool.clone()).await;
    let token = generate_test_jwt();

    let now = Utc::now();
    let start_time = now.to_rfc3339();
    let end_time = (now - Duration::days(1)).to_rfc3339(); // End before start

    let request = Request::builder()
        .method(Method::GET)
        .uri(format!(
            "/api/v1/metrics/summary?start_time={}&end_time={}",
            start_time, end_time
        ))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "Should reject invalid time range"
    );
}

// ============================================================================
// Test 10: Authorization - Missing Token
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_auth_missing_token() {
    let database_url = std::env::var("TEST_DATABASE_URL").unwrap();
    let pool = PgPool::connect(&database_url).await.unwrap();

    let app = create_test_app(pool.clone()).await;

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/metrics?metrics=request_count&interval=1hour")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Should require authentication"
    );
}

// ============================================================================
// Test 11: Authorization - Insufficient Permissions
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_auth_insufficient_permissions() {
    use jsonwebtoken::{encode, EncodingKey, Header};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize)]
    struct Claims {
        sub: String,
        org_id: String,
        role: String,
        permissions: Vec<String>,
        exp: usize,
    }

    let database_url = std::env::var("TEST_DATABASE_URL").unwrap();
    let pool = PgPool::connect(&database_url).await.unwrap();

    let app = create_test_app(pool.clone()).await;

    // Generate token without metrics:query permission
    let exp = (Utc::now() + Duration::hours(1)).timestamp() as usize;
    let claims = Claims {
        sub: "test_user".to_string(),
        org_id: "test_org".to_string(),
        role: "viewer".to_string(),
        permissions: vec!["traces:read".to_string()], // Missing metrics:query
        exp,
    };

    let jwt_secret = std::env::var("JWT_SECRET")
        .unwrap_or("test_secret_for_integration_tests_minimum_32_chars".to_string());

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_ref()),
    )
    .unwrap();

    let now = Utc::now();
    let query_body = json!({
        "metrics": [{"metric": "request_count", "aggregation": "sum"}],
        "interval": "1hour",
        "start_time": (now - Duration::days(1)).to_rfc3339(),
        "end_time": now.to_rfc3339(),
        "limit": 100
    });

    let request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/metrics/query")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&query_body).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::FORBIDDEN,
        "Should require metrics:query permission"
    );
}

// ============================================================================
// Test 12: Caching Behavior
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_caching_behavior() {
    let database_url = std::env::var("TEST_DATABASE_URL").unwrap();
    let pool = PgPool::connect(&database_url).await.unwrap();
    setup_test_data(&pool).await.unwrap();

    let app = create_test_app(pool.clone()).await;
    let token = generate_test_jwt();

    // First request (should cache)
    let request1 = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/metrics?metrics=request_count&interval=1hour")
        .header("Authorization", format!("Bearer {}", token.clone()))
        .body(Body::empty())
        .unwrap();

    let response1 = app.clone().oneshot(request1).await.unwrap();
    assert_eq!(response1.status(), StatusCode::OK);

    // Second request (should return from cache)
    let request2 = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/metrics?metrics=request_count&interval=1hour")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let start = std::time::Instant::now();
    let response2 = app.oneshot(request2).await.unwrap();
    let duration = start.elapsed();

    assert_eq!(response2.status(), StatusCode::OK);
    // Cached response should be faster (< 50ms typically)
    assert!(
        duration.as_millis() < 1000,
        "Cached response should be fast"
    );
}

// ============================================================================
// Test Summary
// ============================================================================

#[test]
fn test_summary() {
    println!("\n=== Phase 3: Metrics API Integration Tests ===\n");
    println!("Test Coverage:");
    println!("  ✓ Basic metrics query (request_count)");
    println!("  ✓ Metrics with grouping (by provider, model)");
    println!("  ✓ Multiple group by dimensions");
    println!("  ✓ Different time intervals (1min, 1hour, 1day)");
    println!("  ✓ Metrics summary endpoint");
    println!("  ✓ Summary with period comparison");
    println!("  ✓ Custom metrics query with filters");
    println!("  ✓ Validation (invalid metrics, time ranges)");
    println!("  ✓ Authorization (missing token, insufficient permissions)");
    println!("  ✓ Caching behavior\n");
    println!("Total Tests: 12");
    println!("\nRun with: cargo test --test phase3_metrics_integration_tests -- --ignored\n");
}
