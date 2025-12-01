//! Canonical benchmark interface for LLM Observatory.
//!
//! This crate provides the standardized benchmark interface used across
//! all 25 modules in the LLM-Dev-Ops organization.
//!
//! # Quick Start
//!
//! ```no_run
//! use llm_observatory_benchmarks::{run_all_benchmarks, BenchmarkResult};
//!
//! // Run all benchmarks
//! let results = run_all_benchmarks();
//!
//! // Process results
//! for result in &results {
//!     println!("{}: {}", result.target_id, result.metrics);
//! }
//! ```
//!
//! # Modules
//!
//! - [`result`] - The canonical `BenchmarkResult` struct
//! - [`io`] - I/O operations for reading/writing results
//! - [`markdown`] - Markdown report generation

#![warn(missing_docs, rust_2018_idioms)]
#![deny(unsafe_code)]

pub mod io;
pub mod markdown;
pub mod result;

pub use result::BenchmarkResult;

use chrono::Utc;

/// Run all registered benchmarks and return results.
///
/// This is the canonical entrypoint for the benchmark system.
/// It executes all registered benchmark targets and returns their results.
///
/// # Returns
///
/// A vector of `BenchmarkResult` containing the results from all benchmarks.
pub fn run_all_benchmarks() -> Vec<BenchmarkResult> {
    let mut results = Vec::new();

    // System health benchmark
    results.push(BenchmarkResult::new(
        "observatory/system",
        serde_json::json!({
            "status": "healthy",
            "version": env!("CARGO_PKG_VERSION"),
            "timestamp": Utc::now().to_rfc3339()
        }),
    ));

    results
}

/// Run all benchmarks and write outputs to canonical directories.
///
/// This function runs all benchmarks and writes the results to:
/// - `benchmarks/output/raw/` - Individual JSON files per benchmark
/// - `benchmarks/output/all_results.json` - Combined JSON file
/// - `benchmarks/output/summary.md` - Markdown summary
///
/// # Returns
///
/// A `Result` containing the benchmark results on success.
///
/// # Errors
///
/// Returns an `io::Error` if writing output files fails.
pub fn run_and_write_all() -> std::io::Result<Vec<BenchmarkResult>> {
    let results = run_all_benchmarks();
    io::write_all_outputs(&results)?;
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_all_benchmarks_returns_results() {
        let results = run_all_benchmarks();
        assert!(!results.is_empty());
    }

    #[test]
    fn test_benchmark_result_has_required_fields() {
        let result = BenchmarkResult::new("test", serde_json::json!({"key": "value"}));
        assert_eq!(result.target_id, "test");
        assert!(result.metrics.is_object());
        assert!(result.timestamp <= Utc::now());
    }
}
