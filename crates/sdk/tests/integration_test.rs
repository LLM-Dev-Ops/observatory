// Copyright 2025 LLM Observatory Contributors
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for the LLM Observatory SDK.
//!
//! These tests require API keys and should be run with:
//! ```bash
//! export OPENAI_API_KEY=sk-...
//! cargo test --features openai -- --ignored
//! ```

use llm_observatory_sdk::{
    cost::{calculate_cost, estimate_cost, CostTracker},
    ChatCompletionRequest, InstrumentedLLM, LLMObservatory, OpenAIClient, TokenUsage,
};

#[tokio::test]
#[ignore] // Requires API key
async fn test_openai_chat_completion() {
    let api_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set");

    let observatory = LLMObservatory::builder()
        .with_service_name("integration-test")
        .build()
        .expect("Failed to create observatory");

    let client = OpenAIClient::new(api_key).with_observatory(observatory);

    let request = ChatCompletionRequest::new("gpt-4o-mini")
        .with_user("What is 2+2? Answer with just the number.");

    let response = client
        .chat_completion(request)
        .await
        .expect("Request failed");

    assert!(!response.content.is_empty());
    assert!(response.content.contains("4"));
    assert!(response.total_tokens() > 0);
    assert!(response.cost_usd > 0.0);
    assert!(!response.trace_id.is_empty());
    assert!(!response.span_id.is_empty());
}

#[tokio::test]
#[ignore] // Requires API key
async fn test_openai_with_metadata() {
    let api_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set");

    let observatory = LLMObservatory::builder()
        .with_service_name("integration-test")
        .with_attribute("test", "metadata")
        .build()
        .expect("Failed to create observatory");

    let client = OpenAIClient::new(api_key).with_observatory(observatory);

    let request = ChatCompletionRequest::new("gpt-4o-mini")
        .with_user("Say hello")
        .with_user_id("test_user_123")
        .with_metadata("session", "test_session")
        .with_metadata("feature", "test_feature");

    let response = client
        .chat_completion(request)
        .await
        .expect("Request failed");

    assert!(response.metadata.get("session") == Some(&"test_session".to_string()));
    assert!(response.metadata.get("feature") == Some(&"test_feature".to_string()));
}

#[tokio::test]
#[ignore] // Requires API key
async fn test_cost_tracking() {
    let api_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set");

    let observatory = LLMObservatory::builder()
        .with_service_name("integration-test")
        .build()
        .expect("Failed to create observatory");

    let client = OpenAIClient::new(api_key).with_observatory(observatory);

    let mut tracker = CostTracker::new();

    let prompts = vec!["What is 1+1?", "What is 2+2?", "What is 3+3?"];

    for prompt in prompts {
        let request = ChatCompletionRequest::new("gpt-4o-mini")
            .with_user(prompt)
            .with_max_tokens(10);

        let response = client
            .chat_completion(request)
            .await
            .expect("Request failed");

        let cost = llm_observatory_sdk::Cost::new(response.cost_usd);
        tracker.record("gpt-4o-mini", &cost, &response.usage);
    }

    assert_eq!(tracker.request_count(), 3);
    assert!(tracker.total_cost() > 0.0);
    assert!(tracker.total_tokens() > 0);
    assert!(tracker.average_cost() > 0.0);
}

#[tokio::test]
async fn test_error_handling_invalid_model() {
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_else(|_| "fake-key".to_string());

    let client = OpenAIClient::new(api_key);

    let request = ChatCompletionRequest::new("invalid-model-12345").with_user("Hello");

    let result = client.chat_completion(request).await;
    assert!(result.is_err());
}

#[test]
fn test_request_validation() {
    // Empty model
    let request = ChatCompletionRequest::new("");
    assert!(request.validate().is_err());

    // No messages
    let request = ChatCompletionRequest::new("gpt-4o-mini");
    assert!(request.validate().is_err());

    // Invalid temperature
    let request = ChatCompletionRequest::new("gpt-4o-mini")
        .with_user("Hello")
        .with_temperature(5.0);
    assert!(request.validate().is_err());

    // Valid request
    let request = ChatCompletionRequest::new("gpt-4o-mini")
        .with_user("Hello")
        .with_temperature(0.7);
    assert!(request.validate().is_ok());
}

