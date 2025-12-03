// Copyright 2025 LLM Observatory Contributors
// SPDX-License-Identifier: Apache-2.0

//! CostOps adapter for Observatory.
//!
//! This module provides cost analytics and token usage correlation by consuming
//! the llm-cost-ops crate from the LLM-Dev-Ops ecosystem.
//!
//! # Features
//!
//! - Cost calculation from token usage
//! - Token normalization across providers
//! - Cost aggregation for analytics
//! - Usage record creation
//!
//! # Example
//!
//! ```ignore
//! use llm_observatory_adapters::upstream::cost::CostAdapter;
//!
//! let adapter = CostAdapter::new();
//!
//! // Calculate cost from span data
//! let cost = adapter.calculate_cost(&span)?;
//!
//! // Create usage record
//! let usage = adapter.create_usage_record(&span, "org_123")?;
//! ```

use llm_cost_ops::{
    CostAggregator, CostCalculator, CostRecord, CostSummary, Currency, IngestionSource,
    ModelIdentifier, PricingStructure, PricingTable, Provider as CostOpsProvider,
    TokenNormalizer, UsageRecord,
};
use llm_observatory_core::span::LlmSpan;
use llm_observatory_core::types::{Cost, Provider as ObsProvider, TokenUsage};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use uuid::Uuid;

/// Errors that can occur during cost operations.
#[derive(Debug, Error)]
pub enum CostAdapterError {
    /// Missing token usage data
    #[error("Missing token usage data for cost calculation")]
    MissingTokenUsage,

    /// Missing organization ID
    #[error("Missing organization ID")]
    MissingOrganizationId,

    /// Provider not supported
    #[error("Provider not supported: {0}")]
    UnsupportedProvider(String),

    /// Pricing not found
    #[error("Pricing not found for model: {0}")]
    PricingNotFound(String),

    /// Cost calculation error
    #[error("Cost calculation error: {0}")]
    CalculationError(String),

    /// Internal error
    #[error("Internal error: {0}")]
    InternalError(String),
}

/// Result type for cost operations.
pub type Result<T> = std::result::Result<T, CostAdapterError>;

/// Cost breakdown with detailed information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostBreakdown {
    /// Total cost in USD
    pub total_usd: f64,
    /// Input/prompt cost
    pub input_cost: f64,
    /// Output/completion cost
    pub output_cost: f64,
    /// Currency
    pub currency: String,
    /// Provider
    pub provider: String,
    /// Model
    pub model: String,
    /// Token counts
    pub tokens: TokenBreakdown,
}

/// Token usage breakdown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBreakdown {
    /// Input/prompt tokens
    pub input_tokens: u64,
    /// Output/completion tokens
    pub output_tokens: u64,
    /// Total tokens
    pub total_tokens: u64,
    /// Cached tokens (if applicable)
    pub cached_tokens: Option<u64>,
}

/// Aggregated cost summary for reporting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostReport {
    /// Total cost
    pub total_cost: f64,
    /// Total requests
    pub total_requests: u64,
    /// Average cost per request
    pub avg_cost_per_request: f64,
    /// Cost by provider
    pub by_provider: HashMap<String, f64>,
    /// Cost by model
    pub by_model: HashMap<String, f64>,
    /// Cost by project (if available)
    pub by_project: HashMap<String, f64>,
    /// Period start
    pub period_start: DateTime<Utc>,
    /// Period end
    pub period_end: DateTime<Utc>,
}

/// Default pricing data for common models (per 1M tokens).
#[derive(Debug, Clone)]
pub struct DefaultPricing {
    /// Input price per 1M tokens
    pub input_price_per_million: f64,
    /// Output price per 1M tokens
    pub output_price_per_million: f64,
}

impl DefaultPricing {
    /// Get default pricing for a model.
    pub fn for_model(provider: &ObsProvider, model: &str) -> Option<Self> {
        match provider {
            ObsProvider::OpenAI => Self::openai_pricing(model),
            ObsProvider::Anthropic => Self::anthropic_pricing(model),
            ObsProvider::Google => Self::google_pricing(model),
            ObsProvider::Mistral => Self::mistral_pricing(model),
            _ => None,
        }
    }

