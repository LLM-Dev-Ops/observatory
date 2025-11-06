// Copyright 2025 LLM Observatory Contributors
// SPDX-License-Identifier: Apache-2.0

//! LLM span definitions following OpenTelemetry GenAI semantic conventions.

use crate::types::{Cost, Latency, Metadata, Provider, TokenUsage, TraceId, SpanId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a single LLM operation (request/response) as an OpenTelemetry span.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmSpan {
    /// Unique span identifier
    pub span_id: SpanId,
    /// Trace identifier this span belongs to
    pub trace_id: TraceId,
    /// Parent span identifier (if part of a chain)
    pub parent_span_id: Option<SpanId>,
    /// Span name/operation type
    pub name: String,
    /// LLM provider
    pub provider: Provider,
    /// Model name
    pub model: String,
    /// Request input (prompt)
    pub input: LlmInput,
    /// Response output
    pub output: Option<LlmOutput>,
    /// Token usage statistics
    pub token_usage: Option<TokenUsage>,
    /// Cost information
    pub cost: Option<Cost>,
    /// Latency metrics
    pub latency: Latency,
    /// Metadata and tags
    pub metadata: Metadata,
    /// Span status
    pub status: SpanStatus,
    /// OpenTelemetry attributes
    #[serde(default)]
    pub attributes: HashMap<String, serde_json::Value>,
    /// Events recorded during span
    #[serde(default)]
    pub events: Vec<SpanEvent>,
}

/// LLM input (prompt).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum LlmInput {
    /// Simple text prompt
    Text {
        /// The prompt text
        prompt: String,
    },
    /// Chat messages
    Chat {
        /// Array of messages
        messages: Vec<ChatMessage>,
    },
    /// Multimodal input
    Multimodal {
        /// Content parts
        parts: Vec<ContentPart>,
    },
}

/// Chat message for conversational models.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Role (system, user, assistant)
    pub role: String,
    /// Message content
    pub content: String,
    /// Optional message name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Content part for multimodal inputs.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ContentPart {
    /// Text content
    Text {
        /// The text
        text: String,
    },
    /// Image content
    Image {
        /// Image URL or base64 data
        source: String,
    },
    /// Audio content
    Audio {
        /// Audio URL or base64 data
        source: String,
    },
}

/// LLM output (completion).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmOutput {
    /// Generated text
    pub content: String,
    /// Finish reason (stop, length, content_filter, etc.)
    pub finish_reason: Option<String>,
    /// Additional output metadata
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Span status following OpenTelemetry conventions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum SpanStatus {
    /// Operation completed successfully
    Ok,
    /// Operation failed
    Error,
    /// Status not set
    Unset,
}

impl Default for SpanStatus {
    fn default() -> Self {
        SpanStatus::Unset
    }
}

/// Event recorded during span execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanEvent {
    /// Event name
    pub name: String,
    /// Timestamp when event occurred
    pub timestamp: DateTime<Utc>,
    /// Event attributes
    #[serde(default)]
    pub attributes: HashMap<String, serde_json::Value>,
}

impl LlmSpan {
    /// Create a new LLM span builder.
    pub fn builder() -> LlmSpanBuilder {
        LlmSpanBuilder::default()
    }

    /// Check if the span represents a successful operation.
    pub fn is_success(&self) -> bool {
        self.status == SpanStatus::Ok
    }

    /// Check if the span represents a failed operation.
    pub fn is_error(&self) -> bool {
        self.status == SpanStatus::Error
    }

    /// Get total tokens used (if available).
    pub fn total_tokens(&self) -> Option<u32> {
        self.token_usage.as_ref().map(|u| u.total_tokens)
    }

    /// Get total cost in USD (if available).
    pub fn total_cost_usd(&self) -> Option<f64> {
        self.cost.as_ref().map(|c| c.amount_usd)
    }

    /// Get duration in milliseconds.
    pub fn duration_ms(&self) -> u64 {
        self.latency.total_ms
    }
}

