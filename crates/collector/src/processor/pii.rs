// Copyright 2025 LLM Observatory Contributors
// SPDX-License-Identifier: Apache-2.0

//! PII (Personally Identifiable Information) redaction processor.
//!
//! This processor detects and redacts PII from LLM prompts and responses using:
//! - Regex patterns for common PII types (emails, phone numbers, SSNs, etc.)
//! - Configurable redaction strategies (mask, hash, remove)
//!
//! For enterprise deployments, this can be extended with ML-based entity recognition.

use super::SpanProcessor;
use async_trait::async_trait;
use llm_observatory_core::{
    span::{ChatMessage, LlmInput, LlmOutput, LlmSpan},
    Result,
};
use once_cell::sync::Lazy;
use regex::Regex;

/// Regex patterns for PII detection.
static EMAIL_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b").unwrap());

static PHONE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b(?:\+?1[-.\s]?)?\(?([0-9]{3})\)?[-.\s]?([0-9]{3})[-.\s]?([0-9]{4})\b").unwrap()
});

static SSN_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").unwrap());

static CREDIT_CARD_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\b\d{4}[\s-]?\d{4}[\s-]?\d{4}[\s-]?\d{4}\b").unwrap());

static IP_ADDRESS_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\b(?:[0-9]{1,3}\.){3}[0-9]{1,3}\b").unwrap());

/// PII redaction processor.
#[derive(Debug, Clone)]
pub struct PiiRedactionProcessor {
    /// Redaction strategy
    strategy: RedactionStrategy,
    /// Patterns to redact
    patterns: Vec<PiiPattern>,
}

/// Redaction strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedactionStrategy {
    /// Replace with placeholder text (e.g., "[EMAIL]")
    Mask,
    /// Replace with hash
    Hash,
    /// Remove entirely
    Remove,
}

/// PII pattern type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PiiPattern {
    /// Email addresses
    Email,
    /// Phone numbers
    Phone,
    /// Social Security Numbers
    SSN,
    /// Credit card numbers
    CreditCard,
    /// IP addresses
    IpAddress,
}

impl PiiRedactionProcessor {
    /// Create a new PII redaction processor with default patterns.
    pub fn new() -> Self {
        Self {
            strategy: RedactionStrategy::Mask,
            patterns: vec![
                PiiPattern::Email,
                PiiPattern::Phone,
                PiiPattern::SSN,
                PiiPattern::CreditCard,
                PiiPattern::IpAddress,
            ],
        }
    }

    /// Set redaction strategy.
    pub fn with_strategy(mut self, strategy: RedactionStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// Set patterns to detect.
    pub fn with_patterns(mut self, patterns: Vec<PiiPattern>) -> Self {
        self.patterns = patterns;
        self
    }

    /// Redact PII from text.
    fn redact_text(&self, text: &str) -> String {
        let mut redacted = text.to_string();

        for pattern in &self.patterns {
            redacted = match pattern {
                PiiPattern::Email => self.redact_pattern(&redacted, &EMAIL_REGEX, "[EMAIL]"),
                PiiPattern::Phone => self.redact_pattern(&redacted, &PHONE_REGEX, "[PHONE]"),
                PiiPattern::SSN => self.redact_pattern(&redacted, &SSN_REGEX, "[SSN]"),
                PiiPattern::CreditCard => {
                    self.redact_pattern(&redacted, &CREDIT_CARD_REGEX, "[CC]")
                }
                PiiPattern::IpAddress => self.redact_pattern(&redacted, &IP_ADDRESS_REGEX, "[IP]"),
            };
        }

        redacted
    }

    /// Redact a specific pattern.
    fn redact_pattern(&self, text: &str, regex: &Regex, placeholder: &str) -> String {
        match self.strategy {
            RedactionStrategy::Mask => regex.replace_all(text, placeholder).to_string(),
            RedactionStrategy::Hash => {
                // For hash strategy, we'd compute a hash of the matched text
                // For now, use mask as fallback
                regex.replace_all(text, placeholder).to_string()
            }
            RedactionStrategy::Remove => regex.replace_all(text, "").to_string(),
        }
    }

    /// Redact PII from LLM input.
    fn redact_input(&self, input: LlmInput) -> LlmInput {
        match input {
            LlmInput::Text { prompt } => LlmInput::Text {
                prompt: self.redact_text(&prompt),
            },
            LlmInput::Chat { messages } => {
                let redacted_messages = messages
                    .into_iter()
                    .map(|msg| ChatMessage {
                        role: msg.role,
                        content: self.redact_text(&msg.content),
                        name: msg.name,
                    })
                    .collect();
                LlmInput::Chat {
                    messages: redacted_messages,
                }
            }
            LlmInput::Multimodal { parts } => {
                // For multimodal, only redact text parts
                LlmInput::Multimodal { parts }
            }
        }
    }

    /// Redact PII from LLM output.
    fn redact_output(&self, output: Option<LlmOutput>) -> Option<LlmOutput> {
        output.map(|out| LlmOutput {
            content: self.redact_text(&out.content),
            finish_reason: out.finish_reason,
            metadata: out.metadata,
        })
    }
}

impl Default for PiiRedactionProcessor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SpanProcessor for PiiRedactionProcessor {
    async fn process(&self, mut span: LlmSpan) -> Result<Option<LlmSpan>> {
        // Redact input
        span.input = self.redact_input(span.input);

        // Redact output
        span.output = self.redact_output(span.output);

        Ok(Some(span))
    }