    fn openai_pricing(model: &str) -> Option<Self> {
        match model {
            m if m.starts_with("gpt-4o-mini") => Some(Self {
                input_price_per_million: 0.15,
                output_price_per_million: 0.60,
            }),
            m if m.starts_with("gpt-4o") => Some(Self {
                input_price_per_million: 2.50,
                output_price_per_million: 10.00,
            }),
            m if m.starts_with("gpt-4-turbo") => Some(Self {
                input_price_per_million: 10.00,
                output_price_per_million: 30.00,
            }),
            m if m.starts_with("gpt-4") => Some(Self {
                input_price_per_million: 30.00,
                output_price_per_million: 60.00,
            }),
            m if m.starts_with("gpt-3.5-turbo") => Some(Self {
                input_price_per_million: 0.50,
                output_price_per_million: 1.50,
            }),
            m if m.starts_with("o1-preview") => Some(Self {
                input_price_per_million: 15.00,
                output_price_per_million: 60.00,
            }),
            m if m.starts_with("o1-mini") => Some(Self {
                input_price_per_million: 3.00,
                output_price_per_million: 12.00,
            }),
            _ => None,
        }
    }

    fn anthropic_pricing(model: &str) -> Option<Self> {
        match model {
            m if m.contains("claude-3-5-sonnet") || m.contains("claude-sonnet-4") => Some(Self {
                input_price_per_million: 3.00,
                output_price_per_million: 15.00,
            }),
            m if m.contains("claude-3-5-haiku") => Some(Self {
                input_price_per_million: 0.80,
                output_price_per_million: 4.00,
            }),
            m if m.contains("claude-3-opus") => Some(Self {
                input_price_per_million: 15.00,
                output_price_per_million: 75.00,
            }),
            m if m.contains("claude-3-sonnet") => Some(Self {
                input_price_per_million: 3.00,
                output_price_per_million: 15.00,
            }),
            m if m.contains("claude-3-haiku") => Some(Self {
                input_price_per_million: 0.25,
                output_price_per_million: 1.25,
            }),
            _ => None,
        }
    }

    fn google_pricing(model: &str) -> Option<Self> {
        match model {
            m if m.contains("gemini-2") && m.contains("pro") => Some(Self {
                input_price_per_million: 1.25,
                output_price_per_million: 5.00,
            }),
            m if m.contains("gemini-2") && m.contains("flash") => Some(Self {
                input_price_per_million: 0.075,
                output_price_per_million: 0.30,
            }),
            m if m.contains("gemini-1.5-pro") => Some(Self {
                input_price_per_million: 1.25,
                output_price_per_million: 5.00,
            }),
            m if m.contains("gemini-1.5-flash") => Some(Self {
                input_price_per_million: 0.075,
                output_price_per_million: 0.30,
            }),
            _ => None,
        }
    }

    fn mistral_pricing(model: &str) -> Option<Self> {
        match model {
            m if m.contains("large") => Some(Self {
                input_price_per_million: 2.00,
                output_price_per_million: 6.00,
            }),
            m if m.contains("small") => Some(Self {
                input_price_per_million: 0.20,
                output_price_per_million: 0.60,
            }),
            _ => None,
        }
    }

    /// Calculate cost from token counts.
    pub fn calculate(&self, input_tokens: u64, output_tokens: u64) -> CostBreakdown {
        let input_cost = (input_tokens as f64 / 1_000_000.0) * self.input_price_per_million;
        let output_cost = (output_tokens as f64 / 1_000_000.0) * self.output_price_per_million;

        CostBreakdown {
            total_usd: input_cost + output_cost,
            input_cost,
            output_cost,
            currency: "USD".to_string(),
            provider: String::new(),
            model: String::new(),
            tokens: TokenBreakdown {
                input_tokens,
                output_tokens,
                total_tokens: input_tokens + output_tokens,
                cached_tokens: None,
            },
        }
    }
}

/// Adapter for consuming llm-cost-ops functionality.
///
/// Provides a simplified interface for Observatory to interact with
/// the LLM-Dev-Ops CostOps for cost analytics and tracking.
pub struct CostAdapter {
    /// Default organization ID
    default_org_id: Option<String>,
    /// Cost records for aggregation
    cost_records: Vec<CostBreakdown>,
}

impl Default for CostAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl CostAdapter {
    /// Create a new CostAdapter.
    pub fn new() -> Self {
        Self {
            default_org_id: None,
            cost_records: Vec::new(),
        }
    }

    /// Create a new CostAdapter with a default organization ID.
    pub fn with_org_id(org_id: impl Into<String>) -> Self {
        Self {
            default_org_id: Some(org_id.into()),
            cost_records: Vec::new(),
        }
    }

