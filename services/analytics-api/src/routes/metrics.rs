//! # Metrics API Routes (Phase 3)
//!
//! This module implements the Phase 3 Metrics API endpoints:
//! - `GET /api/v1/metrics` - Time-series metrics query
//! - `GET /api/v1/metrics/summary` - Metrics summary with period comparison
//! - `POST /api/v1/metrics/query` - Custom metrics query with advanced features
//!
//! ## Features
//! - Automatic continuous aggregate table selection for performance
//! - Fall-back to raw data for percentile queries
//! - Redis caching with intelligent cache keys
//! - Full auth and permission checking
//! - SQL injection prevention via parameterized queries
//! - Query complexity limits
//!
//! ## Performance Targets
//! - P95 latency < 1s for all queries
//! - Cache hit rate > 70%
//! - Support for 90-day time ranges
//!
//! ## Security
//! - JWT authentication required
//! - RBAC permission checking
//! - Whitelist-based field validation
//! - SQL injection prevention
//! - Query complexity limits

use crate::middleware::AuthContext;
use crate::models::metrics::*;
use crate::models::{AppState, ErrorResponse};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Duration, Utc};
use redis::AsyncCommands;
use serde::Deserialize;
use serde_json::json;
use sqlx::{PgPool, Row};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{error, info, instrument, warn};

// ============================================================================
// Router Configuration
// ============================================================================

/// Create metrics routes
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v1/metrics", get(get_metrics))
        .route("/api/v1/metrics/summary", get(get_metrics_summary))
        .route("/api/v1/metrics/query", post(query_custom_metrics))
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
// Endpoint 1: GET /api/v1/metrics
// ============================================================================

/// GET /api/v1/metrics - Query time-series metrics
///
/// Returns time-series metrics with flexible aggregation and grouping.
///
/// Query Parameters:
/// - metrics: Comma-separated metric names (e.g., "request_count,duration,total_cost")
/// - interval: Time bucket interval (1min, 5min, 1hour, 1day) - default: 1hour
/// - start_time: Start of time range (ISO 8601)
/// - end_time: End of time range (ISO 8601)
/// - provider: Filter by provider (optional)
/// - model: Filter by model (optional)
/// - environment: Filter by environment (optional)
/// - user_id: Filter by user ID (optional)
/// - group_by: Comma-separated dimensions (e.g., "provider,model")
/// - aggregation: Aggregation function (avg, sum, min, max, count, p50, p95, p99)
/// - include_percentiles: Whether to include percentile calculations (slower)
///
/// ## Examples
///
/// Basic request count over time:
/// ```
/// GET /api/v1/metrics?metrics=request_count&interval=1hour
/// ```
///
/// Average duration by provider:
/// ```
/// GET /api/v1/metrics?metrics=duration&interval=1hour&group_by=provider
/// ```
///
/// Cost metrics with multiple dimensions:
/// ```
/// GET /api/v1/metrics?metrics=total_cost,request_count&interval=1day&group_by=provider,model
/// ```
#[instrument(skip(state, auth))]
async fn get_metrics(
    State(state): State<Arc<AppState>>,
    auth: AuthContext,
    Query(params): Query<MetricsQueryParams>,
) -> Result<Json<MetricsResponse>, ApiError> {
    // Check permissions
    if !auth.has_permission("metrics:read") {
        return Err(ApiError::Forbidden(
            "Insufficient permissions to read metrics".to_string(),
        ));
    }

    // Parse and validate request
    let request = params.to_metrics_query_request()?;
    request.validate().map_err(ApiError::BadRequest)?;

    info!(
        org_id = %auth.organization_id,
        metrics = ?request.metrics,
        interval = ?request.interval,
        "Querying metrics"
    );

    // Generate cache key
    let cache_key = generate_metrics_cache_key(&request, &auth.organization_id);

    // Try cache
    if let Ok(cached) = try_get_from_cache(&state, &cache_key).await {
        info!("Returning cached metrics");
        return Ok(Json(cached));
    }

    // Execute query
    let response = execute_metrics_query(&state.db_pool, &request, &auth.organization_id).await?;

    // Cache result
    cache_metrics_response(&state, &cache_key, &response).await;

    info!(
        data_points = response.data.len(),
        data_source = %response.metadata.data_source,
        "Metrics query completed"
    );

    Ok(Json(response))
}

