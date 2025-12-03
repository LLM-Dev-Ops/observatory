// Copyright 2025 LLM Observatory Contributors
// SPDX-License-Identifier: Apache-2.0

//! Sentinel adapter for Observatory.
//!
//! This module provides anomaly detection and event consumption by consuming
//! the llm-sentinel-core crate from the LLM-Dev-Ops ecosystem.
//!
//! # Features
//!
//! - Telemetry event creation from Observatory spans
//! - Anomaly detection thresholds
//! - Alert event consumption
//! - Integration with Observatory's sampling system
//!
//! # Example
//!
//! ```ignore
//! use llm_observatory_adapters::upstream::sentinel::SentinelAdapter;
//!
//! let adapter = SentinelAdapter::new("my-service");
//!
//! // Convert span to telemetry event
//! let event = adapter.span_to_telemetry_event(&span)?;
//!
//! // Check for anomalies
//! if let Some(anomaly) = adapter.check_anomaly(&event) {
//!     println!("Anomaly detected: {:?}", anomaly);
//! }
//! ```

use llm_sentinel_core::{
    AnomalyContext, AnomalyDetails, AnomalyEvent, AnomalyType, DetectionMethod, ModelId,
    PromptInfo, ResponseInfo, ServiceId, Severity, TelemetryEvent,
};
use llm_observatory_core::span::{LlmInput, LlmOutput, LlmSpan, SpanStatus};
use llm_observatory_core::types::Provider as ObsProvider;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use uuid::Uuid;

/// Errors that can occur during sentinel operations.
#[derive(Debug, Error)]
pub enum SentinelAdapterError {
    /// Missing required field
    #[error("Missing required field: {0}")]
    MissingField(String),

    /// Invalid event data
    #[error("Invalid event data: {0}")]
    InvalidData(String),

    /// Conversion error
    #[error("Conversion error: {0}")]
    ConversionError(String),

    /// Detection error
    #[error("Detection error: {0}")]
    DetectionError(String),
}

/// Result type for sentinel operations.
pub type Result<T> = std::result::Result<T, SentinelAdapterError>;

/// Anomaly detection thresholds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyThresholds {
    /// Latency threshold in milliseconds
    pub latency_threshold_ms: u64,
    /// Cost threshold in USD
    pub cost_threshold_usd: f64,
    /// Error rate threshold (0.0 - 1.0)
    pub error_rate_threshold: f64,
    /// Token usage spike threshold (multiplier of average)
    pub token_spike_multiplier: f64,
}

impl Default for AnomalyThresholds {
    fn default() -> Self {
        Self {
            latency_threshold_ms: 5000,      // 5 seconds
            cost_threshold_usd: 1.0,         // $1.00
            error_rate_threshold: 0.1,       // 10%
            token_spike_multiplier: 3.0,     // 3x average
        }
    }
}

/// Detected anomaly from Observatory data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedAnomaly {
    /// Anomaly ID
    pub id: Uuid,
    /// Anomaly type
    pub anomaly_type: String,
    /// Severity level
    pub severity: String,
    /// Detection method used
    pub detection_method: String,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f64,
    /// Metric that triggered the anomaly
    pub metric: String,
    /// Observed value
    pub value: f64,
    /// Threshold that was exceeded
    pub threshold: f64,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Related span ID
    pub span_id: Option<String>,
    /// Related trace ID
    pub trace_id: Option<String>,
}

/// Anomaly statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnomalyStats {
    /// Total anomalies detected
    pub total_detected: u64,
    /// Latency anomalies
    pub latency_anomalies: u64,
    /// Cost anomalies
    pub cost_anomalies: u64,
    /// Error anomalies
    pub error_anomalies: u64,
    /// Token usage anomalies
    pub token_anomalies: u64,
}

/// Adapter for consuming llm-sentinel-core functionality.
///
/// Provides a simplified interface for Observatory to interact with
/// the LLM-Dev-Ops Sentinel for anomaly detection and alerting.
pub struct SentinelAdapter {
    /// Service ID for this adapter
    service_id: ServiceId,
    /// Anomaly detection thresholds
    thresholds: AnomalyThresholds,
    /// Detected anomalies
    anomalies: Vec<DetectedAnomaly>,
    /// Statistics
    stats: AnomalyStats,
    /// Baseline latency (for deviation detection)
    baseline_latency_ms: Option<f64>,
    /// Baseline token usage
    baseline_tokens: Option<f64>,
}

