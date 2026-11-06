// Copyright 2025 LLM Observatory Contributors
// SPDX-License-Identifier: Apache-2.0

//! OpenAI provider implementation.
//!
//! This module provides integration with OpenAI's API including:
//! - Model information and pricing
//! - API health checks
//! - Cost calculation for all GPT models

use async_trait::async_trait;
use llm_observatory_core::{
    provider::{LlmProvider, Pricing},
    Error, Result,
};
use serde::{Deserialize, Serialize};

/// OpenAI provider configuration.
#[derive(Debug, Clone)]
pub struct OpenAiProvider {
    /// API key for authentication
    api_key: Option<String>,
    /// Base URL for API (default: https://api.openai.com/v1)
    base_url: String,
    /// Organization ID (optional)
    organization_id: Option<String>,
}

impl OpenAiProvider {
    /// Create a new OpenAI provider with API key.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: Some(api_key.into()),
            base_url: "https://api.openai.com/v1".to_string(),
            organization_id: None,
        }
    }

    /// Create a new OpenAI provider from environment variables.
    ///
    /// Reads from:
    /// - `OPENAI_API_KEY`: Required API key
    /// - `OPENAI_ORGANIZATION`: Optional organization ID
    /// - `OPENAI_BASE_URL`: Optional custom base URL
    pub fn from_env() -> Result<Self> {
        let api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| Error::config("OPENAI_API_KEY environment variable not set"))?;

        let mut provider = Self::new(api_key);

        if let Ok(org_id) = std::env::var("OPENAI_ORGANIZATION") {
            provider.organization_id = Some(org_id);
        }

        if let Ok(base_url) = std::env::var("OPENAI_BASE_URL") {
            provider.base_url = base_url;
        }

        Ok(provider)
    }

    /// Set organization ID.
    pub fn with_organization(mut self, org_id: impl Into<String>) -> Self {
        self.organization_id = Some(org_id.into());
        self
    }

    /// Set custom base URL (for Azure OpenAI or proxies).
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Get supported models.
    pub fn supported_models() -> Vec<&'static str> {
        vec![
            // GPT-4o series
            "gpt-4o",
            "gpt-4o-mini",
            // GPT-4 Turbo
            "gpt-4-turbo",
            "gpt-4-turbo-preview",
            // GPT-4
            "gpt-4",
            "gpt-4-32k",
            // GPT-3.5 Turbo
            "gpt-3.5-turbo",
            "gpt-3.5-turbo-16k",
            // o1 series (reasoning models)
            "o1-preview",
            "o1-mini",
            // Legacy
            "text-davinci-003",
            "text-davinci-002",
        ]
    }

    /// Check if a model is supported.
    pub fn is_model_supported(model: &str) -> bool {
        Self::supported_models().contains(&model)
    }

    /// Get model tier (helps determine features and pricing).
    pub fn get_model_tier(model: &str) -> ModelTier {
        match model {
            m if m.starts_with("gpt-4o") => ModelTier::Flagship,
            m if m.starts_with("o1-") => ModelTier::Reasoning,
            m if m.starts_with("gpt-4") => ModelTier::Advanced,
            m if m.starts_with("gpt-3.5") => ModelTier::Standard,
            _ => ModelTier::Legacy,
        }
    }
}

/// Model tier classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelTier {
    /// Latest flagship models (GPT-4o)
    Flagship,
    /// Reasoning models (o1 series)
    Reasoning,
    /// Advanced models (GPT-4)
    Advanced,
    /// Standard models (GPT-3.5)
    Standard,
    /// Legacy models
    Legacy,
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    fn name(&self) -> &str {
        "openai"
    }

    async fn is_ready(&self) -> Result<bool> {
        if self.api_key.is_none() {
            return Ok(false);
        }

        // In production, we'd make an actual API call to /models
        // For now, just check configuration
        Ok(true)
    }

    async fn get_pricing(&self, model: &str) -> Result<Pricing> {
        crate::pricing::PRICING_DB.get_pricing(model)
    }
}

impl Default for OpenAiProvider {
    fn default() -> Self {
        Self {
            api_key: None,
            base_url: "https://api.openai.com/v1".to_string(),
            organization_id: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_creation() {
        let provider = OpenAiProvider::new("sk-test-key");
        assert_eq!(provider.name(), "openai");
        assert_eq!(provider.api_key, Some("sk-test-key".to_string()));
    }

    #[test]
    fn test_with_organization() {
        let provider = OpenAiProvider::new("key").with_organization("org-123");
        assert_eq!(provider.organization_id, Some("org-123".to_string()));
    }

    #[test]
    fn test_supported_models() {
        let models = OpenAiProvider::supported_models();
        assert!(models.contains(&"gpt-4o"));
        assert!(models.contains(&"gpt-4"));
        assert!(models.contains(&"gpt-3.5-turbo"));
    }

    #[test]
    fn test_model_tier() {
        assert_eq!(
            OpenAiProvider::get_model_tier("gpt-4o"),
            ModelTier::Flagship
        );
        assert_eq!(
            OpenAiProvider::get_model_tier("o1-preview"),
            ModelTier::Reasoning
        );
        assert_eq!(OpenAiProvider::get_model_tier("gpt-4"), ModelTier::Advanced);
        assert_eq!(
            OpenAiProvider::get_model_tier("gpt-3.5-turbo"),
            ModelTier::Standard
        );
    }

    #[tokio::test]
    async fn test_is_ready() {
        let provider = OpenAiProvider::new("test-key");
        let ready = provider.is_ready().await.unwrap();
        assert!(ready);
    }

    #[tokio::test]
    async fn test_get_pricing() {
        let provider = OpenAiProvider::new("test-key");
        let pricing = provider.get_pricing("gpt-4").await.unwrap();
        assert_eq!(pricing.model, "gpt-4");
        assert_eq!(pricing.prompt_cost_per_1k, 0.03);
        assert_eq!(pricing.completion_cost_per_1k, 0.06);
    }
}
