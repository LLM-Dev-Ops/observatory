// Copyright 2025 LLM Observatory Contributors
// SPDX-License-Identifier: Apache-2.0

//! Provider trait definitions and utilities.

use crate::Result;
use async_trait::async_trait;

/// Trait for LLM provider implementations.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Get the provider name.
    fn name(&self) -> &str;

    /// Check if the provider is configured and ready.
    async fn is_ready(&self) -> Result<bool>;

    /// Get pricing information for a model.
    async fn get_pricing(&self, model: &str) -> Result<Pricing>;
}

/// Pricing information for a model.
#[derive(Debug, Clone)]
pub struct Pricing {
    /// Model name
    pub model: String,
    /// Cost per 1000 prompt tokens (USD)
    pub prompt_cost_per_1k: f64,
    /// Cost per 1000 completion tokens (USD)
    pub completion_cost_per_1k: f64,
}

impl Pricing {
    /// Calculate cost for given token usage.
    pub fn calculate_cost(&self, prompt_tokens: u32, completion_tokens: u32) -> f64 {
        let prompt_cost = (prompt_tokens as f64 / 1000.0) * self.prompt_cost_per_1k;
        let completion_cost = (completion_tokens as f64 / 1000.0) * self.completion_cost_per_1k;
        prompt_cost + completion_cost
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pricing_calculation() {
        let pricing = Pricing {
            model: "gpt-4".to_string(),
            prompt_cost_per_1k: 0.03,
            completion_cost_per_1k: 0.06,
        };

        let cost = pricing.calculate_cost(1000, 500);
        assert!((cost - 0.06).abs() < 0.0001); // 0.03 + 0.03
    }
}
