///! Trace query routes
///!
///! This module implements REST API endpoints for querying and retrieving traces.
///!
///! # Endpoints
///! - `GET /api/v1/traces` - List traces with filtering and pagination
///! - `POST /api/v1/traces/search` - Advanced search with complex filters and operators
///! - `GET /api/v1/traces/:trace_id` - Get a single trace by ID
///!
///! # Authentication
///! All endpoints require authentication via JWT token or API key.
///!
///! # Rate Limiting
///! Rate limits are enforced based on user role:
///! - Admin: 100,000 req/min
///! - Developer: 10,000 req/min
///! - Viewer: 1,000 req/min
use crate::middleware::AuthContext;
use crate::models::traces::*;
use crate::models::{AdvancedSearchRequest, AppState, ErrorResponse, Filter};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use redis::AsyncCommands;
use serde_json::json;
use sqlx::{postgres::PgRow, Row};
use std::sync::Arc;
use std::time::Instant;
use tracing::{error, info, instrument, warn};

/// Create trace routes
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v1/traces", get(list_traces))
        .route("/api/v1/traces/search", post(search_traces))
        .route("/api/v1/traces/:trace_id", get(get_trace_by_id))
}

/// GET /api/v1/traces - List traces with filtering and pagination
///
/// Returns a paginated list of traces matching the specified filters.
///
/// # Query Parameters
/// - `from`: Start time (ISO 8601 or relative like "now-1h")
/// - `to`: End time (ISO 8601 or relative)
/// - `trace_id`: Filter by specific trace ID
/// - `project_id`: Filter by project (required for non-admin users)
/// - `provider`: Filter by provider (e.g., "openai")
/// - `model`: Filter by model (e.g., "gpt-4")
/// - `status`: Filter by status code
/// - `min_duration`, `max_duration`: Duration range in ms
/// - `min_cost`, `max_cost`: Cost range in USD
/// - `min_tokens`, `max_tokens`: Token count range
/// - `environment`: Filter by environment
/// - `user_id`, `session_id`: Filter by user or session
/// - `tags`: Comma-separated tags to filter
/// - `search`: Full-text search in input/output
/// - `cursor`: Pagination cursor from previous response
/// - `limit`: Results per page (default: 50, max: 1000)
/// - `sort_by`: Field to sort by (default: "ts")
/// - `sort_order`: "asc" or "desc" (default: "desc")
/// - `fields`: Comma-separated fields to include
/// - `include`: Include related data ("children", "evaluations")
///
/// # Response
/// ```json
/// {
///   "status": "success",
///   "data": [...traces...],
///   "pagination": {
///     "cursor": "base64_encoded_cursor",
///     "has_more": true,
///     "limit": 50
///   },
///   "meta": {
///     "timestamp": "2025-11-05T10:00:00Z",
///     "execution_time_ms": 45,
///     "cached": false,
///     "version": "1.0"
///   }
/// }
/// ```
#[instrument(skip(state, auth))]
async fn list_traces(
    State(state): State<Arc<AppState>>,
    auth: AuthContext,
    Query(query): Query<TraceQuery>,
) -> Result<Json<PaginatedTraceResponse>, ApiError> {
    let start_time = Instant::now();

    info!(
        user_id = %auth.user_id,
        org_id = %auth.org_id,
        role = ?auth.role,
        limit = query.limit,
        "Listing traces"
    );

    // Check permission
    if !auth.has_permission("read:traces") {
        warn!(user_id = %auth.user_id, "Insufficient permissions to read traces");
        return Err(ApiError::Forbidden(
            "Insufficient permissions to read traces".to_string(),
        ));
    }

    // Validate and enforce project access
    let project_id = auth
        .require_project_access(query.project_id.as_deref())
        .map_err(|e| ApiError::Forbidden(e.to_string()))?;

    // Validate limit
    let limit = validate_limit(query.limit)?;

    // Parse cursor if provided
    let cursor = match &query.cursor {
        Some(c) => Some(
            PaginationCursor::decode(c)
                .map_err(|e| ApiError::BadRequest(format!("Invalid cursor: {}", e)))?,
        ),
        None => None,
    };

    // Generate cache key
    let cache_key = generate_cache_key(&auth.user_id, &query, &project_id);

    // Try to get from cache (skip if cursor is present for stable pagination)
    if query.cursor.is_none() {
        if let Ok(mut redis_conn) = state.redis_client.get_async_connection().await {
            if let Ok(cached) = redis_conn.get::<_, String>(&cache_key).await {
                if let Ok(response) = serde_json::from_str::<PaginatedTraceResponse>(&cached) {
                    info!("Returning cached trace list");
                    return Ok(Json(response));
                }
            }
        }
    }

    // Build and execute query
    let traces = query_traces(&state.db_pool, &query, &project_id, cursor, limit + 1).await?;

    // Check if there are more results
    let has_more = traces.len() > limit as usize;
    let mut data = traces;
    if has_more {
        data.pop(); // Remove the extra record
    }

    // Generate next cursor
    let next_cursor = if has_more {
        data.last().map(|t| {
            PaginationCursor {
                timestamp: t.ts,
                trace_id: t.trace_id.clone(),
                span_id: t.span_id.clone(),
            }
            .encode()
        })
    } else {
        None
    };

    let execution_time = start_time.elapsed().as_millis() as u64;

    let response = PaginatedTraceResponse {
        status: ResponseStatus::Success,
        data,
        pagination: PaginationMetadata {
            cursor: next_cursor,
            has_more,
            limit,
            total: None, // Computing total is expensive, omit by default
        },
        meta: ResponseMetadata {
            timestamp: Utc::now(),
            execution_time_ms: execution_time,
            cached: false,
            version: "1.0".to_string(),
            request_id: Some(auth.request_id.clone()),
        },
    };

    // Cache the result (only first page)
    if query.cursor.is_none() {
        if let Ok(mut redis_conn) = state.redis_client.get_async_connection().await {
            let serialized = serde_json::to_string(&response).unwrap();
            let cache_ttl = determine_cache_ttl(&query);
            let _: Result<(), _> = redis_conn.set_ex(&cache_key, serialized, cache_ttl).await;
        }
    }

    info!(
        traces_returned = response.data.len(),
        has_more = has_more,
        execution_time_ms = execution_time,
        "Traces listed successfully"
    );

    Ok(Json(response))
}

