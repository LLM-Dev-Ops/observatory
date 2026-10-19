//! Log data models.
//!
//! This module defines the data structures for storing log records.

use crate::error::{StorageError, StorageResult};
use crate::validation::{validate_hex_string, validate_not_empty, validate_range, Validate};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Log severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LogLevel {
    /// Trace level (most verbose)
    Trace = 1,
    /// Debug level
    Debug = 2,
    /// Info level
    Info = 3,
    /// Warn level
    Warn = 4,
    /// Error level
    Error = 5,
    /// Fatal level (most severe)
    Fatal = 6,
}

/// A log record.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LogRecord {
    /// Unique log record identifier
    pub id: Uuid,

    /// Timestamp of the log record
    pub timestamp: DateTime<Utc>,

    /// Observed timestamp (when the log was received)
    pub observed_timestamp: DateTime<Utc>,

    /// Log severity level (stored as integer in DB)
    pub severity_number: i32,

    /// Log severity text (e.g., "INFO", "ERROR")
    pub severity_text: String,

    /// Log message body
    pub body: String,

    /// Service name
    pub service_name: String,

    /// Trace ID (if associated with a trace)
    pub trace_id: Option<String>,

    /// Span ID (if associated with a span)
    pub span_id: Option<String>,

    /// Trace flags
    pub trace_flags: Option<i32>,

    /// Log attributes as JSON
    pub attributes: serde_json::Value,

    /// Resource attributes as JSON
    pub resource_attributes: serde_json::Value,

    /// Scope name (instrumentation scope)
    pub scope_name: Option<String>,

    /// Scope version
    pub scope_version: Option<String>,

    /// Scope attributes as JSON
    pub scope_attributes: Option<serde_json::Value>,

    /// Created timestamp
    pub created_at: DateTime<Utc>,
}

// TODO: Implement methods for creating and querying logs
impl LogRecord {
    /// Create a new log record.
    pub fn new(/* TODO: Add parameters */) -> Self {
        todo!("Implement LogRecord::new")
    }

    /// Parse log level from severity number.
    pub fn parse_level(severity_number: i32) -> LogLevel {
        match severity_number {
            1..=4 => LogLevel::Trace,
            5..=8 => LogLevel::Debug,
            9..=12 => LogLevel::Info,
            13..=16 => LogLevel::Warn,
            17..=20 => LogLevel::Error,
            21..=24 => LogLevel::Fatal,
            _ => LogLevel::Info, // Default
        }
    }

    /// Get the log level for this record.
    pub fn level(&self) -> LogLevel {
        Self::parse_level(self.severity_number)
    }

    /// Check if this log is an error or fatal level.
    pub fn is_error(&self) -> bool {
        self.severity_number >= LogLevel::Error as i32
    }

    /// Check if this log is associated with a trace.
    pub fn has_trace(&self) -> bool {
        self.trace_id.is_some()
    }
}

impl LogLevel {
    /// Convert to OpenTelemetry severity number.
    pub fn to_severity_number(self) -> i32 {
        match self {
            LogLevel::Trace => 1,
            LogLevel::Debug => 5,
            LogLevel::Info => 9,
            LogLevel::Warn => 13,
            LogLevel::Error => 17,
            LogLevel::Fatal => 21,
        }
    }

    /// Convert to string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Trace => "TRACE",
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
            LogLevel::Fatal => "FATAL",
        }
    }

    /// Parse from string.
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_uppercase().as_str() {
            "TRACE" => Ok(LogLevel::Trace),
            "DEBUG" => Ok(LogLevel::Debug),
            "INFO" => Ok(LogLevel::Info),
            "WARN" | "WARNING" => Ok(LogLevel::Warn),
            "ERROR" => Ok(LogLevel::Error),
            "FATAL" | "CRITICAL" => Ok(LogLevel::Fatal),
            _ => Err(format!("Unknown log level: {}", s)),
        }
    }
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl From<LogLevel> for i32 {
    fn from(level: LogLevel) -> Self {
        level.to_severity_number()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_ordering() {
        assert!(LogLevel::Trace < LogLevel::Debug);
        assert!(LogLevel::Info < LogLevel::Error);
        assert!(LogLevel::Fatal > LogLevel::Warn);
    }

    #[test]
    fn test_log_level_display() {
        assert_eq!(LogLevel::Info.to_string(), "INFO");
        assert_eq!(LogLevel::Error.to_string(), "ERROR");
    }

    #[test]
    fn test_parse_log_level_from_string() {
        assert_eq!(LogLevel::from_str("INFO").unwrap(), LogLevel::Info);
        assert_eq!(LogLevel::from_str("error").unwrap(), LogLevel::Error);
        assert_eq!(LogLevel::from_str("WARNING").unwrap(), LogLevel::Warn);
        assert!(LogLevel::from_str("unknown").is_err());
    }

    #[test]
    fn test_parse_level_from_severity() {
        assert_eq!(LogRecord::parse_level(1), LogLevel::Trace);
        assert_eq!(LogRecord::parse_level(9), LogLevel::Info);
        assert_eq!(LogRecord::parse_level(17), LogLevel::Error);
    }

    #[test]
    fn test_severity_number_conversion() {
        assert_eq!(LogLevel::Info.to_severity_number(), 9);
        assert_eq!(LogLevel::Error.to_severity_number(), 17);
    }

    // TODO: Add more comprehensive tests
}
