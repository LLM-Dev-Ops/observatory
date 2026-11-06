// Copyright 2025 LLM Observatory Contributors
// SPDX-License-Identifier: Apache-2.0

//! Basic example of using the LLM Observatory SDK with OpenAI.
//!
//! This example demonstrates:
//! - Setting up the observatory
//! - Creating an instrumented OpenAI client
//! - Making a simple chat completion request
//! - Viewing trace IDs and costs
//!
//! Run with:
//! ```bash
//! export OPENAI_API_KEY=sk-...
//! cargo run --example basic --features openai
//! ```

use llm_observatory_sdk::{ChatCompletionRequest, InstrumentedLLM, LLMObservatory, OpenAIClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing subscriber for console output
    tracing_subscriber::fmt::init();

    println!("LLM Observatory SDK - Basic Example\n");

    // Get API key from environment
    let api_key =
        std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY environment variable not set");

    // Initialize the observatory
    println!("Initializing observatory...");
    let observatory = LLMObservatory::builder()
        .with_service_name("basic-example")
        .with_environment("development")
        .with_otlp_endpoint("http://localhost:4317")
        .build()?;

    println!("Observatory initialized successfully!");
    println!("Service: {}", observatory.service_name());
    println!("Environment: {}\n", observatory.environment());

    // Create an instrumented OpenAI client
    let client = OpenAIClient::new(api_key).with_observatory(observatory);

    println!("Making LLM request...");

    // Create a chat completion request
    let request = ChatCompletionRequest::new("gpt-4o-mini")
        .with_system("You are a helpful assistant.")
        .with_user("What is the capital of France?")
        .with_temperature(0.7)
        .with_max_tokens(100);

    // Execute the request with automatic instrumentation
    let response = client.chat_completion(request).await?;

    // Display results
    println!("\n=== Response ===");
    println!("Content: {}", response.content);
    println!("\n=== Metrics ===");
    println!("Model: {}", response.model);
    println!("Prompt tokens: {}", response.prompt_tokens());
    println!("Completion tokens: {}", response.completion_tokens());
    println!("Total tokens: {}", response.total_tokens());
    println!("Cost: ${:.6}", response.cost_usd);
    println!("Latency: {} ms", response.latency_ms);
    println!("\n=== Tracing ===");
    println!("Trace ID: {}", response.trace_id);
    println!("Span ID: {}", response.span_id);

    println!("\nExample completed successfully!");

    // Shutdown and flush telemetry
    if let Some(observatory) = client.observatory() {
        observatory.shutdown().await?;
    }

    Ok(())
}