/// POST /api/v1/traces/search - Advanced trace search with complex filters
///
/// Returns a paginated list of traces matching complex filter criteria.
///
/// # Request Body
/// ```json
/// {
///   "filter": {
///     "operator": "and",
///     "filters": [
///       {
///         "field": "provider",
///         "operator": "eq",
///         "value": "openai"
///       },
///       {
///         "field": "duration_ms",
///         "operator": "gte",
///         "value": 1000
///       }
///     ]
///   },
///   "sort_by": "ts",
///   "sort_desc": true,
///   "cursor": null,
///   "limit": 50,
///   "fields": ["trace_id", "model", "duration_ms"]
/// }
/// ```
///
/// # Supported Filter Operators
/// - Comparison: `eq`, `ne`, `gt`, `gte`, `lt`, `lte`
/// - Collection: `in`, `not_in`, `contains`, `not_contains`
/// - String: `starts_with`, `ends_with`, `regex`
/// - Full-text: `search`
///
/// # Logical Operators
/// - `and`: All filters must match
/// - `or`: Any filter must match
/// - `not`: Negates the filter result
///
/// # Response
/// Same format as GET /api/v1/traces
#[instrument(skip(state, auth))]
async fn search_traces(
    State(state): State<Arc<AppState>>,
    auth: AuthContext,
    Json(search_req): Json<AdvancedSearchRequest>,
) -> Result<Json<PaginatedTraceResponse>, ApiError> {
    let start_time = Instant::now();

    info!(
        user_id = %auth.user_id,
        org_id = %auth.org_id,
        role = ?auth.role,
        has_filter = search_req.filter.is_some(),
        limit = search_req.limit,
        "Advanced trace search"
    );

    // Check permission
    if !auth.has_permission("read:traces") {
        warn!(user_id = %auth.user_id, "Insufficient permissions for advanced search");
        return Err(ApiError::Forbidden(
            "Insufficient permissions to search traces".to_string(),
        ));
    }

    // Validate limit
    let limit = validate_limit(search_req.limit)?;

    // Parse cursor if provided
    let cursor = match &search_req.cursor {
        Some(c) => Some(
            PaginationCursor::decode(c)
                .map_err(|e| ApiError::BadRequest(format!("Invalid cursor: {}", e)))?,
        ),
        None => None,
    };

    // Validate sort field
    if let Some(ref sort_by) = search_req.sort_by {
        if !is_valid_sort_field(sort_by) {
            return Err(ApiError::BadRequest(format!(
                "Invalid sort field: {}",
                sort_by
            )));
        }
    }

    // Validate filter if present
    if let Some(ref filter) = search_req.filter {
        filter
            .validate()
            .map_err(|e| ApiError::BadRequest(format!("Invalid filter: {}", e)))?;
    }

    // Generate cache key
    let cache_key = generate_search_cache_key(&auth.user_id, &search_req);

    // Try to get from cache (skip if cursor is present)
    if search_req.cursor.is_none() {
        if let Ok(mut redis_conn) = state.redis_client.get_async_connection().await {
            if let Ok(cached) = redis_conn.get::<_, String>(&cache_key).await {
                if let Ok(response) = serde_json::from_str::<PaginatedTraceResponse>(&cached) {
                    info!("Returning cached search results");
                    return Ok(Json(response));
                }
            }
        }
    }

    // Build and execute query
    let traces =
        execute_advanced_search(&state.db_pool, &search_req, &auth.org_id, cursor, limit + 1)
            .await?;

    // Check if there are more results
    let has_more = traces.len() > limit as usize;
    let mut data = traces;
    if has_more {
        data.pop(); // Remove the extra record
    }

    // Generate next cursor
    let next_cursor = if has_more {
        data.last().map(|t| {
            PaginationCursor {
                timestamp: t.ts,
                trace_id: t.trace_id.clone(),
                span_id: t.span_id.clone(),
            }
            .encode()
        })
    } else {
        None
    };

    let execution_time = start_time.elapsed().as_millis() as u64;

    let response = PaginatedTraceResponse {
        status: ResponseStatus::Success,
        data,
        pagination: PaginationMetadata {
            cursor: next_cursor,
            has_more,
            limit,
            total: None,
        },
        meta: ResponseMetadata {
            timestamp: Utc::now(),
            execution_time_ms: execution_time,
            cached: false,
            version: "1.0".to_string(),
            request_id: Some(auth.request_id.clone()),
        },
    };

    // Cache the result (only first page)
    if search_req.cursor.is_none() {
        if let Ok(mut redis_conn) = state.redis_client.get_async_connection().await {
            let serialized = serde_json::to_string(&response).unwrap();
            let _: Result<(), _> = redis_conn
                .set_ex(&cache_key, serialized, state.cache_ttl)
                .await;
        }
    }

    info!(
        traces_returned = response.data.len(),
        has_more = has_more,
        execution_time_ms = execution_time,
        "Advanced search completed successfully"
    );

    Ok(Json(response))
}

