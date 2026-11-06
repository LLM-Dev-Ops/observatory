//! Trace data models.
//!
//! This module defines the data structures for storing distributed traces,
//! spans, and events.

use crate::error::{StorageError, StorageResult};
use crate::validation::{
    validate_hex_string, validate_not_empty, validate_ordering, validate_status, Validate,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// A distributed trace representing a request flow through the system.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Trace {
    /// Unique trace identifier
    pub id: Uuid,

    /// Trace ID in hex format (for OpenTelemetry compatibility)
    pub trace_id: String,

    /// Service name that originated the trace
    pub service_name: String,

    /// Trace start timestamp
    pub start_time: DateTime<Utc>,

    /// Trace end timestamp (if completed)
    pub end_time: Option<DateTime<Utc>>,

    /// Total duration in microseconds
    pub duration_us: Option<i64>,

    /// Trace status (ok, error, unset)
    pub status: String,

    /// Status message or error description
    pub status_message: Option<String>,

    /// Root span name
    pub root_span_name: Option<String>,

    /// Trace attributes as JSON
    pub attributes: serde_json::Value,

    /// Resource attributes as JSON
    pub resource_attributes: serde_json::Value,

    /// Number of spans in this trace
    pub span_count: i32,

    /// Created timestamp
    pub created_at: DateTime<Utc>,

    /// Updated timestamp
    pub updated_at: DateTime<Utc>,
}

/// A span representing a unit of work within a trace.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TraceSpan {
    /// Unique span identifier
    pub id: Uuid,

    /// Trace ID this span belongs to
    pub trace_id: Uuid,

    /// Span ID in hex format (for OpenTelemetry compatibility)
    pub span_id: String,

    /// Parent span ID (if this is a child span)
    pub parent_span_id: Option<String>,

    /// Span name (operation name)
    pub name: String,

    /// Span kind (internal, server, client, producer, consumer)
    pub kind: String,

    /// Service name
    pub service_name: String,

    /// Span start timestamp
    pub start_time: DateTime<Utc>,

    /// Span end timestamp
    pub end_time: Option<DateTime<Utc>>,

    /// Duration in microseconds
    pub duration_us: Option<i64>,

    /// Span status (ok, error, unset)
    pub status: String,

    /// Status message
    pub status_message: Option<String>,

    /// Span attributes as JSON
    pub attributes: serde_json::Value,

    /// Events attached to this span
    pub events: Option<serde_json::Value>,

    /// Links to other spans
    pub links: Option<serde_json::Value>,

    /// Created timestamp
    pub created_at: DateTime<Utc>,
}

/// An event within a span.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TraceEvent {
    /// Unique event identifier
    pub id: Uuid,

    /// Span ID this event belongs to
    pub span_id: Uuid,

    /// Event name
    pub name: String,

    /// Event timestamp
    pub timestamp: DateTime<Utc>,

    /// Event attributes as JSON
    pub attributes: serde_json::Value,

    /// Created timestamp
    pub created_at: DateTime<Utc>,
}

impl Trace {
    /// Create a new trace.
    pub fn new(trace_id: String, service_name: String, start_time: DateTime<Utc>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            trace_id,
            service_name,
            start_time,
            end_time: None,
            duration_us: None,
            status: "unset".to_string(),
            status_message: None,
            root_span_name: None,
            attributes: serde_json::json!({}),
            resource_attributes: serde_json::json!({}),
            span_count: 0,
            created_at: now,
            updated_at: now,
        }
    }

    /// Calculate and update trace duration.
    pub fn update_duration(&mut self) {
        if let (Some(start), Some(end)) = (&self.start_time, &self.end_time) {
            self.duration_us = Some((end.timestamp_micros() - start.timestamp_micros()) as i64);
        }
    }
}

impl Validate for Trace {
    fn validate(&self) -> StorageResult<()> {
        // Validate trace_id is a valid hex string (32 chars for 16-byte trace ID)
        validate_hex_string(&self.trace_id, 32, "trace_id")
            .map_err(|e| StorageError::validation(e))?;

        // Validate service_name is not empty
        validate_not_empty(&self.service_name, "service_name")
            .map_err(|e| StorageError::validation(e))?;

        // Validate status is one of the allowed values
        validate_status(&self.status, &["ok", "error", "unset"], "status")
            .map_err(|e| StorageError::validation(e))?;

        // Validate timestamps: end_time must be >= start_time if present
        if let Some(end) = &self.end_time {
            validate_ordering(end, &self.start_time, "end_time", "start_time")
                .map_err(|e| StorageError::validation(e))?;
        }

        // Validate duration_us is non-negative if present
        if let Some(duration) = self.duration_us {
            if duration < 0 {
                return Err(StorageError::validation(format!(
                    "duration_us must be non-negative, got: {}",
                    duration
                )));
            }
        }

        // Validate span_count is non-negative
        if self.span_count < 0 {
            return Err(StorageError::validation(format!(
                "span_count must be non-negative, got: {}",
                self.span_count
            )));
        }

        Ok(())
    }
}

