// Copyright 2025 LLM Observatory Contributors
// SPDX-License-Identifier: Apache-2.0

//! Anthropic (Claude) provider implementation.
//!
//! This module provides integration with Anthropic's API including:
//! - Claude model information and pricing
//! - API health checks
//! - Cost calculation for all Claude models

use async_trait::async_trait;
use llm_observatory_core::{
    provider::{LlmProvider, Pricing},
    Error, Result,
};
use serde::{Deserialize, Serialize};

/// Anthropic provider configuration.
#[derive(Debug, Clone)]
pub struct AnthropicProvider {
    /// API key for authentication
    api_key: Option<String>,
    /// Base URL for API (default: https://api.anthropic.com)
    base_url: String,
    /// Anthropic API version
    api_version: String,
}

impl AnthropicProvider {
    /// Create a new Anthropic provider with API key.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: Some(api_key.into()),
            base_url: "https://api.anthropic.com".to_string(),
            api_version: "2023-06-01".to_string(),
        }
    }

    /// Create a new Anthropic provider from environment variables.
    ///
    /// Reads from:
    /// - `ANTHROPIC_API_KEY`: Required API key
    /// - `ANTHROPIC_BASE_URL`: Optional custom base URL
    /// - `ANTHROPIC_API_VERSION`: Optional API version
    pub fn from_env() -> Result<Self> {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| Error::config("ANTHROPIC_API_KEY environment variable not set"))?;

        let mut provider = Self::new(api_key);

        if let Ok(base_url) = std::env::var("ANTHROPIC_BASE_URL") {
            provider.base_url = base_url;
        }

        if let Ok(version) = std::env::var("ANTHROPIC_API_VERSION") {
            provider.api_version = version;
        }

        Ok(provider)
    }

    /// Set custom base URL.
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Set API version.
    pub fn with_api_version(mut self, version: impl Into<String>) -> Self {
        self.api_version = version.into();
        self
    }

    /// Get supported models.
    pub fn supported_models() -> Vec<&'static str> {
        vec![
            // Claude 4 series (Latest - 2025)
            "claude-sonnet-4.5",
            // Claude 3.5 series
            "claude-3-5-sonnet-20241022",
            "claude-3-5-haiku-20241022",
            // Claude 3 series
            "claude-3-opus-20240229",
            "claude-3-sonnet-20240229",
            "claude-3-haiku-20240307",
            // Legacy
            "claude-2.1",
            "claude-2.0",
            "claude-instant-1.2",
        ]
    }

    /// Check if a model is supported.
    pub fn is_model_supported(model: &str) -> bool {
        Self::supported_models().contains(&model)
    }

    /// Get model family (Opus, Sonnet, Haiku).
    pub fn get_model_family(model: &str) -> ModelFamily {
        if model.contains("opus") {
            ModelFamily::Opus
        } else if model.contains("sonnet") {
            ModelFamily::Sonnet
        } else if model.contains("haiku") {
            ModelFamily::Haiku
        } else if model.contains("instant") {
            ModelFamily::Instant
        } else {
            ModelFamily::Unknown
        }
    }

    /// Get model generation (3, 3.5, 4).
    pub fn get_model_generation(model: &str) -> f32 {
        if model.starts_with("claude-4") || model.contains("sonnet-4") {
            4.0
        } else if model.contains("3-5") || model.contains("3.5") {
            3.5
        } else if model.starts_with("claude-3") {
            3.0
        } else if model.starts_with("claude-2") {
            2.0
        } else {
            1.0
        }
    }
}

/// Claude model family classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelFamily {
    /// Opus: Most powerful, highest cost
    Opus,
    /// Sonnet: Balanced performance and cost
    Sonnet,
    /// Haiku: Fastest, lowest cost
    Haiku,
    /// Instant: Legacy fast model
    Instant,
    /// Unknown family
    Unknown,
}

