// Copyright 2025 LLM Observatory Contributors
// SPDX-License-Identifier: Apache-2.0

//! Example demonstrating custom attributes and metadata.
//!
//! This example shows:
//! - Adding custom metadata to requests
//! - Using custom attributes for filtering/grouping
//! - Tracking user sessions
//! - Environment-specific tags
//!
//! Run with:
//! ```bash
//! export OPENAI_API_KEY=sk-...
//! cargo run --example custom_attributes --features openai
//! ```

use llm_observatory_sdk::{ChatCompletionRequest, InstrumentedLLM, LLMObservatory, OpenAIClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("LLM Observatory SDK - Custom Attributes Example\n");

    let api_key =
        std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY environment variable not set");

    // Initialize observatory with custom attributes
    let observatory = LLMObservatory::builder()
        .with_service_name("custom-attributes-example")
        .with_environment("production")
        .with_attribute("region", "us-east-1")
        .with_attribute("team", "ai-research")
        .with_attribute("version", "1.2.3")
        .build()?;

    let client = OpenAIClient::new(api_key).with_observatory(observatory);

    println!("Making requests with custom metadata...\n");

    // Simulate multiple user requests with different metadata
    let users = vec![
        ("user_123", "session_abc", "What is machine learning?"),
        ("user_456", "session_xyz", "Explain neural networks"),
        ("user_789", "session_def", "What is deep learning?"),
    ];

    for (user_id, session_id, prompt) in users {
        println!("Processing request for user: {}", user_id);

        let request = ChatCompletionRequest::new("gpt-4o-mini")
            .with_system("You are an AI tutor.")
            .with_user(prompt)
            .with_user_id(user_id)
            .with_metadata("session_id", session_id)
            .with_metadata("feature", "ai-tutor")
            .with_metadata("experiment", "ab-test-v2")
            .with_temperature(0.7)
            .with_max_tokens(150);

        match client.chat_completion(request).await {
            Ok(response) => {
                println!(
                    "  Response: {}",
                    &response.content[..50.min(response.content.len())]
                );
                println!("  Cost: ${:.6}", response.cost_usd);
                println!("  Trace ID: {}", response.trace_id);

                // Custom metadata is preserved in the response
                if let Some(session) = response.metadata.get("session_id") {
                    println!("  Session: {}", session);
                }
            }
            Err(e) => {
                eprintln!("  Error: {}", e);
            }
        }

        println!();
    }

    println!("All requests completed!");
    println!("\nThese traces can be filtered by:");
    println!("  - user_id");
    println!("  - session_id");
    println!("  - feature (ai-tutor)");
    println!("  - experiment (ab-test-v2)");
    println!("  - region (us-east-1)");
    println!("  - team (ai-research)");

    // Shutdown
    if let Some(observatory) = client.observatory() {
        observatory.shutdown().await?;
    }

    Ok(())
}