/// GET /api/v1/traces/:trace_id - Get a single trace by ID
///
/// Returns a single trace with all its details.
///
/// # Path Parameters
/// - `trace_id`: The trace ID to retrieve
///
/// # Query Parameters
/// - `include`: Include related data ("children", "evaluations")
///
/// # Response
/// ```json
/// {
///   "status": "success",
///   "data": {...trace...},
///   "meta": {
///     "timestamp": "2025-11-05T10:00:00Z",
///     "execution_time_ms": 12,
///     "cached": true,
///     "version": "1.0"
///   }
/// }
/// ```
#[instrument(skip(state, auth))]
async fn get_trace_by_id(
    State(state): State<Arc<AppState>>,
    auth: AuthContext,
    Path(trace_id): Path<String>,
) -> Result<Json<SingleTraceResponse>, ApiError> {
    let start_time = Instant::now();

    info!(
        user_id = %auth.user_id,
        trace_id = %trace_id,
        "Getting trace by ID"
    );

    // Check permission
    if !auth.has_permission("read:traces") {
        return Err(ApiError::Forbidden(
            "Insufficient permissions to read traces".to_string(),
        ));
    }

    // Generate cache key
    let cache_key = format!("trace:{}:{}", auth.user_id, trace_id);

    // Try to get from cache
    if let Ok(mut redis_conn) = state.redis_client.get_async_connection().await {
        if let Ok(cached) = redis_conn.get::<_, String>(&cache_key).await {
            if let Ok(response) = serde_json::from_str::<SingleTraceResponse>(&cached) {
                info!("Returning cached trace");
                return Ok(Json(response));
            }
        }
    }

    // Query database
    let trace = sqlx::query_as::<_, Trace>(
        r#"
        SELECT
            ts, trace_id, span_id, parent_span_id,
            service_name, span_name,
            provider, model,
            input_text, output_text,
            prompt_tokens, completion_tokens, total_tokens,
            prompt_cost_usd, completion_cost_usd, total_cost_usd,
            duration_ms, ttft_ms,
            status_code, error_message,
            user_id, session_id, environment,
            tags, attributes
        FROM llm_traces
        WHERE trace_id = $1
        ORDER BY ts DESC
        LIMIT 1
        "#,
    )
    .bind(&trace_id)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| {
        error!("Database query error: {}", e);
        ApiError::Internal(format!("Failed to fetch trace: {}", e))
    })?;

    let mut trace = trace.ok_or_else(|| {
        warn!(trace_id = %trace_id, "Trace not found");
        ApiError::NotFound(format!("Trace with ID '{}' not found", trace_id))
    })?;

    // Check project access
    // Note: We need to extract project_id from trace attributes if present
    // For now, we'll allow if user has read:traces permission
    // In production, implement proper project-level authorization

    // Fill in calculated fields
    trace.calculate_total_cost();
    trace.calculate_total_tokens();

    let execution_time = start_time.elapsed().as_millis() as u64;

    let response = SingleTraceResponse {
        status: ResponseStatus::Success,
        data: trace,
        meta: ResponseMetadata {
            timestamp: Utc::now(),
            execution_time_ms: execution_time,
            cached: false,
            version: "1.0".to_string(),
            request_id: Some(auth.request_id.clone()),
        },
    };

    // Cache the result
    if let Ok(mut redis_conn) = state.redis_client.get_async_connection().await {
        let serialized = serde_json::to_string(&response).unwrap();
        let _: Result<(), _> = redis_conn.set_ex(&cache_key, serialized, 300).await;
        // 5 min TTL
    }

    info!(
        trace_id = %trace_id,
        execution_time_ms = execution_time,
        "Trace retrieved successfully"
    );

    Ok(Json(response))
}

