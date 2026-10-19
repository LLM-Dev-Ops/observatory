//! Test data fixtures and generators.
//!
//! This module provides factory functions for creating test data
//! for traces, metrics, and logs.

use chrono::{DateTime, Duration, Utc};
use llm_observatory_storage::models::{
    LogLevel, LogRecord, Metric, MetricDataPoint, Trace, TraceEvent, TraceSpan,
};
use uuid::Uuid;

/// Create a test trace with default values
pub fn create_test_trace(trace_id: &str, service_name: &str) -> Trace {
    let now = Utc::now();
    Trace {
        id: Uuid::new_v4(),
        trace_id: trace_id.to_string(),
        service_name: service_name.to_string(),
        start_time: now,
        end_time: Some(now + Duration::milliseconds(100)),
        duration_us: Some(100_000),
        status: "ok".to_string(),
        status_message: None,
        root_span_name: Some("root".to_string()),
        attributes: serde_json::json!({"test": "trace"}),
        resource_attributes: serde_json::json!({"service.version": "1.0.0"}),
        span_count: 1,
        created_at: now,
        updated_at: now,
    }
}

/// Create multiple test traces
pub fn create_test_traces(count: usize, service_name: &str) -> Vec<Trace> {
    (0..count)
        .map(|i| create_test_trace(&format!("trace_{}", i), service_name))
        .collect()
}

/// Create a test span with default values
pub fn create_test_span(
    trace_id: Uuid,
    span_id: &str,
    name: &str,
    service_name: &str,
) -> TraceSpan {
    let now = Utc::now();
    TraceSpan {
        id: Uuid::new_v4(),
        trace_id,
        span_id: span_id.to_string(),
        parent_span_id: None,
        name: name.to_string(),
        kind: "internal".to_string(),
        service_name: service_name.to_string(),
        start_time: now,
        end_time: Some(now + Duration::milliseconds(50)),
        duration_us: Some(50_000),
        status: "ok".to_string(),
        status_message: None,
        attributes: serde_json::json!({"test": "span"}),
        events: None,
        links: None,
        created_at: now,
    }
}

/// Create multiple test spans
pub fn create_test_spans(count: usize, trace_id: Uuid, service_name: &str) -> Vec<TraceSpan> {
    (0..count)
        .map(|i| {
            create_test_span(
                trace_id,
                &format!("span_{}", i),
                &format!("operation_{}", i),
                service_name,
            )
        })
        .collect()
}

/// Create a test trace event
pub fn create_test_event(span_id: Uuid, name: &str) -> TraceEvent {
    let now = Utc::now();
    TraceEvent {
        id: Uuid::new_v4(),
        span_id,
        name: name.to_string(),
        timestamp: now,
        attributes: serde_json::json!({"test": "event"}),
        created_at: now,
    }
}

/// Create a test metric
pub fn create_test_metric(name: &str, metric_type: &str, service_name: &str) -> Metric {
    let now = Utc::now();
    Metric {
        id: Uuid::new_v4(),
        name: name.to_string(),
        description: Some(format!("Test metric: {}", name)),
        unit: Some("count".to_string()),
        metric_type: metric_type.to_string(),
        service_name: service_name.to_string(),
        attributes: serde_json::json!({"test": "metric"}),
        resource_attributes: serde_json::json!({"service.version": "1.0.0"}),
        created_at: now,
        updated_at: now,
    }
}

/// Create multiple test metrics
pub fn create_test_metrics(count: usize, service_name: &str) -> Vec<Metric> {
    (0..count)
        .map(|i| create_test_metric(&format!("metric_{}", i), "counter", service_name))
        .collect()
}

/// Create a test metric data point
pub fn create_test_metric_data_point(metric_id: Uuid, value: f64) -> MetricDataPoint {
    let now = Utc::now();
    MetricDataPoint {
        id: Uuid::new_v4(),
        metric_id,
        timestamp: now,
        value: Some(value),
        count: None,
        sum: None,
        min: None,
        max: None,
        buckets: None,
        quantiles: None,
        exemplars: None,
        attributes: serde_json::json!({}),
        created_at: now,
    }
}

/// Create a histogram data point
pub fn create_histogram_data_point(metric_id: Uuid) -> MetricDataPoint {
    let now = Utc::now();
    MetricDataPoint {
        id: Uuid::new_v4(),
        metric_id,
        timestamp: now,
        value: None,
        count: Some(100),
        sum: Some(5000.0),
        min: Some(10.0),
        max: Some(200.0),
        buckets: Some(serde_json::json!([
            {"boundary": 50.0, "count": 20},
            {"boundary": 100.0, "count": 60},
            {"boundary": 200.0, "count": 20}
        ])),
        quantiles: None,
        exemplars: None,
        attributes: serde_json::json!({}),
        created_at: now,
    }
}

