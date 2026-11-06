//! # Export Routes
//!
//! This module implements the export API endpoints for async data export functionality.
//!
//! ## Endpoints
//! - POST /api/v1/export/traces - Create a new export job
//! - GET /api/v1/export/jobs/:job_id - Get job status
//! - GET /api/v1/export/jobs/:job_id/download - Download completed export
//! - DELETE /api/v1/export/jobs/:job_id - Cancel a pending/processing job
//! - GET /api/v1/export/jobs - List export jobs

use crate::middleware::auth::AuthContext;
use crate::models::*;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use chrono::{Duration, Utc};
use serde::Deserialize;
use sqlx::PgPool;
use std::sync::Arc;
use tracing::{error, info, instrument};
use uuid::Uuid;

// ============================================================================
// Router Configuration
// ============================================================================

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v1/export/traces", post(create_export_job))
        .route("/api/v1/export/jobs", get(list_export_jobs))
        .route("/api/v1/export/jobs/:job_id", get(get_export_job_status))
        .route("/api/v1/export/jobs/:job_id", delete(cancel_export_job))
        .route("/api/v1/export/jobs/:job_id/download", get(download_export))
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
    Conflict(String),
    Internal(String),
    Database(sqlx::Error),
}

impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        error!("Database error: {}", err);
        ApiError::Database(err)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error, message) = match self {
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, "bad_request", msg),
            ApiError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, "unauthorized", msg),
            ApiError::Forbidden(msg) => (StatusCode::FORBIDDEN, "forbidden", msg),
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, "not_found", msg),
            ApiError::Conflict(msg) => (StatusCode::CONFLICT, "conflict", msg),
            ApiError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, "internal_error", msg),
            ApiError::Database(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "database_error",
                "A database error occurred".to_string(),
            ),
        };

        let body = Json(ErrorResponse {
            error: error.to_string(),
            message,
            details: None,
        });

        (status, body).into_response()
    }
}

// ============================================================================
// Endpoint: Create Export Job
// ============================================================================

