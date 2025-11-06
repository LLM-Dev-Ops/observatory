// Copyright 2025 LLM Observatory Contributors
// SPDX-License-Identifier: Apache-2.0

//! Sampling strategies for intelligent trace sampling.
//!
//! Implements both head sampling (probabilistic at SDK level) and tail sampling
//! (decision after trace completion based on actual characteristics).

use llm_observatory_core::span::LlmSpan;
use rand::Rng;

pub use crate::config::SamplingStrategy;

/// Head sampler (probabilistic sampling).
#[derive(Debug, Clone)]
pub struct HeadSampler {
    /// Sampling rate (0.0 to 1.0)
    rate: f64,
}

impl HeadSampler {
    /// Create a new head sampler with the given rate.
    pub fn new(rate: f64) -> Self {
        assert!((0.0..=1.0).contains(&rate), "Sampling rate must be between 0 and 1");
        Self { rate }
    }

    /// Decide whether to sample based on probability.
    pub fn should_sample(&self) -> bool {
        if self.rate >= 1.0 {
            return true;
        }
        if self.rate <= 0.0 {
            return false;
        }

        let mut rng = rand::thread_rng();
        rng.gen::<f64>() < self.rate
    }
}

/// Tail sampler (content-based sampling after trace completion).
#[derive(Debug, Clone)]
pub struct TailSampler {
    /// Always sample errors
    always_sample_errors: bool,
    /// Slow request threshold (ms)
    slow_threshold_ms: u64,
    /// Expensive request threshold (USD)
    expensive_threshold_usd: f64,
}

impl TailSampler {
    /// Create a new tail sampler.
    pub fn new() -> Self {
        Self {
            always_sample_errors: true,
            slow_threshold_ms: 5000,
            expensive_threshold_usd: 1.0,
        }
    }

    /// Set whether to always sample errors.
    pub fn with_sample_errors(mut self, sample: bool) -> Self {
        self.always_sample_errors = sample;
        self
    }

    /// Set slow request threshold.
    pub fn with_slow_threshold_ms(mut self, threshold: u64) -> Self {
        self.slow_threshold_ms = threshold;
        self
    }

    /// Set expensive request threshold.
    pub fn with_expensive_threshold_usd(mut self, threshold: f64) -> Self {
        self.expensive_threshold_usd = threshold;
        self
    }

    /// Decide whether to sample based on span characteristics.
    pub fn should_sample(&self, span: &LlmSpan) -> bool {
        // Always sample errors
        if self.always_sample_errors && span.is_error() {
            return true;
        }

        // Always sample slow requests
        if span.duration_ms() >= self.slow_threshold_ms {
            return true;
        }

        // Always sample expensive requests
        if let Some(cost) = span.total_cost_usd() {
            if cost >= self.expensive_threshold_usd {
                return true;
            }
        }

        // Default: do not sample
        false
    }
}

impl Default for TailSampler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use llm_observatory_core::{
        span::{LlmSpan, LlmInput, SpanStatus},
        types::{Provider, Latency, Cost},
    };
    use chrono::Utc;

    #[test]
    fn test_head_sampler_always() {
        let sampler = HeadSampler::new(1.0);
        assert!(sampler.should_sample());
    }

    #[test]
    fn test_head_sampler_never() {
        let sampler = HeadSampler::new(0.0);
        assert!(!sampler.should_sample());
    }

    #[test]
    fn test_head_sampler_probability() {
        let sampler = HeadSampler::new(0.5);

        // Run 1000 times and check it's roughly 50%
        let mut sampled = 0;
        for _ in 0..1000 {
            if sampler.should_sample() {
                sampled += 1;
            }
        }

        // Should be roughly 500 ± 100
        assert!(sampled > 400 && sampled < 600, "Expected ~500, got {}", sampled);
    }

    #[test]
    fn test_tail_sampler_error() {
        let sampler = TailSampler::new();
        let now = Utc::now();

        let span = LlmSpan {
            span_id: "test".to_string(),
            trace_id: "test".to_string(),
            parent_span_id: None,
            name: "test".to_string(),
            provider: Provider::OpenAI,
            model: "gpt-4".to_string(),
            input: LlmInput::Text {
                prompt: "test".to_string(),
            },
            output: None,
            token_usage: None,
            cost: None,
            latency: Latency::new(now, now),
            metadata: Default::default(),
            status: SpanStatus::Error, // Error status
            attributes: Default::default(),
            events: vec![],
        };

        assert!(sampler.should_sample(&span));
    }

    #[test]
    fn test_tail_sampler_slow() {
        let sampler = TailSampler::new().with_slow_threshold_ms(1000);
        let start = Utc::now();
        let end = start + chrono::Duration::milliseconds(2000);

        let span = LlmSpan {
            span_id: "test".to_string(),
            trace_id: "test".to_string(),
            parent_span_id: None,
            name: "test".to_string(),
            provider: Provider::OpenAI,
            model: "gpt-4".to_string(),
            input: LlmInput::Text {
                prompt: "test".to_string(),
            },
            output: None,
            token_usage: None,
            cost: None,
            latency: Latency::new(start, end), // 2 second duration
            metadata: Default::default(),
            status: SpanStatus::Ok,
            attributes: Default::default(),
            events: vec![],
        };

        assert!(sampler.should_sample(&span));
    }

    #[test]
    fn test_tail_sampler_expensive() {
        let sampler = TailSampler::new().with_expensive_threshold_usd(0.5);
        let now = Utc::now();

        let span = LlmSpan {
            span_id: "test".to_string(),
            trace_id: "test".to_string(),
            parent_span_id: None,
            name: "test".to_string(),
            provider: Provider::OpenAI,
            model: "gpt-4".to_string(),
            input: LlmInput::Text {
                prompt: "test".to_string(),
            },
            output: None,
            token_usage: None,
            cost: Some(Cost::new(1.5)), // $1.50 cost
            latency: Latency::new(now, now),
            metadata: Default::default(),
            status: SpanStatus::Ok,
            attributes: Default::default(),
            events: vec![],
        };

        assert!(sampler.should_sample(&span));
    }

    #[test]
    fn test_tail_sampler_normal() {
        let sampler = TailSampler::new();
        let now = Utc::now();

        let span = LlmSpan {
            span_id: "test".to_string(),
            trace_id: "test".to_string(),
            parent_span_id: None,
            name: "test".to_string(),
            provider: Provider::OpenAI,
            model: "gpt-4".to_string(),
            input: LlmInput::Text {
                prompt: "test".to_string(),
            },
            output: None,
            token_usage: None,
            cost: Some(Cost::new(0.01)), // Cheap
            latency: Latency::new(now, now), // Fast
            metadata: Default::default(),
            status: SpanStatus::Ok, // Not an error
            attributes: Default::default(),
            events: vec![],
        };

        // Should NOT sample (not error, not slow, not expensive)
        assert!(!sampler.should_sample(&span));
    }
}