    /// Set the default organization ID.
    pub fn set_org_id(&mut self, org_id: impl Into<String>) {
        self.default_org_id = Some(org_id.into());
    }

    /// Calculate cost from an LLM span.
    pub fn calculate_cost(&self, span: &LlmSpan) -> Result<CostBreakdown> {
        let token_usage = span
            .token_usage
            .as_ref()
            .ok_or(CostAdapterError::MissingTokenUsage)?;

        let pricing = DefaultPricing::for_model(&span.provider, &span.model).ok_or_else(|| {
            CostAdapterError::PricingNotFound(format!("{}:{}", span.provider, span.model))
        })?;

        let mut breakdown = pricing.calculate(
            token_usage.prompt_tokens as u64,
            token_usage.completion_tokens as u64,
        );

        breakdown.provider = span.provider.to_string();
        breakdown.model = span.model.clone();

        Ok(breakdown)
    }

    /// Calculate cost from token usage.
    pub fn calculate_cost_from_usage(
        &self,
        provider: &ObsProvider,
        model: &str,
        token_usage: &TokenUsage,
    ) -> Result<CostBreakdown> {
        let pricing = DefaultPricing::for_model(provider, model).ok_or_else(|| {
            CostAdapterError::PricingNotFound(format!("{}:{}", provider, model))
        })?;

        let mut breakdown = pricing.calculate(
            token_usage.prompt_tokens as u64,
            token_usage.completion_tokens as u64,
        );

        breakdown.provider = provider.to_string();
        breakdown.model = model.to_string();

        Ok(breakdown)
    }

    /// Convert Observatory Cost to CostBreakdown.
    pub fn from_observatory_cost(cost: &Cost, provider: &str, model: &str) -> CostBreakdown {
        CostBreakdown {
            total_usd: cost.amount_usd,
            input_cost: cost.prompt_cost.unwrap_or(0.0),
            output_cost: cost.completion_cost.unwrap_or(0.0),
            currency: cost.currency.clone(),
            provider: provider.to_string(),
            model: model.to_string(),
            tokens: TokenBreakdown {
                input_tokens: 0,
                output_tokens: 0,
                total_tokens: 0,
                cached_tokens: None,
            },
        }
    }

    /// Convert CostBreakdown to Observatory Cost.
    pub fn to_observatory_cost(breakdown: &CostBreakdown) -> Cost {
        Cost {
            amount_usd: breakdown.total_usd,
            currency: breakdown.currency.clone(),
            prompt_cost: Some(breakdown.input_cost),
            completion_cost: Some(breakdown.output_cost),
        }
    }

    /// Record a cost breakdown.
    pub fn record_cost(&mut self, breakdown: CostBreakdown) {
        self.cost_records.push(breakdown);
    }

    /// Record cost from a span.
    pub fn record_span_cost(&mut self, span: &LlmSpan) -> Result<()> {
        let breakdown = self.calculate_cost(span)?;
        self.record_cost(breakdown);
        Ok(())
    }

    /// Get total cost from recorded breakdowns.
    pub fn total_cost(&self) -> f64 {
        self.cost_records.iter().map(|c| c.total_usd).sum()
    }

    /// Get cost by provider.
    pub fn cost_by_provider(&self) -> HashMap<String, f64> {
        let mut by_provider = HashMap::new();
        for record in &self.cost_records {
            *by_provider.entry(record.provider.clone()).or_insert(0.0) += record.total_usd;
        }
        by_provider
    }

    /// Get cost by model.
    pub fn cost_by_model(&self) -> HashMap<String, f64> {
        let mut by_model = HashMap::new();
        for record in &self.cost_records {
            *by_model.entry(record.model.clone()).or_insert(0.0) += record.total_usd;
        }
        by_model
    }

    /// Generate a cost report.
    pub fn generate_report(
        &self,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
    ) -> CostReport {
        let total_cost = self.total_cost();
        let total_requests = self.cost_records.len() as u64;

        CostReport {
            total_cost,
            total_requests,
            avg_cost_per_request: if total_requests > 0 {
                total_cost / total_requests as f64
            } else {
                0.0
            },
            by_provider: self.cost_by_provider(),
            by_model: self.cost_by_model(),
            by_project: HashMap::new(),
            period_start,
            period_end,
        }
    }

    /// Clear recorded costs.
    pub fn clear(&mut self) {
        self.cost_records.clear();
    }