/// Query traces from database with filters
async fn query_traces(
    pool: &sqlx::PgPool,
    query: &TraceQuery,
    project_id: &str,
    cursor: Option<PaginationCursor>,
    limit: i32,
) -> Result<Vec<Trace>, ApiError> {
    let mut sql = String::from(
        r#"
        SELECT
            ts, trace_id, span_id, parent_span_id,
            service_name, span_name,
            provider, model,
            input_text, output_text,
            prompt_tokens, completion_tokens, total_tokens,
            prompt_cost_usd, completion_cost_usd, total_cost_usd,
            duration_ms, ttft_ms,
            status_code, error_message,
            user_id, session_id, environment,
            tags, attributes
        FROM llm_traces
        WHERE 1=1
        "#,
    );

    let mut bind_index = 1;
    let mut bindings: Vec<Box<dyn sqlx::Encode<'_, sqlx::Postgres> + Send + Sync>> = Vec::new();

    // Time range filters
    if let Some(from) = &query.from {
        sql.push_str(&format!(" AND ts >= ${}", bind_index));
        bind_index += 1;
    }

    if let Some(to) = &query.to {
        sql.push_str(&format!(" AND ts <= ${}", bind_index));
        bind_index += 1;
    }

    // Cursor-based pagination
    if let Some(cursor) = &cursor {
        sql.push_str(&format!(
            " AND (ts, trace_id, span_id) < (${}, ${}, ${})",
            bind_index,
            bind_index + 1,
            bind_index + 2
        ));
        bind_index += 3;
    }

    // Provider filter
    if let Some(provider) = &query.provider {
        sql.push_str(&format!(" AND provider = ${}", bind_index));
        bind_index += 1;
    }

    // Model filter
    if let Some(model) = &query.model {
        sql.push_str(&format!(" AND model = ${}", bind_index));
        bind_index += 1;
    }

    // Status filter
    if let Some(status) = &query.status {
        sql.push_str(&format!(" AND status_code = ${}", bind_index));
        bind_index += 1;
    }

    // Duration filters
    if let Some(min_duration) = query.min_duration {
        sql.push_str(&format!(" AND duration_ms >= ${}", bind_index));
        bind_index += 1;
    }

    if let Some(max_duration) = query.max_duration {
        sql.push_str(&format!(" AND duration_ms <= ${}", bind_index));
        bind_index += 1;
    }

    // Cost filters
    if let Some(min_cost) = query.min_cost {
        sql.push_str(&format!(" AND total_cost_usd >= ${}", bind_index));
        bind_index += 1;
    }

    if let Some(max_cost) = query.max_cost {
        sql.push_str(&format!(" AND total_cost_usd <= ${}", bind_index));
        bind_index += 1;
    }

    // Environment filter
    if let Some(environment) = &query.environment {
        sql.push_str(&format!(" AND environment = ${}", bind_index));
        bind_index += 1;
    }

    // User ID filter
    if let Some(user_id) = &query.user_id {
        sql.push_str(&format!(" AND user_id = ${}", bind_index));
        bind_index += 1;
    }

    // Session ID filter
    if let Some(session_id) = &query.session_id {
        sql.push_str(&format!(" AND session_id = ${}", bind_index));
        bind_index += 1;
    }

    // Project ID filter (always applied for non-admin users)
    if !project_id.is_empty() {
        sql.push_str(&format!(" AND attributes->>'project_id' = ${}", bind_index));
        bind_index += 1;
    }

    // Full-text search
    if let Some(search) = &query.search {
        sql.push_str(&format!(
            " AND (input_text ILIKE ${} OR output_text ILIKE ${})",
            bind_index,
            bind_index + 1
        ));
        bind_index += 2;
    }

    // Order by
    let sort_by = query.sort_by.as_deref().unwrap_or("ts");
    let sort_order = match query.sort_order {
        Some(SortOrder::Asc) => "ASC",
        _ => "DESC",
    };

    sql.push_str(&format!(" ORDER BY {} {}", sort_by, sort_order));

    // Always add secondary sort for stable pagination
    if sort_by != "ts" {
        sql.push_str(&format!(", ts {}", sort_order));
    }
    if sort_by != "trace_id" && sort_by != "span_id" {
        sql.push_str(&format!(
            ", trace_id {}, span_id {}",
            sort_order, sort_order
        ));
    }

    // Limit
    sql.push_str(&format!(" LIMIT ${}", bind_index));

    // Build query with bindings
    let mut sqlx_query = sqlx::query_as::<_, Trace>(&sql);

    // Bind parameters in order
    if let Some(from) = &query.from {
        sqlx_query = sqlx_query.bind(from);
    }

    if let Some(to) = &query.to {
        sqlx_query = sqlx_query.bind(to);
    }

    if let Some(cursor) = &cursor {
        sqlx_query = sqlx_query
            .bind(&cursor.timestamp)
            .bind(&cursor.trace_id)
            .bind(&cursor.span_id);
    }

    if let Some(provider) = &query.provider {
        sqlx_query = sqlx_query.bind(provider);
    }

    if let Some(model) = &query.model {
        sqlx_query = sqlx_query.bind(model);
    }

    if let Some(status) = &query.status {
        sqlx_query = sqlx_query.bind(status);
    }

    if let Some(min_duration) = query.min_duration {
        sqlx_query = sqlx_query.bind(min_duration);
    }

    if let Some(max_duration) = query.max_duration {
        sqlx_query = sqlx_query.bind(max_duration);
    }

    if let Some(min_cost) = query.min_cost {
        sqlx_query = sqlx_query.bind(min_cost);
    }

    if let Some(max_cost) = query.max_cost {
        sqlx_query = sqlx_query.bind(max_cost);
    }

    if let Some(environment) = &query.environment {
        sqlx_query = sqlx_query.bind(environment);
    }

    if let Some(user_id) = &query.user_id {
        sqlx_query = sqlx_query.bind(user_id);
    }

    if let Some(session_id) = &query.session_id {
        sqlx_query = sqlx_query.bind(session_id);
    }

    if !project_id.is_empty() {
        sqlx_query = sqlx_query.bind(project_id);
    }

    // Create search pattern before using it (lifetime issue)
    let search_pattern = query.search.as_ref().map(|s| format!("%{}%", s));
    if let Some(ref pattern) = search_pattern {
        sqlx_query = sqlx_query.bind(pattern).bind(pattern);
    }

    sqlx_query = sqlx_query.bind(limit);

    // Execute query
    let mut traces = sqlx_query.fetch_all(pool).await.map_err(|e| {
        error!("Database query error: {}", e);
        ApiError::Internal(format!("Failed to fetch traces: {}", e))
    })?;

    // Calculate derived fields
    for trace in &mut traces {
        trace.calculate_total_cost();
        trace.calculate_total_tokens();
    }

    Ok(traces)
}