/// Create a test log record
pub fn create_test_log(
    service_name: &str,
    severity: &str,
    body: &str,
    trace_id: Option<&str>,
) -> LogRecord {
    let now = Utc::now();
    let severity_number = match severity.to_uppercase().as_str() {
        "TRACE" => 1,
        "DEBUG" => 5,
        "INFO" => 9,
        "WARN" => 13,
        "ERROR" => 17,
        "FATAL" => 21,
        _ => 9,
    };

    LogRecord {
        id: Uuid::new_v4(),
        timestamp: now,
        observed_timestamp: now,
        severity_number,
        severity_text: severity.to_uppercase(),
        body: body.to_string(),
        service_name: service_name.to_string(),
        trace_id: trace_id.map(|s| s.to_string()),
        span_id: None,
        trace_flags: None,
        attributes: serde_json::json!({"test": "log"}),
        resource_attributes: serde_json::json!({"service.version": "1.0.0"}),
        scope_name: Some("test-scope".to_string()),
        scope_version: Some("1.0.0".to_string()),
        scope_attributes: Some(serde_json::json!({})),
        created_at: now,
    }
}

/// Create multiple test logs
pub fn create_test_logs(count: usize, service_name: &str) -> Vec<LogRecord> {
    let severities = ["INFO", "WARN", "ERROR"];
    (0..count)
        .map(|i| {
            create_test_log(
                service_name,
                severities[i % severities.len()],
                &format!("Test log message {}", i),
                None,
            )
        })
        .collect()
}

/// Create a log with specific attributes
pub fn create_custom_log(
    service_name: &str,
    severity_number: i32,
    body: &str,
    attributes: serde_json::Value,
) -> LogRecord {
    let now = Utc::now();
    let severity_text = match severity_number {
        1..=4 => "TRACE",
        5..=8 => "DEBUG",
        9..=12 => "INFO",
        13..=16 => "WARN",
        17..=20 => "ERROR",
        21..=24 => "FATAL",
        _ => "INFO",
    };

    LogRecord {
        id: Uuid::new_v4(),
        timestamp: now,
        observed_timestamp: now,
        severity_number,
        severity_text: severity_text.to_string(),
        body: body.to_string(),
        service_name: service_name.to_string(),
        trace_id: None,
        span_id: None,
        trace_flags: None,
        attributes,
        resource_attributes: serde_json::json!({}),
        scope_name: None,
        scope_version: None,
        scope_attributes: None,
        created_at: now,
    }
}

/// Builder for creating complex trace scenarios
pub struct TraceBuilder {
    trace_id: String,
    service_name: String,
    spans: Vec<TraceSpan>,
    events: Vec<TraceEvent>,
}

impl TraceBuilder {
    pub fn new(trace_id: &str, service_name: &str) -> Self {
        Self {
            trace_id: trace_id.to_string(),
            service_name: service_name.to_string(),
            spans: Vec::new(),
            events: Vec::new(),
        }
    }

    pub fn with_span(mut self, span_id: &str, name: &str) -> Self {
        let trace_uuid = Uuid::new_v4();
        let span = create_test_span(trace_uuid, span_id, name, &self.service_name);
        self.spans.push(span);
        self
    }

    pub fn with_event(mut self, span_id: Uuid, event_name: &str) -> Self {
        let event = create_test_event(span_id, event_name);
        self.events.push(event);
        self
    }

    pub fn build(self) -> (Trace, Vec<TraceSpan>, Vec<TraceEvent>) {
        let trace = create_test_trace(&self.trace_id, &self.service_name);
        (trace, self.spans, self.events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_test_trace() {
        let trace = create_test_trace("test-trace-123", "test-service");
        assert_eq!(trace.trace_id, "test-trace-123");
        assert_eq!(trace.service_name, "test-service");
        assert_eq!(trace.status, "ok");
    }

    #[test]
    fn test_create_test_traces() {
        let traces = create_test_traces(5, "test-service");
        assert_eq!(traces.len(), 5);
        for (i, trace) in traces.iter().enumerate() {
            assert_eq!(trace.trace_id, format!("trace_{}", i));
        }
    }

    #[test]
    fn test_create_test_span() {
        let trace_id = Uuid::new_v4();
        let span = create_test_span(trace_id, "span-123", "test-operation", "test-service");
        assert_eq!(span.span_id, "span-123");
        assert_eq!(span.name, "test-operation");
        assert_eq!(span.service_name, "test-service");
    }

    #[test]
    fn test_create_test_log() {
        let log = create_test_log(
            "test-service",
            "ERROR",
            "Test error message",
            Some("trace-123"),
        );
        assert_eq!(log.service_name, "test-service");
        assert_eq!(log.severity_text, "ERROR");
        assert_eq!(log.body, "Test error message");
        assert_eq!(log.trace_id, Some("trace-123".to_string()));
    }

    #[test]
    fn test_create_test_metric() {
        let metric = create_test_metric("test.counter", "counter", "test-service");
        assert_eq!(metric.name, "test.counter");
        assert_eq!(metric.metric_type, "counter");
        assert_eq!(metric.service_name, "test-service");
    }

    #[test]
    fn test_trace_builder() {
        let builder = TraceBuilder::new("trace-123", "test-service")
            .with_span("span-1", "operation-1")
            .with_span("span-2", "operation-2");

        let (trace, spans, _events) = builder.build();
        assert_eq!(trace.trace_id, "trace-123");
        assert_eq!(spans.len(), 2);
    }
}