    /// Get the number of recorded costs.
    pub fn record_count(&self) -> usize {
        self.cost_records.len()
    }

    /// Map Observatory Provider to CostOps Provider.
    pub fn map_provider(provider: &ObsProvider) -> CostOpsProvider {
        match provider {
            ObsProvider::OpenAI => CostOpsProvider::OpenAI,
            ObsProvider::Anthropic => CostOpsProvider::Anthropic,
            ObsProvider::Google => CostOpsProvider::GoogleVertexAI,
            ObsProvider::Mistral => CostOpsProvider::Mistral,
            ObsProvider::Cohere => CostOpsProvider::Cohere,
            ObsProvider::SelfHosted => CostOpsProvider::Custom("self-hosted".to_string()),
            ObsProvider::Custom(name) => CostOpsProvider::Custom(name.clone()),
        }
    }

    /// Create a model identifier.
    pub fn create_model_id(model: &str, context_window: Option<u64>) -> ModelIdentifier {
        ModelIdentifier::new(model.to_string(), context_window)
    }

    /// Check if a cost exceeds a threshold.
    pub fn exceeds_threshold(cost: f64, threshold_usd: f64) -> bool {
        cost > threshold_usd
    }

    /// Get supported currencies.
    pub fn supported_currencies() -> Vec<Currency> {
        vec![
            Currency::USD,
            Currency::EUR,
            Currency::GBP,
            Currency::JPY,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use llm_observatory_core::span::{LlmInput, SpanStatus};
    use llm_observatory_core::types::Latency;

    fn create_test_span() -> LlmSpan {
        LlmSpan::builder()
            .span_id("span_123")
            .trace_id("trace_456")
            .name("llm.completion")
            .provider(ObsProvider::OpenAI)
            .model("gpt-4o")
            .input(LlmInput::Text {
                prompt: "Hello".to_string(),
            })
            .token_usage(TokenUsage::new(100, 200))
            .latency(Latency::new(Utc::now(), Utc::now()))
            .status(SpanStatus::Ok)
            .build()
            .unwrap()
    }

    #[test]
    fn test_cost_adapter_creation() {
        let adapter = CostAdapter::new();
        assert_eq!(adapter.record_count(), 0);
    }

    #[test]
    fn test_calculate_cost() {
        let adapter = CostAdapter::new();
        let span = create_test_span();

        let breakdown = adapter.calculate_cost(&span).unwrap();
        assert!(breakdown.total_usd > 0.0);
        assert_eq!(breakdown.provider, "openai");
        assert_eq!(breakdown.model, "gpt-4o");
    }

    #[test]
    fn test_default_pricing() {
        // GPT-4o pricing
        let pricing = DefaultPricing::for_model(&ObsProvider::OpenAI, "gpt-4o").unwrap();
        let breakdown = pricing.calculate(1_000_000, 1_000_000);
        assert_eq!(breakdown.input_cost, 2.50);
        assert_eq!(breakdown.output_cost, 10.00);

        // Claude 3.5 Sonnet pricing
        let pricing =
            DefaultPricing::for_model(&ObsProvider::Anthropic, "claude-3-5-sonnet").unwrap();
        let breakdown = pricing.calculate(1_000_000, 1_000_000);
        assert_eq!(breakdown.input_cost, 3.00);
        assert_eq!(breakdown.output_cost, 15.00);
    }

    #[test]
    fn test_record_and_aggregate() {
        let mut adapter = CostAdapter::new();
        let span = create_test_span();

        adapter.record_span_cost(&span).unwrap();
        adapter.record_span_cost(&span).unwrap();

        assert_eq!(adapter.record_count(), 2);
        assert!(adapter.total_cost() > 0.0);

        let by_provider = adapter.cost_by_provider();
        assert!(by_provider.contains_key("openai"));
    }

    #[test]
    fn test_provider_mapping() {
        assert!(matches!(
            CostAdapter::map_provider(&ObsProvider::OpenAI),
            CostOpsProvider::OpenAI
        ));
        assert!(matches!(
            CostAdapter::map_provider(&ObsProvider::Anthropic),
            CostOpsProvider::Anthropic
        ));
    }

    #[test]
    fn test_exceeds_threshold() {
        assert!(CostAdapter::exceeds_threshold(1.5, 1.0));
        assert!(!CostAdapter::exceeds_threshold(0.5, 1.0));
    }
}
