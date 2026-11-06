// Copyright 2025 LLM Observatory Contributors
// SPDX-License-Identifier: Apache-2.0

//! Pricing engine with real-world LLM pricing data.
//!
//! This module maintains up-to-date pricing information for all major LLM providers
//! based on official pricing pages. Prices are updated as of January 2025.

use llm_observatory_core::{provider::Pricing, Error, Result};
use once_cell::sync::Lazy;
use std::collections::HashMap;

/// Global pricing database singleton.
pub static PRICING_DB: Lazy<PricingDatabase> = Lazy::new(PricingDatabase::new);

/// Comprehensive pricing database for LLM models.
#[derive(Debug, Clone)]
pub struct PricingDatabase {
    prices: HashMap<String, Pricing>,
}

impl PricingDatabase {
    /// Create a new pricing database with current pricing data.
    pub fn new() -> Self {
        let mut db = Self {
            prices: HashMap::new(),
        };
        db.load_openai_pricing();
        db.load_anthropic_pricing();
        db.load_google_pricing();
        db.load_mistral_pricing();
        db
    }

    /// Get pricing for a specific model.
    pub fn get_pricing(&self, model: &str) -> Result<Pricing> {
        self.prices
            .get(model)
            .cloned()
            .ok_or_else(|| Error::not_found(format!("Pricing not found for model: {}", model)))
    }

    /// Check if pricing exists for a model.
    pub fn has_pricing(&self, model: &str) -> bool {
        self.prices.contains_key(model)
    }

    /// List all models with pricing data.
    pub fn list_models(&self) -> Vec<String> {
        self.prices.keys().cloned().collect()
    }

    /// Add custom pricing for a model.
    pub fn add_pricing(&mut self, pricing: Pricing) {
        self.prices.insert(pricing.model.clone(), pricing);
    }

    // OpenAI Pricing (as of January 2025)
    // Source: https://openai.com/api/pricing/
    fn load_openai_pricing(&mut self) {
        // GPT-4o (Latest flagship model)
        self.prices.insert(
            "gpt-4o".to_string(),
            Pricing {
                model: "gpt-4o".to_string(),
                prompt_cost_per_1k: 0.0025,    // $2.50 per 1M input tokens
                completion_cost_per_1k: 0.010, // $10.00 per 1M output tokens
            },
        );

        // GPT-4o mini (Cost-effective variant)
        self.prices.insert(
            "gpt-4o-mini".to_string(),
            Pricing {
                model: "gpt-4o-mini".to_string(),
                prompt_cost_per_1k: 0.00015, // $0.15 per 1M input tokens
                completion_cost_per_1k: 0.0006, // $0.60 per 1M output tokens
            },
        );

        // GPT-4 Turbo
        self.prices.insert(
            "gpt-4-turbo".to_string(),
            Pricing {
                model: "gpt-4-turbo".to_string(),
                prompt_cost_per_1k: 0.01,     // $10 per 1M input tokens
                completion_cost_per_1k: 0.03, // $30 per 1M output tokens
            },
        );

        // GPT-4 (Original)
        self.prices.insert(
            "gpt-4".to_string(),
            Pricing {
                model: "gpt-4".to_string(),
                prompt_cost_per_1k: 0.03,     // $30 per 1M input tokens
                completion_cost_per_1k: 0.06, // $60 per 1M output tokens
            },
        );

        // GPT-3.5 Turbo
        self.prices.insert(
            "gpt-3.5-turbo".to_string(),
            Pricing {
                model: "gpt-3.5-turbo".to_string(),
                prompt_cost_per_1k: 0.0005, // $0.50 per 1M input tokens
                completion_cost_per_1k: 0.0015, // $1.50 per 1M output tokens
            },
        );

        // o1-preview (Reasoning model)
        self.prices.insert(
            "o1-preview".to_string(),
            Pricing {
                model: "o1-preview".to_string(),
                prompt_cost_per_1k: 0.015,    // $15 per 1M input tokens
                completion_cost_per_1k: 0.06, // $60 per 1M output tokens
            },
        );

        // o1-mini (Cost-effective reasoning)
        self.prices.insert(
            "o1-mini".to_string(),
            Pricing {
                model: "o1-mini".to_string(),
                prompt_cost_per_1k: 0.003,     // $3 per 1M input tokens
                completion_cost_per_1k: 0.012, // $12 per 1M output tokens
            },
        );
    }

