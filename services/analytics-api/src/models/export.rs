//! # Export Data Models
//!
//! This module contains data models for the export functionality including:
//! - Export job tracking
//! - Export formats (CSV, JSON, JSONL)
//! - Export requests and responses
//! - Compression options

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

// ============================================================================
// Export Format Types
// ============================================================================

/// Supported export formats
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    /// Comma-separated values
    Csv,
    /// JSON array format
    Json,
    /// JSON Lines format (newline-delimited JSON)
    Jsonl,
}

impl fmt::Display for ExportFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExportFormat::Csv => write!(f, "csv"),
            ExportFormat::Json => write!(f, "json"),
            ExportFormat::Jsonl => write!(f, "jsonl"),
        }
    }
}

impl ExportFormat {
    /// Get the MIME type for this format
    pub fn mime_type(&self) -> &'static str {
        match self {
            ExportFormat::Csv => "text/csv",
            ExportFormat::Json => "application/json",
            ExportFormat::Jsonl => "application/x-ndjson",
        }
    }

    /// Get the file extension for this format
    pub fn file_extension(&self) -> &'static str {
        match self {
            ExportFormat::Csv => "csv",
            ExportFormat::Json => "json",
            ExportFormat::Jsonl => "jsonl",
        }
    }
}

/// Compression options for exports
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CompressionFormat {
    /// No compression
    None,
    /// Gzip compression
    Gzip,
}

impl fmt::Display for CompressionFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompressionFormat::None => write!(f, "none"),
            CompressionFormat::Gzip => write!(f, "gzip"),
        }
    }
}

impl CompressionFormat {
    /// Get the file extension for this compression format
    pub fn file_extension(&self) -> Option<&'static str> {
        match self {
            CompressionFormat::None => None,
            CompressionFormat::Gzip => Some("gz"),
        }
    }
}

// ============================================================================
// Export Job Status
// ============================================================================

/// Export job status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "text")]
#[serde(rename_all = "snake_case")]
pub enum ExportJobStatus {
    /// Job is queued and waiting to be processed
    Pending,
    /// Job is currently being processed
    Processing,
    /// Job completed successfully
    Completed,
    /// Job failed with an error
    Failed,
    /// Job was cancelled
    Cancelled,
    /// Job expired (download link no longer available)
    Expired,
}

impl fmt::Display for ExportJobStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExportJobStatus::Pending => write!(f, "pending"),
            ExportJobStatus::Processing => write!(f, "processing"),
            ExportJobStatus::Completed => write!(f, "completed"),
            ExportJobStatus::Failed => write!(f, "failed"),
            ExportJobStatus::Cancelled => write!(f, "cancelled"),
            ExportJobStatus::Expired => write!(f, "expired"),
        }
    }
}

// ============================================================================
// Export Request Models
// ============================================================================

/// Request to create an export job
#[derive(Debug, Deserialize, Clone)]
pub struct CreateExportRequest {
    /// Export format
    pub format: ExportFormat,

    /// Compression format (default: none)
    #[serde(default = "default_compression")]
    pub compression: CompressionFormat,

    /// Start time for filtering traces
    pub start_time: Option<DateTime<Utc>>,

    /// End time for filtering traces
    pub end_time: Option<DateTime<Utc>>,

    /// Filter by provider
    pub provider: Option<String>,

    /// Filter by model
    pub model: Option<String>,

    /// Filter by environment
    pub environment: Option<String>,

    /// Filter by user ID
    pub user_id: Option<String>,

    /// Filter by status code
    pub status_code: Option<String>,

    /// Maximum number of traces to export (max: 1,000,000)
    #[serde(default = "default_export_limit")]
    pub limit: i32,

    /// Fields to include in export (if not specified, includes all fields)
    pub fields: Option<Vec<String>>,
}

fn default_compression() -> CompressionFormat {
    CompressionFormat::None
}

fn default_export_limit() -> i32 {
    100_000 // Default to 100K traces
}

impl CreateExportRequest {
    /// Validate the export request
    pub fn validate(&self) -> Result<(), String> {
        // Validate time range
        if let (Some(start), Some(end)) = (self.start_time, self.end_time) {
            if start >= end {
                return Err("start_time must be before end_time".to_string());
            }

            // Max 1 year time range
            if end - start > Duration::days(365) {
                return Err("Time range cannot exceed 365 days".to_string());
            }
        }

        // Validate limit
        if self.limit < 1 {
            return Err("limit must be at least 1".to_string());
        }

        if self.limit > 1_000_000 {
            return Err("limit cannot exceed 1,000,000".to_string());
        }

        Ok(())
    }
}

