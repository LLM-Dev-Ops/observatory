// Copyright 2025 LLM Observatory Contributors
// SPDX-License-Identifier: Apache-2.0

//! Instrumentation utilities for creating and managing OpenTelemetry spans.

use crate::{observatory::LLMObservatory, Result};
use chrono::Utc;
use llm_observatory_core::{
    span::{ChatMessage, LlmInput, LlmOutput, LlmSpan, SpanEvent, SpanStatus},
    types::{Cost, Latency, Metadata, Provider, TokenUsage},
};
use opentelemetry::{
    trace::{SpanKind, Status, TraceContextExt, Tracer},
    Context, KeyValue,
};
use std::collections::HashMap;
use std::time::Instant;

/// A wrapper around an OpenTelemetry span with LLM-specific tracking.
///
/// This struct provides a convenient interface for creating instrumented LLM operations
/// with automatic cost tracking, token usage, and semantic conventions.
pub struct InstrumentedSpan {
    context: Context,
    start_time: Instant,
    start_timestamp: chrono::DateTime<Utc>,
    span_id: String,
    trace_id: String,
    provider: Provider,
    model: String,
    input: LlmInput,
    metadata: Metadata,
    events: Vec<SpanEvent>,
}

impl InstrumentedSpan {
    /// Create a new instrumented span.
    fn new(
        context: Context,
        span_id: String,
        trace_id: String,
        provider: Provider,
        model: String,
        input: LlmInput,
        metadata: Metadata,
    ) -> Self {
        Self {
            context,
            start_time: Instant::now(),
            start_timestamp: Utc::now(),
            span_id,
            trace_id,
            provider,
            model,
            input,
            metadata,
            events: Vec::new(),
        }
    }

    /// Get the span ID.
    pub fn span_id(&self) -> &str {
        &self.span_id
    }

    /// Get the trace ID.
    pub fn trace_id(&self) -> &str {
        &self.trace_id
    }

    /// Add an event to the span.
    pub fn add_event(&mut self, name: impl Into<String>, attributes: HashMap<String, serde_json::Value>) {
        self.events.push(SpanEvent {
            name: name.into(),
            timestamp: Utc::now(),
            attributes,
        });
    }

    /// Record the first token received (for TTFT tracking).
    pub fn record_first_token(&mut self) {
        let ttft_ms = self.start_time.elapsed().as_millis() as u64;
        let mut attrs = HashMap::new();
        attrs.insert("ttft_ms".to_string(), serde_json::json!(ttft_ms));
        self.add_event("llm.first_token", attrs);
    }

    /// Finish the span with a successful result.
    pub fn finish_success(
        self,
        output: LlmOutput,
        usage: TokenUsage,
        cost: Cost,
    ) -> Result<LlmSpan> {
        let end_timestamp = Utc::now();
        let latency = Latency::new(self.start_timestamp, end_timestamp);

        // Mark OpenTelemetry span as successful
        let span = self.context.span();
        span.set_status(Status::Ok);
        span.add_event(
            "llm.completion.success",
            vec![
                KeyValue::new("tokens.total", usage.total_tokens as i64),
                KeyValue::new("cost.usd", cost.amount_usd),
            ],
        );

        // Build LlmSpan
        let llm_span = LlmSpan::builder()
            .span_id(self.span_id)
            .trace_id(self.trace_id)
            .name("llm.chat.completion")
            .provider(self.provider)
            .model(self.model)
            .input(self.input)
            .output(output)
            .token_usage(usage)
            .cost(cost)
            .latency(latency)
            .metadata(self.metadata)
            .status(SpanStatus::Ok)
            .build()
            .map_err(|e| crate::Error::internal(e))?;

        Ok(llm_span)
    }

