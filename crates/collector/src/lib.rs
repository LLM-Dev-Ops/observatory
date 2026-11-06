// Copyright 2025 LLM Observatory Contributors
// SPDX-License-Identifier: Apache-2.0

//! OpenTelemetry collector with LLM-specific processing.
//!
//! This crate implements an OTLP-compliant collector that receives traces, metrics,
//! and logs from LLM applications, processes them through LLM-aware pipelines
//! (PII redaction, cost calculation, intelligent sampling), and forwards them
//! to storage backends.

#![warn(missing_docs, rust_2018_idioms)]
#![deny(unsafe_code)]

pub mod config;
pub mod processor;
pub mod receiver;
pub mod sampler;

pub use config::CollectorConfig;
pub use processor::cost::CostCalculationProcessor;
pub use processor::pii::PiiRedactionProcessor;
pub use receiver::otlp::OtlpReceiver;
pub use sampler::{HeadSampler, SamplingStrategy, TailSampler};
