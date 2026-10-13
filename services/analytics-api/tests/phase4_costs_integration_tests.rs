//! # Phase 4: Cost Analysis Integration Tests
//!
//! Comprehensive integration tests for the Phase 4 Cost Analysis endpoints.
//!
//! ## Test Coverage
//! - GET /api/v1/costs/summary - Comprehensive cost summary with trends and breakdowns
//! - GET /api/v1/costs/attribution - Multi-dimensional cost attribution
//! - GET /api/v1/costs/forecast - Linear regression-based cost forecasting
//!
//! ## Test Categories
//! 1. Cost Summary Tests
//! 2. Cost Attribution Tests
//! 3. Cost Forecasting Tests
//! 4. Filtering and Breakdown Tests
//! 5. Trend Analysis Tests
//! 6. Validation and Error Handling Tests
//! 7. Authorization Tests
//! 8. Caching Tests
//!
//! ## Running Tests
//! ```bash
//! # Set up test environment
//! export TEST_DATABASE_URL="postgresql://postgres:postgres@localhost:5432/llm_observatory_test"
//! export TEST_REDIS_URL="redis://localhost:6379"
//! export JWT_SECRET="test_secret_for_integration_tests_minimum_32_chars"
//!
//! # Run all Phase 4 tests
//! cargo test --test phase4_costs_integration_tests -- --ignored
//!
//! # Run specific test
//! cargo test --test phase4_costs_integration_tests test_cost_summary_basic -- --ignored
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
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;

// ============================================================================
// Test Setup and Helpers
// ============================================================================

/// Test helper to create test app with cost routes
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

    // Build test router with protected cost routes
    let protected_routes =
        Router::new()
            .merge(routes::costs::routes())
            .layer(middleware::from_fn_with_state(
                jwt_validator.clone(),
                analytics_api::middleware::auth::require_auth,
            ));

    Router::new()
        .route("/health", get(|| async { "OK" }))
        .merge(protected_routes)
        .with_state(state)
}

/// Generate test JWT token with costs:read permission
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
        org_id: "test_org_costs".to_string(),
        role: "admin".to_string(),
        permissions: vec!["costs:read".to_string(), "traces:read".to_string()],
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

