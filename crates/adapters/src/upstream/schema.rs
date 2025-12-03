// Copyright 2025 LLM Observatory Contributors
// SPDX-License-Identifier: Apache-2.0

//! Schema Registry adapter for Observatory.
//!
//! This module provides schema loading and validation capabilities by consuming
//! the schema-registry-core crate from the LLM-Dev-Ops ecosystem.
//!
//! # Features
//!
//! - Schema loading from the registry
//! - Span data validation against schemas
//! - Schema versioning support
//! - Compatibility checking
//!
//! # Example
//!
//! ```ignore
//! use llm_observatory_adapters::upstream::schema::SchemaAdapter;
//!
//! let adapter = SchemaAdapter::new();
//!
//! // Validate span data against a schema
//! let validation_result = adapter.validate_span_data(&span_json, "observatory.span.v1")?;
//! if validation_result.is_valid {
//!     println!("Span data is valid");
//! }
//! ```

use schema_registry_core::{
    CompatibilityMode, RegisteredSchema, SchemaInput, SchemaMetadata, SchemaState,
    SemanticVersion, SerializationFormat,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Errors that can occur during schema operations.
#[derive(Debug, Error)]
pub enum SchemaAdapterError {
    /// Schema not found in registry
    #[error("Schema not found: {0}")]
    NotFound(String),

    /// Schema validation failed
    #[error("Schema validation failed: {0}")]
    ValidationFailed(String),

    /// Compatibility check failed
    #[error("Schema compatibility check failed: {0}")]
    CompatibilityFailed(String),

    /// Internal error from upstream crate
    #[error("Schema registry error: {0}")]
    RegistryError(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

/// Result type for schema operations.
pub type Result<T> = std::result::Result<T, SchemaAdapterError>;

/// Schema validation result from the registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Whether the data is valid against the schema
    pub is_valid: bool,
    /// Validation errors (if any)
    pub errors: Vec<ValidationError>,
    /// Validation warnings (if any)
    pub warnings: Vec<String>,
}

/// A single validation error.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    /// Error message
    pub message: String,
    /// Path to the invalid field
    pub field_path: Option<String>,
    /// Error code
    pub code: String,
}

/// Schema reference with version information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaRef {
    /// Schema namespace
    pub namespace: String,
    /// Schema name
    pub name: String,
    /// Schema version
    pub version: String,
    /// Full qualified name (namespace.name)
    pub full_name: String,
}

/// Adapter for consuming schema-registry-core functionality.
///
/// Provides a simplified interface for Observatory to interact with
/// the LLM-Dev-Ops Schema Registry for schema loading and validation.
#[derive(Debug, Clone)]
pub struct SchemaAdapter {
    /// Default namespace for Observatory schemas
    default_namespace: String,
    /// Cached schema references
    schema_cache: HashMap<String, SchemaRef>,
}

impl Default for SchemaAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl SchemaAdapter {
    /// Create a new SchemaAdapter with default settings.
    pub fn new() -> Self {
        Self {
            default_namespace: "observatory".to_string(),
            schema_cache: HashMap::new(),
        }
    }

    /// Create a new SchemaAdapter with a custom default namespace.
    pub fn with_namespace(namespace: impl Into<String>) -> Self {
        Self {
            default_namespace: namespace.into(),
            schema_cache: HashMap::new(),
        }
    }

    /// Get the default namespace.
    pub fn default_namespace(&self) -> &str {
        &self.default_namespace
    }

    /// Create a schema input for registration.
    ///
    /// This helper creates a properly formatted SchemaInput that can be
    /// used with the schema registry for registration.
    pub fn create_schema_input(
        &self,
        name: impl Into<String>,
        content: impl Into<String>,
        description: impl Into<String>,
    ) -> SchemaInput {
        SchemaInput {
            name: name.into(),
            namespace: self.default_namespace.clone(),
            format: SerializationFormat::JsonSchema,
            content: content.into(),
            description: description.into(),
            compatibility_mode: CompatibilityMode::Backward,
            auto_activate: false,
            version: None,
            metadata: HashMap::new(),
            tags: vec!["observatory".to_string()],
            examples: vec![],
        }
    }

