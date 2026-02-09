// Copyright 2025 LLM Observatory Contributors
// SPDX-License-Identifier: Apache-2.0

//! Core types, traits, and utilities for LLM Observatory.
//!
//! This crate provides the foundational types and traits used across all
//! LLM Observatory components, including span definitions, provider interfaces,
//! and shared utilities.

#![warn(missing_docs, rust_2018_idioms)]
#![deny(unsafe_code)]

pub mod error;
pub mod execution;
pub mod provider;
pub mod span;
pub mod types;

pub use error::{Error, Result};
pub use execution::{
    Artifact, ArtifactContent, ExecutionContext, ExecutionEvent, ExecutionId, ExecutionResult,
    ExecutionSpan, ExecutionSpanBuilder, ExecutionSpanId, ExecutionSpanKind, ExecutionSpanStatus,
};
