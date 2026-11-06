//! # Cost Analysis API Routes (Phase 4)
//!
//! Comprehensive cost analysis endpoints with advanced features.
//!
//! ## Endpoints
//! - `GET /api/v1/costs/summary` - Comprehensive cost summary with trends and breakdowns
//! - `GET /api/v1/costs/attribution` - Cost attribution by user, team, tag
//! - `GET /api/v1/costs/forecast` - Cost forecasting with linear regression
//!
//! ## Features
//! - Detailed cost breakdowns by provider, model, environment
//! - Trend analysis (daily, weekly growth rates)
//! - Top expensive traces identification
//! - Linear regression forecasting
//! - Cost attribution across multiple dimensions
//! - Redis caching for all endpoints
//!
//! ## Security
//! - JWT authentication required
//! - RBAC permission checking
//! - Organization-level data isolation

use crate::middleware::AuthContext;
use crate::models::costs::*;
use crate::models::{AppState, ErrorResponse};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Duration, Utc};
use redis::AsyncCommands;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{error, info, instrument, warn};

// ============================================================================
// Router Configuration
// ============================================================================

/// Create cost analysis routes
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v1/costs/summary", get(get_cost_summary))
        .route("/api/v1/costs/attribution", get(get_cost_attribution))
        .route("/api/v1/costs/forecast", get(get_cost_forecast))
}

// ============================================================================
// API Error Type
// ============================================================================

#[derive(Debug)]
pub enum ApiError {
    BadRequest(String),
    Unauthorized(String),
    Forbidden(String),
    NotFound(String),
    Internal(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_type, message) = match self {
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, "bad_request", msg),
            ApiError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, "unauthorized", msg),
            ApiError::Forbidden(msg) => (StatusCode::FORBIDDEN, "forbidden", msg),
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, "not_found", msg),
            ApiError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, "internal_error", msg),
        };

        let body = Json(ErrorResponse {
            error: error_type.to_string(),
            message,
            details: None,
        });

        (status, body).into_response()
    }
}

// ============================================================================
// Endpoint 1: GET /api/v1/costs/summary
// ============================================================================

/// GET /api/v1/costs/summary - Comprehensive cost summary
///
/// Returns detailed cost analysis including:
/// - Overall cost statistics
/// - Breakdowns by provider, model, environment
/// - Trend analysis (daily, weekly)
/// - Top expensive traces
///
/// ## Query Parameters
/// - `start_time`: Start of time range (ISO 8601) - default: 30 days ago
/// - `end_time`: End of time range (ISO 8601) - default: now
/// - `provider`: Filter by provider
/// - `model`: Filter by model
/// - `environment`: Filter by environment
/// - `user_id`: Filter by user ID
/// - `include_trends`: Include trend analysis - default: true
/// - `include_top_traces`: Include top expensive traces - default: true
/// - `top_limit`: Number of top traces to return (max 100) - default: 10
///
/// ## Example
/// ```bash
/// curl -X GET 'http://localhost:8080/api/v1/costs/summary?start_time=2025-10-01T00:00:00Z&end_time=2025-11-01T00:00:00Z&include_trends=true' \
///   -H "Authorization: Bearer $JWT_TOKEN"
/// ```
#[instrument(skip(state, auth))]
async fn get_cost_summary(
    State(state): State<Arc<AppState>>,
    auth: AuthContext,
    Query(request): Query<CostSummaryRequest>,
) -> Result<Json<CostSummaryResponse>, ApiError> {
    // Check permissions
    if !auth.has_permission("costs:read") {
        return Err(ApiError::Forbidden(
            "Insufficient permissions to read cost data".to_string(),
        ));
    }

    // Validate request
    request.validate().map_err(ApiError::BadRequest)?;

    info!(
        org_id = %auth.organization_id,
        include_trends = request.include_trends,
        include_top_traces = request.include_top_traces,
        "Querying cost summary"
    );

    // Set defaults
    let end_time = request.end_time.unwrap_or_else(Utc::now);
    let start_time = request
        .start_time
        .unwrap_or_else(|| end_time - Duration::days(30));

    // Generate cache key
    let cache_key =
        generate_summary_cache_key(&request, &auth.organization_id, start_time, end_time);

    // Try cache
    if let Ok(cached) = try_get_from_cache(&state, &cache_key).await {
        info!("Returning cached cost summary");
        return Ok(Json(cached));
    }

    // Execute query
    let response = execute_cost_summary(
        &state.db_pool,
        &request,
        &auth.organization_id,
        start_time,
        end_time,
    )
    .await?;

    // Cache result
    cache_cost_summary(&state, &cache_key, &response).await;

    info!(
        total_cost = response.overview.total_cost,
        "Cost summary completed"
    );

    Ok(Json(response))
}