    // Anthropic Pricing (as of January 2025)
    // Source: https://www.anthropic.com/api
    fn load_anthropic_pricing(&mut self) {
        // Claude Sonnet 4.5 (Latest flagship - announced Jan 2025)
        self.prices.insert(
            "claude-sonnet-4.5".to_string(),
            Pricing {
                model: "claude-sonnet-4.5".to_string(),
                prompt_cost_per_1k: 0.003,     // $3 per 1M input tokens
                completion_cost_per_1k: 0.015, // $15 per 1M output tokens
            },
        );

        // Claude 3.5 Sonnet
        self.prices.insert(
            "claude-3-5-sonnet-20241022".to_string(),
            Pricing {
                model: "claude-3-5-sonnet-20241022".to_string(),
                prompt_cost_per_1k: 0.003,     // $3 per 1M input tokens
                completion_cost_per_1k: 0.015, // $15 per 1M output tokens
            },
        );

        // Claude 3.5 Haiku
        self.prices.insert(
            "claude-3-5-haiku-20241022".to_string(),
            Pricing {
                model: "claude-3-5-haiku-20241022".to_string(),
                prompt_cost_per_1k: 0.001,     // $1 per 1M input tokens
                completion_cost_per_1k: 0.005, // $5 per 1M output tokens
            },
        );

        // Claude 3 Opus
        self.prices.insert(
            "claude-3-opus-20240229".to_string(),
            Pricing {
                model: "claude-3-opus-20240229".to_string(),
                prompt_cost_per_1k: 0.015,     // $15 per 1M input tokens
                completion_cost_per_1k: 0.075, // $75 per 1M output tokens
            },
        );

        // Claude 3 Sonnet
        self.prices.insert(
            "claude-3-sonnet-20240229".to_string(),
            Pricing {
                model: "claude-3-sonnet-20240229".to_string(),
                prompt_cost_per_1k: 0.003,     // $3 per 1M input tokens
                completion_cost_per_1k: 0.015, // $15 per 1M output tokens
            },
        );

        // Claude 3 Haiku
        self.prices.insert(
            "claude-3-haiku-20240307".to_string(),
            Pricing {
                model: "claude-3-haiku-20240307".to_string(),
                prompt_cost_per_1k: 0.00025, // $0.25 per 1M input tokens
                completion_cost_per_1k: 0.00125, // $1.25 per 1M output tokens
            },
        );
    }

    // Google Gemini Pricing (as of January 2025)
    // Source: https://ai.google.dev/pricing
    fn load_google_pricing(&mut self) {
        // Gemini 2.5 Pro (Latest)
        self.prices.insert(
            "gemini-2.5-pro".to_string(),
            Pricing {
                model: "gemini-2.5-pro".to_string(),
                prompt_cost_per_1k: 0.00125, // $1.25 per 1M input tokens
                completion_cost_per_1k: 0.005, // $5 per 1M output tokens
            },
        );

        // Gemini 2.5 Flash
        self.prices.insert(
            "gemini-2.5-flash".to_string(),
            Pricing {
                model: "gemini-2.5-flash".to_string(),
                prompt_cost_per_1k: 0.000075, // $0.075 per 1M input tokens
                completion_cost_per_1k: 0.0003, // $0.30 per 1M output tokens
            },
        );

        // Gemini 1.5 Pro
        self.prices.insert(
            "gemini-1.5-pro".to_string(),
            Pricing {
                model: "gemini-1.5-pro".to_string(),
                prompt_cost_per_1k: 0.00125, // $1.25 per 1M input tokens
                completion_cost_per_1k: 0.005, // $5 per 1M output tokens
            },
        );

        // Gemini 1.5 Flash
        self.prices.insert(
            "gemini-1.5-flash".to_string(),
            Pricing {
                model: "gemini-1.5-flash".to_string(),
                prompt_cost_per_1k: 0.000075, // $0.075 per 1M input tokens
                completion_cost_per_1k: 0.0003, // $0.30 per 1M output tokens
            },
        );
    }

    // Mistral AI Pricing (as of January 2025)
    // Source: https://mistral.ai/technology/#pricing
    fn load_mistral_pricing(&mut self) {
        // Mistral Large
        self.prices.insert(
            "mistral-large-latest".to_string(),
            Pricing {
                model: "mistral-large-latest".to_string(),
                prompt_cost_per_1k: 0.002,     // $2 per 1M input tokens
                completion_cost_per_1k: 0.006, // $6 per 1M output tokens
            },
        );

        // Mistral Small
        self.prices.insert(
            "mistral-small-latest".to_string(),
            Pricing {
                model: "mistral-small-latest".to_string(),
                prompt_cost_per_1k: 0.0002, // $0.20 per 1M input tokens
                completion_cost_per_1k: 0.0006, // $0.60 per 1M output tokens
            },
        );

        // Mistral 7B (Open source, self-hosted - estimated costs)
        self.prices.insert(
            "mistral-7b".to_string(),
            Pricing {
                model: "mistral-7b".to_string(),
                prompt_cost_per_1k: 0.0, // Free (self-hosted)
                completion_cost_per_1k: 0.0,
            },
        );
    }
}