impl ModelFamily {
    /// Get description of the model family.
    pub fn description(&self) -> &'static str {
        match self {
            ModelFamily::Opus => "Most powerful, best for complex tasks",
            ModelFamily::Sonnet => "Balanced intelligence and speed",
            ModelFamily::Haiku => "Fastest responses, cost-effective",
            ModelFamily::Instant => "Legacy fast model",
            ModelFamily::Unknown => "Unknown model family",
        }
    }

    /// Get relative performance tier (1 = fastest/cheapest, 3 = most powerful/expensive).
    pub fn tier(&self) -> u8 {
        match self {
            ModelFamily::Opus => 3,
            ModelFamily::Sonnet => 2,
            ModelFamily::Haiku | ModelFamily::Instant => 1,
            ModelFamily::Unknown => 0,
        }
    }
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    fn name(&self) -> &str {
        "anthropic"
    }

    async fn is_ready(&self) -> Result<bool> {
        if self.api_key.is_none() {
            return Ok(false);
        }

        // In production, we'd make an actual API call to health check endpoint
        // For now, just check configuration
        Ok(true)
    }

    async fn get_pricing(&self, model: &str) -> Result<Pricing> {
        crate::pricing::PRICING_DB.get_pricing(model)
    }
}

impl Default for AnthropicProvider {
    fn default() -> Self {
        Self {
            api_key: None,
            base_url: "https://api.anthropic.com".to_string(),
            api_version: "2023-06-01".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_creation() {
        let provider = AnthropicProvider::new("sk-ant-test-key");
        assert_eq!(provider.name(), "anthropic");
        assert_eq!(provider.api_key, Some("sk-ant-test-key".to_string()));
    }

    #[test]
    fn test_supported_models() {
        let models = AnthropicProvider::supported_models();
        assert!(models.contains(&"claude-3-opus-20240229"));
        assert!(models.contains(&"claude-3-5-sonnet-20241022"));
        assert!(models.contains(&"claude-sonnet-4.5"));
    }

    #[test]
    fn test_model_family() {
        assert_eq!(
            AnthropicProvider::get_model_family("claude-3-opus-20240229"),
            ModelFamily::Opus
        );
        assert_eq!(
            AnthropicProvider::get_model_family("claude-3-5-sonnet-20241022"),
            ModelFamily::Sonnet
        );
        assert_eq!(
            AnthropicProvider::get_model_family("claude-3-haiku-20240307"),
            ModelFamily::Haiku
        );
    }

    #[test]
    fn test_model_generation() {
        assert_eq!(
            AnthropicProvider::get_model_generation("claude-sonnet-4.5"),
            4.0
        );
        assert_eq!(
            AnthropicProvider::get_model_generation("claude-3-5-sonnet-20241022"),
            3.5
        );
        assert_eq!(
            AnthropicProvider::get_model_generation("claude-3-opus-20240229"),
            3.0
        );
    }

    #[test]
    fn test_model_family_tier() {
        assert_eq!(ModelFamily::Opus.tier(), 3);
        assert_eq!(ModelFamily::Sonnet.tier(), 2);
        assert_eq!(ModelFamily::Haiku.tier(), 1);
    }

    #[tokio::test]
    async fn test_is_ready() {
        let provider = AnthropicProvider::new("test-key");
        let ready = provider.is_ready().await.unwrap();
        assert!(ready);
    }

    #[tokio::test]
    async fn test_get_pricing() {
        let provider = AnthropicProvider::new("test-key");
        let pricing = provider
            .get_pricing("claude-3-opus-20240229")
            .await
            .unwrap();
        assert_eq!(pricing.model, "claude-3-opus-20240229");
        assert_eq!(pricing.prompt_cost_per_1k, 0.015);
        assert_eq!(pricing.completion_cost_per_1k, 0.075);
    }

    #[test]
    fn test_model_family_description() {
        assert_eq!(
            ModelFamily::Opus.description(),
            "Most powerful, best for complex tasks"
        );
        assert_eq!(
            ModelFamily::Sonnet.description(),
            "Balanced intelligence and speed"
        );
    }
}
