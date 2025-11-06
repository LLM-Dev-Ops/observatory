// Copyright 2025 LLM Observatory Contributors
// SPDX-License-Identifier: Apache-2.0

//! Core traits for instrumented LLM clients.

use crate::{Error, Result};
use async_trait::async_trait;
use futures::Stream;
use llm_observatory_core::{
    span::ChatMessage,
    types::TokenUsage,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::pin::Pin;

/// Trait for instrumented LLM clients with automatic tracing and cost tracking.
///
/// This trait provides a standardized interface for LLM interactions with built-in
/// observability. Implementations automatically create OpenTelemetry spans, calculate
/// costs, and track token usage.
///
/// # Example Implementation
///
/// ```rust,ignore
/// use llm_observatory_sdk::{InstrumentedLLM, async_trait};
///
/// pub struct MyLLMClient {
///     // client fields...
/// }
///
/// #[async_trait]
/// impl InstrumentedLLM for MyLLMClient {
///     async fn chat_completion(
///         &self,
///         request: ChatCompletionRequest,
///     ) -> Result<ChatCompletionResponse> {
///         // Implementation with automatic instrumentation
///     }
///
///     async fn streaming_completion(
///         &self,
///         request: ChatCompletionRequest,
///     ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>> {
///         // Implementation for streaming
///     }
/// }
/// ```
#[async_trait]
pub trait InstrumentedLLM: Send + Sync {
    /// Execute a chat completion request with automatic instrumentation.
    ///
    /// This method creates an OpenTelemetry span, tracks the request/response,
    /// calculates token usage and cost, and returns a comprehensive response.
    ///
    /// # Arguments
    ///
    /// * `request` - The chat completion request parameters
    ///
    /// # Returns
    ///
    /// A [`ChatCompletionResponse`] containing the generated text, usage metrics,
    /// and cost information.
    async fn chat_completion(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse>;

    /// Execute a streaming chat completion request.
    ///
    /// This method returns a stream of response chunks, allowing for real-time
    /// processing of the LLM output. Each chunk is instrumented and includes
    /// partial token counts.
    ///
    /// # Arguments
    ///
    /// * `request` - The chat completion request parameters
    ///
    /// # Returns
    ///
    /// A stream of [`StreamChunk`] items representing incremental responses.
    async fn streaming_completion(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>>;

    /// Get the provider name (e.g., "openai", "anthropic").
    fn provider_name(&self) -> &str;

    /// Get the default model for this client.
    fn default_model(&self) -> Option<&str> {
        None
    }
}

/// Request parameters for chat completion.
///
/// This struct provides a builder-style API for constructing LLM requests.
///
/// # Example
///
/// ```rust
/// use llm_observatory_sdk::ChatCompletionRequest;
///
/// let request = ChatCompletionRequest::new("gpt-4")
///     .with_message("user", "Hello, world!")
///     .with_message("assistant", "Hi there!")
///     .with_message("user", "How are you?")
///     .with_temperature(0.7)
///     .with_max_tokens(500);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    /// Model identifier
    pub model: String,

    /// Array of chat messages
    pub messages: Vec<ChatMessage>,

    /// Temperature for response randomness (0.0 to 2.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    /// Maximum tokens to generate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,

    /// Top-p nucleus sampling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,

    /// Frequency penalty (-2.0 to 2.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,

    /// Presence penalty (-2.0 to 2.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,

    /// Stop sequences
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,

    /// User identifier for tracking
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,

    /// Enable streaming
    #[serde(default)]
    pub stream: bool,

    /// Custom metadata for tracing
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
}

impl ChatCompletionRequest {
    /// Create a new chat completion request.
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            messages: Vec::new(),
            temperature: None,
            max_tokens: None,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            user: None,
            stream: false,
            metadata: None,
        }
    }

    /// Add a message to the conversation.
    pub fn with_message(mut self, role: impl Into<String>, content: impl Into<String>) -> Self {
        self.messages.push(ChatMessage {
            role: role.into(),
            content: content.into(),
            name: None,
        });
        self
    }

    /// Add a system message.
    pub fn with_system(self, content: impl Into<String>) -> Self {
        self.with_message("system", content)
    }

    /// Add a user message.
    pub fn with_user(self, content: impl Into<String>) -> Self {
        self.with_message("user", content)
    }

    /// Add an assistant message.
    pub fn with_assistant(self, content: impl Into<String>) -> Self {
        self.with_message("assistant", content)
    }

    /// Set the temperature.
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Set the maximum tokens.
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Set the top-p value.
    pub fn with_top_p(mut self, top_p: f32) -> Self {
        self.top_p = Some(top_p);
        self
    }

    /// Set the frequency penalty.
    pub fn with_frequency_penalty(mut self, penalty: f32) -> Self {
        self.frequency_penalty = Some(penalty);
        self
    }

    /// Set the presence penalty.
    pub fn with_presence_penalty(mut self, penalty: f32) -> Self {
        self.presence_penalty = Some(penalty);
        self
    }

    /// Set stop sequences.
    pub fn with_stop(mut self, stop: Vec<String>) -> Self {
        self.stop = Some(stop);
        self
    }

    /// Set the user identifier.
    pub fn with_user_id(mut self, user: impl Into<String>) -> Self {
        self.user = Some(user.into());
        self
    }

    /// Enable streaming mode.
    pub fn with_streaming(mut self, stream: bool) -> Self {
        self.stream = stream;
        self
    }

    /// Add custom metadata.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata
            .get_or_insert_with(HashMap::new)
            .insert(key.into(), value.into());
        self
    }

    /// Validate the request.
    pub fn validate(&self) -> Result<()> {
        if self.model.is_empty() {
            return Err(Error::invalid_input("model cannot be empty"));
        }
        if self.messages.is_empty() {
            return Err(Error::invalid_input("messages cannot be empty"));
        }
        if let Some(temp) = self.temperature {
            if !(0.0..=2.0).contains(&temp) {
                return Err(Error::invalid_input("temperature must be between 0.0 and 2.0"));
            }
        }
        Ok(())
    }
}