    /// Create a schema input for LLM span validation.
    pub fn create_span_schema_input(&self) -> SchemaInput {
        let schema_content = r#"{
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "required": ["span_id", "trace_id", "name", "provider", "model", "input", "latency"],
            "properties": {
                "span_id": {"type": "string"},
                "trace_id": {"type": "string"},
                "parent_span_id": {"type": ["string", "null"]},
                "name": {"type": "string"},
                "provider": {"type": "string"},
                "model": {"type": "string"},
                "input": {"type": "object"},
                "output": {"type": ["object", "null"]},
                "token_usage": {
                    "type": ["object", "null"],
                    "properties": {
                        "prompt_tokens": {"type": "integer", "minimum": 0},
                        "completion_tokens": {"type": "integer", "minimum": 0},
                        "total_tokens": {"type": "integer", "minimum": 0}
                    }
                },
                "cost": {
                    "type": ["object", "null"],
                    "properties": {
                        "amount_usd": {"type": "number", "minimum": 0}
                    }
                },
                "latency": {
                    "type": "object",
                    "required": ["total_ms", "start_time", "end_time"],
                    "properties": {
                        "total_ms": {"type": "integer", "minimum": 0},
                        "ttft_ms": {"type": ["integer", "null"], "minimum": 0},
                        "start_time": {"type": "string", "format": "date-time"},
                        "end_time": {"type": "string", "format": "date-time"}
                    }
                },
                "status": {"type": "string", "enum": ["OK", "ERROR", "UNSET"]}
            }
        }"#;

