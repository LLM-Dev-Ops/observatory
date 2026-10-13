// Copyright 2025 LLM Observatory Contributors
// SPDX-License-Identifier: Apache-2.0

//! LLM provider implementations and pricing engines.
//!
//! This crate provides concrete implementations of the `LlmProvider` trait
//! for various LLM providers (OpenAI, Anthropic, Google, etc.) along with
//! accurate pricing models based on official provider pricing.

#![warn(missing_docs, rust_2018_idioms)]
#![deny(unsafe_code)]

pub mod anthropic;
pub mod openai;
pub mod pricing;

pub use anthropic::AnthropicProvider;
pub use openai::OpenAiProvider;
pub use pricing::{PricingDatabase, PricingEngine};