impl TraceSpan {
    /// Create a new span.
    pub fn new(
        trace_id: Uuid,
        span_id: String,
        name: String,
        service_name: String,
        start_time: DateTime<Utc>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            trace_id,
            span_id,
            parent_span_id: None,
            name,
            kind: "internal".to_string(),
            service_name,
            start_time,
            end_time: None,
            duration_us: None,
            status: "unset".to_string(),
            status_message: None,
            attributes: serde_json::json!({}),
            events: None,
            links: None,
            created_at: Utc::now(),
        }
    }

    /// Calculate and update span duration.
    pub fn update_duration(&mut self) {
        if let (Some(start), Some(end)) = (&self.start_time, &self.end_time) {
            self.duration_us = Some((end.timestamp_micros() - start.timestamp_micros()) as i64);
        }
    }

    /// Check if this span represents an error.
    pub fn is_error(&self) -> bool {
        self.status == "error"
    }
}

impl Validate for TraceSpan {
    fn validate(&self) -> StorageResult<()> {
        // Validate span_id is a valid hex string (16 chars for 8-byte span ID)
        validate_hex_string(&self.span_id, 16, "span_id")
            .map_err(|e| StorageError::validation(e))?;

        // Validate parent_span_id if present
        if let Some(ref parent_id) = self.parent_span_id {
            validate_hex_string(parent_id, 16, "parent_span_id")
                .map_err(|e| StorageError::validation(e))?;
        }

        // Validate name is not empty
        validate_not_empty(&self.name, "name").map_err(|e| StorageError::validation(e))?;

        // Validate kind is one of the allowed values
        validate_status(
            &self.kind,
            &["internal", "server", "client", "producer", "consumer"],
            "kind",
        )
        .map_err(|e| StorageError::validation(e))?;

        // Validate service_name is not empty
        validate_not_empty(&self.service_name, "service_name")
            .map_err(|e| StorageError::validation(e))?;

        // Validate status is one of the allowed values
        validate_status(&self.status, &["ok", "error", "unset"], "status")
            .map_err(|e| StorageError::validation(e))?;

        // Validate timestamps: end_time must be >= start_time if present
        if let Some(end) = &self.end_time {
            validate_ordering(end, &self.start_time, "end_time", "start_time")
                .map_err(|e| StorageError::validation(e))?;
        }

        // Validate duration_us is non-negative if present
        if let Some(duration) = self.duration_us {
            if duration < 0 {
                return Err(StorageError::validation(format!(
                    "duration_us must be non-negative, got: {}",
                    duration
                )));
            }
        }

        Ok(())
    }
}

impl TraceEvent {
    /// Create a new event.
    pub fn new(span_id: Uuid, name: String, timestamp: DateTime<Utc>) -> Self {
        Self {
            id: Uuid::new_v4(),
            span_id,
            name,
            timestamp,
            attributes: serde_json::json!({}),
            created_at: Utc::now(),
        }
    }
}

impl Validate for TraceEvent {
    fn validate(&self) -> StorageResult<()> {
        // Validate name is not empty
        validate_not_empty(&self.name, "name").map_err(|e| StorageError::validation(e))?;

        Ok(())
    }
}