/// Execute cost summary query
async fn execute_cost_summary(
    pool: &PgPool,
    request: &CostSummaryRequest,
    org_id: &str,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
) -> Result<CostSummaryResponse, ApiError> {
    // Query overview
    let overview = query_cost_overview(pool, org_id, start_time, end_time, request).await?;

    // Query breakdowns
    let by_provider =
        query_cost_breakdown(pool, org_id, start_time, end_time, "provider", request).await?;
    let by_model =
        query_cost_breakdown(pool, org_id, start_time, end_time, "model", request).await?;
    let by_environment =
        query_cost_breakdown(pool, org_id, start_time, end_time, "environment", request).await?;

    // Query trends if requested
    let trends = if request.include_trends {
        Some(query_cost_trends(pool, org_id, start_time, end_time, request).await?)
    } else {
        None
    };

    // Query top traces if requested
    let top_traces = if request.include_top_traces {
        Some(query_top_expensive_traces(pool, org_id, start_time, end_time, request).await?)
    } else {
        None
    };

    // Build metadata
    let period_days = (end_time - start_time).num_days();

    let metadata = CostSummaryMetadata {
        start_time,
        end_time,
        period_days,
        generated_at: Utc::now(),
    };

    Ok(CostSummaryResponse {
        metadata,
        overview,
        by_provider,
        by_model,
        by_environment,
        trends,
        top_traces,
    })
}

/// Query cost overview
async fn query_cost_overview(
    pool: &PgPool,
    org_id: &str,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
    request: &CostSummaryRequest,
) -> Result<CostOverview, ApiError> {
    let mut where_clauses = vec![
        "org_id = $1".to_string(),
        "ts >= $2".to_string(),
        "ts < $3".to_string(),
    ];
    let mut param_index = 4;

    if request.provider.is_some() {
        where_clauses.push(format!("provider = ${}", param_index));
        param_index += 1;
    }
    if request.model.is_some() {
        where_clauses.push(format!("model = ${}", param_index));
        param_index += 1;
    }
    if request.environment.is_some() {
        where_clauses.push(format!("environment = ${}", param_index));
        param_index += 1;
    }
    if request.user_id.is_some() {
        where_clauses.push(format!("user_id = ${}", param_index));
    }

    let query_str = format!(
        r#"
        SELECT
            SUM(total_cost_usd) AS total_cost_usd,
            SUM(prompt_cost_usd) AS prompt_cost_usd,
            SUM(completion_cost_usd) AS completion_cost_usd,
            COUNT(*) AS total_requests,
            SUM(total_tokens) AS total_tokens
        FROM llm_traces
        WHERE {}
        "#,
        where_clauses.join(" AND ")
    );

    let mut query = sqlx::query_as::<_, CostOverviewRow>(&query_str)
        .bind(org_id)
        .bind(start_time)
        .bind(end_time);

    if let Some(ref provider) = request.provider {
        query = query.bind(provider);
    }
    if let Some(ref model) = request.model {
        query = query.bind(model);
    }
    if let Some(ref environment) = request.environment {
        query = query.bind(environment);
    }
    if let Some(ref user_id) = request.user_id {
        query = query.bind(user_id);
    }

    let row = query.fetch_one(pool).await.map_err(|e| {
        error!(error = %e, "Failed to query cost overview");
        ApiError::Internal(format!("Database query failed: {}", e))
    })?;

    let total_cost = row.total_cost_usd.unwrap_or(0.0);
    let prompt_cost = row.prompt_cost_usd.unwrap_or(0.0);
    let completion_cost = row.completion_cost_usd.unwrap_or(0.0);
    let total_requests = row.total_requests.unwrap_or(0);
    let total_tokens = row.total_tokens.unwrap_or(0);

    let avg_cost_per_request = if total_requests > 0 {
        total_cost / total_requests as f64
    } else {
        0.0
    };

    let avg_cost_per_1k_tokens = if total_tokens > 0 {
        (total_cost / total_tokens as f64) * 1000.0
    } else {
        0.0
    };

    // Calculate day-over-day and week-over-week changes
    // (Simplified - would need separate queries for accurate calculation)
    let day_over_day_change = None;
    let week_over_week_change = None;

    Ok(CostOverview {
        total_cost,
        prompt_cost,
        completion_cost,
        total_requests,
        total_tokens,
        avg_cost_per_request,
        avg_cost_per_1k_tokens,
        day_over_day_change,
        week_over_week_change,
    })
}

