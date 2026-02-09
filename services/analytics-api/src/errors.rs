///! Standardized error handling with error code catalog
///!
///! This module provides a comprehensive error handling system with:
///! - Consistent error codes across the API
///! - Detailed error messages with context
///! - HTTP status code mapping
///! - Error categories for better client handling
///! - Structured error responses

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Error category for classification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCategory {
    /// Authentication errors (401)
    Authentication,
    /// Authorization errors (403)
    Authorization,
    /// Validation errors (400)
    Validation,
    /// Resource not found (404)
    NotFound,
    /// Conflict errors (409)
    Conflict,
    /// Rate limiting (429)
    RateLimit,
    /// Server errors (500)
    Internal,
    /// Database errors (503)
    Database,
    /// External service errors (502/503)
    External,
    /// Request timeout (504)
    Timeout,
}

/// Standard error code catalog
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    // Authentication errors (1000-1099)
    MissingAuth,
    InvalidToken,
    ExpiredToken,
    InvalidApiKey,

    // Authorization errors (1100-1199)
    InsufficientPermissions,
    ProjectAccessDenied,
    ResourceAccessDenied,

    // Validation errors (1200-1299)
    InvalidRequest,
    MissingRequiredField,
    InvalidFieldValue,
    InvalidFieldFormat,
    InvalidDateRange,
    LimitExceeded,

    // Execution context errors (1250-1252)
    MissingExecutionId,
    MissingParentSpanId,
    ExecutionValidationFailed,

    // Resource errors (1300-1399)
    ResourceNotFound,
    TraceNotFound,
    JobNotFound,
    ExportNotFound,

    // Conflict errors (1400-1499)
    ResourceAlreadyExists,
    DuplicateKey,
    ConcurrencyConflict,

    // Rate limiting (1500-1599)
    RateLimitExceeded,
    QuotaExceeded,
    TooManyRequests,

    // Database errors (1600-1699)
    DatabaseError,
    DatabaseConnectionFailed,
    QueryTimeout,
    DatabaseConstraintViolation,

    // External service errors (1700-1799)
    ExternalServiceError,
    RedisError,
    StorageError,

    // Timeout errors (1800-1899)
    RequestTimeout,
    OperationTimeout,

    // Internal errors (1900-1999)
    InternalServerError,
    NotImplemented,
    ServiceUnavailable,
}

impl ErrorCode {
    /// Get the numeric error code
    pub fn code(&self) -> u32 {
        match self {
            // Authentication (1000-1099)
            ErrorCode::MissingAuth => 1000,
            ErrorCode::InvalidToken => 1001,
            ErrorCode::ExpiredToken => 1002,
            ErrorCode::InvalidApiKey => 1003,

            // Authorization (1100-1199)
            ErrorCode::InsufficientPermissions => 1100,
            ErrorCode::ProjectAccessDenied => 1101,
            ErrorCode::ResourceAccessDenied => 1102,

            // Validation (1200-1299)
            ErrorCode::InvalidRequest => 1200,
            ErrorCode::MissingRequiredField => 1201,
            ErrorCode::InvalidFieldValue => 1202,
            ErrorCode::InvalidFieldFormat => 1203,
            ErrorCode::InvalidDateRange => 1204,
            ErrorCode::LimitExceeded => 1205,
            ErrorCode::MissingExecutionId => 1250,
            ErrorCode::MissingParentSpanId => 1251,
            ErrorCode::ExecutionValidationFailed => 1252,

            // Resources (1300-1399)
            ErrorCode::ResourceNotFound => 1300,
            ErrorCode::TraceNotFound => 1301,
            ErrorCode::JobNotFound => 1302,
            ErrorCode::ExportNotFound => 1303,

            // Conflicts (1400-1499)
            ErrorCode::ResourceAlreadyExists => 1400,
            ErrorCode::DuplicateKey => 1401,
            ErrorCode::ConcurrencyConflict => 1402,

            // Rate limiting (1500-1599)
            ErrorCode::RateLimitExceeded => 1500,
            ErrorCode::QuotaExceeded => 1501,
            ErrorCode::TooManyRequests => 1502,

            // Database (1600-1699)
            ErrorCode::DatabaseError => 1600,
            ErrorCode::DatabaseConnectionFailed => 1601,
            ErrorCode::QueryTimeout => 1602,
            ErrorCode::DatabaseConstraintViolation => 1603,

            // External (1700-1799)
            ErrorCode::ExternalServiceError => 1700,
            ErrorCode::RedisError => 1701,
            ErrorCode::StorageError => 1702,

            // Timeout (1800-1899)
            ErrorCode::RequestTimeout => 1800,
            ErrorCode::OperationTimeout => 1801,

            // Internal (1900-1999)
            ErrorCode::InternalServerError => 1900,
            ErrorCode::NotImplemented => 1901,
            ErrorCode::ServiceUnavailable => 1902,
        }
    }

