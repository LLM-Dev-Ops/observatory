// Copyright 2025 LLM Observatory Contributors
// SPDX-License-Identifier: Apache-2.0

//! Cost calculation utilities for LLM operations.

use crate::{Error, Result};
use llm_observatory_core::types::{Cost, TokenUsage};
use llm_observatory_providers::pricing::{PricingEngine, PRICING_DB};

/// Calculate the cost of an LLM operation.
///
/// This function uses the pricing database to calculate the cost based on
/// token usage and model pricing.
///
/// # Arguments
///
/// * `model` - The model identifier (e.g., "gpt-4", "claude-3-opus-20240229")
/// * `usage` - Token usage statistics
///
/// # Returns
///
/// A [`Cost`] struct with detailed cost breakdown
///
/// # Example
///
/// ```rust
/// use llm_observatory_sdk::{cost::calculate_cost, TokenUsage};
///
/// let usage = TokenUsage::new(1000, 500);
/// let cost = calculate_cost("gpt-4", &usage).unwrap();
/// println!("Total cost: ${:.6}", cost.amount_usd);
/// ```
pub fn calculate_cost(model: &str, usage: &TokenUsage) -> Result<Cost> {
    let (prompt_cost, completion_cost, _total_cost) = PricingEngine::calculate_cost_breakdown(
        model,
        usage.prompt_tokens,
        usage.completion_tokens,
    )
    .map_err(|e| Error::CostCalculation(e.to_string()))?;

    Ok(Cost::with_breakdown(prompt_cost, completion_cost))
}

/// Calculate the cost with a fallback for unknown models.
///
/// If the model is not in the pricing database, this function will use
/// a default cost estimate based on average pricing.
///
/// # Arguments
///
/// * `model` - The model identifier
/// * `usage` - Token usage statistics
/// * `fallback_prompt_cost_per_1k` - Fallback cost per 1000 prompt tokens
/// * `fallback_completion_cost_per_1k` - Fallback cost per 1000 completion tokens
pub fn calculate_cost_with_fallback(
    model: &str,
    usage: &TokenUsage,
    fallback_prompt_cost_per_1k: f64,
    fallback_completion_cost_per_1k: f64,
) -> Cost {
    match calculate_cost(model, usage) {
        Ok(cost) => cost,
        Err(_) => {
            // Use fallback pricing
            let prompt_cost = (usage.prompt_tokens as f64 / 1000.0) * fallback_prompt_cost_per_1k;
            let completion_cost =
                (usage.completion_tokens as f64 / 1000.0) * fallback_completion_cost_per_1k;
            Cost::with_breakdown(prompt_cost, completion_cost)
        }
    }
}

/// Estimate the cost of a request before making it.
///
/// This function provides a rough estimate based on expected token counts.
/// The actual cost may vary based on the actual token usage.
///
/// # Arguments
///
/// * `model` - The model identifier
/// * `estimated_prompt_tokens` - Estimated prompt tokens
/// * `estimated_completion_tokens` - Estimated completion tokens
pub fn estimate_cost(
    model: &str,
    estimated_prompt_tokens: u32,
    estimated_completion_tokens: u32,
) -> Result<f64> {
    let cost = PricingEngine::calculate_cost(model, estimated_prompt_tokens, estimated_completion_tokens)
        .map_err(|e| Error::CostCalculation(e.to_string()))?;
    Ok(cost)
}

/// Get pricing information for a specific model.
///
/// # Arguments
///
/// * `model` - The model identifier
///
/// # Returns
///
/// Cost per 1000 tokens for prompt and completion
pub fn get_model_pricing(model: &str) -> Result<(f64, f64)> {
    let pricing = PRICING_DB
        .get_pricing(model)
        .map_err(|e| Error::CostCalculation(e.to_string()))?;

    Ok((pricing.prompt_cost_per_1k, pricing.completion_cost_per_1k))
}

/// Check if pricing is available for a model.
pub fn has_pricing(model: &str) -> bool {
    PRICING_DB.has_pricing(model)
}

/// List all models with available pricing.
pub fn list_models_with_pricing() -> Vec<String> {
    PRICING_DB.list_models()
}

/// Cost statistics aggregator for tracking cumulative costs.
#[derive(Debug, Clone, Default)]
pub struct CostTracker {
    total_cost: f64,
    total_prompt_tokens: u32,
    total_completion_tokens: u32,
    request_count: usize,
    model_costs: std::collections::HashMap<String, f64>,
}

impl CostTracker {
    /// Create a new cost tracker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a cost.
    pub fn record(&mut self, model: &str, cost: &Cost, usage: &TokenUsage) {
        self.total_cost += cost.amount_usd;
        self.total_prompt_tokens += usage.prompt_tokens;
        self.total_completion_tokens += usage.completion_tokens;
        self.request_count += 1;

        *self.model_costs.entry(model.to_string()).or_insert(0.0) += cost.amount_usd;
    }