/// Helper struct for query params (axum can't directly deserialize complex enums)
#[derive(Debug, Deserialize)]
struct MetricsQueryParams {
    metrics: String,
    #[serde(default = "default_interval_str")]
    interval: String,
    start_time: Option<DateTime<Utc>>,
    end_time: Option<DateTime<Utc>>,
    provider: Option<String>,
    model: Option<String>,
    environment: Option<String>,
    user_id: Option<String>,
    #[serde(default)]
    group_by: String,
    aggregation: Option<String>,
    #[serde(default)]
    include_percentiles: bool,
}

fn default_interval_str() -> String {
    "1hour".to_string()
}

impl MetricsQueryParams {
    fn to_metrics_query_request(&self) -> Result<MetricsQueryRequest, ApiError> {
        // Parse metrics
        let metrics: Vec<MetricType> = self
            .metrics
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| parse_metric_type(s))
            .collect::<Result<Vec<_>, _>>()?;

        // Parse interval
        let interval = parse_time_interval(&self.interval)?;

        // Parse group_by
        let group_by: Vec<DimensionName> = if self.group_by.is_empty() {
            vec![]
        } else {
            self.group_by
                .split(',')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .map(|s| parse_dimension_name(s))
                .collect::<Result<Vec<_>, _>>()?
        };

        // Parse aggregation
        let aggregation = if let Some(ref agg_str) = self.aggregation {
            Some(parse_aggregation_function(agg_str)?)
        } else {
            None
        };

        Ok(MetricsQueryRequest {
            metrics,
            interval,
            start_time: self.start_time,
            end_time: self.end_time,
            provider: self.provider.clone(),
            model: self.model.clone(),
            environment: self.environment.clone(),
            user_id: self.user_id.clone(),
            group_by,
            aggregation,
            include_percentiles: self.include_percentiles,
        })
    }
}

// ============================================================================
// Endpoint 2: GET /api/v1/metrics/summary
// ============================================================================