/// Setup test database with comprehensive cost data
async fn setup_test_data(pool: &PgPool) -> anyhow::Result<()> {
    // Clear existing data
    sqlx::query("DELETE FROM llm_traces WHERE org_id = 'test_org_costs'")
        .execute(pool)
        .await?;

    let now = Utc::now();

    // Insert 60 days of historical data for forecasting
    for day in 0..60 {
        for hour in 0..4 {
            // 4 samples per day
            let ts = now - Duration::days(day as i64) - Duration::hours(hour * 6);

            // Vary providers and models
            let (provider, model) = match (day + hour) % 4 {
                0 => ("openai", "gpt-4"),
                1 => ("openai", "gpt-3.5-turbo"),
                2 => ("anthropic", "claude-3-opus"),
                _ => ("anthropic", "claude-3-sonnet"),
            };

            let environment = if day % 3 == 0 {
                "production"
            } else {
                "staging"
            };
            let user_id = format!("user_{}", (day % 5) + 1); // 5 different users
            let team_id = format!("team_{}", (day % 3) + 1); // 3 different teams

            // Create increasing cost trend for linear regression
            let base_cost = 10.0 + (60 - day) as f64 * 0.5; // Increasing trend
            let tokens = (1000 + day * 50) as i64;

            let status_code = if (day + hour) % 20 == 0 {
                "ERROR"
            } else {
                "OK"
            };

            sqlx::query(
                r#"
                INSERT INTO llm_traces (
                    trace_id, org_id, user_id, team_id, session_id, ts, provider, model,
                    environment, status_code, duration_ms, total_tokens, prompt_tokens,
                    completion_tokens, total_cost_usd, prompt_cost_usd, completion_cost_usd,
                    input_text, output_text, tags
                ) VALUES (
                    gen_random_uuid(), 'test_org_costs', $1, $2, 'session1', $3, $4, $5,
                    $6, $7, $8, $9, $10, $11, $12, $13, $14, 'test input', 'test output', $15
                )
                "#,
            )
            .bind(&user_id)
            .bind(&team_id)
            .bind(ts)
            .bind(provider)
            .bind(model)
            .bind(environment)
            .bind(status_code)
            .bind((1000 + day * 10) as i32) // duration_ms
            .bind(tokens) // total_tokens
            .bind((tokens as f64 * 0.6) as i64) // prompt_tokens
            .bind((tokens as f64 * 0.4) as i64) // completion_tokens
            .bind(base_cost) // total_cost_usd
            .bind(base_cost * 0.6) // prompt_cost_usd
            .bind(base_cost * 0.4) // completion_cost_usd
            .bind(vec!["tag1".to_string(), "tag2".to_string()]) // tags
            .execute(pool)
            .await?;
        }
    }

    // Refresh continuous aggregates
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
// Test 1: Basic Cost Summary
// ============================================================================

#[tokio::test]
#[ignore] // Run with: cargo test --test phase4_costs_integration_tests -- --ignored
async fn test_cost_summary_basic() {
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

    let now = Utc::now();
    let start_time = (now - Duration::days(30)).to_rfc3339();
    let end_time = now.to_rfc3339();

    let request = Request::builder()
        .method(Method::GET)
        .uri(format!(
            "/api/v1/costs/summary?start_time={}&end_time={}",
            start_time, end_time
        ))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Expected 200 OK for cost summary"
    );

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    // Verify response structure
    assert!(json["metadata"].is_object(), "Should have metadata");
    assert!(json["overview"].is_object(), "Should have overview");
    assert!(
        json["by_provider"].is_array(),
        "Should have provider breakdown"
    );
    assert!(json["by_model"].is_array(), "Should have model breakdown");
    assert!(
        json["by_environment"].is_array(),
        "Should have environment breakdown"
    );

    // Verify overview metrics
    assert!(json["overview"]["total_cost"].as_f64().unwrap() > 0.0);
    assert!(json["overview"]["total_requests"].as_u64().unwrap() > 0);
    assert!(json["overview"]["avg_cost_per_request"].as_f64().unwrap() > 0.0);
}

// ============================================================================
// Test 2: Cost Summary with Trends
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_cost_summary_with_trends() {
    let database_url = std::env::var("TEST_DATABASE_URL").unwrap();
    let pool = PgPool::connect(&database_url).await.unwrap();
    setup_test_data(&pool).await.unwrap();

    let app = create_test_app(pool.clone()).await;
    let token = generate_test_jwt();

    let now = Utc::now();
    let start_time = (now - Duration::days(14)).to_rfc3339();
    let end_time = now.to_rfc3339();

    let request = Request::builder()
        .method(Method::GET)
        .uri(format!(
            "/api/v1/costs/summary?start_time={}&end_time={}&include_trends=true",
            start_time, end_time
        ))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    // Verify trends are included
    assert!(json["trends"].is_object(), "Should include trends");
    assert!(
        json["trends"]["daily"].is_array(),
        "Should have daily trends"
    );
    assert!(
        json["trends"]["weekly"].is_array(),
        "Should have weekly trends"
    );
    assert!(
        json["trends"]["growth_rate_daily"].is_number(),
        "Should have daily growth rate"
    );
    assert!(
        json["trends"]["growth_rate_weekly"].is_number(),
        "Should have weekly growth rate"
    );
}

