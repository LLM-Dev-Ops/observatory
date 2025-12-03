// Copyright 2025 LLM Observatory Contributors
// SPDX-License-Identifier: Apache-2.0

//! LLM-Edge-Agent adapter for Observatory.
//!
//! This module provides runtime integration for consuming telemetry from
//! the LLM-Edge-Agent system, which handles telemetry ingress and gateway traces
//! at the edge of the LLM infrastructure.
//!
//! # Features
//!
//! - Telemetry ingress data consumption
//! - Gateway trace processing
//! - Edge metrics aggregation
//! - Request routing metadata extraction
//!
//! # Architecture
//!
//! This is a runtime-only adapter that processes telemetry data structures
//! without requiring compile-time dependencies on the upstream LLM-Edge-Agent
//! crate. Data is consumed via standardized formats (JSON, OpenTelemetry).
//!
//! # Example
//!
//! ```ignore
//! use llm_observatory_adapters::upstream::edge_agent::EdgeAgentAdapter;
//!
//! let adapter = EdgeAgentAdapter::new("edge-node-1");
//!
//! // Process incoming telemetry
//! let telemetry = adapter.parse_telemetry_ingress(&json_data)?;
//!
//! // Extract gateway traces
//! let traces = adapter.extract_gateway_traces(&telemetry)?;
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use uuid::Uuid;

/// Errors that can occur during edge agent operations.
#[derive(Debug, Error)]
pub enum EdgeAgentAdapterError {
    /// Invalid telemetry data
    #[error("Invalid telemetry data: {0}")]
    InvalidTelemetry(String),

    /// Missing required field
    #[error("Missing required field: {0}")]
    MissingField(String),

    /// Parse error
    #[error("Parse error: {0}")]
    ParseError(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Processing error
    #[error("Processing error: {0}")]
    ProcessingError(String),
}

/// Result type for edge agent operations.
pub type Result<T> = std::result::Result<T, EdgeAgentAdapterError>;

/// Edge node identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EdgeNodeId(String);

impl EdgeNodeId {
    /// Create a new edge node ID.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the ID as a string reference.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for EdgeNodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Telemetry ingress event from edge agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryIngressEvent {
    /// Unique event ID
    pub event_id: Uuid,
    /// Source edge node
    pub edge_node_id: EdgeNodeId,
    /// Event timestamp
    pub timestamp: DateTime<Utc>,
    /// Event type (span, metric, log)
    pub event_type: IngressEventType,
    /// Payload data
    pub payload: serde_json::Value,
    /// Metadata
    pub metadata: HashMap<String, String>,
    /// Processing status
    pub status: IngressStatus,
}

/// Type of ingress event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IngressEventType {
    /// Trace span data
    Span,
    /// Metric data point
    Metric,
    /// Log entry
    Log,
    /// Resource information
    Resource,
    /// Custom event
    Custom(String),
}

/// Status of ingress processing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IngressStatus {
    /// Successfully received
    Received,
    /// Validated and queued
    Validated,
    /// Processed successfully
    Processed,
    /// Processing failed
    Failed,
    /// Dropped (rate limited, filtered, etc.)
    Dropped,
}

/// Gateway trace from edge agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayTrace {
    /// Trace ID
    pub trace_id: String,
    /// Span ID
    pub span_id: String,
    /// Parent span ID (if any)
    pub parent_span_id: Option<String>,
    /// Operation name
    pub operation: String,
    /// Source edge node
    pub edge_node_id: EdgeNodeId,
    /// Start time
    pub start_time: DateTime<Utc>,
    /// End time
    pub end_time: Option<DateTime<Utc>>,
    /// Duration in milliseconds
    pub duration_ms: Option<u64>,
    /// Gateway routing information
    pub routing: GatewayRouting,
    /// Request/response metadata
    pub request_metadata: RequestMetadata,
    /// Status code
    pub status_code: Option<u16>,
    /// Error information (if any)
    pub error: Option<GatewayError>,
    /// Additional attributes
    pub attributes: HashMap<String, serde_json::Value>,
}

/// Gateway routing information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayRouting {
    /// Upstream endpoint URL
    pub upstream_url: Option<String>,
    /// Selected backend
    pub backend: Option<String>,
    /// Load balancing strategy used
    pub load_balance_strategy: Option<String>,
    /// Retry count
    pub retry_count: u32,
    /// Circuit breaker state
    pub circuit_breaker_state: Option<String>,
}