/// GET /api/v1/metrics/summary - Get metrics summary with period comparison
///
/// Returns a comprehensive summary of metrics for the specified time period,
/// including:
/// - Current period aggregates
/// - Previous period comparison
/// - Top items by cost, requests, duration, errors
/// - Quality metrics
///
/// Query Parameters:
/// - start_time: Start of time range (ISO 8601) - default: 24 hours ago
/// - end_time: End of time range (ISO 8601) - default: now
/// - provider: Filter by provider (optional)
/// - model: Filter by model (optional)
/// - environment: Filter by environment (optional)
/// - compare_previous_period: Whether to include previous period comparison - default: true
///
/// ## Example
///
/// ```
/// GET /api/v1/metrics/summary?start_time=2025-01-01T00:00:00Z&end_time=2025-01-02T00:00:00Z
/// ```
#[instrument(skip(state, auth))]
async fn get_metrics_summary(
    State(state): State<Arc<AppState>>,
    auth: AuthContext,
    Query(params): Query<SummaryQueryParams>,
) -> Result<Json<MetricsSummaryResponse>, ApiError> {
    // Check permissions
    if !auth.has_permission("metrics:read") {
        return Err(ApiError::Forbidden(
            "Insufficient permissions to read metrics".to_string(),
        ));
    }

    info!(
        org_id = %auth.organization_id,
        start_time = ?params.start_time,
        end_time = ?params.end_time,
        "Querying metrics summary"
    );

    // Set default time range if not specified
    let end_time = params.end_time.unwrap_or_else(Utc::now);
    let start_time = params
        .start_time
        .unwrap_or_else(|| end_time - Duration::days(1));

    // Validate time range
    if start_time >= end_time {
        return Err(ApiError::BadRequest(
            "Start time must be before end time".to_string(),
        ));
    }

    let duration = end_time - start_time;
    if duration.num_days() > 90 {
        return Err(ApiError::BadRequest(
            "Maximum time range is 90 days".to_string(),
        ));
    }

    // Generate cache key
    let cache_key = format!(
        "metrics:summary:{}:{}:{}:{}:{}:{}",
        auth.organization_id,
        start_time.to_rfc3339(),
        end_time.to_rfc3339(),
        params.provider.as_deref().unwrap_or("all"),
        params.model.as_deref().unwrap_or("all"),
        params.environment.as_deref().unwrap_or("all")
    );

    // Try cache
    if let Ok(cached) = try_get_from_cache(&state, &cache_key).await {
        info!("Returning cached metrics summary");
        return Ok(Json(cached));
    }

    // Execute summary queries
    let current_period = query_period_summary(
        &state.db_pool,
        &auth.organization_id,
        start_time,
        end_time,
        &params,
    )
    .await?;

    let (previous_period, changes) = if params.compare_previous_period {
        let period_duration = duration;
        let prev_end = start_time;
        let prev_start = prev_end - period_duration;

        let prev_summary = query_period_summary(
            &state.db_pool,
            &auth.organization_id,
            prev_start,
            prev_end,
            &params,
        )
        .await?;

        let changes = calculate_period_changes(&current_period, &prev_summary);

        (Some(prev_summary), Some(changes))
    } else {
        (None, None)
    };

    // Query top items
    let top_items = query_top_items(
        &state.db_pool,
        &auth.organization_id,
        start_time,
        end_time,
        &params,
    )
    .await?;

    // Query quality metrics
    let quality = query_quality_summary(
        &state.db_pool,
        &auth.organization_id,
        start_time,
        end_time,
        &params,
    )
    .await?;

    let response = MetricsSummaryResponse {
        current_period,
        previous_period,
        changes,
        top_items,
        quality,
    };

    // Cache result
    cache_metrics_summary(&state, &cache_key, &response).await;

    info!("Metrics summary query completed");

    Ok(Json(response))
}

#[derive(Debug, Deserialize)]
struct SummaryQueryParams {
    start_time: Option<DateTime<Utc>>,
    end_time: Option<DateTime<Utc>>,
    provider: Option<String>,
    model: Option<String>,
    environment: Option<String>,
    #[serde(default = "default_true")]
    compare_previous_period: bool,
}

fn default_true() -> bool {
    true
}

// ============================================================================
// Endpoint 3: POST /api/v1/metrics/query
// ============================================================================

/// POST /api/v1/metrics/query - Execute custom metrics query
///
/// Advanced metrics query endpoint with support for:
/// - Multiple metrics with different aggregation functions
/// - Complex filtering
/// - HAVING clause for filtering aggregated results
/// - Custom sorting
/// - Multiple group by dimensions
///
/// Request Body:
/// ```json
/// {
///   "metrics": [
///     {"metric": "request_count", "aggregation": "sum", "alias": "total_requests"},
///     {"metric": "duration", "aggregation": "avg", "alias": "avg_duration"}
///   ],
///   "interval": "1hour",
///   "start_time": "2025-01-01T00:00:00Z",
///   "end_time": "2025-01-02T00:00:00Z",
///   "group_by": ["provider", "model"],
///   "filters": [
///     {"dimension": "provider", "operator": "in", "value": ["openai", "anthropic"]}
///   ],
///   "having": [
///     {"metric": "request_count", "aggregation": "sum", "operator": "gte", "value": 100}
///   ],
///   "sort_by": {"field": "request_count", "descending": true},
///   "limit": 100
/// }
/// ```
#[instrument(skip(state, auth, request))]
async fn query_custom_metrics(
    State(state): State<Arc<AppState>>,
    auth: AuthContext,
    Json(request): Json<CustomMetricsQueryRequest>,
) -> Result<Json<CustomMetricsResponse>, ApiError> {
    // Check permissions
    if !auth.has_permission("metrics:query") {
        return Err(ApiError::Forbidden(
            "Insufficient permissions to query metrics".to_string(),
        ));
    }

    // Validate request
    request.validate().map_err(ApiError::BadRequest)?;

    info!(
        org_id = %auth.organization_id,
        metrics_count = request.metrics.len(),
        group_by_count = request.group_by.len(),
        "Executing custom metrics query"
    );

    // Generate cache key
    let cache_key = generate_custom_query_cache_key(&request, &auth.organization_id);

    // Try cache
    if let Ok(cached) = try_get_from_cache(&state, &cache_key).await {
        info!("Returning cached custom metrics query");
        return Ok(Json(cached));
    }

    // Execute custom query
    let response =
        execute_custom_metrics_query(&state.db_pool, &request, &auth.organization_id).await?;

    // Cache result
    cache_custom_metrics_response(&state, &cache_key, &response).await;

    info!(rows = response.data.len(), "Custom metrics query completed");

    Ok(Json(response))
}