// ============================================================================
// Test 3: Cost Summary with Top Traces
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_cost_summary_with_top_traces() {
    let database_url = std::env::var("TEST_DATABASE_URL").unwrap();
    let pool = PgPool::connect(&database_url).await.unwrap();
    setup_test_data(&pool).await.unwrap();

    let app = create_test_app(pool.clone()).await;
    let token = generate_test_jwt();

    let now = Utc::now();

    let request = Request::builder()
        .method(Method::GET)
        .uri(format!(
            "/api/v1/costs/summary?start_time={}&end_time={}&include_top_traces=true&top_limit=5",
            (now - Duration::days(7)).to_rfc3339(),
            now.to_rfc3339()
        ))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    // Verify top traces are included
    assert!(json["top_traces"].is_array(), "Should include top traces");

    let traces = json["top_traces"].as_array().unwrap();
    if !traces.is_empty() {
        let first_trace = &traces[0];
        assert!(first_trace["trace_id"].is_string());
        assert!(first_trace["cost"].is_number());
        assert!(first_trace["provider"].is_string());
        assert!(first_trace["model"].is_string());
    }
}

// ============================================================================
// Test 4: Cost Summary with Filters
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_cost_summary_with_filters() {
    let database_url = std::env::var("TEST_DATABASE_URL").unwrap();
    let pool = PgPool::connect(&database_url).await.unwrap();
    setup_test_data(&pool).await.unwrap();

    let app = create_test_app(pool.clone()).await;
    let token = generate_test_jwt();

    let now = Utc::now();

    let request = Request::builder()
        .method(Method::GET)
        .uri(format!(
            "/api/v1/costs/summary?start_time={}&end_time={}&provider=openai&environment=production",
            (now - Duration::days(30)).to_rfc3339(),
            now.to_rfc3339()
        ))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    // Verify filtering worked (all providers should be openai or filtered out)
    if let Some(providers) = json["by_provider"].as_array() {
        for provider in providers {
            assert_eq!(
                provider["name"].as_str().unwrap(),
                "openai",
                "Should only include openai provider"
            );
        }
    }
}

// ============================================================================
// Test 5: Cost Attribution by User
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_cost_attribution_by_user() {
    let database_url = std::env::var("TEST_DATABASE_URL").unwrap();
    let pool = PgPool::connect(&database_url).await.unwrap();
    setup_test_data(&pool).await.unwrap();

    let app = create_test_app(pool.clone()).await;
    let token = generate_test_jwt();

    let now = Utc::now();

    let request = Request::builder()
        .method(Method::GET)
        .uri(format!(
            "/api/v1/costs/attribution?start_time={}&end_time={}&dimension=user&limit=10",
            (now - Duration::days(30)).to_rfc3339(),
            now.to_rfc3339()
        ))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    // Verify response structure
    assert!(json["metadata"].is_object());
    assert_eq!(json["metadata"]["dimension"].as_str().unwrap(), "User");
    assert!(json["items"].is_array());
    assert!(json["summary"].is_object());

    // Verify attribution items
    if let Some(items) = json["items"].as_array() {
        if !items.is_empty() {
            let first_item = &items[0];
            assert!(first_item["dimension_value"].is_string());
            assert!(first_item["total_cost"].as_f64().unwrap() > 0.0);
            assert!(first_item["request_count"].as_u64().unwrap() > 0);
            assert!(first_item["cost_percentage"].as_f64().unwrap() > 0.0);
            assert!(first_item["by_provider"].is_object());
            assert!(first_item["by_model"].is_object());
        }
    }

    // Verify summary
    assert!(json["summary"]["total_cost"].as_f64().unwrap() > 0.0);
    assert!(json["summary"]["total_requests"].as_u64().unwrap() > 0);
}

// ============================================================================
// Test 6: Cost Attribution by Team
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_cost_attribution_by_team() {
    let database_url = std::env::var("TEST_DATABASE_URL").unwrap();
    let pool = PgPool::connect(&database_url).await.unwrap();
    setup_test_data(&pool).await.unwrap();

    let app = create_test_app(pool.clone()).await;
    let token = generate_test_jwt();

    let now = Utc::now();

    let request = Request::builder()
        .method(Method::GET)
        .uri(format!(
            "/api/v1/costs/attribution?start_time={}&end_time={}&dimension=team&limit=50",
            (now - Duration::days(30)).to_rfc3339(),
            now.to_rfc3339()
        ))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["metadata"]["dimension"].as_str().unwrap(), "Team");
    assert!(json["items"].is_array());
}