/// Query cost breakdown by dimension
async fn query_cost_breakdown(
    pool: &PgPool,
    org_id: &str,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
    dimension: &str,
    request: &CostSummaryRequest,
) -> Result<Vec<CostBreakdownItem>, ApiError> {
    let mut where_clauses = vec![
        "org_id = $1".to_string(),
        "ts >= $2".to_string(),
        "ts < $3".to_string(),
    ];
    let mut param_index = 4;

    if request.provider.is_some() {
        where_clauses.push(format!("provider = ${}", param_index));
        param_index += 1;
    }
    if request.model.is_some() {
        where_clauses.push(format!("model = ${}", param_index));
        param_index += 1;
    }
    if request.environment.is_some() {
        where_clauses.push(format!("environment = ${}", param_index));
        param_index += 1;
    }
    if request.user_id.is_some() {
        where_clauses.push(format!("user_id = ${}", param_index));
    }

    let query_str = format!(
        r#"
        SELECT
            {} AS name,
            SUM(total_cost_usd) AS cost,
            COUNT(*) AS requests
        FROM llm_traces
        WHERE {}
        GROUP BY {}
        ORDER BY cost DESC
        LIMIT 50
        "#,
        dimension,
        where_clauses.join(" AND "),
        dimension
    );

    let mut query = sqlx::query_as::<_, CostBreakdownRow>(&query_str)
        .bind(org_id)
        .bind(start_time)
        .bind(end_time);

    if let Some(ref provider) = request.provider {
        query = query.bind(provider);
    }
    if let Some(ref model) = request.model {
        query = query.bind(model);
    }
    if let Some(ref environment) = request.environment {
        query = query.bind(environment);
    }
    if let Some(ref user_id) = request.user_id {
        query = query.bind(user_id);
    }

    let rows = query.fetch_all(pool).await.map_err(|e| {
        error!(error = %e, dimension = dimension, "Failed to query cost breakdown");
        ApiError::Internal(format!("Database query failed: {}", e))
    })?;

    let total_cost: f64 = rows.iter().map(|r| r.cost.unwrap_or(0.0)).sum();

    let items: Vec<CostBreakdownItem> = rows
        .into_iter()
        .map(|row| {
            let cost = row.cost.unwrap_or(0.0);
            let requests = row.requests.unwrap_or(0);
            let percentage = if total_cost > 0.0 {
                (cost / total_cost) * 100.0
            } else {
                0.0
            };
            let avg_cost_per_request = if requests > 0 {
                cost / requests as f64
            } else {
                0.0
            };

            CostBreakdownItem {
                name: row.name,
                cost,
                requests,
                percentage,
                avg_cost_per_request,
            }
        })
        .collect();

    Ok(items)
}

/// Query cost trends
async fn query_cost_trends(
    pool: &PgPool,
    org_id: &str,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
    request: &CostSummaryRequest,
) -> Result<CostTrends, ApiError> {
    // Query daily trend
    let daily = query_cost_trend_data(pool, org_id, start_time, end_time, "1 day", request).await?;

    // Query weekly trend
    let weekly =
        query_cost_trend_data(pool, org_id, start_time, end_time, "7 days", request).await?;

    // Calculate growth rates
    let growth_rate_daily = calculate_growth_rate(&daily);
    let growth_rate_weekly = calculate_growth_rate(&weekly);

    Ok(CostTrends {
        daily,
        weekly,
        growth_rate_daily,
        growth_rate_weekly,
    })
}