impl Default for GatewayRouting {
    fn default() -> Self {
        Self {
            upstream_url: None,
            backend: None,
            load_balance_strategy: None,
            retry_count: 0,
            circuit_breaker_state: None,
        }
    }
}

/// Request metadata from gateway.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestMetadata {
    /// Request method
    pub method: Option<String>,
    /// Request path
    pub path: Option<String>,
    /// User agent
    pub user_agent: Option<String>,
    /// Client IP (anonymized)
    pub client_ip: Option<String>,
    /// Request size in bytes
    pub request_size_bytes: Option<u64>,
    /// Response size in bytes
    pub response_size_bytes: Option<u64>,
    /// Content type
    pub content_type: Option<String>,
}

impl Default for RequestMetadata {
    fn default() -> Self {
        Self {
            method: None,
            path: None,
            user_agent: None,
            client_ip: None,
            request_size_bytes: None,
            response_size_bytes: None,
            content_type: None,
        }
    }
}

/// Gateway error information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayError {
    /// Error code
    pub code: String,
    /// Error message
    pub message: String,
    /// Error category
    pub category: ErrorCategory,
    /// Is retryable
    pub retryable: bool,
}

/// Error category for gateway errors.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCategory {
    /// Client error (4xx)
    Client,
    /// Server error (5xx)
    Server,
    /// Network/connectivity error
    Network,
    /// Timeout error
    Timeout,
    /// Rate limiting error
    RateLimit,
    /// Authentication/authorization error
    Auth,
    /// Unknown error
    Unknown,
}

/// Edge metrics snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeMetrics {
    /// Edge node ID
    pub edge_node_id: EdgeNodeId,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Requests per second
    pub requests_per_second: f64,
    /// Average latency in ms
    pub avg_latency_ms: f64,
    /// P99 latency in ms
    pub p99_latency_ms: f64,
    /// Error rate (0.0-1.0)
    pub error_rate: f64,
    /// Active connections
    pub active_connections: u64,
    /// Bytes received
    pub bytes_received: u64,
    /// Bytes sent
    pub bytes_sent: u64,
    /// Queue depth
    pub queue_depth: u64,
}

/// Aggregated edge statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EdgeStats {
    /// Total events received
    pub total_events_received: u64,
    /// Total events processed
    pub total_events_processed: u64,
    /// Total events failed
    pub total_events_failed: u64,
    /// Total events dropped
    pub total_events_dropped: u64,
    /// Total gateway traces
    pub total_gateway_traces: u64,
    /// Average ingress latency (ms)
    pub avg_ingress_latency_ms: f64,
}

/// Adapter for consuming LLM-Edge-Agent telemetry.
///
/// Provides runtime integration for Observatory to ingest telemetry
/// and gateway traces from edge nodes without compile-time dependencies.
pub struct EdgeAgentAdapter {
    /// Edge node identifier
    edge_node_id: EdgeNodeId,
    /// Collected ingress events
    ingress_events: Vec<TelemetryIngressEvent>,
    /// Collected gateway traces
    gateway_traces: Vec<GatewayTrace>,
    /// Statistics
    stats: EdgeStats,
}

impl EdgeAgentAdapter {
    /// Create a new EdgeAgentAdapter.
    pub fn new(edge_node_id: impl Into<String>) -> Self {
        Self {
            edge_node_id: EdgeNodeId::new(edge_node_id),
            ingress_events: Vec::new(),
            gateway_traces: Vec::new(),
            stats: EdgeStats::default(),
        }
    }

    /// Get the edge node ID.
    pub fn edge_node_id(&self) -> &EdgeNodeId {
        &self.edge_node_id
    }

    /// Parse telemetry ingress data from JSON.
    pub fn parse_telemetry_ingress(
        &mut self,
        json_data: &serde_json::Value,
    ) -> Result<TelemetryIngressEvent> {
        let event_type = json_data
            .get("event_type")
            .and_then(|v| v.as_str())
            .map(|s| match s {
                "span" => IngressEventType::Span,
                "metric" => IngressEventType::Metric,
                "log" => IngressEventType::Log,
                "resource" => IngressEventType::Resource,
                other => IngressEventType::Custom(other.to_string()),
            })
            .ok_or_else(|| EdgeAgentAdapterError::MissingField("event_type".to_string()))?;

        let payload = json_data
            .get("payload")
            .cloned()
            .unwrap_or(serde_json::Value::Null);

        let metadata: HashMap<String, String> = json_data
            .get("metadata")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        let event = TelemetryIngressEvent {
            event_id: Uuid::new_v4(),
            edge_node_id: self.edge_node_id.clone(),
            timestamp: Utc::now(),
            event_type,
            payload,
            metadata,
            status: IngressStatus::Received,
        };

        self.ingress_events.push(event.clone());
        self.stats.total_events_received += 1;

        Ok(event)
    }