// ============================================================================
// Test 7: Cost Attribution by Provider
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_cost_attribution_by_provider() {
    let database_url = std::env::var("TEST_DATABASE_URL").unwrap();
    let pool = PgPool::connect(&database_url).await.unwrap();
    setup_test_data(&pool).await.unwrap();

    let app = create_test_app(pool.clone()).await;
    let token = generate_test_jwt();

    let now = Utc::now();

    let request = Request::builder()
        .method(Method::GET)
        .uri(format!(
            "/api/v1/costs/attribution?start_time={}&end_time={}&dimension=provider",
            (now - Duration::days(30)).to_rfc3339(),
            now.to_rfc3339()
        ))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["metadata"]["dimension"].as_str().unwrap(), "Provider");

    // Should have both openai and anthropic
    let items = json["items"].as_array().unwrap();
    assert!(items.len() >= 2, "Should have at least 2 providers");
}

// ============================================================================
// Test 8: Cost Attribution with Minimum Cost Filter
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_cost_attribution_min_cost_filter() {
    let database_url = std::env::var("TEST_DATABASE_URL").unwrap();
    let pool = PgPool::connect(&database_url).await.unwrap();
    setup_test_data(&pool).await.unwrap();

    let app = create_test_app(pool.clone()).await;
    let token = generate_test_jwt();

    let now = Utc::now();

    let request = Request::builder()
        .method(Method::GET)
        .uri(format!(
            "/api/v1/costs/attribution?start_time={}&end_time={}&dimension=user&min_cost=100.0",
            (now - Duration::days(30)).to_rfc3339(),
            now.to_rfc3339()
        ))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    // Verify all items meet minimum cost
    if let Some(items) = json["items"].as_array() {
        for item in items {
            let cost = item["total_cost"].as_f64().unwrap();
            assert!(
                cost >= 100.0,
                "All items should have cost >= 100.0, found: {}",
                cost
            );
        }
    }
}

// ============================================================================
// Test 9: Cost Forecast - Next Month
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_cost_forecast_next_month() {
    let database_url = std::env::var("TEST_DATABASE_URL").unwrap();
    let pool = PgPool::connect(&database_url).await.unwrap();
    setup_test_data(&pool).await.unwrap();

    let app = create_test_app(pool.clone()).await;
    let token = generate_test_jwt();

    let now = Utc::now();

    let request = Request::builder()
        .method(Method::GET)
        .uri(format!(
            "/api/v1/costs/forecast?historical_start={}&historical_end={}&forecast_period=next_month",
            (now - Duration::days(30)).to_rfc3339(),
            now.to_rfc3339()
        ))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    // Verify response structure
    assert!(json["metadata"].is_object());
    assert_eq!(
        json["metadata"]["model_type"].as_str().unwrap(),
        "linear_regression"
    );
    assert_eq!(json["metadata"]["forecast_days"].as_u64().unwrap(), 30);

    assert!(json["historical"].is_array());
    assert!(json["forecast"].is_array());
    assert!(json["summary"].is_object());

    // Verify forecast summary
    assert!(json["summary"]["total_forecasted_cost"].as_f64().unwrap() > 0.0);
    assert!(json["summary"]["avg_daily_cost"].as_f64().unwrap() > 0.0);
    assert!(json["summary"]["r_squared"].as_f64().is_some());
    assert!(json["summary"]["mape"].as_f64().is_some());
}

