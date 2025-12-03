// Copyright 2025 LLM Observatory Contributors
// SPDX-License-Identifier: Apache-2.0

//! Upstream integration adapters for LLM-Dev-Ops ecosystem.
//!
//! This module provides lightweight consumption layers for integrating
//! Observatory with its upstream dependencies:
//!
//! ## Phase 2A - Compile-Time Dependencies
//!
//! - **Schema Registry**: Schema loading and validation
//! - **Config Manager**: Configuration retrieval and management
//! - **Latency Lens**: Latency sampling and metrics hooks
//! - **CostOps**: Cost analytics and token usage correlation
//! - **Sentinel**: Anomaly detection and event consumption
//!
//! ## Phase 2B - Runtime-Only Adapters
//!
//! - **Edge Agent**: Telemetry ingress and gateway traces
//! - **Inference Gateway**: Backend routing logs and inference telemetry
//! - **Orchestrator**: Workflow telemetry and pipeline execution traces
//!
//! # Architecture
//!
//! Phase 2A adapters provide thin wrappers around upstream crate APIs,
//! exposing functionality needed by Observatory while maintaining type
//! compatibility with core Observatory types.
//!
//! Phase 2B adapters are runtime-only consumption layers that process
//! telemetry data without requiring compile-time dependencies on upstream
//! crates. Data is consumed via standardized formats (JSON, OpenTelemetry).
//!
//! # Example
//!
//! ```ignore
//! use llm_observatory_adapters::upstream::prelude::*;
//!
//! // Phase 2A - Compile-time adapters
//! let schema_adapter = SchemaAdapter::new();
//! let config_adapter = ConfigAdapter::new("/path/to/config")?;
//! let latency_adapter = LatencyAdapter::new();
//! let cost_adapter = CostAdapter::new();
//! let sentinel_adapter = SentinelAdapter::new("my-service");
//!
//! // Phase 2B - Runtime adapters
//! let edge_adapter = EdgeAgentAdapter::new("edge-node-1");
//! let gateway_adapter = InferenceGatewayAdapter::new("gateway-1");
//! let orchestrator_adapter = OrchestratorAdapter::new("orchestrator-1");
//! ```

// Phase 2A - Compile-time dependency adapters
pub mod config;
pub mod cost;
pub mod latency;
pub mod schema;
pub mod sentinel;

// Phase 2B - Runtime-only adapters (no compile-time upstream dependencies)
pub mod edge_agent;
pub mod inference_gateway;
pub mod orchestrator;

/// Prelude module for convenient imports.
pub mod prelude {
    // Phase 2A adapters
    pub use super::config::{ConfigAdapter, ConfigAdapterError};
    pub use super::cost::{CostAdapter, CostAdapterError};
    pub use super::latency::{LatencyAdapter, LatencyAdapterError};
    pub use super::schema::{SchemaAdapter, SchemaAdapterError};
    pub use super::sentinel::{SentinelAdapter, SentinelAdapterError};

    // Phase 2B adapters
    pub use super::edge_agent::{EdgeAgentAdapter, EdgeAgentAdapterError};
    pub use super::inference_gateway::{InferenceGatewayAdapter, InferenceGatewayAdapterError};
    pub use super::orchestrator::{OrchestratorAdapter, OrchestratorAdapterError};
}

// Re-export Phase 2A adapters at module level
pub use config::ConfigAdapter;
pub use cost::CostAdapter;
pub use latency::LatencyAdapter;
pub use schema::SchemaAdapter;
pub use sentinel::SentinelAdapter;

// Re-export Phase 2B adapters at module level
pub use edge_agent::EdgeAgentAdapter;
pub use inference_gateway::InferenceGatewayAdapter;
pub use orchestrator::OrchestratorAdapter;
