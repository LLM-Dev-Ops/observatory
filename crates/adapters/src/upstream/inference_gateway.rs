// Copyright 2025 LLM Observatory Contributors
// SPDX-License-Identifier: Apache-2.0

//! LLM-Inference-Gateway adapter for Observatory.
//!
//! This module provides runtime integration for consuming telemetry from
//! the LLM-Inference-Gateway system, which handles backend routing and
//! inference telemetry for LLM requests.
//!
//! # Features
//!
//! - Backend routing log consumption
//! - Inference telemetry processing
//! - Model routing decisions tracking
//! - Load balancing metrics aggregation
//!
//! # Architecture
//!
//! This is a runtime-only adapter that processes routing and inference data
//! without requiring compile-time dependencies on the upstream crate.
//! Data is consumed via standardized formats (JSON, OpenTelemetry).
//!
//! # Example
//!
//! ```ignore
//! use llm_observatory_adapters::upstream::inference_gateway::InferenceGatewayAdapter;
//!
//! let adapter = InferenceGatewayAdapter::new("gateway-1");
//!
//! // Process routing logs
//! let routing_log = adapter.parse_routing_log(&json_data)?;
//!
//! // Extract inference telemetry
//! let telemetry = adapter.extract_inference_telemetry(&routing_log)?;
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use uuid::Uuid;

/// Errors that can occur during inference gateway operations.
#[derive(Debug, Error)]
pub enum InferenceGatewayAdapterError {
    /// Invalid routing data
    #[error("Invalid routing data: {0}")]
    InvalidRouting(String),

    /// Missing required field
    #[error("Missing required field: {0}")]
    MissingField(String),

    /// Parse error
    #[error("Parse error: {0}")]
    ParseError(String),

    /// Backend unavailable
    #[error("Backend unavailable: {0}")]
    BackendUnavailable(String),

    /// Routing decision error
    #[error("Routing decision error: {0}")]
    RoutingError(String),
}

/// Result type for inference gateway operations.
pub type Result<T> = std::result::Result<T, InferenceGatewayAdapterError>;

/// Gateway identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GatewayId(String);

impl GatewayId {
    /// Create a new gateway ID.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the ID as a string reference.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for GatewayId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Backend identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BackendId(String);

impl BackendId {
    /// Create a new backend ID.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the ID as a string reference.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Routing log entry from inference gateway.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingLog {
    /// Unique log ID
    pub log_id: Uuid,
    /// Gateway ID
    pub gateway_id: GatewayId,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Request ID being routed
    pub request_id: String,
    /// Routing decision
    pub decision: RoutingDecision,
    /// Selected backend
    pub selected_backend: Option<BackendId>,
    /// Decision latency in microseconds
    pub decision_latency_us: u64,
    /// Available backends at decision time
    pub available_backends: Vec<BackendInfo>,
    /// Routing strategy used
    pub strategy: RoutingStrategy,
    /// Additional context
    pub context: HashMap<String, serde_json::Value>,
}

/// Routing decision outcome.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutingDecision {
    /// Successfully routed to backend
    Routed,
    /// Queued for later processing
    Queued,
    /// Rejected (rate limited, etc.)
    Rejected,
    /// Fallback to alternative
    Fallback,
    /// No available backend
    NoBackend,
}

/// Routing strategy used for decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutingStrategy {
    /// Round-robin selection
    RoundRobin,
    /// Least connections
    LeastConnections,
    /// Weighted random
    WeightedRandom,
    /// Latency-based
    LatencyBased,
    /// Cost-based
    CostBased,
    /// Model-specific routing
    ModelSpecific,
    /// Custom strategy
    Custom(String),
}

/// Backend information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendInfo {
    /// Backend ID
    pub backend_id: BackendId,
    /// Provider name
    pub provider: String,
    /// Supported models
    pub models: Vec<String>,
    /// Current health status
    pub health: BackendHealth,
    /// Current load (0.0-1.0)
    pub load: f64,
    /// Average latency in ms
    pub avg_latency_ms: f64,
    /// Cost per 1K tokens (USD)
    pub cost_per_1k_tokens: Option<f64>,
}

/// Backend health status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackendHealth {
    /// Healthy and accepting requests
    Healthy,
    /// Degraded performance
    Degraded,
    /// Unhealthy, not accepting requests
    Unhealthy,
    /// Unknown status
    Unknown,
}

