//! Benchmark result types.
//!
//! This module provides the canonical BenchmarkResult struct used for
//! cross-project benchmark consistency.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Canonical benchmark result structure.
///
/// This struct provides a standardized format for benchmark results
/// across all 25 modules in the LLM-Dev-Ops organization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    /// Unique identifier for the benchmark target.
    pub target_id: String,
    /// Metrics data in JSON format.
    pub metrics: serde_json::Value,
    /// Timestamp when the benchmark was executed.
    pub timestamp: DateTime<Utc>,
}

impl BenchmarkResult {
    /// Create a new BenchmarkResult.
    pub fn new(target_id: impl Into<String>, metrics: serde_json::Value) -> Self {
        Self {
            target_id: target_id.into(),
            metrics,
            timestamp: Utc::now(),
        }
    }
}
