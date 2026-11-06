// Copyright 2025 LLM Observatory Contributors
// SPDX-License-Identifier: Apache-2.0

//! Span processors for LLM-specific transformations.

pub mod cost;
pub mod pii;

use async_trait::async_trait;
use llm_observatory_core::{span::LlmSpan, Result};

/// Trait for span processors.
#[async_trait]
pub trait SpanProcessor: Send + Sync {
    /// Process a span, potentially modifying it.
    ///
    /// Returns `Ok(Some(span))` if the span should be forwarded,
    /// `Ok(None)` if the span should be dropped,
    /// or `Err` if processing failed.
    async fn process(&self, span: LlmSpan) -> Result<Option<LlmSpan>>;

    /// Get processor name.
    fn name(&self) -> &str;
}
