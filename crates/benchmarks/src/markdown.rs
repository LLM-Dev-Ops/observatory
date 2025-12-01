//! Markdown output generation for benchmark results.
//!
//! This module provides functionality to generate markdown-formatted
//! benchmark reports for the canonical benchmark interface.

use crate::result::BenchmarkResult;
use std::fmt::Write;

/// Generate a markdown summary from benchmark results.
pub fn generate_summary(results: &[BenchmarkResult]) -> String {
    let mut output = String::new();

    writeln!(output, "# Benchmark Summary").unwrap();
    writeln!(output).unwrap();
    writeln!(output, "Generated: {}", chrono::Utc::now().to_rfc3339()).unwrap();
    writeln!(output).unwrap();
    writeln!(output, "## Results").unwrap();
    writeln!(output).unwrap();
    writeln!(output, "| Target ID | Timestamp | Metrics |").unwrap();
    writeln!(output, "|-----------|-----------|---------|").unwrap();

    for result in results {
        let metrics_preview = result.metrics.to_string();
        let metrics_short = if metrics_preview.len() > 50 {
            format!("{}...", &metrics_preview[..47])
        } else {
            metrics_preview
        };
        writeln!(
            output,
            "| {} | {} | {} |",
            result.target_id,
            result.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
            metrics_short
        ).unwrap();
    }

    writeln!(output).unwrap();
    writeln!(output, "---").unwrap();
    writeln!(output, "Total benchmarks: {}", results.len()).unwrap();

    output
}

/// Generate detailed markdown report.
pub fn generate_detailed_report(results: &[BenchmarkResult]) -> String {
    let mut output = String::new();

    writeln!(output, "# Detailed Benchmark Report").unwrap();
    writeln!(output).unwrap();
    writeln!(output, "Generated: {}", chrono::Utc::now().to_rfc3339()).unwrap();
    writeln!(output).unwrap();

    for result in results {
        writeln!(output, "## {}", result.target_id).unwrap();
        writeln!(output).unwrap();
        writeln!(output, "**Timestamp:** {}", result.timestamp.to_rfc3339()).unwrap();
        writeln!(output).unwrap();
        writeln!(output, "**Metrics:**").unwrap();
        writeln!(output, "```json").unwrap();
        writeln!(output, "{}", serde_json::to_string_pretty(&result.metrics).unwrap_or_default()).unwrap();
        writeln!(output, "```").unwrap();
        writeln!(output).unwrap();
    }

    output
}
