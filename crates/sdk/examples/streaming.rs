// Copyright 2025 LLM Observatory Contributors
// SPDX-License-Identifier: Apache-2.0

//! Streaming completion example.
//!
//! This example demonstrates:
//! - Streaming responses from the LLM
//! - Processing chunks in real-time
//! - Tracking time-to-first-token (TTFT)
//!
//! Run with:
//! ```bash
//! export OPENAI_API_KEY=sk-...
//! cargo run --example streaming --features openai
//! ```

use futures::StreamExt;
use llm_observatory_sdk::{ChatCompletionRequest, InstrumentedLLM, LLMObservatory, OpenAIClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("LLM Observatory SDK - Streaming Example\n");

    let api_key =
        std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY environment variable not set");

    // Initialize observatory
    let observatory = LLMObservatory::builder()
        .with_service_name("streaming-example")
        .with_environment("development")
        .build()?;

    let client = OpenAIClient::new(api_key).with_observatory(observatory);

    println!("Making streaming LLM request...\n");

    // Create a streaming request
    let request = ChatCompletionRequest::new("gpt-4o-mini")
        .with_system("You are a creative storyteller.")
        .with_user("Tell me a very short story about a robot learning to paint.")
        .with_temperature(0.8)
        .with_max_tokens(200)
        .with_streaming(true);

    // Note: Streaming is not yet fully implemented in this example
    // In a production implementation, this would use Server-Sent Events (SSE)
    println!("Note: Streaming API is not yet fully implemented.");
    println!("Using regular completion instead...\n");

    // Fall back to regular completion for now
    let mut regular_request = request;
    regular_request.stream = false;

    let response = client.chat_completion(regular_request).await?;

    println!("=== Response ===");
    println!("{}", response.content);
    println!("\n=== Metrics ===");
    println!("Total tokens: {}", response.total_tokens());
    println!("Cost: ${:.6}", response.cost_usd);
    println!("Latency: {} ms", response.latency_ms);

    // Shutdown
    if let Some(observatory) = client.observatory() {
        observatory.shutdown().await?;
    }

    Ok(())
}