    /// Get the string representation of the error code
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorCode::MissingAuth => "MISSING_AUTHORIZATION",
            ErrorCode::InvalidToken => "INVALID_TOKEN",
            ErrorCode::ExpiredToken => "EXPIRED_TOKEN",
            ErrorCode::InvalidApiKey => "INVALID_API_KEY",
            ErrorCode::InsufficientPermissions => "INSUFFICIENT_PERMISSIONS",
            ErrorCode::ProjectAccessDenied => "PROJECT_ACCESS_DENIED",
            ErrorCode::ResourceAccessDenied => "RESOURCE_ACCESS_DENIED",
            ErrorCode::InvalidRequest => "INVALID_REQUEST",
            ErrorCode::MissingRequiredField => "MISSING_REQUIRED_FIELD",
            ErrorCode::InvalidFieldValue => "INVALID_FIELD_VALUE",
            ErrorCode::InvalidFieldFormat => "INVALID_FIELD_FORMAT",
            ErrorCode::InvalidDateRange => "INVALID_DATE_RANGE",
            ErrorCode::LimitExceeded => "LIMIT_EXCEEDED",
            ErrorCode::MissingExecutionId => "MISSING_EXECUTION_ID",
            ErrorCode::MissingParentSpanId => "MISSING_PARENT_SPAN_ID",
            ErrorCode::ExecutionValidationFailed => "EXECUTION_VALIDATION_FAILED",
            ErrorCode::ResourceNotFound => "RESOURCE_NOT_FOUND",
            ErrorCode::TraceNotFound => "TRACE_NOT_FOUND",
            ErrorCode::JobNotFound => "JOB_NOT_FOUND",
            ErrorCode::ExportNotFound => "EXPORT_NOT_FOUND",
            ErrorCode::ResourceAlreadyExists => "RESOURCE_ALREADY_EXISTS",
            ErrorCode::DuplicateKey => "DUPLICATE_KEY",
            ErrorCode::ConcurrencyConflict => "CONCURRENCY_CONFLICT",
            ErrorCode::RateLimitExceeded => "RATE_LIMIT_EXCEEDED",
            ErrorCode::QuotaExceeded => "QUOTA_EXCEEDED",
            ErrorCode::TooManyRequests => "TOO_MANY_REQUESTS",
            ErrorCode::DatabaseError => "DATABASE_ERROR",
            ErrorCode::DatabaseConnectionFailed => "DATABASE_CONNECTION_FAILED",
            ErrorCode::QueryTimeout => "QUERY_TIMEOUT",
            ErrorCode::DatabaseConstraintViolation => "DATABASE_CONSTRAINT_VIOLATION",
            ErrorCode::ExternalServiceError => "EXTERNAL_SERVICE_ERROR",
            ErrorCode::RedisError => "REDIS_ERROR",
            ErrorCode::StorageError => "STORAGE_ERROR",
            ErrorCode::RequestTimeout => "REQUEST_TIMEOUT",
            ErrorCode::OperationTimeout => "OPERATION_TIMEOUT",
            ErrorCode::InternalServerError => "INTERNAL_SERVER_ERROR",
            ErrorCode::NotImplemented => "NOT_IMPLEMENTED",
            ErrorCode::ServiceUnavailable => "SERVICE_UNAVAILABLE",
        }
    }

    /// Get the error category
    pub fn category(&self) -> ErrorCategory {
        match self {
            ErrorCode::MissingAuth
            | ErrorCode::InvalidToken
            | ErrorCode::ExpiredToken
            | ErrorCode::InvalidApiKey => ErrorCategory::Authentication,

            ErrorCode::InsufficientPermissions
            | ErrorCode::ProjectAccessDenied
            | ErrorCode::ResourceAccessDenied => ErrorCategory::Authorization,

            ErrorCode::InvalidRequest
            | ErrorCode::MissingRequiredField
            | ErrorCode::InvalidFieldValue
            | ErrorCode::InvalidFieldFormat
            | ErrorCode::InvalidDateRange
            | ErrorCode::LimitExceeded
            | ErrorCode::MissingExecutionId
            | ErrorCode::MissingParentSpanId
            | ErrorCode::ExecutionValidationFailed => ErrorCategory::Validation,

            ErrorCode::ResourceNotFound
            | ErrorCode::TraceNotFound
            | ErrorCode::JobNotFound
            | ErrorCode::ExportNotFound => ErrorCategory::NotFound,

            ErrorCode::ResourceAlreadyExists
            | ErrorCode::DuplicateKey
            | ErrorCode::ConcurrencyConflict => ErrorCategory::Conflict,

            ErrorCode::RateLimitExceeded
            | ErrorCode::QuotaExceeded
            | ErrorCode::TooManyRequests => ErrorCategory::RateLimit,

            ErrorCode::DatabaseError
            | ErrorCode::DatabaseConnectionFailed
            | ErrorCode::QueryTimeout
            | ErrorCode::DatabaseConstraintViolation => ErrorCategory::Database,

            ErrorCode::ExternalServiceError | ErrorCode::RedisError | ErrorCode::StorageError => {
                ErrorCategory::External
            }

            ErrorCode::RequestTimeout | ErrorCode::OperationTimeout => ErrorCategory::Timeout,

            ErrorCode::InternalServerError
            | ErrorCode::NotImplemented
            | ErrorCode::ServiceUnavailable => ErrorCategory::Internal,
        }
    }

    /// Get HTTP status code for this error
    pub fn status_code(&self) -> StatusCode {
        match self.category() {
            ErrorCategory::Authentication => StatusCode::UNAUTHORIZED,
            ErrorCategory::Authorization => StatusCode::FORBIDDEN,
            ErrorCategory::Validation => StatusCode::BAD_REQUEST,
            ErrorCategory::NotFound => StatusCode::NOT_FOUND,
            ErrorCategory::Conflict => StatusCode::CONFLICT,
            ErrorCategory::RateLimit => StatusCode::TOO_MANY_REQUESTS,
            ErrorCategory::Database | ErrorCategory::External => StatusCode::SERVICE_UNAVAILABLE,
            ErrorCategory::Timeout => StatusCode::GATEWAY_TIMEOUT,
            ErrorCategory::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// API error with standardized structure
#[derive(Debug)]
pub struct ApiError {
    pub code: ErrorCode,
    pub message: String,
    pub details: Option<String>,
    pub field: Option<String>,
}

impl ApiError {
    /// Create new API error
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            details: None,
            field: None,
        }
    }

    /// Add details to the error
    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }

    /// Add field information to the error
    pub fn with_field(mut self, field: impl Into<String>) -> Self {
        self.field = Some(field.into());
        self
    }

    // Convenience constructors for common errors

    pub fn missing_auth() -> Self {
        Self::new(
            ErrorCode::MissingAuth,
            "Authorization header is required",
        )
    }

    pub fn invalid_token() -> Self {
        Self::new(ErrorCode::InvalidToken, "Invalid or malformed token")
    }

    pub fn expired_token() -> Self {
        Self::new(ErrorCode::ExpiredToken, "Token has expired")
    }

    pub fn insufficient_permissions(action: &str) -> Self {
        Self::new(
            ErrorCode::InsufficientPermissions,
            format!("Insufficient permissions to {}", action),
        )
    }

    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::InvalidRequest, message)
    }

    pub fn missing_field(field: &str) -> Self {
        Self::new(
            ErrorCode::MissingRequiredField,
            format!("Required field '{}' is missing", field),
        )
        .with_field(field)
    }

    pub fn invalid_field(field: &str, reason: &str) -> Self {
        Self::new(
            ErrorCode::InvalidFieldValue,
            format!("Invalid value for field '{}': {}", field, reason),
        )
        .with_field(field)
    }

    pub fn not_found(resource: &str) -> Self {
        Self::new(
            ErrorCode::ResourceNotFound,
            format!("{} not found", resource),
        )
    }

    pub fn trace_not_found(trace_id: &str) -> Self {
        Self::new(
            ErrorCode::TraceNotFound,
            format!("Trace {} not found", trace_id),
        )
        .with_details("The requested trace does not exist or you don't have access to it")
    }

    pub fn rate_limit_exceeded() -> Self {
        Self::new(
            ErrorCode::RateLimitExceeded,
            "Rate limit exceeded. Please slow down your requests.",
        )
    }

    pub fn database_error(details: impl Into<String>) -> Self {
        Self::new(ErrorCode::DatabaseError, "Database operation failed")
            .with_details(details)
    }

    pub fn internal_error() -> Self {
        Self::new(
            ErrorCode::InternalServerError,
            "An internal server error occurred",
        )
    }

    pub fn service_unavailable(service: &str) -> Self {
        Self::new(
            ErrorCode::ServiceUnavailable,
            format!("Service {} is temporarily unavailable", service),
        )
    }

    pub fn missing_execution_id() -> Self {
        Self::new(
            ErrorCode::MissingExecutionId,
            "x-execution-id header is required",
        )
    }

    pub fn missing_parent_span_id() -> Self {
        Self::new(
            ErrorCode::MissingParentSpanId,
            "x-execution-parent-span-id header is required",
        )
    }

    pub fn execution_validation_failed(reason: impl Into<String>) -> Self {
        Self::new(ErrorCode::ExecutionValidationFailed, reason)
            .with_details("Execution context validation failed. Ensure valid execution headers are provided.")
    }
}