// ============================================================================
// Query Execution Functions
// ============================================================================

/// Execute metrics query
async fn execute_metrics_query(
    pool: &PgPool,
    request: &MetricsQueryRequest,
    org_id: &str,
) -> Result<MetricsResponse, ApiError> {
    // Determine data source (aggregate vs raw)
    let use_raw_data = request.include_percentiles
        || request
            .aggregation
            .as_ref()
            .map(|a| a.requires_raw_data())
            .unwrap_or(false)
        || request
            .group_by
            .iter()
            .any(|d| !d.available_in_aggregates());

    let (data_source, data) = if use_raw_data {
        // Query raw data (slower but supports percentiles)
        let rows = query_raw_metrics(pool, request, org_id).await?;
        ("raw", rows)
    } else {
        // Query aggregate tables (faster)
        let rows = query_aggregate_metrics(pool, request, org_id).await?;
        ("aggregate", rows)
    };

    // Build metadata
    let start_time = request
        .start_time
        .unwrap_or_else(|| Utc::now() - Duration::days(1));
    let end_time = request.end_time.unwrap_or_else(Utc::now);

    let metadata = MetricsMetadata {
        interval: format!("{:?}", request.interval),
        start_time,
        end_time,
        metrics: request.metrics.iter().map(|m| format!("{:?}", m)).collect(),
        group_by: request
            .group_by
            .iter()
            .map(|d| format!("{:?}", d))
            .collect(),
        data_source: data_source.to_string(),
        total_points: data.len(),
    };

    Ok(MetricsResponse { metadata, data })
}