/// Validate and clamp limit
fn validate_limit(limit: i32) -> Result<i32, ApiError> {
    if limit < 1 {
        return Err(ApiError::BadRequest("Limit must be at least 1".to_string()));
    }

    if limit > 1000 {
        return Err(ApiError::BadRequest("Limit cannot exceed 1000".to_string()));
    }

    Ok(limit)
}

/// Generate cache key for trace list query
fn generate_cache_key(user_id: &str, query: &TraceQuery, project_id: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();

    user_id.hash(&mut hasher);
    project_id.hash(&mut hasher);
    query.from.hash(&mut hasher);
    query.to.hash(&mut hasher);
    query.provider.hash(&mut hasher);
    query.model.hash(&mut hasher);
    query.status.hash(&mut hasher);
    query.min_duration.hash(&mut hasher);
    query.max_duration.hash(&mut hasher);
    query.environment.hash(&mut hasher);
    query.limit.hash(&mut hasher);

    let hash = hasher.finish();
    format!("traces:list:{:x}", hash)
}

/// Determine cache TTL based on query parameters
fn determine_cache_ttl(query: &TraceQuery) -> u64 {
    // If querying recent data (no 'to' or 'to' is recent), use shorter TTL
    if query.to.is_none()
        || query
            .to
            .map(|t| (Utc::now() - t).num_hours() < 1)
            .unwrap_or(false)
    {
        60 // 1 minute for recent data
    } else {
        300 // 5 minutes for historical data
    }
}

