// Copyright 2025 LLM Observatory Contributors
// SPDX-License-Identifier: Apache-2.0

//! Upstream integration adapters for LLM-Dev-Ops ecosystem.
//!
//! This module provides lightweight consumption layers for integrating
//! Observatory with its upstream dependencies:
//!
//! - **Schema Registry**: Schema loading and validation
//! - **Config Manager**: Configuration retrieval and management
//! - **Latency Lens**: Latency sampling and metrics hooks
//! - **CostOps**: Cost analytics and token usage correlation
//! - **Sentinel**: Anomaly detection and event consumption
//!
//! # Architecture
//!
//! Each adapter provides a thin wrapper around the upstream crate's API,
//! exposing only the functionality needed by Observatory while maintaining
//! type compatibility with core Observatory types.
//!
//! # Example
//!
//! ```ignore
//! use llm_observatory_adapters::upstream::prelude::*;
//!
//! // Initialize adapters
//! let schema_adapter = SchemaAdapter::new();
//! let config_adapter = ConfigAdapter::new("/path/to/config")?;
//! let latency_adapter = LatencyAdapter::new();
//! let cost_adapter = CostAdapter::new();
//! let sentinel_adapter = SentinelAdapter::new();
//! ```

pub mod config;
pub mod cost;
pub mod latency;
pub mod schema;
pub mod sentinel;

/// Prelude module for convenient imports.
pub mod prelude {
    pub use super::config::{ConfigAdapter, ConfigAdapterError};
    pub use super::cost::{CostAdapter, CostAdapterError};
    pub use super::latency::{LatencyAdapter, LatencyAdapterError};
    pub use super::schema::{SchemaAdapter, SchemaAdapterError};
    pub use super::sentinel::{SentinelAdapter, SentinelAdapterError};
}

/// Re-export all adapters at module level.
pub use config::ConfigAdapter;
pub use cost::CostAdapter;
pub use latency::LatencyAdapter;
pub use schema::SchemaAdapter;
pub use sentinel::SentinelAdapter;