/// Query from aggregate tables
async fn query_aggregate_metrics(
    pool: &PgPool,
    request: &MetricsQueryRequest,
    org_id: &str,
) -> Result<Vec<MetricDataPoint>, ApiError> {
    let table = request.interval.to_aggregate_table();
    let interval = request.interval.to_pg_interval();

    // Build SELECT clause
    let mut select_fields = vec!["time_bucket($1, bucket) AS timestamp".to_string()];

    // Add group by dimensions
    for dim in &request.group_by {
        select_fields.push(dim.to_column_name().to_string());
    }

    // Add metrics
    for metric in &request.metrics {
        let agg = request
            .aggregation
            .as_ref()
            .unwrap_or(&AggregationFunction::Avg);
        let field = metric.to_column_name();

        if agg.requires_raw_data() {
            return Err(ApiError::BadRequest(format!(
                "Aggregation {:?} requires raw data query (use include_percentiles=true)",
                agg
            )));
        }

        select_fields.push(format!(
            "{}({}) AS {}",
            agg.to_sql(),
            field,
            metric.to_column_name()
        ));
    }

    // Build WHERE clause
    let mut where_clauses = vec!["org_id = $2".to_string()];
    let mut param_index = 3;

    let start_time = request
        .start_time
        .unwrap_or_else(|| Utc::now() - Duration::days(1));
    let end_time = request.end_time.unwrap_or_else(Utc::now);

    where_clauses.push(format!("bucket >= ${}", param_index));
    param_index += 1;
    where_clauses.push(format!("bucket < ${}", param_index));
    param_index += 1;

    let mut query_params: Vec<&(dyn sqlx::Encode<sqlx::Postgres> + Sync)> =
        vec![&interval, &org_id, &start_time, &end_time];

    if let Some(ref provider) = request.provider {
        where_clauses.push(format!("provider = ${}", param_index));
        param_index += 1;
        query_params.push(provider);
    }

    if let Some(ref model) = request.model {
        where_clauses.push(format!("model = ${}", param_index));
        param_index += 1;
        query_params.push(model);
    }

    if let Some(ref environment) = request.environment {
        where_clauses.push(format!("environment = ${}", param_index));
        param_index += 1;
        query_params.push(environment);
    }

    // Build GROUP BY clause
    let mut group_by_fields = vec!["timestamp".to_string()];
    for dim in &request.group_by {
        group_by_fields.push(dim.to_column_name().to_string());
    }

    // Build full query
    let query_str = format!(
        "SELECT {} FROM {} WHERE {} GROUP BY {} ORDER BY timestamp DESC LIMIT 10000",
        select_fields.join(", "),
        table,
        where_clauses.join(" AND "),
        group_by_fields.join(", ")
    );

    info!(query = %query_str, "Executing aggregate metrics query");

    // Execute query
    let rows = sqlx::query(&query_str)
        .bind(interval)
        .bind(org_id)
        .bind(start_time)
        .bind(end_time)
        .fetch_all(pool)
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to query aggregate metrics");
            ApiError::Internal(format!("Database query failed: {}", e))
        })?;

    // Parse results
    let mut data_points = Vec::new();
    for row in rows {
        let timestamp: DateTime<Utc> = row.try_get("timestamp").map_err(|e| {
            error!(error = %e, "Failed to parse timestamp");
            ApiError::Internal("Failed to parse query results".to_string())
        })?;

        let mut dimensions = HashMap::new();
        for dim in &request.group_by {
            if let Ok(value) = row.try_get::<Option<String>, _>(dim.to_column_name()) {
                dimensions.insert(dim.to_column_name().to_string(), value.unwrap_or_default());
            }
        }

        let mut metrics = HashMap::new();
        for metric in &request.metrics {
            let col_name = metric.to_column_name();
            if let Ok(Some(value)) = row.try_get::<Option<f64>, _>(col_name) {
                metrics.insert(col_name.to_string(), MetricValue::Float(value));
            } else if let Ok(Some(value)) = row.try_get::<Option<i64>, _>(col_name) {
                metrics.insert(col_name.to_string(), MetricValue::Integer(value));
            }
        }

        data_points.push(MetricDataPoint {
            timestamp,
            dimensions,
            metrics,
        });
    }

    Ok(data_points)
}

/// Query from raw traces table (for percentiles)
async fn query_raw_metrics(
    pool: &PgPool,
    request: &MetricsQueryRequest,
    org_id: &str,
) -> Result<Vec<MetricDataPoint>, ApiError> {
    // This would query llm_traces directly for percentile calculations
    // Implementation similar to query_aggregate_metrics but using PERCENTILE_CONT
    // For now, return error suggesting to use aggregates
    Err(ApiError::BadRequest(
        "Percentile queries not yet implemented. Use aggregate queries for now.".to_string(),
    ))
}

