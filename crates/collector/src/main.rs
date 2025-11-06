// Copyright 2025 LLM Observatory Contributors
// SPDX-License-Identifier: Apache-2.0

//! LLM Observatory Collector binary.

use clap::Parser;
use llm_observatory_collector::{receiver::Receiver, CollectorConfig, OtlpReceiver};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Configuration file path
    #[arg(short, long, value_name = "FILE")]
    config: Option<String>,

    /// gRPC endpoint
    #[arg(long, default_value = "0.0.0.0:4317")]
    grpc_endpoint: String,

    /// HTTP endpoint
    #[arg(long, default_value = "0.0.0.0:4318")]
    http_endpoint: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "llm_observatory_collector=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let args = Args::parse();

    // Load configuration
    let config = match args.config {
        Some(path) => {
            tracing::info!("Loading configuration from: {}", path);
            CollectorConfig::from_file(&path)?
        }
        None => {
            tracing::info!("Using default configuration");
            CollectorConfig::default()
        }
    };

    tracing::info!(
        "Starting LLM Observatory Collector v{}",
        env!("CARGO_PKG_VERSION")
    );

    // Create receiver
    let mut receiver = OtlpReceiver::new(
        config.receiver.grpc_endpoint,
        config.receiver.http_endpoint,
    )
    .with_grpc(config.receiver.enable_grpc)
    .with_http(config.receiver.enable_http);

    // Start receiver
    receiver.start().await?;

    tracing::info!("Collector started successfully");
    tracing::info!("Press Ctrl+C to shutdown");

    // Wait for shutdown signal
    tokio::signal::ctrl_c().await?;

    tracing::info!("Shutdown signal received, stopping collector...");
    receiver.stop().await?;

    tracing::info!("Collector stopped gracefully");
    Ok(())
}