/// Execute advanced search with complex filters
async fn execute_advanced_search(
    pool: &sqlx::PgPool,
    search_req: &AdvancedSearchRequest,
    org_id: &str,
    cursor: Option<PaginationCursor>,
    limit: i32,
) -> Result<Vec<Trace>, ApiError> {
    // Base SELECT clause with field selection
    let select_fields = if let Some(ref fields) = search_req.fields {
        // Validate and filter requested fields
        let valid_fields: Vec<&str> = fields
            .iter()
            .filter(|f| is_valid_trace_field(f))
            .map(|s| s.as_str())
            .collect();

        if valid_fields.is_empty() {
            // If no valid fields, use all fields
            "*".to_string()
        } else {
            // Always include required fields for pagination
            let mut field_set: std::collections::HashSet<&str> = valid_fields.into_iter().collect();
            field_set.insert("ts");
            field_set.insert("trace_id");
            field_set.insert("span_id");

            field_set.into_iter().collect::<Vec<_>>().join(", ")
        }
    } else {
        "*".to_string()
    };

    let mut sql = format!("SELECT {} FROM llm_traces WHERE 1=1", select_fields);

    let mut param_index = 1;
    let mut params: Vec<String> = Vec::new();

    // Add organization filter
    if !org_id.is_empty() {
        sql.push_str(&format!(" AND attributes->>'org_id' = ${}", param_index));
        params.push(org_id.to_string());
        param_index += 1;
    }

    // Add cursor pagination
    if let Some(ref cursor) = cursor {
        sql.push_str(&format!(
            " AND (ts, trace_id, span_id) < (${}, ${}, ${})",
            param_index,
            param_index + 1,
            param_index + 2
        ));
        params.push(cursor.timestamp.to_rfc3339());
        params.push(cursor.trace_id.clone());
        params.push(cursor.span_id.clone());
        param_index += 3;
    }

    // Add advanced filters
    if let Some(ref filter) = search_req.filter {
        let (filter_sql, filter_params) = filter
            .to_sql(&mut param_index)
            .map_err(|e| ApiError::BadRequest(format!("Filter error: {}", e)))?;

        sql.push_str(&format!(" AND ({})", filter_sql));
        params.extend(filter_params);
    }

    // Add sorting
    let sort_by = search_req.sort_by.as_deref().unwrap_or("ts");
    let sort_order = if search_req.sort_desc { "DESC" } else { "ASC" };

    sql.push_str(&format!(" ORDER BY {} {}", sort_by, sort_order));

    // Always add secondary sort for stable pagination
    if sort_by != "ts" {
        sql.push_str(&format!(", ts {}", sort_order));
    }
    if sort_by != "trace_id" && sort_by != "span_id" {
        sql.push_str(&format!(
            ", trace_id {}, span_id {}",
            sort_order, sort_order
        ));
    }

    // Add limit
    sql.push_str(&format!(" LIMIT ${}", param_index));
    params.push(limit.to_string());

    info!(
        sql = %sql,
        param_count = params.len(),
        "Executing advanced search query"
    );

    // Build and execute query
    let mut query = sqlx::query_as::<_, Trace>(&sql);

    // Bind all parameters
    for param in params {
        query = query.bind(param);
    }

    let mut traces = query.fetch_all(pool).await.map_err(|e| {
        error!("Advanced search query error: {}", e);
        ApiError::Internal(format!("Failed to execute search: {}", e))
    })?;

    // Calculate derived fields
    for trace in &mut traces {
        trace.calculate_total_cost();
        trace.calculate_total_tokens();
    }

    Ok(traces)
}