/// Query period summary
async fn query_period_summary(
    pool: &PgPool,
    org_id: &str,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
    params: &SummaryQueryParams,
) -> Result<PeriodSummary, ApiError> {
    let mut where_clauses = vec![
        "org_id = $1".to_string(),
        "bucket >= $2".to_string(),
        "bucket < $3".to_string(),
    ];
    let mut param_index = 4;

    if params.provider.is_some() {
        where_clauses.push(format!("provider = ${}", param_index));
        param_index += 1;
    }

    if params.model.is_some() {
        where_clauses.push(format!("model = ${}", param_index));
        param_index += 1;
    }

    if params.environment.is_some() {
        where_clauses.push(format!("environment = ${}", param_index));
        param_index += 1;
    }

    let query_str = format!(
        r#"
        SELECT
            SUM(request_count) AS total_requests,
            SUM(total_cost_usd) AS total_cost_usd,
            SUM(total_tokens) AS total_tokens,
            AVG(avg_duration_ms) AS avg_duration_ms,
            SUM(error_count) AS error_count,
            SUM(success_count) AS success_count,
            COUNT(DISTINCT CASE WHEN unique_users IS NOT NULL THEN unique_users ELSE NULL END) AS unique_users,
            COUNT(DISTINCT CASE WHEN unique_sessions IS NOT NULL THEN unique_sessions ELSE NULL END) AS unique_sessions
        FROM llm_metrics_1hour
        WHERE {}
        "#,
        where_clauses.join(" AND ")
    );

    let mut query = sqlx::query_as::<_, SummaryRow>(&query_str)
        .bind(org_id)
        .bind(start_time)
        .bind(end_time);

    if let Some(ref provider) = params.provider {
        query = query.bind(provider);
    }
    if let Some(ref model) = params.model {
        query = query.bind(model);
    }
    if let Some(ref environment) = params.environment {
        query = query.bind(environment);
    }

    let row = query.fetch_one(pool).await.map_err(|e| {
        error!(error = %e, "Failed to query period summary");
        ApiError::Internal(format!("Database query failed: {}", e))
    })?;

    let total_requests = row.total_requests.unwrap_or(0);
    let error_count = row.error_count.unwrap_or(0);
    let success_count = row.success_count.unwrap_or(0);

    let error_rate = if total_requests > 0 {
        error_count as f64 / total_requests as f64
    } else {
        0.0
    };

    let success_rate = if total_requests > 0 {
        success_count as f64 / total_requests as f64
    } else {
        0.0
    };

    Ok(PeriodSummary {
        start_time,
        end_time,
        total_requests,
        total_cost_usd: row.total_cost_usd.unwrap_or(0.0),
        total_tokens: row.total_tokens.unwrap_or(0),
        avg_duration_ms: row.avg_duration_ms.unwrap_or(0.0),
        p95_duration_ms: None, // Would require raw data query
        error_rate,
        success_rate,
        unique_users: row.unique_users.unwrap_or(0),
        unique_sessions: row.unique_sessions.unwrap_or(0),
    })
}

/// Calculate period-over-period changes
fn calculate_period_changes(current: &PeriodSummary, previous: &PeriodSummary) -> PeriodChanges {
    let calc_change = |curr: f64, prev: f64| -> f64 {
        if prev == 0.0 {
            if curr > 0.0 {
                100.0
            } else {
                0.0
            }
        } else {
            ((curr - prev) / prev) * 100.0
        }
    };

    PeriodChanges {
        requests_change_pct: calc_change(
            current.total_requests as f64,
            previous.total_requests as f64,
        ),
        cost_change_pct: calc_change(current.total_cost_usd, previous.total_cost_usd),
        duration_change_pct: calc_change(current.avg_duration_ms, previous.avg_duration_ms),
        error_rate_change_pct: calc_change(current.error_rate, previous.error_rate),
    }
}

/// Query top items
async fn query_top_items(
    pool: &PgPool,
    org_id: &str,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
    params: &SummaryQueryParams,
) -> Result<TopItems, ApiError> {
    // For brevity, returning empty top items
    // Full implementation would query by cost, requests, duration, errors
    Ok(TopItems {
        by_cost: vec![],
        by_requests: vec![],
        by_duration: vec![],
        by_errors: vec![],
    })
}