impl SentinelAdapter {
    /// Create a new SentinelAdapter with default thresholds.
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            service_id: ServiceId::new(service_name),
            thresholds: AnomalyThresholds::default(),
            anomalies: Vec::new(),
            stats: AnomalyStats::default(),
            baseline_latency_ms: None,
            baseline_tokens: None,
        }
    }

    /// Create a new SentinelAdapter with custom thresholds.
    pub fn with_thresholds(
        service_name: impl Into<String>,
        thresholds: AnomalyThresholds,
    ) -> Self {
        Self {
            service_id: ServiceId::new(service_name),
            thresholds,
            anomalies: Vec::new(),
            stats: AnomalyStats::default(),
            baseline_latency_ms: None,
            baseline_tokens: None,
        }
    }

    /// Get the service ID.
    pub fn service_id(&self) -> &ServiceId {
        &self.service_id
    }

    /// Get the current thresholds.
    pub fn thresholds(&self) -> &AnomalyThresholds {
        &self.thresholds
    }

    /// Update thresholds.
    pub fn set_thresholds(&mut self, thresholds: AnomalyThresholds) {
        self.thresholds = thresholds;
    }

    /// Set baseline latency for deviation detection.
    pub fn set_baseline_latency(&mut self, latency_ms: f64) {
        self.baseline_latency_ms = Some(latency_ms);
    }

    /// Set baseline token usage for spike detection.
    pub fn set_baseline_tokens(&mut self, tokens: f64) {
        self.baseline_tokens = Some(tokens);
    }

    /// Convert an LLM span to a Sentinel telemetry event.
    pub fn span_to_telemetry_event(&self, span: &LlmSpan) -> Result<TelemetryEvent> {
        let prompt_text = self.extract_prompt_text(&span.input)?;
        let prompt_tokens = span
            .token_usage
            .as_ref()
            .map(|u| u.prompt_tokens)
            .unwrap_or(0);

        let (response_text, response_tokens) = match &span.output {
            Some(output) => (
                output.content.clone(),
                span.token_usage
                    .as_ref()
                    .map(|u| u.completion_tokens)
                    .unwrap_or(0),
            ),
            None => (String::new(), 0),
        };

        let cost_usd = span.cost.as_ref().map(|c| c.amount_usd).unwrap_or(0.0);

        let mut metadata = HashMap::new();
        if let Some(user_id) = &span.metadata.user_id {
            metadata.insert("user_id".to_string(), user_id.clone());
        }
        if let Some(session_id) = &span.metadata.session_id {
            metadata.insert("session_id".to_string(), session_id.clone());
        }
        if let Some(env) = &span.metadata.environment {
            metadata.insert("environment".to_string(), env.clone());
        }

        let errors = if span.status == SpanStatus::Error {
            vec!["Request failed".to_string()]
        } else {
            vec![]
        };

        Ok(TelemetryEvent::new(
            self.service_id.clone(),
            ModelId::new(&span.model),
            PromptInfo {
                text: prompt_text,
                tokens: prompt_tokens,
                embedding: None,
            },
            ResponseInfo {
                text: response_text,
                tokens: response_tokens,
                finish_reason: span
                    .output
                    .as_ref()
                    .and_then(|o| o.finish_reason.clone())
                    .unwrap_or_else(|| "unknown".to_string()),
                embedding: None,
            },
            span.latency.total_ms as f64,
            cost_usd,
        ))
    }

    /// Extract prompt text from LLM input.
    fn extract_prompt_text(&self, input: &LlmInput) -> Result<String> {
        match input {
            LlmInput::Text { prompt } => Ok(prompt.clone()),
            LlmInput::Chat { messages } => {
                let text = messages
                    .iter()
                    .map(|m| format!("{}: {}", m.role, m.content))
                    .collect::<Vec<_>>()
                    .join("\n");
                Ok(text)
            }
            LlmInput::Multimodal { parts } => {
                let text = parts
                    .iter()
                    .filter_map(|p| match p {
                        llm_observatory_core::span::ContentPart::Text { text } => Some(text.clone()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                Ok(text)
            }
        }
    }

    /// Check a span for anomalies.
    pub fn check_span_anomaly(&mut self, span: &LlmSpan) -> Option<DetectedAnomaly> {
        // Check latency anomaly
        if span.latency.total_ms > self.thresholds.latency_threshold_ms {
            let anomaly = DetectedAnomaly {
                id: Uuid::new_v4(),
                anomaly_type: "LatencySpike".to_string(),
                severity: self.calculate_severity(
                    span.latency.total_ms as f64,
                    self.thresholds.latency_threshold_ms as f64,
                ),
                detection_method: "Threshold".to_string(),
                confidence: 0.9,
                metric: "latency_ms".to_string(),
                value: span.latency.total_ms as f64,
                threshold: self.thresholds.latency_threshold_ms as f64,
                timestamp: Utc::now(),
                span_id: Some(span.span_id.clone()),
                trace_id: Some(span.trace_id.clone()),
            };

            self.record_anomaly(anomaly.clone(), AnomalyType::LatencySpike);
            return Some(anomaly);
        }

        // Check cost anomaly
        if let Some(cost) = &span.cost {
            if cost.amount_usd > self.thresholds.cost_threshold_usd {
                let anomaly = DetectedAnomaly {
                    id: Uuid::new_v4(),
                    anomaly_type: "CostAnomaly".to_string(),
                    severity: self.calculate_severity(
                        cost.amount_usd,
                        self.thresholds.cost_threshold_usd,
                    ),
                    detection_method: "Threshold".to_string(),
                    confidence: 0.95,
                    metric: "cost_usd".to_string(),
                    value: cost.amount_usd,
                    threshold: self.thresholds.cost_threshold_usd,
                    timestamp: Utc::now(),
                    span_id: Some(span.span_id.clone()),
                    trace_id: Some(span.trace_id.clone()),
                };

                self.record_anomaly(anomaly.clone(), AnomalyType::CostAnomaly);
                return Some(anomaly);
            }
        }

        // Check error status
        if span.status == SpanStatus::Error {
            let anomaly = DetectedAnomaly {
                id: Uuid::new_v4(),
                anomaly_type: "ErrorRateIncrease".to_string(),
                severity: "High".to_string(),
                detection_method: "StatusCheck".to_string(),
                confidence: 1.0,
                metric: "error".to_string(),
                value: 1.0,
                threshold: 0.0,
                timestamp: Utc::now(),
                span_id: Some(span.span_id.clone()),
                trace_id: Some(span.trace_id.clone()),
            };

            self.record_anomaly(anomaly.clone(), AnomalyType::ErrorRateIncrease);
            return Some(anomaly);
        }

        // Check token spike (if baseline is set)
        if let (Some(baseline), Some(usage)) = (self.baseline_tokens, &span.token_usage) {
            let total = usage.total_tokens as f64;
            if total > baseline * self.thresholds.token_spike_multiplier {
                let anomaly = DetectedAnomaly {
                    id: Uuid::new_v4(),
                    anomaly_type: "TokenUsageSpike".to_string(),
                    severity: "Medium".to_string(),
                    detection_method: "BaselineDeviation".to_string(),
                    confidence: 0.85,
                    metric: "total_tokens".to_string(),
                    value: total,
                    threshold: baseline * self.thresholds.token_spike_multiplier,
                    timestamp: Utc::now(),
                    span_id: Some(span.span_id.clone()),
                    trace_id: Some(span.trace_id.clone()),
                };

                self.record_anomaly(anomaly.clone(), AnomalyType::TokenUsageSpike);
                return Some(anomaly);
            }
        }

        None
    }

    /// Calculate severity based on value vs threshold.
    fn calculate_severity(&self, value: f64, threshold: f64) -> String {
        let ratio = value / threshold;
        if ratio > 5.0 {
            "Critical".to_string()
        } else if ratio > 3.0 {
            "High".to_string()
        } else if ratio > 2.0 {
            "Medium".to_string()
        } else {
            "Low".to_string()
        }
    }

    /// Record an anomaly and update statistics.
    fn record_anomaly(&mut self, anomaly: DetectedAnomaly, anomaly_type: AnomalyType) {
        self.anomalies.push(anomaly);
        self.stats.total_detected += 1;

        match anomaly_type {
            AnomalyType::LatencySpike => self.stats.latency_anomalies += 1,
            AnomalyType::CostAnomaly => self.stats.cost_anomalies += 1,
            AnomalyType::ErrorRateIncrease => self.stats.error_anomalies += 1,
            AnomalyType::TokenUsageSpike => self.stats.token_anomalies += 1,
            _ => {}
        }
    }

    /// Get all detected anomalies.
    pub fn anomalies(&self) -> &[DetectedAnomaly] {
        &self.anomalies
    }

    /// Get anomaly statistics.
    pub fn stats(&self) -> &AnomalyStats {
        &self.stats
    }

    /// Clear anomaly history.
    pub fn clear_anomalies(&mut self) {
        self.anomalies.clear();
        self.stats = AnomalyStats::default();
    }

    /// Check if a span should be sampled based on anomaly detection.
    ///
    /// This implements tail-based sampling where we always sample
    /// spans that have anomalies.
    pub fn should_sample(&self, span: &LlmSpan) -> bool {
        // Always sample errors
        if span.status == SpanStatus::Error {
            return true;
        }

        // Always sample slow requests
        if span.latency.total_ms > self.thresholds.latency_threshold_ms {
            return true;
        }

        // Always sample expensive requests
        if let Some(cost) = &span.cost {
            if cost.amount_usd > self.thresholds.cost_threshold_usd {
                return true;
            }
        }

        false
    }

    /// Create an AnomalyEvent from a DetectedAnomaly.
    pub fn to_anomaly_event(&self, detected: &DetectedAnomaly, model: &str) -> AnomalyEvent {
        let anomaly_type = match detected.anomaly_type.as_str() {
            "LatencySpike" => AnomalyType::LatencySpike,
            "CostAnomaly" => AnomalyType::CostAnomaly,
            "ErrorRateIncrease" => AnomalyType::ErrorRateIncrease,
            "TokenUsageSpike" => AnomalyType::TokenUsageSpike,
            other => AnomalyType::Custom(other.to_string()),
        };

        let severity = match detected.severity.as_str() {
            "Critical" => Severity::Critical,
            "High" => Severity::High,
            "Medium" => Severity::Medium,
            _ => Severity::Low,
        };

        let detection_method = match detected.detection_method.as_str() {
            "Threshold" => DetectionMethod::ZScore, // Using ZScore as proxy for threshold
            "BaselineDeviation" => DetectionMethod::Mad,
            "StatusCheck" => DetectionMethod::Custom("StatusCheck".to_string()),
            other => DetectionMethod::Custom(other.to_string()),
        };

        let details = AnomalyDetails {
            metric: detected.metric.clone(),
            value: detected.value,
            baseline: detected.threshold,
            threshold: detected.threshold,
            deviation_sigma: None,
            additional: HashMap::new(),
        };

        let context = AnomalyContext {
            trace_id: detected.trace_id.clone(),
            user_id: None,
            region: None,
            time_window: "instant".to_string(),
            sample_count: 1,
            additional: HashMap::new(),
        };

        AnomalyEvent::new(
            severity,
            anomaly_type,
            self.service_id.clone(),
            ModelId::new(model),
            detection_method,
            detected.confidence,
            details,
            context,
        )
    }

    /// Get supported anomaly types.
    pub fn supported_anomaly_types() -> Vec<AnomalyType> {
        vec![
            AnomalyType::LatencySpike,
            AnomalyType::ThroughputDegradation,
            AnomalyType::ErrorRateIncrease,
            AnomalyType::TokenUsageSpike,
            AnomalyType::CostAnomaly,
            AnomalyType::InputDrift,
            AnomalyType::OutputDrift,
            AnomalyType::QualityDegradation,
        ]
    }

    /// Get supported severity levels.
    pub fn supported_severities() -> Vec<Severity> {
        vec![Severity::Low, Severity::Medium, Severity::High, Severity::Critical]
    }

    /// Get supported detection methods.
    pub fn supported_detection_methods() -> Vec<DetectionMethod> {
        vec![
            DetectionMethod::ZScore,
            DetectionMethod::Iqr,
            DetectionMethod::Mad,
            DetectionMethod::Cusum,
            DetectionMethod::IsolationForest,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use llm_observatory_core::span::LlmInput;
    use llm_observatory_core::types::{Cost, Latency, TokenUsage};

    fn create_test_span(latency_ms: u64, cost_usd: f64, status: SpanStatus) -> LlmSpan {
        let start = Utc::now();
        let end = start + chrono::Duration::milliseconds(latency_ms as i64);

        LlmSpan::builder()
            .span_id("span_123")
            .trace_id("trace_456")
            .name("llm.completion")
            .provider(ObsProvider::OpenAI)
            .model("gpt-4")
            .input(LlmInput::Text {
                prompt: "Hello".to_string(),
            })
            .token_usage(TokenUsage::new(100, 200))
            .cost(Cost::new(cost_usd))
            .latency(Latency::new(start, end))
            .status(status)
            .build()
            .unwrap()
    }

    #[test]
    fn test_sentinel_adapter_creation() {
        let adapter = SentinelAdapter::new("test-service");
        assert_eq!(adapter.service_id().as_str(), "test-service");
    }

    #[test]
    fn test_detect_latency_anomaly() {
        let mut adapter = SentinelAdapter::new("test-service");
        let span = create_test_span(10000, 0.01, SpanStatus::Ok); // 10 seconds

        let anomaly = adapter.check_span_anomaly(&span);
        assert!(anomaly.is_some());
        assert_eq!(anomaly.unwrap().anomaly_type, "LatencySpike");
    }

    #[test]
    fn test_detect_cost_anomaly() {
        let mut adapter = SentinelAdapter::new("test-service");
        let span = create_test_span(100, 5.0, SpanStatus::Ok); // $5

        let anomaly = adapter.check_span_anomaly(&span);
        assert!(anomaly.is_some());
        assert_eq!(anomaly.unwrap().anomaly_type, "CostAnomaly");
    }

    #[test]
    fn test_detect_error_anomaly() {
        let mut adapter = SentinelAdapter::new("test-service");
        let span = create_test_span(100, 0.01, SpanStatus::Error);

        let anomaly = adapter.check_span_anomaly(&span);
        assert!(anomaly.is_some());
        assert_eq!(anomaly.unwrap().anomaly_type, "ErrorRateIncrease");
    }

    #[test]
    fn test_no_anomaly_normal_span() {
        let mut adapter = SentinelAdapter::new("test-service");
        let span = create_test_span(100, 0.01, SpanStatus::Ok);

        let anomaly = adapter.check_span_anomaly(&span);
        assert!(anomaly.is_none());
    }

    #[test]
    fn test_should_sample() {
        let adapter = SentinelAdapter::new("test-service");

        // Should sample errors
        let error_span = create_test_span(100, 0.01, SpanStatus::Error);
        assert!(adapter.should_sample(&error_span));

        // Should sample slow requests
        let slow_span = create_test_span(10000, 0.01, SpanStatus::Ok);
        assert!(adapter.should_sample(&slow_span));

        // Should sample expensive requests
        let expensive_span = create_test_span(100, 5.0, SpanStatus::Ok);
        assert!(adapter.should_sample(&expensive_span));

        // Should not sample normal requests
        let normal_span = create_test_span(100, 0.01, SpanStatus::Ok);
        assert!(!adapter.should_sample(&normal_span));
    }

    #[test]
    fn test_stats_tracking() {
        let mut adapter = SentinelAdapter::new("test-service");

        adapter.check_span_anomaly(&create_test_span(10000, 0.01, SpanStatus::Ok));
        adapter.check_span_anomaly(&create_test_span(100, 5.0, SpanStatus::Ok));
        adapter.check_span_anomaly(&create_test_span(100, 0.01, SpanStatus::Error));

        let stats = adapter.stats();
        assert_eq!(stats.total_detected, 3);
        assert_eq!(stats.latency_anomalies, 1);
        assert_eq!(stats.cost_anomalies, 1);
        assert_eq!(stats.error_anomalies, 1);
    }

    #[test]
    fn test_span_to_telemetry_event() {
        let adapter = SentinelAdapter::new("test-service");
        let span = create_test_span(100, 0.01, SpanStatus::Ok);

        let event = adapter.span_to_telemetry_event(&span);
        assert!(event.is_ok());
    }
}
