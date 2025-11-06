// Copyright 2025 LLM Observatory Contributors
// SPDX-License-Identifier: Apache-2.0

//! Error types for the LLM Observatory SDK.

/// Result type alias using the SDK's Error type.
pub type Result<T> = std::result::Result<T, Error>;

/// Error types for SDK operations.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// OpenTelemetry initialization error
    #[error("OpenTelemetry error: {0}")]
    OpenTelemetry(String),

    /// HTTP request error
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// API error from provider
    #[error("API error: {status} - {message}")]
    Api {
        /// HTTP status code
        status: u16,
        /// Error message
        message: String,
    },

    /// Rate limit exceeded
    #[error("Rate limit exceeded: {0}")]
    RateLimit(String),

    /// Authentication error
    #[error("Authentication error: {0}")]
    Auth(String),

    /// Invalid API key
    #[error("Invalid API key")]
    InvalidApiKey,

    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Invalid input
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Streaming error
    #[error("Streaming error: {0}")]
    Stream(String),

    /// Cost calculation error
    #[error("Cost calculation error: {0}")]
    CostCalculation(String),

    /// Model not found
    #[error("Model not found: {0}")]
    ModelNotFound(String),

    /// Timeout error
    #[error("Request timeout")]
    Timeout,

    /// Internal SDK error
    #[error("Internal error: {0}")]
    Internal(String),

    /// Core error propagation
    #[error("Core error: {0}")]
    Core(#[from] llm_observatory_core::Error),
}

impl Error {
    /// Create a configuration error.
    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }

    /// Create an API error.
    pub fn api(status: u16, message: impl Into<String>) -> Self {
        Self::Api {
            status,
            message: message.into(),
        }
    }

    /// Create a rate limit error.
    pub fn rate_limit(msg: impl Into<String>) -> Self {
        Self::RateLimit(msg.into())
    }

    /// Create an authentication error.
    pub fn auth(msg: impl Into<String>) -> Self {
        Self::Auth(msg.into())
    }

    /// Create an invalid input error.
    pub fn invalid_input(msg: impl Into<String>) -> Self {
        Self::InvalidInput(msg.into())
    }

    /// Create a streaming error.
    pub fn stream(msg: impl Into<String>) -> Self {
        Self::Stream(msg.into())
    }

    /// Create a cost calculation error.
    pub fn cost_calculation(msg: impl Into<String>) -> Self {
        Self::CostCalculation(msg.into())
    }

    /// Create an internal error.
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }

    /// Check if the error is retryable.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Error::RateLimit(_) | Error::Timeout | Error::Api { status: 429, .. }
        )
    }

    /// Check if the error is an authentication error.
    pub fn is_auth_error(&self) -> bool {
        matches!(
            self,
            Error::Auth(_) | Error::InvalidApiKey | Error::Api { status: 401, .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = Error::config("test config error");
        assert!(matches!(err, Error::Config(_)));
    }

    #[test]
    fn test_is_retryable() {
        let rate_limit = Error::rate_limit("too many requests");
        assert!(rate_limit.is_retryable());

        let timeout = Error::Timeout;
        assert!(timeout.is_retryable());

        let config = Error::config("bad config");
        assert!(!config.is_retryable());
    }

    #[test]
    fn test_is_auth_error() {
        let invalid_key = Error::InvalidApiKey;
        assert!(invalid_key.is_auth_error());

        let auth_err = Error::auth("invalid token");
        assert!(auth_err.is_auth_error());

        let api_401 = Error::api(401, "unauthorized");
        assert!(api_401.is_auth_error());

        let api_500 = Error::api(500, "server error");
        assert!(!api_500.is_auth_error());
    }
}