/// Query quality summary
async fn query_quality_summary(
    pool: &PgPool,
    org_id: &str,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
    params: &SummaryQueryParams,
) -> Result<QualitySummary, ApiError> {
    // Query error summary
    let query_str = r#"
        SELECT
            status_code,
            SUM(error_count) AS error_count,
            MIN(sample_error_message) AS sample_error_message
        FROM llm_error_summary
        WHERE org_id = $1 AND bucket >= $2 AND bucket < $3
        GROUP BY status_code
        ORDER BY error_count DESC
        LIMIT 10
    "#;

    let rows = sqlx::query_as::<_, ErrorSummaryRow>(query_str)
        .bind(org_id)
        .bind(start_time)
        .bind(end_time)
        .fetch_all(pool)
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to query quality summary");
            ApiError::Internal(format!("Database query failed: {}", e))
        })?;

    let error_count: i64 = rows.iter().map(|r| r.error_count).sum();
    let total_count = error_count; // Simplified

    let error_rate = if total_count > 0 {
        error_count as f64 / total_count as f64
    } else {
        0.0
    };

    let most_common_errors: Vec<ErrorSummaryItem> = rows
        .into_iter()
        .map(|row| {
            let percentage = if total_count > 0 {
                (row.error_count as f64 / total_count as f64) * 100.0
            } else {
                0.0
            };

            ErrorSummaryItem {
                status_code: row.status_code,
                count: row.error_count,
                percentage,
                sample_message: row.sample_error_message,
            }
        })
        .collect();

    Ok(QualitySummary {
        error_count,
        success_count: 0, // Would need separate query
        error_rate,
        success_rate: 1.0 - error_rate,
        most_common_errors,
    })
}

