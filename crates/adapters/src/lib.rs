//! Benchmark target adapters and upstream integrations for LLM Observatory.
//!
//! This crate provides two main capabilities:
//!
//! 1. **Benchmark Targets**: The canonical `BenchTarget` trait and registry
//!    for benchmark targets across the LLM Observatory project.
//!
//! 2. **Upstream Integrations**: Consumption adapters for the LLM-Dev-Ops
//!    ecosystem, providing integration with:
//!    - Schema Registry (schema validation)
//!    - Config Manager (configuration management)
//!    - Latency Lens (latency profiling)
//!    - CostOps (cost analytics)
//!    - Sentinel (anomaly detection)
//!
//! # Quick Start - Benchmarks
//!
//! ```ignore
//! use llm_observatory_adapters::{BenchTarget, all_targets};
//! use llm_observatory_benchmarks::BenchmarkResult;
//!
//! // Get all registered targets
//! let targets = all_targets();
//!
//! // Run each target
//! for target in &targets {
//!     let result = target.run();
//!     println!("{}: {}", target.id(), result.metrics);
//! }
//! ```
//!
//! # Quick Start - Upstream Integrations
//!
//! ```ignore
//! use llm_observatory_adapters::upstream::prelude::*;
//!
//! // Schema validation
//! let schema_adapter = SchemaAdapter::new();
//! let result = schema_adapter.validate_span_json(&span_json);
//!
//! // Configuration management
//! let config_adapter = ConfigAdapter::in_memory();
//! let endpoint = config_adapter.get_string(ObservatoryConfigKey::OtlpEndpoint);
//!
//! // Latency profiling
//! let latency_adapter = LatencyAdapter::new();
//! let measurement = latency_adapter.start_measurement();
//!
//! // Cost analytics
//! let cost_adapter = CostAdapter::new();
//! let breakdown = cost_adapter.calculate_cost(&span)?;
//!
//! // Anomaly detection
//! let sentinel_adapter = SentinelAdapter::new("my-service");
//! let anomaly = sentinel_adapter.check_span_anomaly(&span);
//! ```

#![warn(missing_docs, rust_2018_idioms)]
#![deny(unsafe_code)]

pub mod upstream;

pub use llm_observatory_benchmarks::BenchmarkResult;

/// Canonical benchmark target trait.
///
/// Implement this trait for any component that should be benchmarkable
/// through the canonical benchmark interface.
///
/// # Example
///
/// ```ignore
/// use llm_observatory_adapters::BenchTarget;
/// use llm_observatory_benchmarks::BenchmarkResult;
///
/// struct MyTarget {
///     name: String,
/// }
///
/// impl BenchTarget for MyTarget {
///     fn id(&self) -> String {
///         self.name.clone()
///     }
///
///     fn run(&self) -> BenchmarkResult {
///         BenchmarkResult::new(
///             self.id(),
///             serde_json::json!({"status": "ok"})
///         )
///     }
/// }
/// ```
pub trait BenchTarget: Send + Sync {
    /// Returns the unique identifier for this benchmark target.
    fn id(&self) -> String;

    /// Run the benchmark and return results.
    fn run(&self) -> BenchmarkResult;
}

/// Registry of all available benchmark targets.
///
/// Returns all registered benchmark targets for the project.
/// Other crates can register targets by depending on this crate
/// and implementing the `BenchTarget` trait.
pub fn all_targets() -> Vec<Box<dyn BenchTarget>> {
    // Return empty vector by default - targets are registered by other crates
    Vec::new()
}

// Re-export upstream adapters at crate root for convenience
pub use upstream::{ConfigAdapter, CostAdapter, LatencyAdapter, SchemaAdapter, SentinelAdapter};