// Conversion from LlmSpan (core crate) to TraceSpan (storage model)
//
// NOTE: This conversion creates a TraceSpan with a placeholder trace_id UUID.
// For production use, you should use TraceWriter::write_span_from_llm() instead,
// which properly resolves the trace UUID by querying the database.
//
// This From implementation is useful for:
// - Testing and development
// - Cases where you'll manually set the trace_id afterward
// - Batch operations where trace_id will be resolved separately
//
// Example of recommended usage:
// ```no_run
// use llm_observatory_storage::writers::TraceWriter;
//
// let writer = TraceWriter::new(pool);
// let trace_span = writer.write_span_from_llm(llm_span).await?;
// // trace_span now has the correct trace_id UUID
// ```
#[cfg(feature = "llm-span-conversion")]
impl From<llm_observatory_core::span::LlmSpan> for TraceSpan {
    fn from(span: llm_observatory_core::span::LlmSpan) -> Self {
        use llm_observatory_core::span::SpanStatus;

        // Convert status
        let status = match span.status {
            SpanStatus::Ok => "ok",
            SpanStatus::Error => "error",
            SpanStatus::Unset => "unset",
        };

        // Build attributes JSON from span data
        let mut attributes = serde_json::Map::new();

        // Add LLM-specific attributes
        attributes.insert(
            "llm.provider".to_string(),
            serde_json::json!(span.provider.as_str()),
        );
        attributes.insert("llm.model".to_string(), serde_json::json!(span.model));

        // Add token usage if available
        if let Some(ref usage) = span.token_usage {
            attributes.insert(
                "llm.usage.prompt_tokens".to_string(),
                serde_json::json!(usage.prompt_tokens),
            );
            attributes.insert(
                "llm.usage.completion_tokens".to_string(),
                serde_json::json!(usage.completion_tokens),
            );
            attributes.insert(
                "llm.usage.total_tokens".to_string(),
                serde_json::json!(usage.total_tokens),
            );
        }

        // Add cost if available
        if let Some(ref cost) = span.cost {
            attributes.insert(
                "llm.cost.amount_usd".to_string(),
                serde_json::json!(cost.amount_usd),
            );
            if let Some(prompt_cost) = cost.prompt_cost {
                attributes.insert(
                    "llm.cost.prompt_usd".to_string(),
                    serde_json::json!(prompt_cost),
                );
            }
            if let Some(completion_cost) = cost.completion_cost {
                attributes.insert(
                    "llm.cost.completion_usd".to_string(),
                    serde_json::json!(completion_cost),
                );
            }
        }

        // Add latency metrics
        attributes.insert(
            "llm.latency.total_ms".to_string(),
            serde_json::json!(span.latency.total_ms),
        );
        if let Some(ttft_ms) = span.latency.ttft_ms {
            attributes.insert(
                "llm.latency.ttft_ms".to_string(),
                serde_json::json!(ttft_ms),
            );
        }

        // Add input/output
        attributes.insert(
            "llm.input".to_string(),
            serde_json::to_value(&span.input).unwrap_or(serde_json::json!({})),
        );
        if let Some(ref output) = span.output {
            attributes.insert(
                "llm.output".to_string(),
                serde_json::to_value(output).unwrap_or(serde_json::json!({})),
            );
        }

        // Add metadata
        if let Some(ref user_id) = span.metadata.user_id {
            attributes.insert("user.id".to_string(), serde_json::json!(user_id));
        }
        if let Some(ref session_id) = span.metadata.session_id {
            attributes.insert("session.id".to_string(), serde_json::json!(session_id));
        }
        if let Some(ref environment) = span.metadata.environment {
            attributes.insert(
                "deployment.environment".to_string(),
                serde_json::json!(environment),
            );
        }

        // Merge custom attributes
        for (key, value) in span.attributes {
            attributes.insert(key, value);
        }

        // Convert events
        let events = if !span.events.is_empty() {
            Some(serde_json::to_value(&span.events).unwrap_or(serde_json::json!([])))
        } else {
            None
        };

        // Placeholder trace_id - use TraceWriter::write_span_from_llm() for proper UUID resolution
        // This will be replaced by the actual trace UUID when using write_span_from_llm()
        let trace_uuid = Uuid::new_v4();

        let mut trace_span = Self {
            id: Uuid::new_v4(),
            trace_id: trace_uuid,
            span_id: span.span_id,
            parent_span_id: span.parent_span_id,
            name: span.name,
            kind: "internal".to_string(), // LlmSpan doesn't specify kind, default to internal
            service_name: span
                .metadata
                .environment
                .unwrap_or_else(|| "llm-service".to_string()),
            start_time: span.latency.start_time,
            end_time: Some(span.latency.end_time),
            duration_us: Some(span.latency.total_ms as i64 * 1000), // Convert ms to us
            status: status.to_string(),
            status_message: None,
            attributes: serde_json::Value::Object(attributes),
            events,
            links: None,
            created_at: Utc::now(),
        };

        trace_span.update_duration();
        trace_span
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trace_new() {
        let trace = Trace::new(
            "trace123".to_string(),
            "test-service".to_string(),
            Utc::now(),
        );

        assert_eq!(trace.trace_id, "trace123");
        assert_eq!(trace.service_name, "test-service");
        assert_eq!(trace.status, "unset");
        assert_eq!(trace.span_count, 0);
    }

    #[test]
    fn test_trace_update_duration() {
        let mut trace = Trace::new(
            "trace123".to_string(),
            "test-service".to_string(),
            Utc::now(),
        );

        trace.end_time = Some(trace.start_time + chrono::Duration::seconds(5));
        trace.update_duration();

        assert!(trace.duration_us.is_some());
        assert_eq!(trace.duration_us.unwrap(), 5_000_000); // 5 seconds in microseconds
    }

    #[test]
    fn test_span_is_error() {
        let mut span = TraceSpan::new(
            Uuid::new_v4(),
            "span123".to_string(),
            "test-span".to_string(),
            "test-service".to_string(),
            Utc::now(),
        );

        assert!(!span.is_error());

        span.status = "error".to_string();
        assert!(span.is_error());
    }
}
