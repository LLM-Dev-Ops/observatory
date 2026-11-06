///! Data models for trace querying and response formatting
///!
///! This module defines the request and response structures for the trace query API.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Query parameters for listing traces
#[derive(Debug, Clone, Deserialize)]
pub struct TraceQuery {
    // Time range
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,

    // Identifiers
    pub trace_id: Option<String>,
    pub project_id: Option<String>,
    pub session_id: Option<String>,
    pub user_id: Option<String>,

    // Provider/Model filters
    pub provider: Option<String>,
    pub model: Option<String>,
    pub operation_type: Option<String>,

    // Performance filters
    pub min_duration: Option<i32>,
    pub max_duration: Option<i32>,
    pub min_cost: Option<f64>,
    pub max_cost: Option<f64>,
    pub min_tokens: Option<i32>,
    pub max_tokens: Option<i32>,

    // Status filters
    pub status: Option<String>,

    // Metadata filters
    pub environment: Option<String>,
    pub tags: Option<String>, // Comma-separated

    // Search
    pub search: Option<String>,

    // Pagination
    pub cursor: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i32,

    // Sorting
    pub sort_by: Option<String>,
    pub sort_order: Option<SortOrder>,

    // Field selection
    pub fields: Option<String>,  // Comma-separated
    pub include: Option<String>, // Comma-separated: children,evaluations
}

fn default_limit() -> i32 {
    50
}

impl Default for TraceQuery {
    fn default() -> Self {
        Self {
            from: None,
            to: None,
            trace_id: None,
            project_id: None,
            session_id: None,
            user_id: None,
            provider: None,
            model: None,
            operation_type: None,
            min_duration: None,
            max_duration: None,
            min_cost: None,
            max_cost: None,
            min_tokens: None,
            max_tokens: None,
            status: None,
            environment: None,
            tags: None,
            search: None,
            cursor: None,
            limit: 50,
            sort_by: Some("ts".to_string()),
            sort_order: Some(SortOrder::Desc),
            fields: None,
            include: None,
        }
    }
}

/// Sort order
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    Asc,
    Desc,
}

/// Pagination cursor for stable pagination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationCursor {
    pub timestamp: DateTime<Utc>,
    pub trace_id: String,
    pub span_id: String,
}

impl PaginationCursor {
    /// Encode cursor to base64 string
    pub fn encode(&self) -> String {
        let json = serde_json::to_string(self).unwrap();
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, json.as_bytes())
    }

    /// Decode cursor from base64 string
    pub fn decode(cursor: &str) -> Result<Self, String> {
        let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, cursor)
            .map_err(|e| format!("Invalid cursor format: {}", e))?;

        let json =
            String::from_utf8(bytes).map_err(|e| format!("Invalid cursor encoding: {}", e))?;

        serde_json::from_str(&json).map_err(|e| format!("Invalid cursor structure: {}", e))
    }
}

/// Trace response structure
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Trace {
    // Core identifiers
    pub ts: DateTime<Utc>,
    pub trace_id: String,
    pub span_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_span_id: Option<String>,

    // Service info
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span_name: Option<String>,

    // LLM-specific fields
    pub provider: String,
    pub model: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_text: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_text: Option<String>,

    // Token usage
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_tokens: Option<i32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_tokens: Option<i32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<i32>,

    // Cost
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_cost_usd: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_cost_usd: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_cost_usd: Option<f64>,

    // Performance
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<i32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttft_ms: Option<i32>,

    // Status
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_code: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,

    // Metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributes: Option<serde_json::Value>,
}

impl Trace {
    /// Calculate total cost if not present
    pub fn calculate_total_cost(&mut self) {
        if self.total_cost_usd.is_none() {
            self.total_cost_usd = match (self.prompt_cost_usd, self.completion_cost_usd) {
                (Some(p), Some(c)) => Some(p + c),
                (Some(p), None) => Some(p),
                (None, Some(c)) => Some(c),
                (None, None) => None,
            };
        }
    }

    /// Calculate total tokens if not present
    pub fn calculate_total_tokens(&mut self) {
        if self.total_tokens.is_none() {
            self.total_tokens = match (self.prompt_tokens, self.completion_tokens) {
                (Some(p), Some(c)) => Some(p + c),
                (Some(p), None) => Some(p),
                (None, Some(c)) => Some(c),
                (None, None) => None,
            };
        }
    }
}

/// Paginated trace response
#[derive(Debug, Serialize, Deserialize)]
pub struct PaginatedTraceResponse {
    pub status: ResponseStatus,
    pub data: Vec<Trace>,
    pub pagination: PaginationMetadata,
    pub meta: ResponseMetadata,
}

/// Response status
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResponseStatus {
    Success,
    Error,
}

/// Pagination metadata
#[derive(Debug, Serialize, Deserialize)]
pub struct PaginationMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
    pub has_more: bool,
    pub limit: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<i64>,
}

/// Response metadata
#[derive(Debug, Serialize, Deserialize)]
pub struct ResponseMetadata {
    pub timestamp: DateTime<Utc>,
    pub execution_time_ms: u64,
    pub cached: bool,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

/// Single trace response
#[derive(Debug, Serialize, Deserialize)]
pub struct SingleTraceResponse {
    pub status: ResponseStatus,
    pub data: Trace,
    pub meta: ResponseMetadata,
}

/// Trace statistics
#[derive(Debug, Serialize, Deserialize)]
pub struct TraceStats {
    pub total_count: i64,
    pub total_cost_usd: f64,
    pub total_tokens: i64,
    pub avg_duration_ms: f64,
    pub min_duration_ms: i32,
    pub max_duration_ms: i32,
    pub success_count: i64,
    pub error_count: i64,
    pub success_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pagination_cursor_encoding() {
        let cursor = PaginationCursor {
            timestamp: Utc::now(),
            trace_id: "trace123".to_string(),
            span_id: "span456".to_string(),
        };

        let encoded = cursor.encode();
        let decoded = PaginationCursor::decode(&encoded).unwrap();

        assert_eq!(cursor.timestamp.timestamp(), decoded.timestamp.timestamp());
        assert_eq!(cursor.trace_id, decoded.trace_id);
        assert_eq!(cursor.span_id, decoded.span_id);
    }

    #[test]
    fn test_trace_cost_calculation() {
        let mut trace = Trace {
            ts: Utc::now(),
            trace_id: "test".to_string(),
            span_id: "test".to_string(),
            parent_span_id: None,
            service_name: None,
            span_name: None,
            provider: "openai".to_string(),
            model: "gpt-4".to_string(),
            input_text: None,
            output_text: None,
            prompt_tokens: Some(100),
            completion_tokens: Some(50),
            total_tokens: None,
            prompt_cost_usd: Some(0.003),
            completion_cost_usd: Some(0.006),
            total_cost_usd: None,
            duration_ms: Some(1000),
            ttft_ms: None,
            status_code: Some("OK".to_string()),
            error_message: None,
            user_id: None,
            session_id: None,
            environment: None,
            tags: None,
            attributes: None,
        };

        trace.calculate_total_cost();
        trace.calculate_total_tokens();

        assert_eq!(trace.total_cost_usd, Some(0.009));
        assert_eq!(trace.total_tokens, Some(150));
    }

    #[test]
    fn test_trace_query_defaults() {
        let query = TraceQuery::default();

        assert_eq!(query.limit, 50);
        assert_eq!(query.sort_by, Some("ts".to_string()));
        assert!(matches!(query.sort_order, Some(SortOrder::Desc)));
    }
}