/// Query cost trend data with time bucketing
async fn query_cost_trend_data(
    pool: &PgPool,
    org_id: &str,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
    interval: &str,
    request: &CostSummaryRequest,
) -> Result<Vec<CostDataPoint>, ApiError> {
    let mut where_clauses = vec![
        "org_id = $2".to_string(),
        "ts >= $3".to_string(),
        "ts < $4".to_string(),
    ];
    let mut param_index = 5;

    if request.provider.is_some() {
        where_clauses.push(format!("provider = ${}", param_index));
        param_index += 1;
    }
    if request.model.is_some() {
        where_clauses.push(format!("model = ${}", param_index));
        param_index += 1;
    }
    if request.environment.is_some() {
        where_clauses.push(format!("environment = ${}", param_index));
        param_index += 1;
    }
    if request.user_id.is_some() {
        where_clauses.push(format!("user_id = ${}", param_index));
    }

    let query_str = format!(
        r#"
        SELECT
            time_bucket($1, ts) AS date,
            SUM(total_cost_usd) AS cost,
            COUNT(*) AS requests
        FROM llm_traces
        WHERE {}
        GROUP BY date
        ORDER BY date ASC
        "#,
        where_clauses.join(" AND ")
    );

    let mut query = sqlx::query_as::<_, CostTrendRow>(&query_str)
        .bind(interval)
        .bind(org_id)
        .bind(start_time)
        .bind(end_time);

    if let Some(ref provider) = request.provider {
        query = query.bind(provider);
    }
    if let Some(ref model) = request.model {
        query = query.bind(model);
    }
    if let Some(ref environment) = request.environment {
        query = query.bind(environment);
    }
    if let Some(ref user_id) = request.user_id {
        query = query.bind(user_id);
    }

    let rows = query.fetch_all(pool).await.map_err(|e| {
        error!(error = %e, "Failed to query cost trend");
        ApiError::Internal(format!("Database query failed: {}", e))
    })?;

    let data_points: Vec<CostDataPoint> = rows
        .into_iter()
        .map(|row| CostDataPoint {
            date: row.date,
            cost: row.cost.unwrap_or(0.0),
            requests: row.requests.unwrap_or(0),
        })
        .collect();

    Ok(data_points)
}

/// Calculate growth rate from trend data
fn calculate_growth_rate(data: &[CostDataPoint]) -> f64 {
    if data.len() < 2 {
        return 0.0;
    }

    let costs: Vec<(f64, f64)> = data
        .iter()
        .enumerate()
        .map(|(i, point)| (i as f64, point.cost))
        .collect();

    let (slope, _, _) = calculate_linear_regression(&costs);
    slope
}

/// Query top expensive traces
async fn query_top_expensive_traces(
    pool: &PgPool,
    org_id: &str,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
    request: &CostSummaryRequest,
) -> Result<Vec<ExpensiveTrace>, ApiError> {
    let mut where_clauses = vec![
        "org_id = $1".to_string(),
        "ts >= $2".to_string(),
        "ts < $3".to_string(),
    ];
    let mut param_index = 4;

    if request.provider.is_some() {
        where_clauses.push(format!("provider = ${}", param_index));
        param_index += 1;
    }
    if request.model.is_some() {
        where_clauses.push(format!("model = ${}", param_index));
        param_index += 1;
    }
    if request.environment.is_some() {
        where_clauses.push(format!("environment = ${}", param_index));
        param_index += 1;
    }
    if request.user_id.is_some() {
        where_clauses.push(format!("user_id = ${}", param_index));
    }

    let query_str = format!(
        r#"
        SELECT
            trace_id,
            ts,
            provider,
            model,
            total_cost_usd,
            total_tokens,
            duration_ms,
            user_id
        FROM llm_traces
        WHERE {}
        ORDER BY total_cost_usd DESC
        LIMIT ${}
        "#,
        where_clauses.join(" AND "),
        param_index
    );

    let mut query = sqlx::query_as::<_, ExpensiveTraceRow>(&query_str)
        .bind(org_id)
        .bind(start_time)
        .bind(end_time);

    if let Some(ref provider) = request.provider {
        query = query.bind(provider);
    }
    if let Some(ref model) = request.model {
        query = query.bind(model);
    }
    if let Some(ref environment) = request.environment {
        query = query.bind(environment);
    }
    if let Some(ref user_id) = request.user_id {
        query = query.bind(user_id);
    }

    query = query.bind(request.top_limit);

    let rows = query.fetch_all(pool).await.map_err(|e| {
        error!(error = %e, "Failed to query top expensive traces");
        ApiError::Internal(format!("Database query failed: {}", e))
    })?;

    let traces: Vec<ExpensiveTrace> = rows
        .into_iter()
        .map(|row| ExpensiveTrace {
            trace_id: row.trace_id,
            timestamp: row.ts,
            provider: row.provider,
            model: row.model,
            cost: row.total_cost_usd,
            tokens: row.total_tokens,
            duration_ms: row.duration_ms,
            user_id: row.user_id,
        })
        .collect();

    Ok(traces)
}