/// Validate that a field is a valid trace field
fn is_valid_trace_field(field: &str) -> bool {
    matches!(
        field,
        "ts" | "trace_id"
            | "span_id"
            | "parent_span_id"
            | "service_name"
            | "span_name"
            | "provider"
            | "model"
            | "input_text"
            | "output_text"
            | "prompt_tokens"
            | "completion_tokens"
            | "total_tokens"
            | "prompt_cost_usd"
            | "completion_cost_usd"
            | "total_cost_usd"
            | "duration_ms"
            | "ttft_ms"
            | "status_code"
            | "error_message"
            | "user_id"
            | "session_id"
            | "environment"
            | "tags"
            | "attributes"
    )
}

/// Validate that a field is a valid sort field
fn is_valid_sort_field(field: &str) -> bool {
    matches!(
        field,
        "ts" | "trace_id"
            | "span_id"
            | "provider"
            | "model"
            | "duration_ms"
            | "ttft_ms"
            | "total_cost_usd"
            | "total_tokens"
            | "status_code"
            | "environment"
    )
}

/// Generate cache key for advanced search
fn generate_search_cache_key(user_id: &str, search_req: &AdvancedSearchRequest) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();

    user_id.hash(&mut hasher);

    // Hash the filter if present
    if let Some(ref filter) = search_req.filter {
        // Serialize filter to JSON for consistent hashing
        if let Ok(filter_json) = serde_json::to_string(filter) {
            filter_json.hash(&mut hasher);
        }
    }

    search_req.sort_by.hash(&mut hasher);
    search_req.sort_desc.hash(&mut hasher);
    search_req.limit.hash(&mut hasher);
    search_req.fields.hash(&mut hasher);

    let hash = hasher.finish();
    format!("traces:search:{:x}", hash)
}