// ============================================================================
// Test 10: Cost Forecast with Confidence Intervals
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_cost_forecast_with_confidence_intervals() {
    let database_url = std::env::var("TEST_DATABASE_URL").unwrap();
    let pool = PgPool::connect(&database_url).await.unwrap();
    setup_test_data(&pool).await.unwrap();

    let app = create_test_app(pool.clone()).await;
    let token = generate_test_jwt();

    let now = Utc::now();

    let request = Request::builder()
        .method(Method::GET)
        .uri(format!(
            "/api/v1/costs/forecast?historical_start={}&historical_end={}&forecast_period=next_week&include_confidence_intervals=true",
            (now - Duration::days(30)).to_rfc3339(),
            now.to_rfc3339()
        ))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    // Verify confidence intervals are included
    if let Some(forecast) = json["forecast"].as_array() {
        if !forecast.is_empty() {
            let first_point = &forecast[0];
            assert!(first_point["forecasted_cost"].is_number());
            assert!(
                first_point["lower_bound"].is_number(),
                "Should include lower confidence bound"
            );
            assert!(
                first_point["upper_bound"].is_number(),
                "Should include upper confidence bound"
            );
        }
    }
}

// ============================================================================
// Test 11: Cost Forecast - Next Quarter
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_cost_forecast_next_quarter() {
    let database_url = std::env::var("TEST_DATABASE_URL").unwrap();
    let pool = PgPool::connect(&database_url).await.unwrap();
    setup_test_data(&pool).await.unwrap();

    let app = create_test_app(pool.clone()).await;
    let token = generate_test_jwt();

    let now = Utc::now();

    let request = Request::builder()
        .method(Method::GET)
        .uri(format!(
            "/api/v1/costs/forecast?historical_start={}&historical_end={}&forecast_period=next_quarter",
            (now - Duration::days(60)).to_rfc3339(),
            now.to_rfc3339()
        ))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["metadata"]["forecast_days"].as_u64().unwrap(), 90);

    // Verify we have 90 forecast points
    let forecast = json["forecast"].as_array().unwrap();
    assert_eq!(forecast.len(), 90, "Should have 90 forecast data points");
}

// ============================================================================
// Test 12: Validation - Time Range Too Large
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_validation_time_range_too_large() {
    let database_url = std::env::var("TEST_DATABASE_URL").unwrap();
    let pool = PgPool::connect(&database_url).await.unwrap();

    let app = create_test_app(pool.clone()).await;
    let token = generate_test_jwt();

    let now = Utc::now();

    // Try to query more than 365 days
    let request = Request::builder()
        .method(Method::GET)
        .uri(format!(
            "/api/v1/costs/summary?start_time={}&end_time={}",
            (now - Duration::days(400)).to_rfc3339(),
            now.to_rfc3339()
        ))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "Should reject time range > 365 days"
    );
}

// ============================================================================
// Test 13: Validation - Invalid Attribution Dimension
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_validation_invalid_attribution_dimension() {
    let database_url = std::env::var("TEST_DATABASE_URL").unwrap();
    let pool = PgPool::connect(&database_url).await.unwrap();

    let app = create_test_app(pool.clone()).await;
    let token = generate_test_jwt();

    let now = Utc::now();

    let request = Request::builder()
        .method(Method::GET)
        .uri(format!(
            "/api/v1/costs/attribution?start_time={}&end_time={}&dimension=invalid_dimension",
            (now - Duration::days(30)).to_rfc3339(),
            now.to_rfc3339()
        ))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "Should reject invalid attribution dimension"
    );
}

// ============================================================================
// Test 14: Validation - Insufficient Historical Data for Forecast
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_validation_insufficient_historical_data() {
    let database_url = std::env::var("TEST_DATABASE_URL").unwrap();
    let pool = PgPool::connect(&database_url).await.unwrap();

    let app = create_test_app(pool.clone()).await;
    let token = generate_test_jwt();

    let now = Utc::now();

    // Try to forecast with only 3 days of history (minimum is 7)
    let request = Request::builder()
        .method(Method::GET)
        .uri(format!(
            "/api/v1/costs/forecast?historical_start={}&historical_end={}&forecast_period=next_month",
            (now - Duration::days(3)).to_rfc3339(),
            now.to_rfc3339()
        ))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "Should reject insufficient historical data"
    );
}