// ============================================================================
// Endpoint 2: GET /api/v1/costs/attribution
// ============================================================================

/// GET /api/v1/costs/attribution - Cost attribution by dimension
///
/// Attributes costs across different dimensions (user, team, tag, provider, model, environment).
///
/// ## Query Parameters
/// - `start_time`: Start of time range (ISO 8601) - required
/// - `end_time`: End of time range (ISO 8601) - required
/// - `dimension`: Attribution dimension (user, team, tag, provider, model, environment) - required
/// - `provider`: Filter by provider
/// - `model`: Filter by model
/// - `environment`: Filter by environment
/// - `limit`: Max items to return (max 1000) - default: 100
/// - `min_cost`: Minimum cost threshold
///
/// ## Example
/// ```bash
/// curl -X GET 'http://localhost:8080/api/v1/costs/attribution?start_time=2025-10-01T00:00:00Z&end_time=2025-11-01T00:00:00Z&dimension=user&limit=50' \
///   -H "Authorization: Bearer $JWT_TOKEN"
/// ```
#[instrument(skip(state, auth))]
async fn get_cost_attribution(
    State(state): State<Arc<AppState>>,
    auth: AuthContext,
    Query(request): Query<CostAttributionRequest>,
) -> Result<Json<CostAttributionResponse>, ApiError> {
    // Check permissions
    if !auth.has_permission("costs:read") {
        return Err(ApiError::Forbidden(
            "Insufficient permissions to read cost data".to_string(),
        ));
    }

    // Validate request
    request.validate().map_err(ApiError::BadRequest)?;

    info!(
        org_id = %auth.organization_id,
        dimension = ?request.dimension,
        "Querying cost attribution"
    );

    // Generate cache key
    let cache_key = generate_attribution_cache_key(&request, &auth.organization_id);

    // Try cache
    if let Ok(cached) = try_get_from_cache(&state, &cache_key).await {
        info!("Returning cached cost attribution");
        return Ok(Json(cached));
    }

    // Execute query
    let response =
        execute_cost_attribution(&state.db_pool, &request, &auth.organization_id).await?;

    // Cache result
    cache_cost_attribution(&state, &cache_key, &response).await;

    info!(items = response.items.len(), "Cost attribution completed");

    Ok(Json(response))
}

