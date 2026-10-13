// Copyright 2025 LLM Observatory Contributors
// SPDX-License-Identifier: Apache-2.0

//! Example demonstrating comprehensive error handling.
//!
//! This example shows:
//! - Handling API errors (rate limits, auth errors, etc.)
//! - Retry logic for transient errors
//! - Error span recording
//! - Graceful degradation
//!
//! Run with:
//! ```bash
//! export OPENAI_API_KEY=sk-...
//! cargo run --example error_handling --features openai
//! ```

use llm_observatory_sdk::{
    ChatCompletionRequest, Error, InstrumentedLLM, LLMObservatory, OpenAIClient,
};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("LLM Observatory SDK - Error Handling Example\n");

    let api_key =
        std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY environment variable not set");

    let observatory = LLMObservatory::builder()
        .with_service_name("error-handling-example")
        .with_environment("development")
        .build()?;

    let client = OpenAIClient::new(api_key).with_observatory(observatory);

    // Example 1: Handle invalid model
    println!("=== Example 1: Invalid Model ===");
    let request = ChatCompletionRequest::new("invalid-model-name").with_user("Hello!");

    match client.chat_completion(request).await {
        Ok(_) => println!("Unexpected success"),
        Err(e) => {
            println!("Expected error: {}", e);
            if let Error::Api { status, message } = &e {
                println!("  Status code: {}", status);
                println!("  Message: {}", message);
            }
        }
    }
    println!();

    // Example 2: Handle validation errors
    println!("=== Example 2: Validation Error ===");
    let invalid_request = ChatCompletionRequest::new("gpt-4o-mini").with_temperature(5.0); // Invalid temperature

    match invalid_request.validate() {
        Ok(_) => println!("Unexpected validation success"),
        Err(e) => println!("Validation error: {}", e),
    }
    println!();

    // Example 3: Retry logic for transient errors
    println!("=== Example 3: Retry Logic ===");
    let result = retry_with_backoff(
        || async {
            let request = ChatCompletionRequest::new("gpt-4o-mini")
                .with_user("What is 2+2?")
                .with_max_tokens(10);

            client.chat_completion(request).await
        },
        3,
        Duration::from_secs(1),
    )
    .await;

    match result {
        Ok(response) => {
            println!("Success after retry!");
            println!("Response: {}", response.content);
        }
        Err(e) => {
            println!("Failed after all retries: {}", e);
        }
    }
    println!();

    // Example 4: Check error types
    println!("=== Example 4: Error Type Checking ===");
    let errors = vec![
        Error::rate_limit("Too many requests"),
        Error::auth("Invalid API key"),
        Error::api(500, "Internal server error"),
        Error::config("Missing configuration"),
    ];

    for error in errors {
        println!("Error: {}", error);
        println!("  Is retryable? {}", error.is_retryable());
        println!("  Is auth error? {}", error.is_auth_error());
        println!();
    }

    println!("Error handling example completed!");

    // Shutdown
    if let Some(observatory) = client.observatory() {
        observatory.shutdown().await?;
    }

    Ok(())
}

/// Retry a future with exponential backoff.
async fn retry_with_backoff<F, Fut, T>(
    mut f: F,
    max_retries: u32,
    initial_delay: Duration,
) -> Result<T, Error>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, Error>>,
{
    let mut retries = 0;
    let mut delay = initial_delay;

    loop {
        match f().await {
            Ok(result) => return Ok(result),
            Err(e) if e.is_retryable() && retries < max_retries => {
                retries += 1;
                println!(
                    "  Retrying ({}/{}) after {:?}...",
                    retries, max_retries, delay
                );
                sleep(delay).await;
                delay *= 2; // Exponential backoff
            }
            Err(e) => return Err(e),
        }
    }
}