/// Inference telemetry from a routed request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceTelemetry {
    /// Telemetry ID
    pub telemetry_id: Uuid,
    /// Related request ID
    pub request_id: String,
    /// Trace ID (for distributed tracing)
    pub trace_id: Option<String>,
    /// Gateway that processed this request
    pub gateway_id: GatewayId,
    /// Backend that served the request
    pub backend_id: BackendId,
    /// Model used
    pub model: String,
    /// Provider
    pub provider: String,
    /// Request timestamp
    pub request_time: DateTime<Utc>,
    /// Response timestamp
    pub response_time: Option<DateTime<Utc>>,
    /// Total latency in milliseconds
    pub total_latency_ms: Option<u64>,
    /// Time to first token in milliseconds
    pub ttft_ms: Option<u64>,
    /// Token usage
    pub token_usage: Option<InferenceTokenUsage>,
    /// Request status
    pub status: InferenceStatus,
    /// Error details (if failed)
    pub error: Option<InferenceError>,
    /// Streaming enabled
    pub streaming: bool,
    /// Request metadata
    pub metadata: HashMap<String, String>,
}

/// Token usage for inference request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceTokenUsage {
    /// Prompt/input tokens
    pub prompt_tokens: u32,
    /// Completion/output tokens
    pub completion_tokens: u32,
    /// Total tokens
    pub total_tokens: u32,
    /// Cached tokens (if applicable)
    pub cached_tokens: Option<u32>,
}

/// Inference request status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InferenceStatus {
    /// Successfully completed
    Success,
    /// Partially completed (streaming interrupted)
    Partial,
    /// Failed
    Failed,
    /// Timed out
    Timeout,
    /// Cancelled by client
    Cancelled,
}

/// Inference error details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceError {
    /// Error code
    pub code: String,
    /// Error message
    pub message: String,
    /// Error source (backend, gateway, etc.)
    pub source: ErrorSource,
    /// Is retryable
    pub retryable: bool,
}

/// Source of inference error.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorSource {
    /// Error from backend/provider
    Backend,
    /// Error from gateway
    Gateway,
    /// Error from client
    Client,
    /// Network error
    Network,
    /// Unknown source
    Unknown,
}

/// Gateway statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GatewayStats {
    /// Total routing decisions
    pub total_routing_decisions: u64,
    /// Successful routes
    pub successful_routes: u64,
    /// Failed routes
    pub failed_routes: u64,
    /// Fallback routes
    pub fallback_routes: u64,
    /// Total inference requests
    pub total_inference_requests: u64,
    /// Successful inferences
    pub successful_inferences: u64,
    /// Failed inferences
    pub failed_inferences: u64,
    /// Average routing latency (us)
    pub avg_routing_latency_us: f64,
    /// Average inference latency (ms)
    pub avg_inference_latency_ms: f64,
}

/// Load balancing metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancingMetrics {
    /// Gateway ID
    pub gateway_id: GatewayId,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Requests per backend
    pub requests_per_backend: HashMap<String, u64>,
    /// Load per backend
    pub load_per_backend: HashMap<String, f64>,
    /// Backend health status
    pub backend_health: HashMap<String, BackendHealth>,
}

/// Adapter for consuming LLM-Inference-Gateway telemetry.
///
/// Provides runtime integration for Observatory to ingest routing logs
/// and inference telemetry from the gateway without compile-time dependencies.
pub struct InferenceGatewayAdapter {
    /// Gateway identifier
    gateway_id: GatewayId,
    /// Collected routing logs
    routing_logs: Vec<RoutingLog>,
    /// Collected inference telemetry
    inference_telemetry: Vec<InferenceTelemetry>,
    /// Backend registry
    backends: HashMap<String, BackendInfo>,
    /// Statistics
    stats: GatewayStats,
}

impl InferenceGatewayAdapter {
    /// Create a new InferenceGatewayAdapter.
    pub fn new(gateway_id: impl Into<String>) -> Self {
        Self {
            gateway_id: GatewayId::new(gateway_id),
            routing_logs: Vec::new(),
            inference_telemetry: Vec::new(),
            backends: HashMap::new(),
            stats: GatewayStats::default(),
        }
    }

    /// Get the gateway ID.
    pub fn gateway_id(&self) -> &GatewayId {
        &self.gateway_id
    }