/// Execute cost attribution query
async fn execute_cost_attribution(
    pool: &PgPool,
    request: &CostAttributionRequest,
    org_id: &str,
) -> Result<CostAttributionResponse, ApiError> {
    let dimension_col = request.dimension.to_column_name();

    // Build query
    let mut where_clauses = vec![
        "org_id = $1".to_string(),
        "ts >= $2".to_string(),
        "ts < $3".to_string(),
    ];
    let mut param_index = 4;

    if request.provider.is_some() {
        where_clauses.push(format!("provider = ${}", param_index));
        param_index += 1;
    }
    if request.model.is_some() {
        where_clauses.push(format!("model = ${}", param_index));
        param_index += 1;
    }
    if request.environment.is_some() {
        where_clauses.push(format!("environment = ${}", param_index));
        param_index += 1;
    }

    let query_str = format!(
        r#"
        SELECT
            {} AS dimension_value,
            SUM(total_cost_usd) AS total_cost,
            SUM(prompt_cost_usd) AS prompt_cost,
            SUM(completion_cost_usd) AS completion_cost,
            COUNT(*) AS request_count,
            SUM(total_tokens) AS total_tokens
        FROM llm_traces
        WHERE {} AND {} IS NOT NULL
        GROUP BY {}
        ORDER BY total_cost DESC
        LIMIT ${}
        "#,
        dimension_col,
        where_clauses.join(" AND "),
        dimension_col,
        dimension_col,
        param_index
    );

    let mut query = sqlx::query_as::<_, AttributionRow>(&query_str)
        .bind(org_id)
        .bind(request.start_time)
        .bind(request.end_time);

    if let Some(ref provider) = request.provider {
        query = query.bind(provider);
    }
    if let Some(ref model) = request.model {
        query = query.bind(model);
    }
    if let Some(ref environment) = request.environment {
        query = query.bind(environment);
    }

    query = query.bind(request.limit);

    let rows = query.fetch_all(pool).await.map_err(|e| {
        error!(error = %e, "Failed to query cost attribution");
        ApiError::Internal(format!("Database query failed: {}", e))
    })?;

    let total_cost: f64 = rows.iter().map(|r| r.total_cost.unwrap_or(0.0)).sum();
    let total_requests: i64 = rows.iter().map(|r| r.request_count.unwrap_or(0)).sum();

    let items: Vec<AttributionItem> = rows
        .into_iter()
        .filter(|row| {
            if let Some(min_cost) = request.min_cost {
                row.total_cost.unwrap_or(0.0) >= min_cost
            } else {
                true
            }
        })
        .map(|row| {
            let cost = row.total_cost.unwrap_or(0.0);
            let requests = row.request_count.unwrap_or(0);
            let cost_percentage = if total_cost > 0.0 {
                (cost / total_cost) * 100.0
            } else {
                0.0
            };
            let avg_cost_per_request = if requests > 0 {
                cost / requests as f64
            } else {
                0.0
            };

            AttributionItem {
                dimension_value: row.dimension_value,
                total_cost: cost,
                prompt_cost: row.prompt_cost.unwrap_or(0.0),
                completion_cost: row.completion_cost.unwrap_or(0.0),
                request_count: requests,
                total_tokens: row.total_tokens.unwrap_or(0),
                cost_percentage,
                avg_cost_per_request,
                by_provider: HashMap::new(), // Would require additional query
                by_model: HashMap::new(),    // Would require additional query
            }
        })
        .collect();

    let metadata = AttributionMetadata {
        dimension: format!("{:?}", request.dimension),
        start_time: request.start_time,
        end_time: request.end_time,
        total_items: items.len(),
    };

    let summary = AttributionSummary {
        total_cost,
        total_requests,
        unique_items: items.len() as i64,
        avg_cost_per_item: if !items.is_empty() {
            total_cost / items.len() as f64
        } else {
            0.0
        },
    };

    Ok(CostAttributionResponse {
        metadata,
        items,
        summary,
    })
}

// ============================================================================
// Endpoint 3: GET /api/v1/costs/forecast
// ============================================================================

/// GET /api/v1/costs/forecast - Cost forecasting with linear regression
///
/// Forecasts future costs based on historical data using linear regression.
///
/// ## Query Parameters
/// - `historical_start`: Historical data start (ISO 8601) - default: 30 days ago
/// - `historical_end`: Historical data end (ISO 8601) - default: now
/// - `forecast_period`: Forecast period (next_week, next_month, next_quarter, custom) - default: next_month
/// - `provider`: Filter by provider
/// - `model`: Filter by model
/// - `environment`: Filter by environment
/// - `include_confidence_intervals`: Include confidence intervals - default: true
///
/// ## Example
/// ```bash
/// curl -X GET 'http://localhost:8080/api/v1/costs/forecast?forecast_period=next_month&include_confidence_intervals=true' \
///   -H "Authorization: Bearer $JWT_TOKEN"
/// ```
#[instrument(skip(state, auth))]
async fn get_cost_forecast(
    State(state): State<Arc<AppState>>,
    auth: AuthContext,
    Query(request): Query<CostForecastRequest>,
) -> Result<Json<CostForecastResponse>, ApiError> {
    // Check permissions
    if !auth.has_permission("costs:read") {
        return Err(ApiError::Forbidden(
            "Insufficient permissions to read cost data".to_string(),
        ));
    }

    // Validate request
    request.validate().map_err(ApiError::BadRequest)?;

    info!(
        org_id = %auth.organization_id,
        forecast_period = ?request.forecast_period,
        "Querying cost forecast"
    );

    // Set defaults
    let historical_end = request.historical_end.unwrap_or_else(Utc::now);
    let historical_start = request
        .historical_start
        .unwrap_or_else(|| historical_end - Duration::days(30));

    // Generate cache key
    let cache_key = generate_forecast_cache_key(
        &request,
        &auth.organization_id,
        historical_start,
        historical_end,
    );

    // Try cache
    if let Ok(cached) = try_get_from_cache(&state, &cache_key).await {
        info!("Returning cached cost forecast");
        return Ok(Json(cached));
    }

    // Execute forecast
    let response = execute_cost_forecast(
        &state.db_pool,
        &request,
        &auth.organization_id,
        historical_start,
        historical_end,
    )
    .await?;

    // Cache result (shorter TTL for forecasts)
    let mut redis_conn = state
        .redis_client
        .get_async_connection()
        .await
        .map_err(|e| {
            warn!(error = %e, "Redis connection error");
            ApiError::Internal("Cache error".to_string())
        })?;
    if let Ok(serialized) = serde_json::to_string(&response) {
        let _: Result<(), _> = redis_conn
            .set_ex(&cache_key, serialized, 1800) // 30 min cache
            .await;
    }

    info!("Cost forecast completed");

    Ok(Json(response))
}

