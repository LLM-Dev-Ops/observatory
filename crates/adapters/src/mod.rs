//! Benchmark target adapters.
//!
//! This module provides the canonical BenchTarget trait and registry
//! for benchmark targets across the LLM Observatory project.

use llm_observatory_benchmarks::BenchmarkResult;

/// Canonical benchmark target trait.
///
/// Implement this trait for any component that should be benchmarkable
/// through the canonical benchmark interface.
pub trait BenchTarget {
    /// Returns the unique identifier for this benchmark target.
    fn id(&self) -> String;

    /// Run the benchmark and return results.
    fn run(&self) -> BenchmarkResult;
}

/// Registry of all available benchmark targets.
///
/// Returns all registered benchmark targets for the project.
pub fn all_targets() -> Vec<Box<dyn BenchTarget>> {
    // Return empty vector by default - targets are registered by other crates
    Vec::new()
}
