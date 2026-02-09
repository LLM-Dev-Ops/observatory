// Copyright 2025 LLM Observatory Contributors
// SPDX-License-Identifier: Apache-2.0

//! Execution context middleware for the Analytics API.
//!
//! Extracts agentic execution context from HTTP headers, creates a repo-level
//! execution span, and injects the [`ExecutionContext`] into request extensions.
//! Enforces that every externally-invoked operation has valid execution context
//! when running in enforcing mode.

use axum::{
    extract::{FromRequestParts, Request},
    http::{request::Parts, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use chrono::Utc;
use llm_observatory_core::execution::{
    headers, ExecutionContext, ExecutionSpan, ExecutionSpanKind, ExecutionSpanStatus,
};
use serde_json::json;
use tracing::{info, warn};
use uuid::Uuid;

/// Configuration for the execution context middleware.
#[derive(Debug, Clone)]
pub struct ExecutionMiddlewareConfig {
    /// The repository name to use for repo-level spans.
    pub repo_name: String,
    /// Whether to enforce execution context (reject requests without it).
    /// Set to `false` for gradual rollout / backwards compatibility.
    pub enforce: bool,
}

impl ExecutionMiddlewareConfig {
    /// Create a new config in enforcing mode.
    pub fn new(repo_name: impl Into<String>) -> Self {
        Self {
            repo_name: repo_name.into(),
            enforce: true,
        }
    }

    /// Create a new config in permissive mode (does not reject requests
    /// missing execution headers).
    pub fn permissive(repo_name: impl Into<String>) -> Self {
        Self {
            repo_name: repo_name.into(),
            enforce: false,
        }
    }
}

/// Execution context error response.
#[derive(Debug)]
pub struct ExecutionError {
    pub status: StatusCode,
    pub code: &'static str,
    pub message: String,
}

impl IntoResponse for ExecutionError {
    fn into_response(self) -> Response {
        let body = Json(json!({
            "error": {
                "code": self.code,
                "message": self.message,
            },
            "meta": {
                "timestamp": Utc::now().to_rfc3339(),
            }
        }));
        (self.status, body).into_response()
    }
}

/// Middleware function that extracts execution context from HTTP headers
/// and creates a repo-level execution span.
///
/// # Headers
///
/// - `x-execution-id` (required in enforce mode): The top-level execution ID.
/// - `x-execution-parent-span-id` (required in enforce mode): The caller's span ID.
/// - `x-execution-repo-name` (optional): Override the configured repo name.
///
/// When valid headers are present, this middleware injects both an
/// [`ExecutionContext`] and an [`ExecutionSpan`] (repo-level) into the
/// request extensions. Route handlers can extract these via the
/// `FromRequestParts` impl on `ExecutionContext`.
pub async fn execution_context_middleware(
    config: ExecutionMiddlewareConfig,
    mut req: Request,
    next: Next,
) -> Result<Response, ExecutionError> {
    let execution_id = req
        .headers()
        .get(headers::X_EXECUTION_ID)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let parent_span_id = req
        .headers()
        .get(headers::X_EXECUTION_PARENT_SPAN_ID)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let repo_name_override = req
        .headers()
        .get(headers::X_EXECUTION_REPO_NAME)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    if config.enforce {
        let exec_id = execution_id.ok_or_else(|| ExecutionError {
            status: StatusCode::BAD_REQUEST,
            code: "MISSING_EXECUTION_ID",
            message: format!(
                "Header '{}' is required for all operations",
                headers::X_EXECUTION_ID
            ),
        })?;

        let parent_id = parent_span_id.ok_or_else(|| ExecutionError {
            status: StatusCode::BAD_REQUEST,
            code: "MISSING_PARENT_SPAN_ID",
            message: format!(
                "Header '{}' is required for all operations",
                headers::X_EXECUTION_PARENT_SPAN_ID
            ),
        })?;

        let repo_name = repo_name_override.unwrap_or_else(|| config.repo_name.clone());
        let repo_span_id = Uuid::new_v4().to_string();

        let repo_span = ExecutionSpan::builder()
            .span_id(repo_span_id.clone())
            .execution_id(exec_id.clone())
            .parent_span_id(parent_id.clone())
            .kind(ExecutionSpanKind::Repo)
            .repo_name(repo_name.clone())
            .status(ExecutionSpanStatus::Running)
            .build()
            .map_err(|e| ExecutionError {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                code: "EXECUTION_SPAN_CREATION_FAILED",
                message: format!("Failed to create repo span: {}", e),
            })?;

        let ctx = ExecutionContext {
            execution_id: exec_id.clone(),
            parent_span_id: parent_id,
            repo_span_id: Some(repo_span_id),
            repo_name,
        };

        info!(
            execution_id = %ctx.execution_id,
            repo_span_id = ?ctx.repo_span_id,
            "Execution context established (enforced)"
        );

        req.extensions_mut().insert(ctx);
        req.extensions_mut().insert(repo_span);
    } else if let (Some(exec_id), Some(parent_id)) = (&execution_id, &parent_span_id) {
        // Permissive mode: create context when headers are present
        let repo_name = repo_name_override.unwrap_or_else(|| config.repo_name.clone());
        let repo_span_id = Uuid::new_v4().to_string();

        if let Ok(repo_span) = ExecutionSpan::builder()
            .span_id(repo_span_id.clone())
            .execution_id(exec_id.clone())
            .parent_span_id(parent_id.clone())
            .kind(ExecutionSpanKind::Repo)
            .repo_name(repo_name.clone())
            .build()
        {
            let ctx = ExecutionContext {
                execution_id: exec_id.clone(),
                parent_span_id: parent_id.clone(),
                repo_span_id: Some(repo_span_id),
                repo_name,
            };

            info!(
                execution_id = %ctx.execution_id,
                "Execution context established (permissive)"
            );

            req.extensions_mut().insert(ctx);
            req.extensions_mut().insert(repo_span);
        }
    } else {
        // No execution headers in permissive mode
        warn!("No execution context headers found (permissive mode, proceeding without context)");
    }

    Ok(next.run(req).await)
}

/// Newtype wrapper for extracting [`ExecutionContext`] from request parts.
///
/// This wrapper exists because Rust's orphan rule prevents implementing
/// `FromRequestParts` (axum trait) directly on `ExecutionContext` (core crate type).
///
/// Use this in route handlers to access the execution context:
///
/// ```ignore
/// async fn my_handler(
///     ReqExecutionContext(exec_ctx): ReqExecutionContext,
///     // ...
/// ) -> impl IntoResponse {
///     // exec_ctx.execution_id, exec_ctx.repo_span_id, etc.
/// }
/// ```
#[derive(Debug, Clone)]
pub struct ReqExecutionContext(pub ExecutionContext);

impl std::ops::Deref for ReqExecutionContext {
    type Target = ExecutionContext;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[async_trait::async_trait]
impl<S> FromRequestParts<S> for ReqExecutionContext
where
    S: Send + Sync,
{
    type Rejection = ExecutionError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<ExecutionContext>()
            .cloned()
            .map(ReqExecutionContext)
            .ok_or(ExecutionError {
                status: StatusCode::BAD_REQUEST,
                code: "MISSING_EXECUTION_CONTEXT",
                message: "Execution context not found. Ensure execution middleware is applied \
                          and x-execution-id / x-execution-parent-span-id headers are provided."
                    .to_string(),
            })
    }
}