/// Execute cost forecast
async fn execute_cost_forecast(
    pool: &PgPool,
    request: &CostForecastRequest,
    org_id: &str,
    historical_start: DateTime<Utc>,
    historical_end: DateTime<Utc>,
) -> Result<CostForecastResponse, ApiError> {
    // Query historical data
    let historical =
        query_forecast_historical_data(pool, org_id, historical_start, historical_end, request)
            .await?;

    if historical.len() < 2 {
        return Err(ApiError::BadRequest(
            "Insufficient historical data for forecasting (need at least 2 data points)"
                .to_string(),
        ));
    }

    // Prepare data for linear regression
    let data_points: Vec<(f64, f64)> = historical
        .iter()
        .enumerate()
        .map(|(i, point)| (i as f64, point.cost))
        .collect();

    // Calculate linear regression
    let (slope, intercept, r_squared) = calculate_linear_regression(&data_points);

    // Generate forecast
    let forecast_days = request.forecast_period.to_days();
    let mut forecast_points = Vec::new();

    for i in 0..forecast_days {
        let x = (historical.len() + i as usize) as f64;
        let forecasted_cost = intercept + slope * x;

        // Calculate confidence interval (simplified - using standard error)
        let (lower_bound, upper_bound) = if request.include_confidence_intervals {
            let std_error = forecasted_cost * 0.1; // Simplified 10% margin
            (
                Some((forecasted_cost - 1.96 * std_error).max(0.0)),
                Some(forecasted_cost + 1.96 * std_error),
            )
        } else {
            (None, None)
        };

        forecast_points.push(ForecastDataPoint {
            date: historical_end + Duration::days(i as i64 + 1),
            forecasted_cost: forecasted_cost.max(0.0),
            lower_bound,
            upper_bound,
        });
    }

    // Calculate MAPE
    let actual: Vec<f64> = historical.iter().map(|p| p.cost).collect();
    let predicted: Vec<f64> = data_points
        .iter()
        .map(|(x, _)| intercept + slope * x)
        .collect();
    let mape = calculate_mape(&actual, &predicted);

    let total_forecasted_cost: f64 = forecast_points.iter().map(|p| p.forecasted_cost).sum();
    let avg_daily_cost = if !forecast_points.is_empty() {
        total_forecasted_cost / forecast_points.len() as f64
    } else {
        0.0
    };
    let projected_monthly_cost = avg_daily_cost * 30.0;

    let metadata = ForecastMetadata {
        historical_start,
        historical_end,
        forecast_start: historical_end + Duration::days(1),
        forecast_end: historical_end + Duration::days(forecast_days as i64),
        forecast_days,
        model_type: "linear_regression".to_string(),
        generated_at: Utc::now(),
    };

    let summary = ForecastSummary {
        total_forecasted_cost,
        avg_daily_cost,
        projected_monthly_cost,
        r_squared,
        mape,
    };

    Ok(CostForecastResponse {
        metadata,
        historical,
        forecast: forecast_points,
        summary,
    })
}