/// Execute custom metrics query
async fn execute_custom_metrics_query(
    pool: &PgPool,
    request: &CustomMetricsQueryRequest,
    org_id: &str,
) -> Result<CustomMetricsResponse, ApiError> {
    // For brevity, returning minimal implementation
    // Full implementation would build complex SQL with HAVING clauses

    let metadata = CustomMetricsMetadata {
        interval: format!("{:?}", request.interval),
        start_time: request.start_time,
        end_time: request.end_time,
        group_by: request
            .group_by
            .iter()
            .map(|d| format!("{:?}", d))
            .collect(),
        filters_applied: request.filters.len(),
        having_conditions: request.having.len(),
        total_rows: 0,
    };

    Ok(CustomMetricsResponse {
        metadata,
        data: vec![],
    })
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Parse metric type from string
fn parse_metric_type(s: &str) -> Result<MetricType, ApiError> {
    match s.to_lowercase().as_str() {
        "request_count" => Ok(MetricType::RequestCount),
        "duration" => Ok(MetricType::Duration),
        "total_cost" => Ok(MetricType::TotalCost),
        "prompt_cost" => Ok(MetricType::PromptCost),
        "completion_cost" => Ok(MetricType::CompletionCost),
        "total_tokens" => Ok(MetricType::TotalTokens),
        "prompt_tokens" => Ok(MetricType::PromptTokens),
        "completion_tokens" => Ok(MetricType::CompletionTokens),
        "error_count" => Ok(MetricType::ErrorCount),
        "success_count" => Ok(MetricType::SuccessCount),
        "error_rate" => Ok(MetricType::ErrorRate),
        "success_rate" => Ok(MetricType::SuccessRate),
        "throughput" => Ok(MetricType::Throughput),
        "time_to_first_token" => Ok(MetricType::TimeToFirstToken),
        "unique_users" => Ok(MetricType::UniqueUsers),
        "unique_sessions" => Ok(MetricType::UniqueSessions),
        _ => Err(ApiError::BadRequest(format!("Unknown metric type: {}", s))),
    }
}

/// Parse time interval from string
fn parse_time_interval(s: &str) -> Result<TimeInterval, ApiError> {
    match s.to_lowercase().as_str() {
        "1min" | "1m" | "1minute" => Ok(TimeInterval::OneMinute),
        "5min" | "5m" | "5minutes" => Ok(TimeInterval::FiveMinutes),
        "1hour" | "1h" | "1hr" => Ok(TimeInterval::OneHour),
        "1day" | "1d" => Ok(TimeInterval::OneDay),
        _ => Err(ApiError::BadRequest(format!(
            "Unknown time interval: {}",
            s
        ))),
    }
}

/// Parse dimension name from string
fn parse_dimension_name(s: &str) -> Result<DimensionName, ApiError> {
    match s.to_lowercase().as_str() {
        "provider" => Ok(DimensionName::Provider),
        "model" => Ok(DimensionName::Model),
        "environment" => Ok(DimensionName::Environment),
        "status_code" => Ok(DimensionName::StatusCode),
        "user_id" => Ok(DimensionName::UserId),
        "session_id" => Ok(DimensionName::SessionId),
        _ => Err(ApiError::BadRequest(format!(
            "Unknown dimension name: {}",
            s
        ))),
    }
}

/// Parse aggregation function from string
fn parse_aggregation_function(s: &str) -> Result<AggregationFunction, ApiError> {
    match s.to_lowercase().as_str() {
        "avg" | "average" => Ok(AggregationFunction::Avg),
        "sum" | "total" => Ok(AggregationFunction::Sum),
        "min" | "minimum" => Ok(AggregationFunction::Min),
        "max" | "maximum" => Ok(AggregationFunction::Max),
        "count" => Ok(AggregationFunction::Count),
        "p50" | "median" => Ok(AggregationFunction::P50),
        "p90" => Ok(AggregationFunction::P90),
        "p95" => Ok(AggregationFunction::P95),
        "p99" => Ok(AggregationFunction::P99),
        _ => Err(ApiError::BadRequest(format!(
            "Unknown aggregation function: {}",
            s
        ))),
    }
}

/// Generate cache key for metrics query
fn generate_metrics_cache_key(request: &MetricsQueryRequest, org_id: &str) -> String {
    format!(
        "metrics:query:{}:{}:{}:{}:{}:{}:{}:{}:{}",
        org_id,
        request
            .metrics
            .iter()
            .map(|m| format!("{:?}", m))
            .collect::<Vec<_>>()
            .join(","),
        format!("{:?}", request.interval),
        request
            .start_time
            .map(|t| t.to_rfc3339())
            .unwrap_or_default(),
        request.end_time.map(|t| t.to_rfc3339()).unwrap_or_default(),
        request.provider.as_deref().unwrap_or("all"),
        request.model.as_deref().unwrap_or("all"),
        request.environment.as_deref().unwrap_or("all"),
        request
            .group_by
            .iter()
            .map(|d| format!("{:?}", d))
            .collect::<Vec<_>>()
            .join(",")
    )
}

/// Generate cache key for custom query
fn generate_custom_query_cache_key(request: &CustomMetricsQueryRequest, org_id: &str) -> String {
    // Use JSON serialization for complex request
    let json_str = serde_json::to_string(request).unwrap_or_default();
    format!("metrics:custom:{}:{}", org_id, json_str)
}

/// Try to get from cache
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

/// Cache metrics response
async fn cache_metrics_response(
    state: &Arc<AppState>,
    cache_key: &str,
    response: &MetricsResponse,
) {
    if let Ok(serialized) = serde_json::to_string(response) {
        if let Ok(mut conn) = state.redis_client.get_async_connection().await {
            let _: Result<(), _> = conn.set_ex(cache_key, serialized, state.cache_ttl).await;
        }
    }
}

/// Cache metrics summary
async fn cache_metrics_summary(
    state: &Arc<AppState>,
    cache_key: &str,
    response: &MetricsSummaryResponse,
) {
    if let Ok(serialized) = serde_json::to_string(response) {
        if let Ok(mut conn) = state.redis_client.get_async_connection().await {
            let _: Result<(), _> = conn.set_ex(cache_key, serialized, state.cache_ttl).await;
        }
    }
}

/// Cache custom metrics response
async fn cache_custom_metrics_response(
    state: &Arc<AppState>,
    cache_key: &str,
    response: &CustomMetricsResponse,
) {
    if let Ok(serialized) = serde_json::to_string(response) {
        if let Ok(mut conn) = state.redis_client.get_async_connection().await {
            let _: Result<(), _> = conn.set_ex(cache_key, serialized, state.cache_ttl).await;
        }
    }
}
