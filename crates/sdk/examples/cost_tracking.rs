// Copyright 2025 LLM Observatory Contributors
// SPDX-License-Identifier: Apache-2.0

//! Example demonstrating cost tracking and analysis.
//!
//! This example shows:
//! - Tracking costs across multiple requests
//! - Comparing costs between models
//! - Cost estimation before making requests
//! - Aggregating cost statistics
//!
//! Run with:
//! ```bash
//! export OPENAI_API_KEY=sk-...
//! cargo run --example cost_tracking --features openai
//! ```

use llm_observatory_sdk::{
    cost::{estimate_cost, CostTracker},
    ChatCompletionRequest, InstrumentedLLM, LLMObservatory, OpenAIClient,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("LLM Observatory SDK - Cost Tracking Example\n");

    let api_key =
        std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY environment variable not set");

    let observatory = LLMObservatory::builder()
        .with_service_name("cost-tracking-example")
        .with_environment("development")
        .build()?;

    let client = OpenAIClient::new(api_key).with_observatory(observatory);

    // Example 1: Estimate cost before making request
    println!("=== Example 1: Cost Estimation ===");
    let estimated_cost = estimate_cost("gpt-4o-mini", 500, 200)?;
    println!(
        "Estimated cost for gpt-4o-mini (500 prompt, 200 completion tokens): ${:.6}",
        estimated_cost
    );
    println!();

    // Example 2: Track costs across multiple requests
    println!("=== Example 2: Cost Tracking ===");
    let mut cost_tracker = CostTracker::new();

    let test_cases = vec![
        ("gpt-4o-mini", "What is Rust?"),
        ("gpt-4o-mini", "Explain async programming"),
        ("gpt-4o-mini", "What is ownership?"),
    ];

    for (model, prompt) in test_cases {
        println!("Request: {} - {}", model, prompt);

        let request = ChatCompletionRequest::new(model)
            .with_user(prompt)
            .with_max_tokens(100);

        match client.chat_completion(request).await {
            Ok(response) => {
                println!(
                    "  Tokens: {} (prompt: {}, completion: {})",
                    response.total_tokens(),
                    response.prompt_tokens(),
                    response.completion_tokens()
                );
                println!("  Cost: ${:.6}", response.cost_usd);

                // Track the cost
                let cost = llm_observatory_sdk::Cost::new(response.cost_usd);
                cost_tracker.record(model, &cost, &response.usage);
            }
            Err(e) => {
                eprintln!("  Error: {}", e);
            }
        }
        println!();
    }

    // Display aggregated statistics
    println!("=== Cost Summary ===");
    println!("{}", cost_tracker.summary());
    println!();

    println!("=== Cost by Model ===");
    for (model, cost) in cost_tracker.costs_by_model() {
        println!("  {}: ${:.6}", model, cost);
    }
    println!();

    // Example 3: Compare costs across models
    println!("=== Example 3: Model Cost Comparison ===");
    let models = vec!["gpt-4o", "gpt-4o-mini", "gpt-3.5-turbo"];
    let tokens_prompt = 1000;
    let tokens_completion = 500;

    println!(
        "Cost comparison for {} prompt tokens and {} completion tokens:",
        tokens_prompt, tokens_completion
    );

    for model in models {
        match estimate_cost(model, tokens_prompt, tokens_completion) {
            Ok(cost) => {
                println!("  {}: ${:.6}", model, cost);
            }
            Err(e) => {
                println!("  {}: Error - {}", model, e);
            }
        }
    }
    println!();

    // Example 4: Cost-aware model selection
    println!("=== Example 4: Cost-Aware Model Selection ===");
    let budget_usd = 0.01;
    let estimated_tokens = 1500;

    println!("Budget: ${:.6}", budget_usd);
    println!("Estimated tokens: {}", estimated_tokens);
    println!("Affordable models:");

    for model in ["gpt-4o", "gpt-4o-mini", "gpt-3.5-turbo"] {
        if let Ok(cost) = estimate_cost(model, estimated_tokens, 500) {
            if cost <= budget_usd {
                println!("  {} (${:.6})", model, cost);
            }
        }
    }
    println!();

    println!("Cost tracking example completed!");

    // Shutdown
    if let Some(observatory) = client.observatory() {
        observatory.shutdown().await?;
    }

    Ok(())
}