// ============================================================================
// Test 15: Validation - Attribution Limit Exceeded
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_validation_attribution_limit_exceeded() {
    let database_url = std::env::var("TEST_DATABASE_URL").unwrap();
    let pool = PgPool::connect(&database_url).await.unwrap();

    let app = create_test_app(pool.clone()).await;
    let token = generate_test_jwt();

    let now = Utc::now();

    // Try to request more than max limit (1000)
    let request = Request::builder()
        .method(Method::GET)
        .uri(format!(
            "/api/v1/costs/attribution?start_time={}&end_time={}&dimension=user&limit=2000",
            (now - Duration::days(30)).to_rfc3339(),
            now.to_rfc3339()
        ))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "Should reject limit > 1000"
    );
}

// ============================================================================
// Test 16: Authorization - Missing Token
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_auth_missing_token() {
    let database_url = std::env::var("TEST_DATABASE_URL").unwrap();
    let pool = PgPool::connect(&database_url).await.unwrap();

    let app = create_test_app(pool.clone()).await;

    let now = Utc::now();

    let request = Request::builder()
        .method(Method::GET)
        .uri(format!(
            "/api/v1/costs/summary?start_time={}&end_time={}",
            (now - Duration::days(30)).to_rfc3339(),
            now.to_rfc3339()
        ))
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
// Test 17: Authorization - Insufficient Permissions
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

    // Generate token without costs:read permission
    let exp = (Utc::now() + Duration::hours(1)).timestamp() as usize;
    let claims = Claims {
        sub: "test_user".to_string(),
        org_id: "test_org_costs".to_string(),
        role: "viewer".to_string(),
        permissions: vec!["traces:read".to_string()], // Missing costs:read
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

    let request = Request::builder()
        .method(Method::GET)
        .uri(format!(
            "/api/v1/costs/summary?start_time={}&end_time={}",
            (now - Duration::days(30)).to_rfc3339(),
            now.to_rfc3339()
        ))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::FORBIDDEN,
        "Should require costs:read permission"
    );
}

// ============================================================================
// Test 18: Caching Behavior - Summary
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_caching_cost_summary() {
    let database_url = std::env::var("TEST_DATABASE_URL").unwrap();
    let pool = PgPool::connect(&database_url).await.unwrap();
    setup_test_data(&pool).await.unwrap();

    let app = create_test_app(pool.clone()).await;
    let token = generate_test_jwt();

    let now = Utc::now();
    let uri = format!(
        "/api/v1/costs/summary?start_time={}&end_time={}",
        (now - Duration::days(30)).to_rfc3339(),
        now.to_rfc3339()
    );

    // First request (should cache)
    let request1 = Request::builder()
        .method(Method::GET)
        .uri(&uri)
        .header("Authorization", format!("Bearer {}", token.clone()))
        .body(Body::empty())
        .unwrap();

    let response1 = app.clone().oneshot(request1).await.unwrap();
    assert_eq!(response1.status(), StatusCode::OK);

    // Second request (should return from cache)
    let request2 = Request::builder()
        .method(Method::GET)
        .uri(&uri)
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let start = std::time::Instant::now();
    let response2 = app.oneshot(request2).await.unwrap();
    let duration = start.elapsed();

    assert_eq!(response2.status(), StatusCode::OK);
    // Cached response should be very fast
    assert!(
        duration.as_millis() < 500,
        "Cached response should be fast, took: {}ms",
        duration.as_millis()
    );
}