    /// Get the total cost.
    pub fn total_cost(&self) -> f64 {
        self.total_cost
    }

    /// Get the total prompt tokens.
    pub fn total_prompt_tokens(&self) -> u32 {
        self.total_prompt_tokens
    }

    /// Get the total completion tokens.
    pub fn total_completion_tokens(&self) -> u32 {
        self.total_completion_tokens
    }

    /// Get the total tokens.
    pub fn total_tokens(&self) -> u32 {
        self.total_prompt_tokens + self.total_completion_tokens
    }

    /// Get the number of requests.
    pub fn request_count(&self) -> usize {
        self.request_count
    }

    /// Get the average cost per request.
    pub fn average_cost(&self) -> f64 {
        if self.request_count == 0 {
            0.0
        } else {
            self.total_cost / self.request_count as f64
        }
    }

    /// Get cost breakdown by model.
    pub fn costs_by_model(&self) -> &std::collections::HashMap<String, f64> {
        &self.model_costs
    }

    /// Reset the tracker.
    pub fn reset(&mut self) {
        self.total_cost = 0.0;
        self.total_prompt_tokens = 0;
        self.total_completion_tokens = 0;
        self.request_count = 0;
        self.model_costs.clear();
    }

    /// Get a summary as a formatted string.
    pub fn summary(&self) -> String {
        format!(
            "Requests: {}, Total Cost: ${:.6}, Avg Cost: ${:.6}, Tokens: {} (prompt: {}, completion: {})",
            self.request_count,
            self.total_cost,
            self.average_cost(),
            self.total_tokens(),
            self.total_prompt_tokens,
            self.total_completion_tokens
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_cost() {
        let usage = TokenUsage::new(1000, 500);
        let cost = calculate_cost("gpt-4", &usage).unwrap();

        // GPT-4 pricing: $0.03 per 1k prompt, $0.06 per 1k completion
        // Expected: (1000/1000 * 0.03) + (500/1000 * 0.06) = 0.03 + 0.03 = 0.06
        assert!((cost.amount_usd - 0.06).abs() < 0.0001);
    }

    #[test]
    fn test_calculate_cost_with_fallback() {
        let usage = TokenUsage::new(1000, 500);

        // Test with known model
        let cost = calculate_cost_with_fallback("gpt-4", &usage, 0.01, 0.02);
        assert!((cost.amount_usd - 0.06).abs() < 0.0001);

        // Test with unknown model (should use fallback)
        let cost = calculate_cost_with_fallback("unknown-model", &usage, 0.01, 0.02);
        // Expected: (1000/1000 * 0.01) + (500/1000 * 0.02) = 0.01 + 0.01 = 0.02
        assert!((cost.amount_usd - 0.02).abs() < 0.0001);
    }

    #[test]
    fn test_has_pricing() {
        assert!(has_pricing("gpt-4"));
        assert!(has_pricing("claude-3-opus-20240229"));
        assert!(!has_pricing("unknown-model"));
    }

    #[test]
    fn test_cost_tracker() {
        let mut tracker = CostTracker::new();

        let usage1 = TokenUsage::new(1000, 500);
        let cost1 = calculate_cost("gpt-4", &usage1).unwrap();
        tracker.record("gpt-4", &cost1, &usage1);

        assert_eq!(tracker.request_count(), 1);
        assert!((tracker.total_cost() - 0.06).abs() < 0.0001);
        assert_eq!(tracker.total_prompt_tokens(), 1000);
        assert_eq!(tracker.total_completion_tokens(), 500);

        let usage2 = TokenUsage::new(500, 250);
        let cost2 = calculate_cost("gpt-4", &usage2).unwrap();
        tracker.record("gpt-4", &cost2, &usage2);

        assert_eq!(tracker.request_count(), 2);
        assert_eq!(tracker.total_prompt_tokens(), 1500);
        assert_eq!(tracker.total_completion_tokens(), 750);

        // Test reset
        tracker.reset();
        assert_eq!(tracker.request_count(), 0);
        assert_eq!(tracker.total_cost(), 0.0);
    }

    #[test]
    fn test_cost_tracker_average() {
        let mut tracker = CostTracker::new();

        let usage = TokenUsage::new(1000, 500);
        let cost = calculate_cost("gpt-4", &usage).unwrap();

        tracker.record("gpt-4", &cost, &usage);
        tracker.record("gpt-4", &cost, &usage);

        assert!((tracker.average_cost() - 0.06).abs() < 0.0001);
    }
}