// ============================================================================
// Export Response Models
// ============================================================================

/// Response after creating an export job
#[derive(Debug, Serialize)]
pub struct CreateExportResponse {
    /// Unique job ID
    pub job_id: String,

    /// Current job status
    pub status: ExportJobStatus,

    /// When the job was created
    pub created_at: DateTime<Utc>,

    /// Estimated completion time (if available)
    pub estimated_completion_at: Option<DateTime<Utc>>,

    /// URL to check job status
    pub status_url: String,
}

/// Export job details
#[derive(Debug, Serialize)]
pub struct ExportJob {
    /// Unique job ID
    pub job_id: String,

    /// Organization ID
    pub org_id: String,

    /// Job status
    pub status: ExportJobStatus,

    /// Export format
    pub format: ExportFormat,

    /// Compression format
    pub compression: CompressionFormat,

    /// When the job was created
    pub created_at: DateTime<Utc>,

    /// When the job started processing
    pub started_at: Option<DateTime<Utc>>,

    /// When the job completed
    pub completed_at: Option<DateTime<Utc>>,

    /// When the download link expires
    pub expires_at: Option<DateTime<Utc>>,

    /// Number of traces exported
    pub trace_count: Option<i64>,

    /// File size in bytes
    pub file_size_bytes: Option<i64>,

    /// Download URL (if completed)
    pub download_url: Option<String>,

    /// Error message (if failed)
    pub error_message: Option<String>,

    /// Processing progress (0-100)
    pub progress_percent: Option<i32>,
}

/// Export job status response
#[derive(Debug, Serialize)]
pub struct ExportJobStatusResponse {
    /// Job details
    #[serde(flatten)]
    pub job: ExportJob,

    /// Metadata about the export
    pub metadata: ExportJobMetadata,
}

/// Metadata about an export job
#[derive(Debug, Serialize)]
pub struct ExportJobMetadata {
    /// Whether the job can be cancelled
    pub can_cancel: bool,

    /// Whether the job can be downloaded
    pub can_download: bool,

    /// Whether the job has expired
    pub is_expired: bool,

    /// Time remaining before expiration (in seconds)
    pub seconds_until_expiration: Option<i64>,
}

// ============================================================================
// Database Row Types
// ============================================================================

/// Export job row from database
#[derive(Debug, sqlx::FromRow)]
pub struct ExportJobRow {
    pub job_id: Uuid,
    pub org_id: String,
    pub status: String,
    pub format: String,
    pub compression: String,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub trace_count: Option<i64>,
    pub file_size_bytes: Option<i64>,
    pub file_path: Option<String>,
    pub error_message: Option<String>,
    pub progress_percent: Option<i32>,

    // Filter parameters (stored as JSON or separate columns)
    pub filter_start_time: Option<DateTime<Utc>>,
    pub filter_end_time: Option<DateTime<Utc>>,
    pub filter_provider: Option<String>,
    pub filter_model: Option<String>,
    pub filter_environment: Option<String>,
    pub filter_user_id: Option<String>,
    pub filter_status_code: Option<String>,
    pub filter_limit: i32,
}

impl ExportJobRow {
    /// Convert database row to ExportJob model
    pub fn to_export_job(&self, base_url: &str) -> ExportJob {
        let status = match self.status.as_str() {
            "pending" => ExportJobStatus::Pending,
            "processing" => ExportJobStatus::Processing,
            "completed" => ExportJobStatus::Completed,
            "failed" => ExportJobStatus::Failed,
            "cancelled" => ExportJobStatus::Cancelled,
            "expired" => ExportJobStatus::Expired,
            _ => ExportJobStatus::Failed,
        };

        let format = match self.format.as_str() {
            "csv" => ExportFormat::Csv,
            "json" => ExportFormat::Json,
            "jsonl" => ExportFormat::Jsonl,
            _ => ExportFormat::Json,
        };

        let compression = match self.compression.as_str() {
            "gzip" => CompressionFormat::Gzip,
            _ => CompressionFormat::None,
        };

        let download_url = if status == ExportJobStatus::Completed && self.file_path.is_some() {
            Some(format!(
                "{}/api/v1/export/jobs/{}/download",
                base_url, self.job_id
            ))
        } else {
            None
        };

        ExportJob {
            job_id: self.job_id.to_string(),
            org_id: self.org_id.clone(),
            status,
            format,
            compression,
            created_at: self.created_at,
            started_at: self.started_at,
            completed_at: self.completed_at,
            expires_at: self.expires_at,
            trace_count: self.trace_count,
            file_size_bytes: self.file_size_bytes,
            download_url,
            error_message: self.error_message.clone(),
            progress_percent: self.progress_percent,
        }
    }

