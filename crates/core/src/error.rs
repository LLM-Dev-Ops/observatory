// Copyright 2025 LLM Observatory Contributors
// SPDX-License-Identifier: Apache-2.0

//! Error types for LLM Observatory.

/// Result type alias using LLM Observatory's Error type.
pub type Result<T> = std::result::Result<T, Error>;

/// Error types for LLM Observatory operations.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Provider error
    #[error("Provider error: {0}")]
    Provider(String),

    /// Storage error
    #[error("Storage error: {0}")]
    Storage(String),

    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// OpenTelemetry error
    #[error("OpenTelemetry error: {0}")]
    OpenTelemetry(String),

    /// Authentication error
    #[error("Authentication error: {0}")]
    Auth(String),

    /// Not found error
    #[error("Resource not found: {0}")]
    NotFound(String),

    /// Invalid input error
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),

    /// Execution context error (missing context, validation failure, enforcement rejection)
    #[error("Execution error: {0}")]
    Execution(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl Error {
    /// Create a configuration error.
    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }

    /// Create a provider error.
    pub fn provider(msg: impl Into<String>) -> Self {
        Self::Provider(msg.into())
    }

    /// Create a storage error.
    pub fn storage(msg: impl Into<String>) -> Self {
        Self::Storage(msg.into())
    }

    /// Create an authentication error.
    pub fn auth(msg: impl Into<String>) -> Self {
        Self::Auth(msg.into())
    }

    /// Create a not found error.
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::NotFound(msg.into())
    }

    /// Create an invalid input error.
    pub fn invalid_input(msg: impl Into<String>) -> Self {
        Self::InvalidInput(msg.into())
    }

    /// Create an internal error.
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }

    /// Create an execution error.
    pub fn execution(msg: impl Into<String>) -> Self {
        Self::Execution(msg.into())
    }
}