#[test]
fn test_cost_calculation() {
    let usage = TokenUsage::new(1000, 500);

    // GPT-4o mini: $0.15 per 1M input, $0.60 per 1M output
    let cost = calculate_cost("gpt-4o-mini", &usage).expect("Cost calculation failed");

    // Expected: (1000/1000 * 0.00015) + (500/1000 * 0.0006) = 0.00015 + 0.0003 = 0.00045
    assert!((cost.amount_usd - 0.00045).abs() < 0.000001);
}

#[test]
fn test_cost_estimation() {
    let estimated = estimate_cost("gpt-4o-mini", 1000, 500).expect("Estimation failed");
    assert!(estimated > 0.0);
    assert!(estimated < 0.01); // Should be less than a penny
}

#[test]
fn test_cost_tracker() {
    let mut tracker = CostTracker::new();

    let usage1 = TokenUsage::new(100, 50);
    let cost1 = llm_observatory_sdk::Cost::new(0.001);
    tracker.record("model1", &cost1, &usage1);

    let usage2 = TokenUsage::new(200, 100);
    let cost2 = llm_observatory_sdk::Cost::new(0.002);
    tracker.record("model2", &cost2, &usage2);

    assert_eq!(tracker.request_count(), 2);
    assert_eq!(tracker.total_prompt_tokens(), 300);
    assert_eq!(tracker.total_completion_tokens(), 150);
    assert!((tracker.total_cost() - 0.003).abs() < 0.000001);
    assert!((tracker.average_cost() - 0.0015).abs() < 0.000001);

    let summary = tracker.summary();
    assert!(summary.contains("Requests: 2"));
}

#[test]
fn test_observatory_builder() {
    let result = LLMObservatory::builder()
        .with_service_name("test-service")
        .with_environment("test")
        .with_sampling_rate(0.5)
        .with_attribute("key", "value")
        .build();

    assert!(result.is_ok());

    let observatory = result.unwrap();
    assert_eq!(observatory.service_name(), "test-service");
    assert_eq!(observatory.environment(), "test");
}

#[test]
fn test_observatory_builder_validation() {
    // Missing service name
    let result = LLMObservatory::builder().build();
    assert!(result.is_err());
}

#[test]
fn test_error_types() {
    use llm_observatory_sdk::Error;

    let rate_limit = Error::rate_limit("too many requests");
    assert!(rate_limit.is_retryable());
    assert!(!rate_limit.is_auth_error());

    let auth_error = Error::auth("invalid key");
    assert!(!auth_error.is_retryable());
    assert!(auth_error.is_auth_error());

    let api_error = Error::api(429, "rate limit");
    assert!(api_error.is_retryable());

    let config_error = Error::config("bad config");
    assert!(!config_error.is_retryable());
    assert!(!config_error.is_auth_error());
}

#[test]
fn test_request_builder() {
    let request = ChatCompletionRequest::new("gpt-4o")
        .with_system("You are helpful")
        .with_user("Hello")
        .with_assistant("Hi there")
        .with_user("How are you?")
        .with_temperature(0.7)
        .with_max_tokens(100)
        .with_top_p(0.9)
        .with_user_id("user123")
        .with_metadata("key", "value");

    assert_eq!(request.model, "gpt-4o");
    assert_eq!(request.messages.len(), 4);
    assert_eq!(request.temperature, Some(0.7));
    assert_eq!(request.max_tokens, Some(100));
    assert_eq!(request.top_p, Some(0.9));
    assert_eq!(request.user, Some("user123".to_string()));
    assert_eq!(
        request.metadata.as_ref().unwrap().get("key"),
        Some(&"value".to_string())
    );
}