        self.create_schema_input("LlmSpan", schema_content, "Schema for LLM Observatory spans")
    }

    /// Validate JSON data against a simple schema structure.
    ///
    /// This is a lightweight validation that checks required fields
    /// without requiring a full schema registry connection.
    pub fn validate_span_json(&self, json_data: &serde_json::Value) -> ValidationResult {
        let mut errors = Vec::new();

        // Check required fields
        let required_fields = ["span_id", "trace_id", "name", "provider", "model", "input", "latency"];

        for field in required_fields {
            if json_data.get(field).is_none() {
                errors.push(ValidationError {
                    message: format!("Missing required field: {}", field),
                    field_path: Some(field.to_string()),
                    code: "REQUIRED_FIELD_MISSING".to_string(),
                });
            }
        }

        // Validate latency structure if present
        if let Some(latency) = json_data.get("latency") {
            if latency.get("total_ms").is_none() {
                errors.push(ValidationError {
                    message: "Missing required field: latency.total_ms".to_string(),
                    field_path: Some("latency.total_ms".to_string()),
                    code: "REQUIRED_FIELD_MISSING".to_string(),
                });
            }
        }

        // Validate token_usage if present
        if let Some(token_usage) = json_data.get("token_usage") {
            if !token_usage.is_null() {
                if let Some(total) = token_usage.get("total_tokens") {
                    if let Some(total_val) = total.as_i64() {
                        if total_val < 0 {
                            errors.push(ValidationError {
                                message: "token_usage.total_tokens must be non-negative".to_string(),
                                field_path: Some("token_usage.total_tokens".to_string()),
                                code: "INVALID_VALUE".to_string(),
                            });
                        }
                    }
                }
            }
        }

        ValidationResult {
            is_valid: errors.is_empty(),
            errors,
            warnings: vec![],
        }
    }

    /// Create a schema reference.
    pub fn create_schema_ref(
        &self,
        name: impl Into<String>,
        version: impl Into<String>,
    ) -> SchemaRef {
        let name = name.into();
        let namespace = self.default_namespace.clone();
        SchemaRef {
            full_name: format!("{}.{}", namespace, name),
            namespace,
            name,
            version: version.into(),
        }
    }

    /// Parse a semantic version string.
    pub fn parse_version(version_str: &str) -> Result<SemanticVersion> {
        let parts: Vec<&str> = version_str.split('.').collect();
        if parts.len() < 3 {
            return Err(SchemaAdapterError::ValidationFailed(
                "Invalid version format. Expected major.minor.patch".to_string(),
            ));
        }

        let major = parts[0].parse::<u32>().map_err(|_| {
            SchemaAdapterError::ValidationFailed("Invalid major version".to_string())
        })?;
        let minor = parts[1].parse::<u32>().map_err(|_| {
            SchemaAdapterError::ValidationFailed("Invalid minor version".to_string())
        })?;
        let patch = parts[2].parse::<u32>().map_err(|_| {
            SchemaAdapterError::ValidationFailed("Invalid patch version".to_string())
        })?;

        Ok(SemanticVersion::new(major, minor, patch))
    }

    /// Get supported serialization formats.
    pub fn supported_formats() -> Vec<SerializationFormat> {
        vec![
            SerializationFormat::JsonSchema,
            SerializationFormat::Avro,
            SerializationFormat::Protobuf,
        ]
    }

    /// Get supported compatibility modes.
    pub fn supported_compatibility_modes() -> Vec<CompatibilityMode> {
        vec![
            CompatibilityMode::Backward,
            CompatibilityMode::Forward,
            CompatibilityMode::Full,
            CompatibilityMode::None,
            CompatibilityMode::BackwardTransitive,
            CompatibilityMode::ForwardTransitive,
            CompatibilityMode::FullTransitive,
        ]
    }

    /// Check if a schema state allows modifications.
    pub fn is_modifiable_state(state: &SchemaState) -> bool {
        matches!(
            state,
            SchemaState::Draft | SchemaState::Registered | SchemaState::ValidationFailed
        )
    }

    /// Check if a schema state is terminal.
    pub fn is_terminal_state(state: &SchemaState) -> bool {
        matches!(state, SchemaState::Archived | SchemaState::Abandoned)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_adapter_creation() {
        let adapter = SchemaAdapter::new();
        assert_eq!(adapter.default_namespace(), "observatory");
    }

    #[test]
    fn test_schema_adapter_with_namespace() {
        let adapter = SchemaAdapter::with_namespace("custom");
        assert_eq!(adapter.default_namespace(), "custom");
    }

    #[test]
    fn test_create_schema_ref() {
        let adapter = SchemaAdapter::new();
        let schema_ref = adapter.create_schema_ref("LlmSpan", "1.0.0");
        assert_eq!(schema_ref.namespace, "observatory");
        assert_eq!(schema_ref.name, "LlmSpan");
        assert_eq!(schema_ref.full_name, "observatory.LlmSpan");
    }

    #[test]
    fn test_parse_version() {
        let version = SchemaAdapter::parse_version("1.2.3").unwrap();
        assert_eq!(version, SemanticVersion::new(1, 2, 3));
    }

    #[test]
    fn test_validate_span_json_valid() {
        let adapter = SchemaAdapter::new();
        let valid_json = serde_json::json!({
            "span_id": "span_123",
            "trace_id": "trace_456",
            "name": "llm.completion",
            "provider": "openai",
            "model": "gpt-4",
            "input": {"type": "text", "prompt": "Hello"},
            "latency": {
                "total_ms": 100,
                "start_time": "2025-01-01T00:00:00Z",
                "end_time": "2025-01-01T00:00:00Z"
            }
        });

        let result = adapter.validate_span_json(&valid_json);
        assert!(result.is_valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_validate_span_json_invalid() {
        let adapter = SchemaAdapter::new();
        let invalid_json = serde_json::json!({
            "span_id": "span_123"
            // Missing required fields
        });

        let result = adapter.validate_span_json(&invalid_json);
        assert!(!result.is_valid);
        assert!(!result.errors.is_empty());
    }
}