/// Query historical data for forecasting
async fn query_forecast_historical_data(
    pool: &PgPool,
    org_id: &str,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
    request: &CostForecastRequest,
) -> Result<Vec<CostDataPoint>, ApiError> {
    let mut where_clauses = vec![
        "org_id = $2".to_string(),
        "ts >= $3".to_string(),
        "ts < $4".to_string(),
    ];
    let mut param_index = 5;

    if request.provider.is_some() {
        where_clauses.push(format!("provider = ${}", param_index));
        param_index += 1;
    }
    if request.model.is_some() {
        where_clauses.push(format!("model = ${}", param_index));
        param_index += 1;
    }
    if request.environment.is_some() {
        where_clauses.push(format!("environment = ${}", param_index));
    }

    let query_str = format!(
        r#"
        SELECT
            time_bucket($1, ts) AS date,
            SUM(total_cost_usd) AS cost
        FROM llm_traces
        WHERE {}
        GROUP BY date
        ORDER BY date ASC
        "#,
        where_clauses.join(" AND ")
    );

    let mut query = sqlx::query_as::<_, ForecastHistoricalRow>(&query_str)
        .bind("1 day")
        .bind(org_id)
        .bind(start_time)
        .bind(end_time);

    if let Some(ref provider) = request.provider {
        query = query.bind(provider);
    }
    if let Some(ref model) = request.model {
        query = query.bind(model);
    }
    if let Some(ref environment) = request.environment {
        query = query.bind(environment);
    }

    let rows = query.fetch_all(pool).await.map_err(|e| {
        error!(error = %e, "Failed to query forecast historical data");
        ApiError::Internal(format!("Database query failed: {}", e))
    })?;

    let data_points: Vec<CostDataPoint> = rows
        .into_iter()
        .map(|row| CostDataPoint {
            date: row.date,
            cost: row.cost.unwrap_or(0.0),
            requests: 0, // Not needed for forecast
        })
        .collect();

    Ok(data_points)
}

// ============================================================================
// Helper Functions
// ============================================================================

fn generate_summary_cache_key(
    request: &CostSummaryRequest,
    org_id: &str,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
) -> String {
    format!(
        "costs:summary:{}:{}:{}:{}:{}:{}:{}:{}",
        org_id,
        start_time.to_rfc3339(),
        end_time.to_rfc3339(),
        request.provider.as_deref().unwrap_or("all"),
        request.model.as_deref().unwrap_or("all"),
        request.environment.as_deref().unwrap_or("all"),
        request.include_trends,
        request.include_top_traces
    )
}

fn generate_attribution_cache_key(request: &CostAttributionRequest, org_id: &str) -> String {
    format!(
        "costs:attribution:{}:{}:{}:{:?}:{}",
        org_id,
        request.start_time.to_rfc3339(),
        request.end_time.to_rfc3339(),
        request.dimension,
        request.limit
    )
}

fn generate_forecast_cache_key(
    request: &CostForecastRequest,
    org_id: &str,
    historical_start: DateTime<Utc>,
    historical_end: DateTime<Utc>,
) -> String {
    format!(
        "costs:forecast:{}:{}:{}:{:?}",
        org_id,
        historical_start.to_rfc3339(),
        historical_end.to_rfc3339(),
        request.forecast_period
    )
}

async fn try_get_from_cache<T: serde::de::DeserializeOwned>(
    state: &Arc<AppState>,
    cache_key: &str,
) -> Result<T, ()> {
    let mut redis_conn = state
        .redis_client
        .get_async_connection()
        .await
        .map_err(|e| {
            warn!(error = %e, "Redis connection error");
        })?;

    let cached: String = redis_conn.get(cache_key).await.map_err(|_| ())?;
    serde_json::from_str(&cached).map_err(|_| ())
}

async fn cache_cost_summary(
    state: &Arc<AppState>,
    cache_key: &str,
    response: &CostSummaryResponse,
) {
    if let Ok(serialized) = serde_json::to_string(response) {
        if let Ok(mut conn) = state.redis_client.get_async_connection().await {
            let _: Result<(), _> = conn.set_ex(cache_key, serialized, state.cache_ttl).await;
        }
    }
}

async fn cache_cost_attribution(
    state: &Arc<AppState>,
    cache_key: &str,
    response: &CostAttributionResponse,
) {
    if let Ok(serialized) = serde_json::to_string(response) {
        if let Ok(mut conn) = state.redis_client.get_async_connection().await {
            let _: Result<(), _> = conn.set_ex(cache_key, serialized, state.cache_ttl).await;
        }
    }
}