impl Default for PricingDatabase {
    fn default() -> Self {
        Self::new()
    }
}

/// Pricing engine for calculating LLM costs.
pub struct PricingEngine;

impl PricingEngine {
    /// Calculate cost for a given model and token usage.
    ///
    /// # Arguments
    /// * `model` - Model identifier (e.g., "gpt-4", "claude-3-opus")
    /// * `prompt_tokens` - Number of input tokens
    /// * `completion_tokens` - Number of output tokens
    ///
    /// # Returns
    /// Total cost in USD
    pub fn calculate_cost(model: &str, prompt_tokens: u32, completion_tokens: u32) -> Result<f64> {
        let pricing = PRICING_DB.get_pricing(model)?;
        Ok(pricing.calculate_cost(prompt_tokens, completion_tokens))
    }

    /// Calculate cost breakdown for a given model and token usage.
    ///
    /// # Returns
    /// (prompt_cost, completion_cost, total_cost) in USD
    pub fn calculate_cost_breakdown(
        model: &str,
        prompt_tokens: u32,
        completion_tokens: u32,
    ) -> Result<(f64, f64, f64)> {
        let pricing = PRICING_DB.get_pricing(model)?;
        let prompt_cost = (prompt_tokens as f64 / 1000.0) * pricing.prompt_cost_per_1k;
        let completion_cost = (completion_tokens as f64 / 1000.0) * pricing.completion_cost_per_1k;
        let total_cost = prompt_cost + completion_cost;
        Ok((prompt_cost, completion_cost, total_cost))
    }

    /// Estimate cost for a given model and approximate token count.
    pub fn estimate_cost(model: &str, estimated_tokens: u32) -> Result<f64> {
        // Assume 70/30 split between prompt and completion (common pattern)
        let prompt_tokens = (estimated_tokens as f64 * 0.7) as u32;
        let completion_tokens = (estimated_tokens as f64 * 0.3) as u32;
        Self::calculate_cost(model, prompt_tokens, completion_tokens)
    }

    /// Compare costs across different models for the same token usage.
    pub fn compare_costs(
        models: &[&str],
        prompt_tokens: u32,
        completion_tokens: u32,
    ) -> Vec<(String, Result<f64>)> {
        models
            .iter()
            .map(|model| {
                let cost = Self::calculate_cost(model, prompt_tokens, completion_tokens);
                (model.to_string(), cost)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_gpt4_pricing() {
        let cost = PricingEngine::calculate_cost("gpt-4", 1000, 500).unwrap();
        // $0.03 for 1k prompt + $0.03 for 500 completion = $0.06
        assert!((cost - 0.06).abs() < 0.0001);
    }

    #[test]
    fn test_anthropic_claude_pricing() {
        let cost = PricingEngine::calculate_cost("claude-3-opus-20240229", 1000, 1000).unwrap();
        // $0.015 for 1k prompt + $0.075 for 1k completion = $0.09
        assert!((cost - 0.09).abs() < 0.0001);
    }

    #[test]
    fn test_cost_breakdown() {
        let (prompt_cost, completion_cost, total) =
            PricingEngine::calculate_cost_breakdown("gpt-4o", 1000, 1000).unwrap();

        assert!((prompt_cost - 0.0025).abs() < 0.0001);
        assert!((completion_cost - 0.010).abs() < 0.0001);
        assert!((total - 0.0125).abs() < 0.0001);
    }

    #[test]
    fn test_model_comparison() {
        let models = vec!["gpt-4o", "gpt-4o-mini", "claude-3-5-sonnet-20241022"];
        let comparisons = PricingEngine::compare_costs(&models, 1000, 1000);

        assert_eq!(comparisons.len(), 3);
        for (model, result) in comparisons {
            assert!(result.is_ok(), "Failed for model: {}", model);
        }
    }

    #[test]
    fn test_unknown_model() {
        let result = PricingEngine::calculate_cost("unknown-model", 1000, 1000);
        assert!(result.is_err());
    }

    #[test]
    fn test_pricing_database_list() {
        let models = PRICING_DB.list_models();
        assert!(models.len() > 15); // Should have at least 15 models
        assert!(models.contains(&"gpt-4".to_string()));
        assert!(models.contains(&"claude-3-opus-20240229".to_string()));
    }
}