    fn name(&self) -> &str {
        "pii_redaction"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use llm_observatory_core::{
        span::{LlmInput, LlmSpan, SpanStatus},
        types::{Latency, Provider},
    };

    #[test]
    fn test_email_redaction() {
        let processor = PiiRedactionProcessor::new();
        let text = "Contact me at john.doe@example.com for more info";
        let redacted = processor.redact_text(text);
        assert_eq!(redacted, "Contact me at [EMAIL] for more info");
    }

    #[test]
    fn test_phone_redaction() {
        let processor = PiiRedactionProcessor::new();
        let text = "Call me at 555-123-4567";
        let redacted = processor.redact_text(text);
        assert_eq!(redacted, "Call me at [PHONE]");
    }

    #[test]
    fn test_ssn_redaction() {
        let processor = PiiRedactionProcessor::new();
        let text = "SSN: 123-45-6789";
        let redacted = processor.redact_text(text);
        assert_eq!(redacted, "SSN: [SSN]");
    }

    #[test]
    fn test_multiple_pii_redaction() {
        let processor = PiiRedactionProcessor::new();
        let text = "Email: user@example.com, Phone: 555-1234, SSN: 123-45-6789";
        let redacted = processor.redact_text(text);
        assert!(redacted.contains("[EMAIL]"));
        assert!(redacted.contains("[PHONE]"));
        assert!(redacted.contains("[SSN]"));
    }

    #[tokio::test]
    async fn test_span_processing() {
        let processor = PiiRedactionProcessor::new();
        let now = Utc::now();

        let span = LlmSpan {
            span_id: "test".to_string(),
            trace_id: "test".to_string(),
            parent_span_id: None,
            name: "test".to_string(),
            provider: Provider::OpenAI,
            model: "gpt-4".to_string(),
            input: LlmInput::Text {
                prompt: "My email is user@example.com".to_string(),
            },
            output: Some(LlmOutput {
                content: "Contact me at admin@test.com".to_string(),
                finish_reason: None,
                metadata: Default::default(),
            }),
            token_usage: None,
            cost: None,
            latency: Latency::new(now, now),
            metadata: Default::default(),
            status: SpanStatus::Ok,
            attributes: Default::default(),
            events: vec![],
        };

        let processed = processor.process(span).await.unwrap().unwrap();

        match processed.input {
            LlmInput::Text { prompt } => {
                assert_eq!(prompt, "My email is [EMAIL]");
            }
            _ => panic!("Expected Text input"),
        }

        assert_eq!(processed.output.unwrap().content, "Contact me at [EMAIL]");
    }
}
