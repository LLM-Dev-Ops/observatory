// Copyright 2025 LLM Observatory Contributors
// SPDX-License-Identifier: Apache-2.0

//! Cost calculation processor.
//!
//! This processor automatically calculates the cost of LLM requests based on:
//! - Token usage (prompt + completion tokens)
//! - Model pricing (from pricing database)
//! - Provider-specific pricing rules

use super::SpanProcessor;
use async_trait::async_trait;
use llm_observatory_core::{
    span::LlmSpan,
    types::Cost,
    Result,
};
use llm_observatory_providers::PricingEngine;

/// Cost calculation processor.
#[derive(Debug, Clone, Default)]
pub struct CostCalculationProcessor {
    /// Enable cost breakdown
    include_breakdown: bool,
}

impl CostCalculationProcessor {
    /// Create a new cost calculation processor.
    pub fn new() -> Self {
        Self {
            include_breakdown: true,
        }
    }

    /// Set whether to include cost breakdown.
    pub fn with_breakdown(mut self, include_breakdown: bool) -> Self {
        self.include_breakdown = include_breakdown;
        self
    }

    /// Calculate cost for a span.
    fn calculate_cost(&self, span: &LlmSpan) -> Result<Option<Cost>> {
        // Only calculate if we have token usage
        let usage = match &span.token_usage {
            Some(u) => u,
            None => return Ok(None),
        };

        if self.include_breakdown {
            // Calculate with breakdown
            let (prompt_cost, completion_cost, _total) = PricingEngine::calculate_cost_breakdown(
                &span.model,
                usage.prompt_tokens,
                usage.completion_tokens,
            )?;

            Ok(Some(Cost::with_breakdown(prompt_cost, completion_cost)))
        } else {
            // Calculate total only
            let total = PricingEngine::calculate_cost(
                &span.model,
                usage.prompt_tokens,
                usage.completion_tokens,
            )?;

            Ok(Some(Cost::new(total)))
        }
    }
}

#[async_trait]
impl SpanProcessor for CostCalculationProcessor {
    async fn process(&self, mut span: LlmSpan) -> Result<Option<LlmSpan>> {
        // Only calculate if cost is not already set
        if span.cost.is_none() {
            if let Ok(Some(cost)) = self.calculate_cost(&span) {
                span.cost = Some(cost);
            }
            // If calculation fails (e.g., unknown model), just skip
        }

        Ok(Some(span))
    }

    fn name(&self) -> &str {
        "cost_calculation"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use llm_observatory_core::{
        span::{LlmSpan, LlmInput, SpanStatus},
        types::{Provider, Latency, TokenUsage},
    };
    use chrono::Utc;

    #[tokio::test]
    async fn test_cost_calculation_gpt4() {
        let processor = CostCalculationProcessor::new();
        let now = Utc::now();

        let span = LlmSpan {
            span_id: "test".to_string(),
            trace_id: "test".to_string(),
            parent_span_id: None,
            name: "test".to_string(),
            provider: Provider::OpenAI,
            model: "gpt-4".to_string(),
            input: LlmInput::Text {
                prompt: "Test".to_string(),
            },
            output: None,
            token_usage: Some(TokenUsage::new(1000, 500)),
            cost: None,
            latency: Latency::new(now, now),
            metadata: Default::default(),
            status: SpanStatus::Ok,
            attributes: Default::default(),
            events: vec![],
        };

        let processed = processor.process(span).await.unwrap().unwrap();
        let cost = processed.cost.unwrap();

        // GPT-4: $0.03/1k input, $0.06/1k output
        // 1000 prompt + 500 completion = $0.03 + $0.03 = $0.06
        assert!((cost.amount_usd - 0.06).abs() < 0.0001);
        assert_eq!(cost.prompt_cost, Some(0.03));
        assert_eq!(cost.completion_cost, Some(0.03));
    }

    #[tokio::test]
    async fn test_cost_calculation_claude() {
        let processor = CostCalculationProcessor::new();
        let now = Utc::now();

        let span = LlmSpan {
            span_id: "test".to_string(),
            trace_id: "test".to_string(),
            parent_span_id: None,
            name: "test".to_string(),
            provider: Provider::Anthropic,
            model: "claude-3-5-sonnet-20241022".to_string(),
            input: LlmInput::Text {
                prompt: "Test".to_string(),
            },
            output: None,
            token_usage: Some(TokenUsage::new(1000, 1000)),
            cost: None,
            latency: Latency::new(now, now),
            metadata: Default::default(),
            status: SpanStatus::Ok,
            attributes: Default::default(),
            events: vec![],
        };

        let processed = processor.process(span).await.unwrap().unwrap();
        let cost = processed.cost.unwrap();

        // Claude Sonnet: $0.003/1k input, $0.015/1k output
        // 1000 + 1000 = $0.003 + $0.015 = $0.018
        assert!((cost.amount_usd - 0.018).abs() < 0.0001);
    }

    #[tokio::test]
    async fn test_no_token_usage() {
        let processor = CostCalculationProcessor::new();
        let now = Utc::now();

        let span = LlmSpan {
            span_id: "test".to_string(),
            trace_id: "test".to_string(),
            parent_span_id: None,
            name: "test".to_string(),
            provider: Provider::OpenAI,
            model: "gpt-4".to_string(),
            input: LlmInput::Text {
                prompt: "Test".to_string(),
            },
            output: None,
            token_usage: None, // No token usage
            cost: None,
            latency: Latency::new(now, now),
            metadata: Default::default(),
            status: SpanStatus::Ok,
            attributes: Default::default(),
            events: vec![],
        };

        let processed = processor.process(span).await.unwrap().unwrap();
        assert!(processed.cost.is_none());
    }
}