    /// Check if the job is expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            return Utc::now() > expires_at;
        }
        false
    }

    /// Get seconds until expiration
    pub fn seconds_until_expiration(&self) -> Option<i64> {
        self.expires_at.map(|expires_at| {
            let now = Utc::now();
            if expires_at > now {
                (expires_at - now).num_seconds()
            } else {
                0
            }
        })
    }
}

// ============================================================================
// Export Statistics
// ============================================================================

/// Statistics about export jobs
#[derive(Debug, Serialize)]
pub struct ExportStatistics {
    /// Total number of export jobs
    pub total_jobs: i64,

    /// Jobs by status
    pub by_status: ExportStatusBreakdown,

    /// Total traces exported
    pub total_traces_exported: i64,

    /// Total data exported (bytes)
    pub total_bytes_exported: i64,

    /// Average export time (seconds)
    pub avg_export_time_seconds: Option<f64>,
}

/// Export job count by status
#[derive(Debug, Serialize)]
pub struct ExportStatusBreakdown {
    pub pending: i64,
    pub processing: i64,
    pub completed: i64,
    pub failed: i64,
    pub cancelled: i64,
    pub expired: i64,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_format_display() {
        assert_eq!(ExportFormat::Csv.to_string(), "csv");
        assert_eq!(ExportFormat::Json.to_string(), "json");
        assert_eq!(ExportFormat::Jsonl.to_string(), "jsonl");
    }

    #[test]
    fn test_export_format_mime_type() {
        assert_eq!(ExportFormat::Csv.mime_type(), "text/csv");
        assert_eq!(ExportFormat::Json.mime_type(), "application/json");
        assert_eq!(ExportFormat::Jsonl.mime_type(), "application/x-ndjson");
    }

    #[test]
    fn test_compression_format_extension() {
        assert_eq!(CompressionFormat::None.file_extension(), None);
        assert_eq!(CompressionFormat::Gzip.file_extension(), Some("gz"));
    }

    #[test]
    fn test_export_request_validation_time_range() {
        let mut request = CreateExportRequest {
            format: ExportFormat::Json,
            compression: CompressionFormat::None,
            start_time: Some(Utc::now()),
            end_time: Some(Utc::now() - Duration::days(1)),
            provider: None,
            model: None,
            environment: None,
            user_id: None,
            status_code: None,
            limit: 1000,
            fields: None,
        };

        // Invalid: start_time after end_time
        assert!(request.validate().is_err());

        // Valid: correct time range
        request.start_time = Some(Utc::now() - Duration::days(1));
        request.end_time = Some(Utc::now());
        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_export_request_validation_limit() {
        let mut request = CreateExportRequest {
            format: ExportFormat::Json,
            compression: CompressionFormat::None,
            start_time: None,
            end_time: None,
            provider: None,
            model: None,
            environment: None,
            user_id: None,
            status_code: None,
            limit: 0,
            fields: None,
        };

        // Invalid: limit too low
        assert!(request.validate().is_err());

        // Invalid: limit too high
        request.limit = 2_000_000;
        assert!(request.validate().is_err());

        // Valid: within limits
        request.limit = 100_000;
        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_export_request_validation_max_time_range() {
        let request = CreateExportRequest {
            format: ExportFormat::Json,
            compression: CompressionFormat::None,
            start_time: Some(Utc::now() - Duration::days(400)),
            end_time: Some(Utc::now()),
            provider: None,
            model: None,
            environment: None,
            user_id: None,
            status_code: None,
            limit: 1000,
            fields: None,
        };

        // Invalid: time range > 365 days
        assert!(request.validate().is_err());
    }

    #[test]
    fn test_export_job_status_display() {
        assert_eq!(ExportJobStatus::Pending.to_string(), "pending");
        assert_eq!(ExportJobStatus::Processing.to_string(), "processing");
        assert_eq!(ExportJobStatus::Completed.to_string(), "completed");
        assert_eq!(ExportJobStatus::Failed.to_string(), "failed");
    }
}