/// Error response structure sent to clients
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: ErrorInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<ErrorMeta>,
}

#[derive(Debug, Serialize)]
pub struct ErrorInfo {
    pub code: u32,
    pub error_code: String,
    pub category: ErrorCategory,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ErrorMeta {
    pub timestamp: String,
    pub request_id: Option<String>,
    pub documentation_url: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.code.status_code();
        let response = ErrorResponse {
            error: ErrorInfo {
                code: self.code.code(),
                error_code: self.code.as_str().to_string(),
                category: self.code.category(),
                message: self.message,
                details: self.details,
                field: self.field,
            },
            meta: Some(ErrorMeta {
                timestamp: Utc::now().to_rfc3339(),
                request_id: None,
                documentation_url: format!(
                    "https://docs.llm-observatory.io/errors/{}",
                    self.code.code()
                ),
            }),
        };

        (status, Json(response)).into_response()
    }
}

/// Convert from sqlx errors
impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => Self::not_found("Resource"),
            sqlx::Error::Database(db_err) => {
                // Check for specific database errors
                if let Some(code) = db_err.code() {
                    if code == "23505" {
                        // PostgreSQL unique violation
                        return Self::new(ErrorCode::DuplicateKey, "Resource already exists");
                    }
                    if code == "23503" {
                        // PostgreSQL foreign key violation
                        return Self::new(
                            ErrorCode::InvalidFieldValue,
                            "Referenced resource does not exist",
                        );
                    }
                }
                Self::database_error(db_err.message())
            }
            sqlx::Error::PoolTimedOut => {
                Self::new(ErrorCode::QueryTimeout, "Database connection timeout")
            }
            _ => Self::database_error(err.to_string()),
        }
    }
}

