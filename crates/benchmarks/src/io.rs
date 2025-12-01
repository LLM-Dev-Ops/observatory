//! I/O operations for benchmark results.
//!
//! This module provides functionality to read and write benchmark
//! results to the filesystem in various formats.

use crate::result::BenchmarkResult;
use crate::markdown;
use std::fs;
use std::io;
use std::path::Path;

/// Default output directory path.
pub const OUTPUT_DIR: &str = "benchmarks/output";

/// Raw results directory path.
pub const RAW_DIR: &str = "benchmarks/output/raw";

/// Summary file path.
pub const SUMMARY_FILE: &str = "benchmarks/output/summary.md";

/// Ensure output directories exist.
pub fn ensure_output_dirs() -> io::Result<()> {
    fs::create_dir_all(OUTPUT_DIR)?;
    fs::create_dir_all(RAW_DIR)?;
    Ok(())
}

/// Write benchmark results to JSON file.
pub fn write_results_json(results: &[BenchmarkResult], path: impl AsRef<Path>) -> io::Result<()> {
    let json = serde_json::to_string_pretty(results)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    fs::write(path, json)
}

/// Write individual result to raw directory.
pub fn write_raw_result(result: &BenchmarkResult) -> io::Result<()> {
    ensure_output_dirs()?;
    let filename = format!("{}/{}.json", RAW_DIR, result.target_id.replace('/', "_"));
    let json = serde_json::to_string_pretty(result)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    fs::write(filename, json)
}

/// Write summary markdown file.
pub fn write_summary(results: &[BenchmarkResult]) -> io::Result<()> {
    ensure_output_dirs()?;
    let summary = markdown::generate_summary(results);
    fs::write(SUMMARY_FILE, summary)
}

/// Write all benchmark outputs (raw JSON and summary).
pub fn write_all_outputs(results: &[BenchmarkResult]) -> io::Result<()> {
    ensure_output_dirs()?;

    // Write individual raw results
    for result in results {
        write_raw_result(result)?;
    }

    // Write combined JSON
    write_results_json(results, format!("{}/all_results.json", OUTPUT_DIR))?;

    // Write summary
    write_summary(results)?;

    Ok(())
}

/// Read results from JSON file.
pub fn read_results_json(path: impl AsRef<Path>) -> io::Result<Vec<BenchmarkResult>> {
    let content = fs::read_to_string(path)?;
    serde_json::from_str(&content)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
}