/// API error type
#[derive(Debug)]
pub enum ApiError {
    BadRequest(String),
    NotFound(String),
    Forbidden(String),
    Internal(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_code, message) = match self {
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, "BAD_REQUEST", msg),
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, "NOT_FOUND", msg),
            ApiError::Forbidden(msg) => (StatusCode::FORBIDDEN, "FORBIDDEN", msg),
            ApiError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", msg),
        };

        let body = Json(json!({
            "error": {
                "code": error_code,
                "message": message,
            },
            "meta": {
                "timestamp": Utc::now().to_rfc3339(),
            }
        }));

        (status, body).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{FieldFilter, FilterOperator, FilterValue, LogicalOperator};

    #[test]
    fn test_validate_limit() {
        assert!(validate_limit(1).is_ok());
        assert!(validate_limit(50).is_ok());
        assert!(validate_limit(1000).is_ok());

        assert!(validate_limit(0).is_err());
        assert!(validate_limit(-1).is_err());
        assert!(validate_limit(1001).is_err());
    }

    #[test]
    fn test_determine_cache_ttl() {
        // Query with no end time (recent data)
        let query = TraceQuery {
            to: None,
            ..Default::default()
        };
        assert_eq!(determine_cache_ttl(&query), 60);

        // Query with recent end time
        let recent_query = TraceQuery {
            to: Some(Utc::now() - chrono::Duration::minutes(30)),
            ..Default::default()
        };
        assert_eq!(determine_cache_ttl(&recent_query), 60);

        // Query with old end time
        let old_query = TraceQuery {
            to: Some(Utc::now() - chrono::Duration::days(7)),
            ..Default::default()
        };
        assert_eq!(determine_cache_ttl(&old_query), 300);
    }

    #[test]
    fn test_is_valid_trace_field() {
        // Valid fields
        assert!(is_valid_trace_field("ts"));
        assert!(is_valid_trace_field("trace_id"));
        assert!(is_valid_trace_field("provider"));
        assert!(is_valid_trace_field("model"));
        assert!(is_valid_trace_field("duration_ms"));
        assert!(is_valid_trace_field("total_cost_usd"));

        // Invalid fields (SQL injection attempts)
        assert!(!is_valid_trace_field("DROP TABLE"));
        assert!(!is_valid_trace_field("1; DELETE FROM llm_traces"));
        assert!(!is_valid_trace_field("invalid_field"));
        assert!(!is_valid_trace_field(""));
    }

    #[test]
    fn test_is_valid_sort_field() {
        // Valid sort fields
        assert!(is_valid_sort_field("ts"));
        assert!(is_valid_sort_field("duration_ms"));
        assert!(is_valid_sort_field("total_cost_usd"));
        assert!(is_valid_sort_field("provider"));

        // Invalid sort fields (not sortable or injection attempts)
        assert!(!is_valid_sort_field("input_text")); // Not sortable
        assert!(!is_valid_sort_field("output_text")); // Not sortable
        assert!(!is_valid_sort_field("DROP TABLE"));
        assert!(!is_valid_sort_field(""));
    }

    #[test]
    fn test_generate_search_cache_key() {
        // Same request should generate same key
        let search_req1 = AdvancedSearchRequest {
            filter: Some(Filter::Field(FieldFilter {
                field: "provider".to_string(),
                operator: FilterOperator::Eq,
                value: FilterValue::String("openai".to_string()),
            })),
            sort_by: Some("ts".to_string()),
            sort_desc: true,
            cursor: None,
            limit: 50,
            fields: None,
        };

        let search_req2 = AdvancedSearchRequest {
            filter: Some(Filter::Field(FieldFilter {
                field: "provider".to_string(),
                operator: FilterOperator::Eq,
                value: FilterValue::String("openai".to_string()),
            })),
            sort_by: Some("ts".to_string()),
            sort_desc: true,
            cursor: None,
            limit: 50,
            fields: None,
        };

        let key1 = generate_search_cache_key("user123", &search_req1);
        let key2 = generate_search_cache_key("user123", &search_req2);
        assert_eq!(key1, key2);

        // Different request should generate different key
        let search_req3 = AdvancedSearchRequest {
            filter: Some(Filter::Field(FieldFilter {
                field: "provider".to_string(),
                operator: FilterOperator::Eq,
                value: FilterValue::String("anthropic".to_string()),
            })),
            sort_by: Some("ts".to_string()),
            sort_desc: true,
            cursor: None,
            limit: 50,
            fields: None,
        };

        let key3 = generate_search_cache_key("user123", &search_req3);
        assert_ne!(key1, key3);

        // Different user should generate different key
        let key4 = generate_search_cache_key("user456", &search_req1);
        assert_ne!(key1, key4);
    }

    #[test]
    fn test_advanced_search_request_validation() {
        // Test with valid filter
        let valid_filter = Filter::Field(FieldFilter {
            field: "provider".to_string(),
            operator: FilterOperator::Eq,
            value: FilterValue::String("openai".to_string()),
        });
        assert!(valid_filter.validate().is_ok());

        // Test with logical operator
        let logical_filter = Filter::Logical {
            operator: LogicalOperator::And,
            filters: vec![
                Filter::Field(FieldFilter {
                    field: "provider".to_string(),
                    operator: FilterOperator::Eq,
                    value: FilterValue::String("openai".to_string()),
                }),
                Filter::Field(FieldFilter {
                    field: "duration_ms".to_string(),
                    operator: FilterOperator::Gte,
                    value: FilterValue::Int(1000),
                }),
            ],
        };
        assert!(logical_filter.validate().is_ok());

        // Test with invalid field name
        let invalid_filter = Filter::Field(FieldFilter {
            field: "DROP TABLE".to_string(),
            operator: FilterOperator::Eq,
            value: FilterValue::String("test".to_string()),
        });
        assert!(invalid_filter.validate().is_err());
    }
}