/// Create a new export job
#[instrument(skip(state, auth))]
async fn create_export_job(
    State(state): State<Arc<AppState>>,
    auth: AuthContext,
    Json(request): Json<CreateExportRequest>,
) -> Result<Json<CreateExportResponse>, ApiError> {
    // Check permissions
    if !auth.has_permission("exports:create") {
        return Err(ApiError::Forbidden(
            "Insufficient permissions to create exports".to_string(),
        ));
    }

    // Validate request
    request.validate().map_err(ApiError::BadRequest)?;

    info!(
        "Creating export job for org_id={}, format={:?}, limit={}",
        auth.organization_id, request.format, request.limit
    );

    // Create job in database
    let job_id = Uuid::new_v4();
    let created_at = Utc::now();

    // Default expiration: 7 days from now (will be set when completed)
    let estimated_completion = created_at + Duration::minutes(5); // Estimate based on limit

    sqlx::query(
        r#"
        INSERT INTO export_jobs (
            job_id, org_id, status, format, compression,
            created_at, filter_start_time, filter_end_time,
            filter_provider, filter_model, filter_environment,
            filter_user_id, filter_status_code, filter_limit
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
        "#,
    )
    .bind(&job_id)
    .bind(&auth.organization_id)
    .bind("pending")
    .bind(request.format.to_string())
    .bind(request.compression.to_string())
    .bind(created_at)
    .bind(request.start_time)
    .bind(request.end_time)
    .bind(&request.provider)
    .bind(&request.model)
    .bind(&request.environment)
    .bind(&request.user_id)
    .bind(&request.status_code)
    .bind(request.limit)
    .execute(&state.db_pool)
    .await?;

    info!("Export job created: job_id={}", job_id);

    // In a real implementation, this would trigger an async worker
    // For now, we'll just create the job record
    // TODO: Trigger async processing (e.g., send to job queue)

    let response = CreateExportResponse {
        job_id: job_id.to_string(),
        status: ExportJobStatus::Pending,
        created_at,
        estimated_completion_at: Some(estimated_completion),
        status_url: format!("/api/v1/export/jobs/{}", job_id),
    };

    Ok(Json(response))
}

// ============================================================================
// Endpoint: List Export Jobs
// ============================================================================

#[derive(Debug, Deserialize)]
struct ListJobsQuery {
    /// Filter by status
    status: Option<String>,
    /// Number of jobs to return (max 100)
    #[serde(default = "default_list_limit")]
    limit: i32,
    /// Offset for pagination
    #[serde(default)]
    offset: i32,
}

fn default_list_limit() -> i32 {
    20
}

/// List export jobs for the authenticated organization
#[instrument(skip(state, auth))]
async fn list_export_jobs(
    State(state): State<Arc<AppState>>,
    auth: AuthContext,
    Query(query): Query<ListJobsQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Check permissions
    if !auth.has_permission("exports:read") {
        return Err(ApiError::Forbidden(
            "Insufficient permissions to read exports".to_string(),
        ));
    }

    // Validate limit
    let limit = if query.limit > 100 { 100 } else { query.limit };

    // Build query
    let mut sql = String::from(
        r#"
        SELECT
            job_id, org_id, status, format, compression,
            created_at, started_at, completed_at, expires_at,
            trace_count, file_size_bytes, NULL as file_path,
            error_message, progress_percent,
            filter_start_time, filter_end_time, filter_provider,
            filter_model, filter_environment, filter_user_id,
            filter_status_code, filter_limit
        FROM export_jobs
        WHERE org_id = $1
        "#,
    );

    let mut param_index = 2;
    if let Some(status) = &query.status {
        sql.push_str(&format!(" AND status = ${}", param_index));
        param_index += 1;
    }

    sql.push_str(" ORDER BY created_at DESC");
    sql.push_str(&format!(
        " LIMIT ${} OFFSET ${}",
        param_index,
        param_index + 1
    ));

    let mut query_builder = sqlx::query_as::<_, ExportJobRow>(&sql).bind(&auth.organization_id);

    if let Some(status) = &query.status {
        query_builder = query_builder.bind(status);
    }

    query_builder = query_builder.bind(limit).bind(query.offset);

    let jobs = query_builder.fetch_all(&state.db_pool).await?;

    // Convert to response format
    let base_url =
        std::env::var("API_BASE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());

    let job_responses: Vec<ExportJob> = jobs
        .iter()
        .map(|job| job.to_export_job(&base_url))
        .collect();

    // Get total count
    let total_count_query = if query.status.is_some() {
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM export_jobs WHERE org_id = $1 AND status = $2",
        )
        .bind(&auth.organization_id)
        .bind(query.status.as_ref().unwrap())
    } else {
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM export_jobs WHERE org_id = $1")
            .bind(&auth.organization_id)
    };

    let total = total_count_query.fetch_one(&state.db_pool).await?;

    let response = serde_json::json!({
        "jobs": job_responses,
        "pagination": {
            "total": total,
            "limit": limit,
            "offset": query.offset,
            "has_more": (query.offset + limit) < total as i32
        }
    });

    Ok(Json(response))
}

// ============================================================================
// Endpoint: Get Export Job Status
// ============================================================================

/// Get the status of a specific export job
#[instrument(skip(state, auth))]
async fn get_export_job_status(
    State(state): State<Arc<AppState>>,
    auth: AuthContext,
    Path(job_id): Path<String>,
) -> Result<Json<ExportJobStatusResponse>, ApiError> {
    // Check permissions
    if !auth.has_permission("exports:read") {
        return Err(ApiError::Forbidden(
            "Insufficient permissions to read exports".to_string(),
        ));
    }

    // Parse job ID
    let job_uuid = Uuid::parse_str(&job_id)
        .map_err(|_| ApiError::BadRequest("Invalid job ID format".to_string()))?;

    // Fetch job from database
    let job_row = sqlx::query_as::<_, ExportJobRow>(
        r#"
        SELECT
            job_id, org_id, status, format, compression,
            created_at, started_at, completed_at, expires_at,
            trace_count, file_size_bytes, file_path,
            error_message, progress_percent,
            filter_start_time, filter_end_time, filter_provider,
            filter_model, filter_environment, filter_user_id,
            filter_status_code, filter_limit
        FROM export_jobs
        WHERE job_id = $1 AND org_id = $2
        "#,
    )
    .bind(&job_uuid)
    .bind(&auth.organization_id)
    .fetch_optional(&state.db_pool)
    .await?;

    let job_row = job_row.ok_or_else(|| ApiError::NotFound("Export job not found".to_string()))?;

    let base_url =
        std::env::var("API_BASE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
    let job = job_row.to_export_job(&base_url);

    // Build metadata
    let can_cancel = matches!(
        job.status,
        ExportJobStatus::Pending | ExportJobStatus::Processing
    );
    let can_download = job.status == ExportJobStatus::Completed && !job_row.is_expired();
    let is_expired = job_row.is_expired();
    let seconds_until_expiration = job_row.seconds_until_expiration();

    let response = ExportJobStatusResponse {
        job,
        metadata: ExportJobMetadata {
            can_cancel,
            can_download,
            is_expired,
            seconds_until_expiration,
        },
    };

    Ok(Json(response))
}

// ============================================================================
// Endpoint: Cancel Export Job
// ============================================================================

/// Cancel a pending or processing export job
#[instrument(skip(state, auth))]
async fn cancel_export_job(
    State(state): State<Arc<AppState>>,
    auth: AuthContext,
    Path(job_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    // Check permissions
    if !auth.has_permission("exports:cancel") {
        return Err(ApiError::Forbidden(
            "Insufficient permissions to cancel exports".to_string(),
        ));
    }

    // Parse job ID
    let job_uuid = Uuid::parse_str(&job_id)
        .map_err(|_| ApiError::BadRequest("Invalid job ID format".to_string()))?;

    // Update job status to cancelled
    let result = sqlx::query(
        r#"
        UPDATE export_jobs
        SET status = 'cancelled'
        WHERE job_id = $1
          AND org_id = $2
          AND status IN ('pending', 'processing')
        "#,
    )
    .bind(&job_uuid)
    .bind(&auth.organization_id)
    .execute(&state.db_pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::Conflict(
            "Job cannot be cancelled (not found or already completed/cancelled)".to_string(),
        ));
    }

    info!("Export job cancelled: job_id={}", job_id);

    Ok(StatusCode::NO_CONTENT)
}

// ============================================================================
// Endpoint: Download Export
// ============================================================================

/// Download a completed export file
#[instrument(skip(state, auth))]
async fn download_export(
    State(state): State<Arc<AppState>>,
    auth: AuthContext,
    Path(job_id): Path<String>,
) -> Result<Response, ApiError> {
    // Check permissions
    if !auth.has_permission("exports:download") {
        return Err(ApiError::Forbidden(
            "Insufficient permissions to download exports".to_string(),
        ));
    }

    // Parse job ID
    let job_uuid = Uuid::parse_str(&job_id)
        .map_err(|_| ApiError::BadRequest("Invalid job ID format".to_string()))?;

    // Fetch job from database
    let job_row = sqlx::query_as::<_, ExportJobRow>(
        r#"
        SELECT
            job_id, org_id, status, format, compression,
            created_at, started_at, completed_at, expires_at,
            trace_count, file_size_bytes, file_path,
            error_message, progress_percent,
            filter_start_time, filter_end_time, filter_provider,
            filter_model, filter_environment, filter_user_id,
            filter_status_code, filter_limit
        FROM export_jobs
        WHERE job_id = $1 AND org_id = $2
        "#,
    )
    .bind(&job_uuid)
    .bind(&auth.organization_id)
    .fetch_optional(&state.db_pool)
    .await?;

    let job_row = job_row.ok_or_else(|| ApiError::NotFound("Export job not found".to_string()))?;

    // Verify job is completed
    if job_row.status != "completed" {
        return Err(ApiError::Conflict(format!(
            "Export job is not completed (status: {})",
            job_row.status
        )));
    }

    // Verify not expired
    if job_row.is_expired() {
        return Err(ApiError::Conflict(
            "Export download link has expired".to_string(),
        ));
    }

    // Get file path
    let file_path = job_row
        .file_path
        .ok_or_else(|| ApiError::Internal("Export file path not available".to_string()))?;

    // In a real implementation, this would:
    // 1. Read the file from storage (S3, local filesystem, etc.)
    // 2. Stream it to the client with appropriate headers
    // 3. Handle compression if needed
    //
    // For now, we'll return a placeholder response

    info!(
        "Export download requested: job_id={}, file_path={}",
        job_id, file_path
    );

    // Return 501 Not Implemented with helpful message
    // In production, this would stream the actual file
    Err(ApiError::Internal(
        "Export download is not yet fully implemented. The export job was created successfully, but file retrieval needs to be connected to your storage backend (S3, filesystem, etc.)".to_string(),
    ))
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Process an export job (would be called by background worker)
#[allow(dead_code)]
async fn process_export_job(pool: &PgPool, job_id: Uuid) -> Result<(), Box<dyn std::error::Error>> {
    // Update status to processing
    sqlx::query(
        r#"
        UPDATE export_jobs
        SET status = 'processing', started_at = NOW()
        WHERE job_id = $1 AND status = 'pending'
        "#,
    )
    .bind(&job_id)
    .execute(pool)
    .await?;

    // Fetch job details
    let job = sqlx::query_as::<_, ExportJobRow>(
        r#"
        SELECT
            job_id, org_id, status, format, compression,
            created_at, started_at, completed_at, expires_at,
            trace_count, file_size_bytes, file_path,
            error_message, progress_percent,
            filter_start_time, filter_end_time, filter_provider,
            filter_model, filter_environment, filter_user_id,
            filter_status_code, filter_limit
        FROM export_jobs
        WHERE job_id = $1
        "#,
    )
    .bind(&job_id)
    .fetch_one(pool)
    .await?;

    // TODO: Implement actual export logic
    // 1. Query traces based on filters
    // 2. Format data (CSV, JSON, JSONL)
    // 3. Apply compression if requested
    // 4. Save to file storage
    // 5. Update job with results

    // For now, just mark as completed
    let expires_at = Utc::now() + Duration::days(7);

    sqlx::query(
        r#"
        UPDATE export_jobs
        SET status = 'completed',
            completed_at = NOW(),
            expires_at = $2,
            trace_count = 0,
            file_size_bytes = 0,
            progress_percent = 100
        WHERE job_id = $1
        "#,
    )
    .bind(&job_id)
    .bind(expires_at)
    .execute(pool)
    .await?;

    info!("Export job completed: job_id={}", job_id);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_list_limit() {
        assert_eq!(default_list_limit(), 20);
    }

    #[test]
    fn test_job_id_parsing() {
        let valid_uuid = "550e8400-e29b-41d4-a716-446655440000";
        assert!(Uuid::parse_str(valid_uuid).is_ok());

        let invalid_uuid = "invalid-uuid";
        assert!(Uuid::parse_str(invalid_uuid).is_err());
    }
}