/// Builder for creating LlmSpan instances.
#[derive(Default)]
pub struct LlmSpanBuilder {
    span_id: Option<SpanId>,
    trace_id: Option<TraceId>,
    parent_span_id: Option<SpanId>,
    name: Option<String>,
    provider: Option<Provider>,
    model: Option<String>,
    input: Option<LlmInput>,
    output: Option<LlmOutput>,
    token_usage: Option<TokenUsage>,
    cost: Option<Cost>,
    latency: Option<Latency>,
    metadata: Option<Metadata>,
    status: SpanStatus,
    attributes: HashMap<String, serde_json::Value>,
    events: Vec<SpanEvent>,
}

impl LlmSpanBuilder {
    /// Set span ID.
    pub fn span_id(mut self, id: impl Into<SpanId>) -> Self {
        self.span_id = Some(id.into());
        self
    }

    /// Set trace ID.
    pub fn trace_id(mut self, id: impl Into<TraceId>) -> Self {
        self.trace_id = Some(id.into());
        self
    }

    /// Set parent span ID.
    pub fn parent_span_id(mut self, id: impl Into<SpanId>) -> Self {
        self.parent_span_id = Some(id.into());
        self
    }

    /// Set span name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set provider.
    pub fn provider(mut self, provider: Provider) -> Self {
        self.provider = Some(provider);
        self
    }

    /// Set model name.
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set input.
    pub fn input(mut self, input: LlmInput) -> Self {
        self.input = Some(input);
        self
    }

    /// Set output.
    pub fn output(mut self, output: LlmOutput) -> Self {
        self.output = Some(output);
        self
    }

    /// Set token usage.
    pub fn token_usage(mut self, usage: TokenUsage) -> Self {
        self.token_usage = Some(usage);
        self
    }

    /// Set cost.
    pub fn cost(mut self, cost: Cost) -> Self {
        self.cost = Some(cost);
        self
    }

    /// Set latency.
    pub fn latency(mut self, latency: Latency) -> Self {
        self.latency = Some(latency);
        self
    }

    /// Set metadata.
    pub fn metadata(mut self, metadata: Metadata) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Set status.
    pub fn status(mut self, status: SpanStatus) -> Self {
        self.status = status;
        self
    }

    /// Add an attribute.
    pub fn attribute(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.attributes.insert(key.into(), value);
        self
    }

    /// Add an event.
    pub fn event(mut self, event: SpanEvent) -> Self {
        self.events.push(event);
        self
    }

    /// Build the LlmSpan.
    pub fn build(self) -> Result<LlmSpan, &'static str> {
        Ok(LlmSpan {
            span_id: self.span_id.ok_or("span_id is required")?,
            trace_id: self.trace_id.ok_or("trace_id is required")?,
            parent_span_id: self.parent_span_id,
            name: self.name.ok_or("name is required")?,
            provider: self.provider.ok_or("provider is required")?,
            model: self.model.ok_or("model is required")?,
            input: self.input.ok_or("input is required")?,
            output: self.output,
            token_usage: self.token_usage,
            cost: self.cost,
            latency: self.latency.ok_or("latency is required")?,
            metadata: self.metadata.unwrap_or_default(),
            status: self.status,
            attributes: self.attributes,
            events: self.events,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_span_builder() {
        let now = Utc::now();
        let latency = Latency::new(now, now);

        let span = LlmSpan::builder()
            .span_id("span_123")
            .trace_id("trace_456")
            .name("llm.completion")
            .provider(Provider::OpenAI)
            .model("gpt-4")
            .input(LlmInput::Text {
                prompt: "Hello".to_string(),
            })
            .latency(latency)
            .status(SpanStatus::Ok)
            .build()
            .expect("Failed to build span");

        assert_eq!(span.span_id, "span_123");
        assert_eq!(span.trace_id, "trace_456");
        assert_eq!(span.provider, Provider::OpenAI);
        assert!(span.is_success());
    }
}