    /// Process and validate an ingress event.
    pub fn process_ingress_event(&mut self, event: &mut TelemetryIngressEvent) -> Result<()> {
        // Validate the event
        if event.payload.is_null() {
            event.status = IngressStatus::Failed;
            self.stats.total_events_failed += 1;
            return Err(EdgeAgentAdapterError::InvalidTelemetry(
                "Empty payload".to_string(),
            ));
        }

        event.status = IngressStatus::Validated;

        // Process based on event type
        match &event.event_type {
            IngressEventType::Span => {
                // Extract span data and potentially create gateway trace
                if let Some(trace) = self.extract_gateway_trace_from_payload(&event.payload)? {
                    self.gateway_traces.push(trace);
                    self.stats.total_gateway_traces += 1;
                }
            }
            _ => {
                // Other event types - mark as processed
            }
        }

        event.status = IngressStatus::Processed;
        self.stats.total_events_processed += 1;

        Ok(())
    }

    /// Extract gateway trace from span payload.
    fn extract_gateway_trace_from_payload(
        &self,
        payload: &serde_json::Value,
    ) -> Result<Option<GatewayTrace>> {
        let trace_id = match payload.get("trace_id").and_then(|v| v.as_str()) {
            Some(id) => id.to_string(),
            None => return Ok(None), // Not a traceable span
        };

        let span_id = payload
            .get("span_id")
            .and_then(|v| v.as_str())
            .unwrap_or(&Uuid::new_v4().to_string())
            .to_string();

        let operation = payload
            .get("operation")
            .or_else(|| payload.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let routing = GatewayRouting {
            upstream_url: payload
                .get("upstream_url")
                .and_then(|v| v.as_str())
                .map(String::from),
            backend: payload
                .get("backend")
                .and_then(|v| v.as_str())
                .map(String::from),
            load_balance_strategy: payload
                .get("lb_strategy")
                .and_then(|v| v.as_str())
                .map(String::from),
            retry_count: payload
                .get("retry_count")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u32,
            circuit_breaker_state: payload
                .get("circuit_breaker")
                .and_then(|v| v.as_str())
                .map(String::from),
        };

        let request_metadata = RequestMetadata {
            method: payload
                .get("method")
                .and_then(|v| v.as_str())
                .map(String::from),
            path: payload
                .get("path")
                .and_then(|v| v.as_str())
                .map(String::from),
            user_agent: payload
                .get("user_agent")
                .and_then(|v| v.as_str())
                .map(String::from),
            client_ip: payload
                .get("client_ip")
                .and_then(|v| v.as_str())
                .map(String::from),
            request_size_bytes: payload.get("request_size").and_then(|v| v.as_u64()),
            response_size_bytes: payload.get("response_size").and_then(|v| v.as_u64()),
            content_type: payload
                .get("content_type")
                .and_then(|v| v.as_str())
                .map(String::from),
        };

        let trace = GatewayTrace {
            trace_id,
            span_id,
            parent_span_id: payload
                .get("parent_span_id")
                .and_then(|v| v.as_str())
                .map(String::from),
            operation,
            edge_node_id: self.edge_node_id.clone(),
            start_time: Utc::now(),
            end_time: None,
            duration_ms: payload.get("duration_ms").and_then(|v| v.as_u64()),
            routing,
            request_metadata,
            status_code: payload
                .get("status_code")
                .and_then(|v| v.as_u64())
                .map(|v| v as u16),
            error: None,
            attributes: HashMap::new(),
        };

        Ok(Some(trace))
    }

    /// Parse gateway traces from JSON array.
    pub fn parse_gateway_traces(
        &mut self,
        json_data: &serde_json::Value,
    ) -> Result<Vec<GatewayTrace>> {
        let traces_array = json_data
            .as_array()
            .ok_or_else(|| EdgeAgentAdapterError::ParseError("Expected array".to_string()))?;

        let mut traces = Vec::new();
        for trace_json in traces_array {
            if let Some(trace) = self.extract_gateway_trace_from_payload(trace_json)? {
                traces.push(trace.clone());
                self.gateway_traces.push(trace);
                self.stats.total_gateway_traces += 1;
            }
        }

        Ok(traces)
    }

    /// Get all collected ingress events.
    pub fn ingress_events(&self) -> &[TelemetryIngressEvent] {
        &self.ingress_events
    }

    /// Get all collected gateway traces.
    pub fn gateway_traces(&self) -> &[GatewayTrace] {
        &self.gateway_traces
    }

    /// Get statistics.
    pub fn stats(&self) -> &EdgeStats {
        &self.stats
    }

    /// Clear all collected data.
    pub fn clear(&mut self) {
        self.ingress_events.clear();
        self.gateway_traces.clear();
        self.stats = EdgeStats::default();
    }

    /// Create edge metrics from current state.
    pub fn create_metrics_snapshot(&self) -> EdgeMetrics {
        let processed = self.stats.total_events_processed as f64;
        let failed = self.stats.total_events_failed as f64;
        let total = processed + failed;

        EdgeMetrics {
            edge_node_id: self.edge_node_id.clone(),
            timestamp: Utc::now(),
            requests_per_second: 0.0, // Would need time tracking for real value
            avg_latency_ms: self.stats.avg_ingress_latency_ms,
            p99_latency_ms: 0.0, // Would need latency tracking
            error_rate: if total > 0.0 { failed / total } else { 0.0 },
            active_connections: 0,
            bytes_received: 0,
            bytes_sent: 0,
            queue_depth: self.ingress_events.len() as u64,
        }
    }

    /// Check if an event should be sampled (for tail-based sampling).
    pub fn should_sample_event(&self, event: &TelemetryIngressEvent) -> bool {
        // Always sample failed events
        if event.status == IngressStatus::Failed {
            return true;
        }

        // Always sample spans (for tracing)
        if event.event_type == IngressEventType::Span {
            return true;
        }

        // Sample custom events
        if matches!(event.event_type, IngressEventType::Custom(_)) {
            return true;
        }

        false
    }

    /// Convert a gateway trace to an Observatory-compatible span format.
    pub fn trace_to_span_json(&self, trace: &GatewayTrace) -> serde_json::Value {
        serde_json::json!({
            "trace_id": trace.trace_id,
            "span_id": trace.span_id,
            "parent_span_id": trace.parent_span_id,
            "name": trace.operation,
            "start_time": trace.start_time.to_rfc3339(),
            "end_time": trace.end_time.map(|t| t.to_rfc3339()),
            "duration_ms": trace.duration_ms,
            "status_code": trace.status_code,
            "attributes": {
                "edge.node_id": trace.edge_node_id.as_str(),
                "http.method": trace.request_metadata.method,
                "http.url": trace.request_metadata.path,
                "http.status_code": trace.status_code,
                "gateway.upstream_url": trace.routing.upstream_url,
                "gateway.backend": trace.routing.backend,
                "gateway.retry_count": trace.routing.retry_count,
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edge_agent_adapter_creation() {
        let adapter = EdgeAgentAdapter::new("edge-node-1");
        assert_eq!(adapter.edge_node_id().as_str(), "edge-node-1");
    }

    #[test]
    fn test_parse_telemetry_ingress() {
        let mut adapter = EdgeAgentAdapter::new("edge-node-1");

        let json_data = serde_json::json!({
            "event_type": "span",
            "payload": {
                "trace_id": "abc123",
                "span_id": "span456",
                "operation": "llm.completion",
                "duration_ms": 150
            },
            "metadata": {
                "source": "edge-agent"
            }
        });

        let event = adapter.parse_telemetry_ingress(&json_data);
        assert!(event.is_ok());

        let event = event.unwrap();
        assert_eq!(event.event_type, IngressEventType::Span);
        assert_eq!(event.status, IngressStatus::Received);
    }

    #[test]
    fn test_process_ingress_event() {
        let mut adapter = EdgeAgentAdapter::new("edge-node-1");

        let json_data = serde_json::json!({
            "event_type": "span",
            "payload": {
                "trace_id": "trace123",
                "span_id": "span456",
                "operation": "gateway.route"
            }
        });

        let mut event = adapter.parse_telemetry_ingress(&json_data).unwrap();
        let result = adapter.process_ingress_event(&mut event);

        assert!(result.is_ok());
        assert_eq!(event.status, IngressStatus::Processed);
        assert_eq!(adapter.stats().total_events_processed, 1);
        assert_eq!(adapter.stats().total_gateway_traces, 1);
    }

    #[test]
    fn test_parse_gateway_traces() {
        let mut adapter = EdgeAgentAdapter::new("edge-node-1");

        let json_data = serde_json::json!([
            {
                "trace_id": "trace1",
                "span_id": "span1",
                "operation": "route",
                "duration_ms": 100
            },
            {
                "trace_id": "trace2",
                "span_id": "span2",
                "operation": "forward",
                "duration_ms": 200
            }
        ]);

        let traces = adapter.parse_gateway_traces(&json_data);
        assert!(traces.is_ok());
        assert_eq!(traces.unwrap().len(), 2);
        assert_eq!(adapter.gateway_traces().len(), 2);
    }

    #[test]
    fn test_should_sample_event() {
        let adapter = EdgeAgentAdapter::new("edge-node-1");

        let span_event = TelemetryIngressEvent {
            event_id: Uuid::new_v4(),
            edge_node_id: EdgeNodeId::new("node1"),
            timestamp: Utc::now(),
            event_type: IngressEventType::Span,
            payload: serde_json::Value::Null,
            metadata: HashMap::new(),
            status: IngressStatus::Received,
        };
        assert!(adapter.should_sample_event(&span_event));

        let failed_event = TelemetryIngressEvent {
            event_id: Uuid::new_v4(),
            edge_node_id: EdgeNodeId::new("node1"),
            timestamp: Utc::now(),
            event_type: IngressEventType::Metric,
            payload: serde_json::Value::Null,
            metadata: HashMap::new(),
            status: IngressStatus::Failed,
        };
        assert!(adapter.should_sample_event(&failed_event));

        let metric_event = TelemetryIngressEvent {
            event_id: Uuid::new_v4(),
            edge_node_id: EdgeNodeId::new("node1"),
            timestamp: Utc::now(),
            event_type: IngressEventType::Metric,
            payload: serde_json::Value::Null,
            metadata: HashMap::new(),
            status: IngressStatus::Processed,
        };
        assert!(!adapter.should_sample_event(&metric_event));
    }

    #[test]
    fn test_trace_to_span_json() {
        let adapter = EdgeAgentAdapter::new("edge-node-1");

        let trace = GatewayTrace {
            trace_id: "trace123".to_string(),
            span_id: "span456".to_string(),
            parent_span_id: None,
            operation: "llm.completion".to_string(),
            edge_node_id: EdgeNodeId::new("edge-node-1"),
            start_time: Utc::now(),
            end_time: None,
            duration_ms: Some(150),
            routing: GatewayRouting::default(),
            request_metadata: RequestMetadata::default(),
            status_code: Some(200),
            error: None,
            attributes: HashMap::new(),
        };

        let json = adapter.trace_to_span_json(&trace);
        assert_eq!(json["trace_id"], "trace123");
        assert_eq!(json["span_id"], "span456");
        assert_eq!(json["duration_ms"], 150);
    }

    #[test]
    fn test_stats_tracking() {
        let mut adapter = EdgeAgentAdapter::new("edge-node-1");

        // Process multiple events
        for i in 0..5 {
            let json_data = serde_json::json!({
                "event_type": "span",
                "payload": {
                    "trace_id": format!("trace{}", i),
                    "operation": "test"
                }
            });

            let mut event = adapter.parse_telemetry_ingress(&json_data).unwrap();
            adapter.process_ingress_event(&mut event).unwrap();
        }

        let stats = adapter.stats();
        assert_eq!(stats.total_events_received, 5);
        assert_eq!(stats.total_events_processed, 5);
        assert_eq!(stats.total_gateway_traces, 5);
    }

    #[test]
    fn test_clear() {
        let mut adapter = EdgeAgentAdapter::new("edge-node-1");

        let json_data = serde_json::json!({
            "event_type": "span",
            "payload": { "trace_id": "test" }
        });

        adapter.parse_telemetry_ingress(&json_data).unwrap();
        assert!(!adapter.ingress_events().is_empty());

        adapter.clear();
        assert!(adapter.ingress_events().is_empty());
        assert!(adapter.gateway_traces().is_empty());
        assert_eq!(adapter.stats().total_events_received, 0);
    }
}