    /// Finish the span with an error.
    pub fn finish_error(self, error: &str) -> Result<LlmSpan> {
        let end_timestamp = Utc::now();
        let latency = Latency::new(self.start_timestamp, end_timestamp);

        // Mark OpenTelemetry span as error
        let span = self.context.span();
        span.set_status(Status::error(error.to_string()));
        span.add_event("llm.completion.error", vec![KeyValue::new("error", error.to_string())]);

        // Build LlmSpan
        let llm_span = LlmSpan::builder()
            .span_id(self.span_id)
            .trace_id(self.trace_id)
            .name("llm.chat.completion")
            .provider(self.provider)
            .model(self.model)
            .input(self.input)
            .latency(latency)
            .metadata(self.metadata)
            .status(SpanStatus::Error)
            .build()
            .map_err(|e| crate::Error::internal(e))?;

        Ok(llm_span)
    }
}

/// Builder for creating instrumented spans.
pub struct SpanBuilder {
    observatory: LLMObservatory,
    operation_name: String,
    provider: Provider,
    model: String,
    messages: Vec<ChatMessage>,
    metadata: Metadata,
    attributes: HashMap<String, String>,
}

impl SpanBuilder {
    /// Create a new span builder.
    pub fn new(
        observatory: LLMObservatory,
        provider: Provider,
        model: impl Into<String>,
    ) -> Self {
        Self {
            observatory,
            operation_name: "llm.chat.completion".to_string(),
            provider,
            model: model.into(),
            messages: Vec::new(),
            metadata: Metadata::default(),
            attributes: HashMap::new(),
        }
    }

    /// Set the operation name.
    pub fn operation_name(mut self, name: impl Into<String>) -> Self {
        self.operation_name = name.into();
        self
    }

    /// Add messages to the span.
    pub fn messages(mut self, messages: Vec<ChatMessage>) -> Self {
        self.messages = messages;
        self
    }

    /// Set metadata.
    pub fn metadata(mut self, metadata: Metadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Add a custom attribute.
    pub fn attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }

    /// Build and start the instrumented span.
    pub fn start(self) -> InstrumentedSpan {
        let tracer = self.observatory.tracer();

        // Create OpenTelemetry span with semantic conventions
        let mut span_builder = tracer
            .span_builder(self.operation_name.clone())
            .with_kind(SpanKind::Client);

        // Add standard GenAI semantic convention attributes
        let mut otel_attributes = vec![
            KeyValue::new("gen_ai.system", self.provider.as_str().to_string()),
            KeyValue::new("gen_ai.request.model", self.model.clone()),
            KeyValue::new("service.name", self.observatory.service_name().to_string()),
            KeyValue::new("deployment.environment", self.observatory.environment().to_string()),
        ];

        // Add custom attributes
        for (key, value) in self.attributes {
            otel_attributes.push(KeyValue::new(key, value));
        }

        // Add metadata attributes
        if let Some(user_id) = &self.metadata.user_id {
            otel_attributes.push(KeyValue::new("user.id", user_id.clone()));
        }
        if let Some(session_id) = &self.metadata.session_id {
            otel_attributes.push(KeyValue::new("session.id", session_id.clone()));
        }
        if let Some(env) = &self.metadata.environment {
            otel_attributes.push(KeyValue::new("environment", env.clone()));
        }

        span_builder = span_builder.with_attributes(otel_attributes);

        let span = tracer.build(span_builder);
        let context = Context::current_with_span(span);

        // Extract span and trace IDs
        let span = context.span();
        let span_context = span.span_context();
        let span_id = format!("{:x}", span_context.span_id());
        let trace_id = format!("{:x}", span_context.trace_id());

        // Create LLM input
        let input = LlmInput::Chat {
            messages: self.messages,
        };

        InstrumentedSpan::new(
            context,
            span_id,
            trace_id,
            self.provider,
            self.model,
            input,
            self.metadata,
        )
    }
}

/// Helper function to create a span builder.
pub fn create_span(
    observatory: &LLMObservatory,
    provider: Provider,
    model: impl Into<String>,
) -> SpanBuilder {
    SpanBuilder::new(observatory.clone(), provider, model)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_builder() {
        // Note: This test requires a valid observatory instance
        // In practice, this would be tested with integration tests
    }
}