/// Convert from Redis errors
impl From<redis::RedisError> for ApiError {
    fn from(err: redis::RedisError) -> Self {
        Self::new(ErrorCode::RedisError, "Redis operation failed").with_details(err.to_string())
    }
}

/// Convert from JSON errors
impl From<serde_json::Error> for ApiError {
    fn from(err: serde_json::Error) -> Self {
        Self::new(ErrorCode::InvalidRequest, "Invalid JSON in request body")
            .with_details(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_numbers() {
        assert_eq!(ErrorCode::MissingAuth.code(), 1000);
        assert_eq!(ErrorCode::InsufficientPermissions.code(), 1100);
        assert_eq!(ErrorCode::InvalidRequest.code(), 1200);
        assert_eq!(ErrorCode::ResourceNotFound.code(), 1300);
        assert_eq!(ErrorCode::ResourceAlreadyExists.code(), 1400);
        assert_eq!(ErrorCode::RateLimitExceeded.code(), 1500);
        assert_eq!(ErrorCode::DatabaseError.code(), 1600);
        assert_eq!(ErrorCode::ExternalServiceError.code(), 1700);
        assert_eq!(ErrorCode::RequestTimeout.code(), 1800);
        assert_eq!(ErrorCode::InternalServerError.code(), 1900);
    }

    #[test]
    fn test_error_categories() {
        assert_eq!(
            ErrorCode::InvalidToken.category(),
            ErrorCategory::Authentication
        );
        assert_eq!(
            ErrorCode::InsufficientPermissions.category(),
            ErrorCategory::Authorization
        );
        assert_eq!(
            ErrorCode::InvalidRequest.category(),
            ErrorCategory::Validation
        );
        assert_eq!(
            ErrorCode::ResourceNotFound.category(),
            ErrorCategory::NotFound
        );
        assert_eq!(
            ErrorCode::DuplicateKey.category(),
            ErrorCategory::Conflict
        );
        assert_eq!(
            ErrorCode::RateLimitExceeded.category(),
            ErrorCategory::RateLimit
        );
    }

    #[test]
    fn test_status_codes() {
        assert_eq!(ErrorCode::InvalidToken.status_code(), StatusCode::UNAUTHORIZED);
        assert_eq!(
            ErrorCode::InsufficientPermissions.status_code(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            ErrorCode::InvalidRequest.status_code(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            ErrorCode::ResourceNotFound.status_code(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(ErrorCode::DuplicateKey.status_code(), StatusCode::CONFLICT);
        assert_eq!(
            ErrorCode::RateLimitExceeded.status_code(),
            StatusCode::TOO_MANY_REQUESTS
        );
        assert_eq!(
            ErrorCode::DatabaseError.status_code(),
            StatusCode::SERVICE_UNAVAILABLE
        );
    }

    #[test]
    fn test_error_code_strings() {
        assert_eq!(ErrorCode::MissingAuth.as_str(), "MISSING_AUTHORIZATION");
        assert_eq!(ErrorCode::InvalidToken.as_str(), "INVALID_TOKEN");
        assert_eq!(
            ErrorCode::InsufficientPermissions.as_str(),
            "INSUFFICIENT_PERMISSIONS"
        );
    }

    #[test]
    fn test_api_error_construction() {
        let error = ApiError::invalid_request("Test error");
        assert_eq!(error.code, ErrorCode::InvalidRequest);
        assert_eq!(error.message, "Test error");
        assert!(error.details.is_none());
        assert!(error.field.is_none());
    }

    #[test]
    fn test_api_error_with_details() {
        let error = ApiError::invalid_request("Test error").with_details("More info");
        assert_eq!(error.details, Some("More info".to_string()));
    }

    #[test]
    fn test_api_error_with_field() {
        let error = ApiError::invalid_field("email", "must be valid");
        assert_eq!(error.field, Some("email".to_string()));
        assert!(error.message.contains("email"));
        assert!(error.message.contains("must be valid"));
    }

    #[test]
    fn test_convenience_constructors() {
        let error = ApiError::missing_auth();
        assert_eq!(error.code, ErrorCode::MissingAuth);

        let error = ApiError::invalid_token();
        assert_eq!(error.code, ErrorCode::InvalidToken);

        let error = ApiError::insufficient_permissions("delete traces");
        assert_eq!(error.code, ErrorCode::InsufficientPermissions);
        assert!(error.message.contains("delete traces"));

        let error = ApiError::not_found("Trace");
        assert_eq!(error.code, ErrorCode::ResourceNotFound);
        assert!(error.message.contains("Trace"));
    }
}