    /// Register a backend.
    pub fn register_backend(&mut self, backend: BackendInfo) {
        self.backends
            .insert(backend.backend_id.as_str().to_string(), backend);
    }

    /// Get registered backends.
    pub fn backends(&self) -> &HashMap<String, BackendInfo> {
        &self.backends
    }

    /// Parse a routing log from JSON.
    pub fn parse_routing_log(&mut self, json_data: &serde_json::Value) -> Result<RoutingLog> {
        let request_id = json_data
            .get("request_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| InferenceGatewayAdapterError::MissingField("request_id".to_string()))?
            .to_string();

        let decision = json_data
            .get("decision")
            .and_then(|v| v.as_str())
            .map(|s| match s {
                "routed" => RoutingDecision::Routed,
                "queued" => RoutingDecision::Queued,
                "rejected" => RoutingDecision::Rejected,
                "fallback" => RoutingDecision::Fallback,
                _ => RoutingDecision::NoBackend,
            })
            .unwrap_or(RoutingDecision::Routed);

        let strategy = json_data
            .get("strategy")
            .and_then(|v| v.as_str())
            .map(|s| match s {
                "round_robin" => RoutingStrategy::RoundRobin,
                "least_connections" => RoutingStrategy::LeastConnections,
                "weighted_random" => RoutingStrategy::WeightedRandom,
                "latency_based" => RoutingStrategy::LatencyBased,
                "cost_based" => RoutingStrategy::CostBased,
                "model_specific" => RoutingStrategy::ModelSpecific,
                other => RoutingStrategy::Custom(other.to_string()),
            })
            .unwrap_or(RoutingStrategy::RoundRobin);

        let selected_backend = json_data
            .get("selected_backend")
            .and_then(|v| v.as_str())
            .map(|s| BackendId::new(s));

        let log = RoutingLog {
            log_id: Uuid::new_v4(),
            gateway_id: self.gateway_id.clone(),
            timestamp: Utc::now(),
            request_id,
            decision: decision.clone(),
            selected_backend,
            decision_latency_us: json_data
                .get("decision_latency_us")
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
            available_backends: Vec::new(),
            strategy,
            context: HashMap::new(),
        };

        self.routing_logs.push(log.clone());
        self.stats.total_routing_decisions += 1;

        match decision {
            RoutingDecision::Routed => self.stats.successful_routes += 1,
            RoutingDecision::Fallback => self.stats.fallback_routes += 1,
            RoutingDecision::Rejected | RoutingDecision::NoBackend => self.stats.failed_routes += 1,
            _ => {}
        }

        Ok(log)
    }

    /// Parse inference telemetry from JSON.
    pub fn parse_inference_telemetry(
        &mut self,
        json_data: &serde_json::Value,
    ) -> Result<InferenceTelemetry> {
        let request_id = json_data
            .get("request_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| InferenceGatewayAdapterError::MissingField("request_id".to_string()))?
            .to_string();

        let backend_id = json_data
            .get("backend_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| InferenceGatewayAdapterError::MissingField("backend_id".to_string()))?;

        let model = json_data
            .get("model")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let provider = json_data
            .get("provider")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let status = json_data
            .get("status")
            .and_then(|v| v.as_str())
            .map(|s| match s {
                "success" => InferenceStatus::Success,
                "partial" => InferenceStatus::Partial,
                "failed" => InferenceStatus::Failed,
                "timeout" => InferenceStatus::Timeout,
                "cancelled" => InferenceStatus::Cancelled,
                _ => InferenceStatus::Failed,
            })
            .unwrap_or(InferenceStatus::Success);

        let token_usage = json_data.get("token_usage").and_then(|v| {
            Some(InferenceTokenUsage {
                prompt_tokens: v.get("prompt_tokens")?.as_u64()? as u32,
                completion_tokens: v.get("completion_tokens")?.as_u64()? as u32,
                total_tokens: v.get("total_tokens")?.as_u64()? as u32,
                cached_tokens: v
                    .get("cached_tokens")
                    .and_then(|c| c.as_u64())
                    .map(|c| c as u32),
            })
        });

        let telemetry = InferenceTelemetry {
            telemetry_id: Uuid::new_v4(),
            request_id,
            trace_id: json_data
                .get("trace_id")
                .and_then(|v| v.as_str())
                .map(String::from),
            gateway_id: self.gateway_id.clone(),
            backend_id: BackendId::new(backend_id),
            model,
            provider,
            request_time: Utc::now(),
            response_time: None,
            total_latency_ms: json_data.get("total_latency_ms").and_then(|v| v.as_u64()),
            ttft_ms: json_data.get("ttft_ms").and_then(|v| v.as_u64()),
            token_usage,
            status: status.clone(),
            error: None,
            streaming: json_data
                .get("streaming")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            metadata: HashMap::new(),
        };

        self.inference_telemetry.push(telemetry.clone());
        self.stats.total_inference_requests += 1;

        match status {
            InferenceStatus::Success => self.stats.successful_inferences += 1,
            _ => self.stats.failed_inferences += 1,
        }

        if let Some(latency) = telemetry.total_latency_ms {
            let n = self.stats.total_inference_requests as f64;
            self.stats.avg_inference_latency_ms =
                (self.stats.avg_inference_latency_ms * (n - 1.0) + latency as f64) / n;
        }

        Ok(telemetry)
    }

    /// Get all routing logs.
    pub fn routing_logs(&self) -> &[RoutingLog] {
        &self.routing_logs
    }

    /// Get all inference telemetry.
    pub fn inference_telemetry(&self) -> &[InferenceTelemetry] {
        &self.inference_telemetry
    }

    /// Get statistics.
    pub fn stats(&self) -> &GatewayStats {
        &self.stats
    }

    /// Clear all collected data.
    pub fn clear(&mut self) {
        self.routing_logs.clear();
        self.inference_telemetry.clear();
        self.stats = GatewayStats::default();
    }

    /// Create load balancing metrics snapshot.
    pub fn create_lb_metrics(&self) -> LoadBalancingMetrics {
        let mut requests_per_backend: HashMap<String, u64> = HashMap::new();
        let mut load_per_backend: HashMap<String, f64> = HashMap::new();
        let mut backend_health: HashMap<String, BackendHealth> = HashMap::new();

        // Count requests per backend
        for telemetry in &self.inference_telemetry {
            let backend_key = telemetry.backend_id.as_str().to_string();
            *requests_per_backend.entry(backend_key).or_insert(0) += 1;
        }

        // Get load and health from registered backends
        for (id, backend) in &self.backends {
            load_per_backend.insert(id.clone(), backend.load);
            backend_health.insert(id.clone(), backend.health.clone());
        }

        LoadBalancingMetrics {
            gateway_id: self.gateway_id.clone(),
            timestamp: Utc::now(),
            requests_per_backend,
            load_per_backend,
            backend_health,
        }
    }

    /// Check if inference should be sampled (for tail-based sampling).
    pub fn should_sample_inference(&self, telemetry: &InferenceTelemetry) -> bool {
        // Always sample failures
        if telemetry.status != InferenceStatus::Success {
            return true;
        }

        // Always sample slow requests (> 5 seconds)
        if let Some(latency) = telemetry.total_latency_ms {
            if latency > 5000 {
                return true;
            }
        }

        // Always sample high token usage (> 10K tokens)
        if let Some(usage) = &telemetry.token_usage {
            if usage.total_tokens > 10000 {
                return true;
            }
        }

        false
    }

    /// Convert inference telemetry to Observatory span format.
    pub fn telemetry_to_span_json(&self, telemetry: &InferenceTelemetry) -> serde_json::Value {
        serde_json::json!({
            "trace_id": telemetry.trace_id,
            "span_id": telemetry.telemetry_id.to_string(),
            "name": format!("inference.{}", telemetry.provider),
            "model": telemetry.model,
            "provider": telemetry.provider,
            "start_time": telemetry.request_time.to_rfc3339(),
            "end_time": telemetry.response_time.map(|t| t.to_rfc3339()),
            "duration_ms": telemetry.total_latency_ms,
            "ttft_ms": telemetry.ttft_ms,
            "token_usage": telemetry.token_usage.as_ref().map(|u| serde_json::json!({
                "prompt_tokens": u.prompt_tokens,
                "completion_tokens": u.completion_tokens,
                "total_tokens": u.total_tokens
            })),
            "status": match telemetry.status {
                InferenceStatus::Success => "ok",
                _ => "error"
            },
            "attributes": {
                "gateway.id": self.gateway_id.as_str(),
                "backend.id": telemetry.backend_id.as_str(),
                "inference.streaming": telemetry.streaming
            }
        })
    }

    /// Get routing decision for a model.
    pub fn select_backend_for_model(&self, model: &str) -> Option<&BackendInfo> {
        self.backends
            .values()
            .find(|b| b.health == BackendHealth::Healthy && b.models.contains(&model.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inference_gateway_adapter_creation() {
        let adapter = InferenceGatewayAdapter::new("gateway-1");
        assert_eq!(adapter.gateway_id().as_str(), "gateway-1");
    }

    #[test]
    fn test_parse_routing_log() {
        let mut adapter = InferenceGatewayAdapter::new("gateway-1");

        let json_data = serde_json::json!({
            "request_id": "req-123",
            "decision": "routed",
            "strategy": "least_connections",
            "selected_backend": "backend-openai",
            "decision_latency_us": 150
        });

        let log = adapter.parse_routing_log(&json_data);
        assert!(log.is_ok());

        let log = log.unwrap();
        assert_eq!(log.request_id, "req-123");
        assert_eq!(log.decision, RoutingDecision::Routed);
        assert_eq!(log.strategy, RoutingStrategy::LeastConnections);
        assert!(log.selected_backend.is_some());
    }

    #[test]
    fn test_parse_inference_telemetry() {
        let mut adapter = InferenceGatewayAdapter::new("gateway-1");

        let json_data = serde_json::json!({
            "request_id": "req-123",
            "backend_id": "backend-openai",
            "model": "gpt-4",
            "provider": "openai",
            "status": "success",
            "total_latency_ms": 1500,
            "ttft_ms": 200,
            "streaming": true,
            "token_usage": {
                "prompt_tokens": 100,
                "completion_tokens": 500,
                "total_tokens": 600
            }
        });

        let telemetry = adapter.parse_inference_telemetry(&json_data);
        assert!(telemetry.is_ok());

        let telemetry = telemetry.unwrap();
        assert_eq!(telemetry.model, "gpt-4");
        assert_eq!(telemetry.provider, "openai");
        assert_eq!(telemetry.status, InferenceStatus::Success);
        assert_eq!(telemetry.total_latency_ms, Some(1500));
        assert!(telemetry.token_usage.is_some());
    }

    #[test]
    fn test_register_backend() {
        let mut adapter = InferenceGatewayAdapter::new("gateway-1");

        let backend = BackendInfo {
            backend_id: BackendId::new("backend-openai"),
            provider: "OpenAI".to_string(),
            models: vec!["gpt-4".to_string(), "gpt-3.5-turbo".to_string()],
            health: BackendHealth::Healthy,
            load: 0.5,
            avg_latency_ms: 200.0,
            cost_per_1k_tokens: Some(0.03),
        };

        adapter.register_backend(backend);
        assert_eq!(adapter.backends().len(), 1);
    }

    #[test]
    fn test_should_sample_inference() {
        let adapter = InferenceGatewayAdapter::new("gateway-1");

        // Failed request should be sampled
        let failed = InferenceTelemetry {
            telemetry_id: Uuid::new_v4(),
            request_id: "req-1".to_string(),
            trace_id: None,
            gateway_id: GatewayId::new("gateway-1"),
            backend_id: BackendId::new("backend-1"),
            model: "gpt-4".to_string(),
            provider: "openai".to_string(),
            request_time: Utc::now(),
            response_time: None,
            total_latency_ms: Some(100),
            ttft_ms: None,
            token_usage: None,
            status: InferenceStatus::Failed,
            error: None,
            streaming: false,
            metadata: HashMap::new(),
        };
        assert!(adapter.should_sample_inference(&failed));

        // Slow request should be sampled
        let slow = InferenceTelemetry {
            status: InferenceStatus::Success,
            total_latency_ms: Some(10000),
            ..failed.clone()
        };
        assert!(adapter.should_sample_inference(&slow));

        // High token usage should be sampled
        let high_tokens = InferenceTelemetry {
            status: InferenceStatus::Success,
            total_latency_ms: Some(100),
            token_usage: Some(InferenceTokenUsage {
                prompt_tokens: 5000,
                completion_tokens: 10000,
                total_tokens: 15000,
                cached_tokens: None,
            }),
            ..failed.clone()
        };
        assert!(adapter.should_sample_inference(&high_tokens));

        // Normal request should not be sampled
        let normal = InferenceTelemetry {
            status: InferenceStatus::Success,
            total_latency_ms: Some(100),
            token_usage: Some(InferenceTokenUsage {
                prompt_tokens: 100,
                completion_tokens: 200,
                total_tokens: 300,
                cached_tokens: None,
            }),
            ..failed
        };
        assert!(!adapter.should_sample_inference(&normal));
    }

    #[test]
    fn test_stats_tracking() {
        let mut adapter = InferenceGatewayAdapter::new("gateway-1");

        // Process routing logs
        for i in 0..5 {
            let json_data = serde_json::json!({
                "request_id": format!("req-{}", i),
                "decision": "routed",
                "selected_backend": "backend-1"
            });
            adapter.parse_routing_log(&json_data).unwrap();
        }

        // Process inference telemetry
        for i in 0..3 {
            let json_data = serde_json::json!({
                "request_id": format!("req-{}", i),
                "backend_id": "backend-1",
                "model": "gpt-4",
                "provider": "openai",
                "status": "success",
                "total_latency_ms": 1000
            });
            adapter.parse_inference_telemetry(&json_data).unwrap();
        }

        let stats = adapter.stats();
        assert_eq!(stats.total_routing_decisions, 5);
        assert_eq!(stats.successful_routes, 5);
        assert_eq!(stats.total_inference_requests, 3);
        assert_eq!(stats.successful_inferences, 3);
    }

    #[test]
    fn test_create_lb_metrics() {
        let mut adapter = InferenceGatewayAdapter::new("gateway-1");

        // Register backends
        adapter.register_backend(BackendInfo {
            backend_id: BackendId::new("backend-1"),
            provider: "OpenAI".to_string(),
            models: vec!["gpt-4".to_string()],
            health: BackendHealth::Healthy,
            load: 0.3,
            avg_latency_ms: 150.0,
            cost_per_1k_tokens: Some(0.03),
        });

        adapter.register_backend(BackendInfo {
            backend_id: BackendId::new("backend-2"),
            provider: "Anthropic".to_string(),
            models: vec!["claude-3".to_string()],
            health: BackendHealth::Degraded,
            load: 0.7,
            avg_latency_ms: 200.0,
            cost_per_1k_tokens: Some(0.025),
        });

        let metrics = adapter.create_lb_metrics();
        assert_eq!(metrics.backend_health.len(), 2);
        assert_eq!(metrics.load_per_backend.len(), 2);
    }

    #[test]
    fn test_telemetry_to_span_json() {
        let adapter = InferenceGatewayAdapter::new("gateway-1");

        let telemetry = InferenceTelemetry {
            telemetry_id: Uuid::new_v4(),
            request_id: "req-123".to_string(),
            trace_id: Some("trace-abc".to_string()),
            gateway_id: GatewayId::new("gateway-1"),
            backend_id: BackendId::new("backend-openai"),
            model: "gpt-4".to_string(),
            provider: "openai".to_string(),
            request_time: Utc::now(),
            response_time: None,
            total_latency_ms: Some(1500),
            ttft_ms: Some(200),
            token_usage: Some(InferenceTokenUsage {
                prompt_tokens: 100,
                completion_tokens: 500,
                total_tokens: 600,
                cached_tokens: None,
            }),
            status: InferenceStatus::Success,
            error: None,
            streaming: true,
            metadata: HashMap::new(),
        };

        let json = adapter.telemetry_to_span_json(&telemetry);
        assert_eq!(json["model"], "gpt-4");
        assert_eq!(json["provider"], "openai");
        assert_eq!(json["duration_ms"], 1500);
    }

    #[test]
    fn test_clear() {
        let mut adapter = InferenceGatewayAdapter::new("gateway-1");

        let json_data = serde_json::json!({
            "request_id": "req-123",
            "decision": "routed"
        });
        adapter.parse_routing_log(&json_data).unwrap();

        let json_data = serde_json::json!({
            "request_id": "req-123",
            "backend_id": "backend-1"
        });
        adapter.parse_inference_telemetry(&json_data).unwrap();

        assert!(!adapter.routing_logs().is_empty());
        assert!(!adapter.inference_telemetry().is_empty());

        adapter.clear();

        assert!(adapter.routing_logs().is_empty());
        assert!(adapter.inference_telemetry().is_empty());
        assert_eq!(adapter.stats().total_routing_decisions, 0);
    }
}
