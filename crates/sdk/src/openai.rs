// Copyright 2025 LLM Observatory Contributors
// SPDX-License-Identifier: Apache-2.0

//! OpenAI client implementation with automatic instrumentation.

use crate::{
    cost::calculate_cost,
    instrument::create_span,
    observatory::LLMObservatory,
    traits::{
        ChatCompletionRequest, ChatCompletionResponse, InstrumentedLLM, StreamChunk,
    },
    Error, Result,
};
use async_trait::async_trait;
use futures::Stream;
use llm_observatory_core::{
    span::{ChatMessage, LlmOutput},
    types::{Provider, TokenUsage},
};
use reqwest::{header, Client};
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::time::Duration;

/// Configuration for OpenAI client.
#[derive(Debug, Clone)]
pub struct OpenAIConfig {
    /// API key for authentication
    pub api_key: String,
    /// Base URL for API (default: https://api.openai.com/v1)
    pub base_url: String,
    /// Request timeout in seconds
    pub timeout_seconds: u64,
    /// Organization ID (optional)
    pub organization: Option<String>,
}

impl OpenAIConfig {
    /// Create a new config with the given API key.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: "https://api.openai.com/v1".to_string(),
            timeout_seconds: 60,
            organization: None,
        }
    }

    /// Set a custom base URL.
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Set the request timeout.
    pub fn with_timeout(mut self, seconds: u64) -> Self {
        self.timeout_seconds = seconds;
        self
    }

    /// Set the organization ID.
    pub fn with_organization(mut self, org: impl Into<String>) -> Self {
        self.organization = Some(org.into());
        self
    }
}

/// OpenAI client with automatic instrumentation.
///
/// This client wraps the OpenAI API with automatic OpenTelemetry tracing,
/// cost calculation, and token usage tracking.
///
/// # Example
///
/// ```rust,no_run
/// use llm_observatory_sdk::{LLMObservatory, OpenAIClient, InstrumentedLLM};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let observatory = LLMObservatory::builder()
///         .with_service_name("my-app")
///         .build()?;
///
///     let client = OpenAIClient::new("sk-...")
///         .with_observatory(observatory);
///
///     let request = llm_observatory_sdk::ChatCompletionRequest::new("gpt-4")
///         .with_user("Hello, how are you?");
///
///     let response = client.chat_completion(request).await?;
///     println!("Response: {}", response.content);
///     println!("Cost: ${:.6}", response.cost_usd);
///
///     Ok(())
/// }
/// ```
pub struct OpenAIClient {
    config: OpenAIConfig,
    client: Client,
    observatory: Option<LLMObservatory>,
}

impl OpenAIClient {
    /// Create a new OpenAI client with the given API key.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self::with_config(OpenAIConfig::new(api_key))
    }

    /// Create a new OpenAI client with custom configuration.
    pub fn with_config(config: OpenAIConfig) -> Self {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            header::HeaderValue::from_str(&format!("Bearer {}", config.api_key))
                .expect("Invalid API key"),
        );
        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json"),
        );

        if let Some(org) = &config.organization {
            headers.insert(
                "OpenAI-Organization",
                header::HeaderValue::from_str(org).expect("Invalid organization"),
            );
        }

        let client = Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            config,
            client,
            observatory: None,
        }
    }

    /// Attach an observatory for automatic instrumentation.
    pub fn with_observatory(mut self, observatory: LLMObservatory) -> Self {
        self.observatory = Some(observatory);
        self
    }

    /// Get the observatory if attached.
    pub fn observatory(&self) -> Option<&LLMObservatory> {
        self.observatory.as_ref()
    }

    /// Execute a chat completion without instrumentation.
    ///
    /// This is useful for testing or when you want to manage tracing manually.
    pub async fn chat_completion_raw(
        &self,
        request: &ChatCompletionRequest,
    ) -> Result<OpenAIChatResponse> {
        request.validate()?;

        let url = format!("{}/chat/completions", self.config.base_url);
        let response = self.client.post(&url).json(request).send().await?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            return Err(Error::api(status.as_u16(), error_body));
        }

        let openai_response: OpenAIChatResponse = response.json().await?;
        Ok(openai_response)
    }
}

#[async_trait]
impl InstrumentedLLM for OpenAIClient {
    async fn chat_completion(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse> {
        request.validate()?;

        // Create instrumented span if observatory is attached
        let mut span = if let Some(observatory) = &self.observatory {
            Some(
                create_span(observatory, Provider::OpenAI, &request.model)
                    .messages(request.messages.clone())
                    .start(),
            )
        } else {
            None
        };

        // Execute the request
        let result = self.chat_completion_raw(&request).await;

        match result {
            Ok(openai_response) => {
                // Extract response data
                let choice = openai_response
                    .choices
                    .first()
                    .ok_or_else(|| Error::internal("No choices in response"))?;

                let content = choice.message.content.clone();
                let finish_reason = choice.finish_reason.clone();

                // Build token usage
                let usage = TokenUsage::new(
                    openai_response.usage.prompt_tokens,
                    openai_response.usage.completion_tokens,
                );

                // Calculate cost
                let cost = calculate_cost(&request.model, &usage)?;

                // Create LLM output
                let output = LlmOutput {
                    content: content.clone(),
                    finish_reason: Some(finish_reason.clone()),
                    metadata: Default::default(),
                };

                // Finish the span
                let (trace_id, span_id, latency_ms) = if let Some(span) = span.take() {
                    let llm_span = span.finish_success(output, usage.clone(), cost.clone())?;
                    (
                        llm_span.trace_id.clone(),
                        llm_span.span_id.clone(),
                        llm_span.latency.total_ms,
                    )
                } else {
                    (String::new(), String::new(), 0)
                };

                Ok(ChatCompletionResponse {
                    id: openai_response.id,
                    content,
                    model: openai_response.model,
                    finish_reason: Some(finish_reason),
                    usage,
                    cost_usd: cost.amount_usd,
                    latency_ms,
                    trace_id,
                    span_id,
                    metadata: request.metadata.unwrap_or_default(),
                })
            }
            Err(e) => {
                // Finish span with error
                if let Some(span) = span.take() {
                    let _ = span.finish_error(&e.to_string());
                }
                Err(e)
            }
        }
    }

    async fn streaming_completion(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>> {
        request.validate()?;

        // For now, return an error as streaming requires more complex implementation
        // In a full implementation, this would use SSE (Server-Sent Events)
        Err(Error::internal(
            "Streaming not yet implemented. Use chat_completion for non-streaming requests.",
        ))
    }

    fn provider_name(&self) -> &str {
        "openai"
    }

    fn default_model(&self) -> Option<&str> {
        Some("gpt-4o")
    }
}

// OpenAI API types

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenAIChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    frequency_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    presence_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    user: Option<String>,
    #[serde(default)]
    stream: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIChatResponse {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<OpenAIChoice>,
    pub usage: OpenAIUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIChoice {
    pub index: usize,
    pub message: ChatMessage,
    pub finish_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_builder() {
        let config = OpenAIConfig::new("test-key")
            .with_base_url("https://custom.api.com")
            .with_timeout(120)
            .with_organization("org-123");

        assert_eq!(config.api_key, "test-key");
        assert_eq!(config.base_url, "https://custom.api.com");
        assert_eq!(config.timeout_seconds, 120);
        assert_eq!(config.organization, Some("org-123".to_string()));
    }

    #[test]
    fn test_client_creation() {
        let client = OpenAIClient::new("test-key");
        assert!(client.observatory.is_none());
        assert_eq!(client.provider_name(), "openai");
        assert_eq!(client.default_model(), Some("gpt-4o"));
    }
}