/// Response from a chat completion request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    /// Unique identifier for the completion
    pub id: String,

    /// Generated content
    pub content: String,

    /// Model used for generation
    pub model: String,

    /// Finish reason (stop, length, content_filter, etc.)
    pub finish_reason: Option<String>,

    /// Token usage statistics
    pub usage: TokenUsage,

    /// Cost in USD
    pub cost_usd: f64,

    /// Latency in milliseconds
    pub latency_ms: u64,

    /// OpenTelemetry trace ID
    pub trace_id: String,

    /// OpenTelemetry span ID
    pub span_id: String,

    /// Additional metadata
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl ChatCompletionResponse {
    /// Get the total tokens used.
    pub fn total_tokens(&self) -> u32 {
        self.usage.total_tokens
    }

    /// Get the prompt tokens.
    pub fn prompt_tokens(&self) -> u32 {
        self.usage.prompt_tokens
    }

    /// Get the completion tokens.
    pub fn completion_tokens(&self) -> u32 {
        self.usage.completion_tokens
    }
}

/// Chunk of a streaming response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    /// Chunk identifier
    pub id: String,

    /// Incremental content (delta)
    pub delta: String,

    /// Model being used
    pub model: String,

    /// Finish reason if this is the final chunk
    pub finish_reason: Option<String>,

    /// Partial token count (if available)
    pub partial_tokens: Option<u32>,

    /// Index of this chunk in the stream
    pub index: usize,
}

impl StreamChunk {
    /// Check if this is the final chunk.
    pub fn is_final(&self) -> bool {
        self.finish_reason.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_builder() {
        let request = ChatCompletionRequest::new("gpt-4")
            .with_user("Hello")
            .with_temperature(0.7)
            .with_max_tokens(100);

        assert_eq!(request.model, "gpt-4");
        assert_eq!(request.messages.len(), 1);
        assert_eq!(request.temperature, Some(0.7));
        assert_eq!(request.max_tokens, Some(100));
    }

    #[test]
    fn test_request_validation() {
        let valid_request = ChatCompletionRequest::new("gpt-4").with_user("Hello");
        assert!(valid_request.validate().is_ok());

        let empty_model = ChatCompletionRequest::new("").with_user("Hello");
        assert!(empty_model.validate().is_err());

        let no_messages = ChatCompletionRequest::new("gpt-4");
        assert!(no_messages.validate().is_err());

        let invalid_temp = ChatCompletionRequest::new("gpt-4")
            .with_user("Hello")
            .with_temperature(3.0);
        assert!(invalid_temp.validate().is_err());
    }

    #[test]
    fn test_stream_chunk() {
        let chunk = StreamChunk {
            id: "chunk_1".to_string(),
            delta: "Hello".to_string(),
            model: "gpt-4".to_string(),
            finish_reason: None,
            partial_tokens: Some(10),
            index: 0,
        };

        assert!(!chunk.is_final());

        let final_chunk = StreamChunk {
            finish_reason: Some("stop".to_string()),
            ..chunk
        };

        assert!(final_chunk.is_final());
    }
}
