//! CLI for LLM Observatory.
//!
//! This crate provides the command-line interface for LLM Observatory,
//! including the canonical benchmark `run` subcommand.

#![warn(missing_docs, rust_2018_idioms)]
#![deny(unsafe_code)]

use clap::{Parser, Subcommand};

/// LLM Observatory CLI.
#[derive(Parser, Debug)]
#[command(name = "observatory")]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Subcommand to run.
    #[command(subcommand)]
    pub command: Commands,
}

/// Available CLI commands.
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Run all benchmarks and write results to canonical output directories.
    ///
    /// This command executes all registered benchmark targets and writes
    /// results to:
    /// - benchmarks/output/raw/ - Individual JSON files per benchmark
    /// - benchmarks/output/all_results.json - Combined JSON file
    /// - benchmarks/output/summary.md - Markdown summary
    Run {
        /// Output directory override (optional).
        #[arg(short, long)]
        output: Option<String>,

        /// Output format: json, markdown, or both (default: both).
        #[arg(short, long, default_value = "both")]
        format: String,

        /// Verbose output.
        #[arg(short, long)]
        verbose: bool,
    },

    /// Show benchmark status and configuration.
    Status {
        /// Show detailed status information.
        #[arg(short, long)]
        detailed: bool,
    },
}

/// Run the CLI with the given arguments.
///
/// # Returns
///
/// Returns `Ok(())` on success, or an error if the command fails.
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run {
            output: _,
            format: _,
            verbose,
        } => {
            if verbose {
                println!("Running all benchmarks...");
            }

            let results = llm_observatory_benchmarks::run_and_write_all()?;

            println!("Completed {} benchmarks", results.len());
            println!("Results written to benchmarks/output/");

            if verbose {
                for result in &results {
                    println!("  - {}: {}", result.target_id, result.metrics);
                }
            }

            Ok(())
        }
        Commands::Status { detailed } => {
            println!("LLM Observatory Benchmark System");
            println!("Version: {}", env!("CARGO_PKG_VERSION"));

            if detailed {
                println!("\nOutput directories:");
                println!("  - benchmarks/output/");
                println!("  - benchmarks/output/raw/");
                println!("\nOutput files:");
                println!("  - benchmarks/output/summary.md");
                println!("  - benchmarks/output/all_results.json");
            }

            Ok(())
        }
    }
}