// ============================================================================
// Test 19: Caching Behavior - Forecast
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_caching_cost_forecast() {
    let database_url = std::env::var("TEST_DATABASE_URL").unwrap();
    let pool = PgPool::connect(&database_url).await.unwrap();
    setup_test_data(&pool).await.unwrap();

    let app = create_test_app(pool.clone()).await;
    let token = generate_test_jwt();

    let now = Utc::now();
    let uri = format!(
        "/api/v1/costs/forecast?historical_start={}&historical_end={}&forecast_period=next_week",
        (now - Duration::days(30)).to_rfc3339(),
        now.to_rfc3339()
    );

    // First request
    let request1 = Request::builder()
        .method(Method::GET)
        .uri(&uri)
        .header("Authorization", format!("Bearer {}", token.clone()))
        .body(Body::empty())
        .unwrap();

    let response1 = app.clone().oneshot(request1).await.unwrap();
    assert_eq!(response1.status(), StatusCode::OK);

    // Second request (cached)
    let request2 = Request::builder()
        .method(Method::GET)
        .uri(&uri)
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let start = std::time::Instant::now();
    let response2 = app.oneshot(request2).await.unwrap();
    let duration = start.elapsed();

    assert_eq!(response2.status(), StatusCode::OK);
    assert!(duration.as_millis() < 500, "Cached response should be fast");
}

// ============================================================================
// Test 20: Organization Isolation
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_organization_isolation() {
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
    setup_test_data(&pool).await.unwrap();

    let app = create_test_app(pool.clone()).await;

    // Generate token for different organization
    let exp = (Utc::now() + Duration::hours(1)).timestamp() as usize;
    let claims = Claims {
        sub: "test_user".to_string(),
        org_id: "different_org".to_string(), // Different org
        role: "admin".to_string(),
        permissions: vec!["costs:read".to_string()],
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

    let request = Request::builder()
        .method(Method::GET)
        .uri(format!(
            "/api/v1/costs/summary?start_time={}&end_time={}",
            (now - Duration::days(30)).to_rfc3339(),
            now.to_rfc3339()
        ))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    // Should return 0 costs (no data for different_org)
    assert_eq!(
        json["overview"]["total_cost"].as_f64().unwrap(),
        0.0,
        "Should not see costs from other organizations"
    );
    assert_eq!(
        json["overview"]["total_requests"].as_u64().unwrap(),
        0,
        "Should not see requests from other organizations"
    );
}

// ============================================================================
// Test Summary
// ============================================================================

#[test]
fn test_summary() {
    println!("\n=== Phase 4: Cost Analysis Integration Tests ===\n");
    println!("Test Coverage:");
    println!("  ✓ Cost Summary - Basic (with overview, breakdowns)");
    println!("  ✓ Cost Summary - With trends (daily, weekly growth)");
    println!("  ✓ Cost Summary - With top expensive traces");
    println!("  ✓ Cost Summary - With filters (provider, model, environment)");
    println!("  ✓ Cost Attribution - By user");
    println!("  ✓ Cost Attribution - By team");
    println!("  ✓ Cost Attribution - By provider");
    println!("  ✓ Cost Attribution - With minimum cost filter");
    println!("  ✓ Cost Forecast - Next month (linear regression)");
    println!("  ✓ Cost Forecast - With confidence intervals");
    println!("  ✓ Cost Forecast - Next quarter (90 days)");
    println!("  ✓ Validation - Time range limits");
    println!("  ✓ Validation - Invalid attribution dimension");
    println!("  ✓ Validation - Insufficient historical data");
    println!("  ✓ Validation - Attribution limit exceeded");
    println!("  ✓ Authorization - Missing token");
    println!("  ✓ Authorization - Insufficient permissions");
    println!("  ✓ Caching - Cost summary");
    println!("  ✓ Caching - Cost forecast");
    println!("  ✓ Organization isolation\n");
    println!("Total Tests: 20");
    println!("\nRun with: cargo test --test phase4_costs_integration_tests -- --ignored\n");
}
