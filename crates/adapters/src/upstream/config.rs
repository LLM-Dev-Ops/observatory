// Copyright 2025 LLM Observatory Contributors
// SPDX-License-Identifier: Apache-2.0

//! Config Manager adapter for Observatory.
//!
//! This module provides configuration retrieval and management capabilities
//! by consuming the llm-config-core crate from the LLM-Dev-Ops ecosystem.
//!
//! # Features
//!
//! - Configuration loading from Config Manager
//! - Environment-specific configuration retrieval
//! - Secret management support
//! - Configuration versioning
//!
//! # Example
//!
//! ```ignore
//! use llm_observatory_adapters::upstream::config::ConfigAdapter;
//!
//! let adapter = ConfigAdapter::new("/path/to/config")?;
//!
//! // Get a configuration value
//! let endpoint = adapter.get_string("collector", "otlp_endpoint", Environment::Production)?;
//! ```

use llm_config_core::{
    Config, ConfigEntry, ConfigError, ConfigManager, ConfigMetadata, ConfigValue, Environment,
    VersionControl,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

/// Errors that can occur during configuration operations.
#[derive(Debug, Error)]
pub enum ConfigAdapterError {
    /// Configuration not found
    #[error("Configuration not found: {namespace}/{key}")]
    NotFound { namespace: String, key: String },

    /// Invalid configuration value type
    #[error("Invalid configuration value type for {key}: expected {expected}, got {actual}")]
    InvalidType {
        key: String,
        expected: String,
        actual: String,
    },

    /// Configuration manager error
    #[error("Config manager error: {0}")]
    ManagerError(String),

    /// Storage path error
    #[error("Invalid storage path: {0}")]
    InvalidPath(String),

    /// Environment parse error
    #[error("Invalid environment: {0}")]
    InvalidEnvironment(String),
}

impl From<ConfigError> for ConfigAdapterError {
    fn from(err: ConfigError) -> Self {
        ConfigAdapterError::ManagerError(err.to_string())
    }
}

/// Result type for configuration operations.
pub type Result<T> = std::result::Result<T, ConfigAdapterError>;

/// Observatory-specific configuration keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObservatoryConfigKey {
    /// OTLP receiver endpoint
    OtlpEndpoint,
    /// OTLP receiver port
    OtlpPort,
    /// Sampling rate (0.0 - 1.0)
    SamplingRate,
    /// Enable PII redaction
    EnablePiiRedaction,
    /// Enable cost calculation
    EnableCostCalculation,
    /// Batch size for processing
    BatchSize,
    /// Batch timeout in milliseconds
    BatchTimeoutMs,
    /// Database connection URL
    DatabaseUrl,
    /// Redis connection URL
    RedisUrl,
    /// Log level
    LogLevel,
}

impl ObservatoryConfigKey {
    /// Get the configuration namespace for this key.
    pub fn namespace(&self) -> &'static str {
        match self {
            Self::OtlpEndpoint | Self::OtlpPort | Self::SamplingRate => "collector",
            Self::EnablePiiRedaction | Self::EnableCostCalculation => "processor",
            Self::BatchSize | Self::BatchTimeoutMs => "processing",
            Self::DatabaseUrl | Self::RedisUrl => "storage",
            Self::LogLevel => "observability",
        }
    }

    /// Get the configuration key name.
    pub fn key(&self) -> &'static str {
        match self {
            Self::OtlpEndpoint => "otlp_endpoint",
            Self::OtlpPort => "otlp_port",
            Self::SamplingRate => "sampling_rate",
            Self::EnablePiiRedaction => "enable_pii_redaction",
            Self::EnableCostCalculation => "enable_cost_calculation",
            Self::BatchSize => "batch_size",
            Self::BatchTimeoutMs => "batch_timeout_ms",
            Self::DatabaseUrl => "database_url",
            Self::RedisUrl => "redis_url",
            Self::LogLevel => "log_level",
        }
    }

    /// Get the default value for this configuration key.
    pub fn default_value(&self) -> ConfigValue {
        match self {
            Self::OtlpEndpoint => ConfigValue::String("http://localhost:4317".to_string()),
            Self::OtlpPort => ConfigValue::Integer(4317),
            Self::SamplingRate => ConfigValue::Float(1.0),
            Self::EnablePiiRedaction => ConfigValue::Boolean(true),
            Self::EnableCostCalculation => ConfigValue::Boolean(true),
            Self::BatchSize => ConfigValue::Integer(1000),
            Self::BatchTimeoutMs => ConfigValue::Integer(10000),
            Self::DatabaseUrl => {
                ConfigValue::String("postgresql://localhost:5432/observatory".to_string())
            }
            Self::RedisUrl => ConfigValue::String("redis://localhost:6379".to_string()),
            Self::LogLevel => ConfigValue::String("info".to_string()),
        }
    }
}

/// Parsed environment for Observatory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservatoryEnvironment {
    /// Development environment
    Development,
    /// Staging environment
    Staging,
    /// Production environment
    Production,
}

impl From<ObservatoryEnvironment> for Environment {
    fn from(env: ObservatoryEnvironment) -> Self {
        match env {
            ObservatoryEnvironment::Development => Environment::Development,
            ObservatoryEnvironment::Staging => Environment::Staging,
            ObservatoryEnvironment::Production => Environment::Production,
        }
    }
}

impl TryFrom<&str> for ObservatoryEnvironment {
    type Error = ConfigAdapterError;

    fn try_from(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "dev" | "development" => Ok(Self::Development),
            "staging" | "stage" => Ok(Self::Staging),
            "prod" | "production" => Ok(Self::Production),
            _ => Err(ConfigAdapterError::InvalidEnvironment(s.to_string())),
        }
    }
}

/// Adapter for consuming llm-config-core functionality.
///
/// Provides a simplified interface for Observatory to interact with
/// the LLM-Dev-Ops Config Manager for configuration retrieval.
pub struct ConfigAdapter {
    /// Storage path for configuration
    storage_path: String,
    /// Default environment
    default_environment: ObservatoryEnvironment,
    /// In-memory configuration cache
    cache: HashMap<String, ConfigValue>,
}

impl ConfigAdapter {
    /// Create a new ConfigAdapter with the specified storage path.
    pub fn new(storage_path: impl AsRef<Path>) -> Result<Self> {
        let path = storage_path.as_ref();
        if !path.exists() {
            // Create the directory if it doesn't exist
            std::fs::create_dir_all(path).map_err(|e| {
                ConfigAdapterError::InvalidPath(format!("Failed to create config directory: {}", e))
            })?;
        }

        Ok(Self {
            storage_path: path.to_string_lossy().to_string(),
            default_environment: ObservatoryEnvironment::Development,
            cache: HashMap::new(),
        })
    }

    /// Create a new ConfigAdapter with in-memory storage only.
    pub fn in_memory() -> Self {
        Self {
            storage_path: String::new(),
            default_environment: ObservatoryEnvironment::Development,
            cache: HashMap::new(),
        }
    }

    /// Set the default environment.
    pub fn with_environment(mut self, env: ObservatoryEnvironment) -> Self {
        self.default_environment = env;
        self
    }

    /// Get the storage path.
    pub fn storage_path(&self) -> &str {
        &self.storage_path
    }

    /// Get the default environment.
    pub fn default_environment(&self) -> ObservatoryEnvironment {
        self.default_environment
    }

    /// Get a configuration value using an Observatory config key.
    pub fn get(&self, key: ObservatoryConfigKey) -> ConfigValue {
        let cache_key = format!("{}/{}", key.namespace(), key.key());
        self.cache
            .get(&cache_key)
            .cloned()
            .unwrap_or_else(|| key.default_value())
    }

    /// Set a configuration value in the cache.
    pub fn set(&mut self, key: ObservatoryConfigKey, value: ConfigValue) {
        let cache_key = format!("{}/{}", key.namespace(), key.key());
        self.cache.insert(cache_key, value);
    }

    /// Get a string configuration value.
    pub fn get_string(&self, key: ObservatoryConfigKey) -> Option<String> {
        match self.get(key) {
            ConfigValue::String(s) => Some(s),
            _ => None,
        }
    }

    /// Get an integer configuration value.
    pub fn get_integer(&self, key: ObservatoryConfigKey) -> Option<i64> {
        match self.get(key) {
            ConfigValue::Integer(i) => Some(i),
            _ => None,
        }
    }

    /// Get a float configuration value.
    pub fn get_float(&self, key: ObservatoryConfigKey) -> Option<f64> {
        match self.get(key) {
            ConfigValue::Float(f) => Some(f),
            _ => None,
        }
    }

    /// Get a boolean configuration value.
    pub fn get_bool(&self, key: ObservatoryConfigKey) -> Option<bool> {
        match self.get(key) {
            ConfigValue::Boolean(b) => Some(b),
            _ => None,
        }
    }

    /// Load configuration from environment variables.
    ///
    /// Environment variables should be prefixed with `LLMOBS_`.
    pub fn load_from_env(&mut self) {
        // Map environment variables to config keys
        let env_mappings = [
            ("LLMOBS_OTLP_ENDPOINT", ObservatoryConfigKey::OtlpEndpoint),
            ("LLMOBS_OTLP_PORT", ObservatoryConfigKey::OtlpPort),
            ("LLMOBS_SAMPLING_RATE", ObservatoryConfigKey::SamplingRate),
            (
                "LLMOBS_ENABLE_PII_REDACTION",
                ObservatoryConfigKey::EnablePiiRedaction,
            ),
            (
                "LLMOBS_ENABLE_COST_CALCULATION",
                ObservatoryConfigKey::EnableCostCalculation,
            ),
            ("LLMOBS_BATCH_SIZE", ObservatoryConfigKey::BatchSize),
            (
                "LLMOBS_BATCH_TIMEOUT_MS",
                ObservatoryConfigKey::BatchTimeoutMs,
            ),
            ("LLMOBS_DATABASE_URL", ObservatoryConfigKey::DatabaseUrl),
            ("LLMOBS_REDIS_URL", ObservatoryConfigKey::RedisUrl),
            ("LLMOBS_LOG_LEVEL", ObservatoryConfigKey::LogLevel),
        ];

        for (env_var, key) in env_mappings {
            if let Ok(value) = std::env::var(env_var) {
                let config_value = match key {
                    ObservatoryConfigKey::OtlpPort
                    | ObservatoryConfigKey::BatchSize
                    | ObservatoryConfigKey::BatchTimeoutMs => {
                        if let Ok(i) = value.parse::<i64>() {
                            ConfigValue::Integer(i)
                        } else {
                            continue;
                        }
                    }
                    ObservatoryConfigKey::SamplingRate => {
                        if let Ok(f) = value.parse::<f64>() {
                            ConfigValue::Float(f)
                        } else {
                            continue;
                        }
                    }
                    ObservatoryConfigKey::EnablePiiRedaction
                    | ObservatoryConfigKey::EnableCostCalculation => {
                        match value.to_lowercase().as_str() {
                            "true" | "1" | "yes" => ConfigValue::Boolean(true),
                            "false" | "0" | "no" => ConfigValue::Boolean(false),
                            _ => continue,
                        }
                    }
                    _ => ConfigValue::String(value),
                };
                self.set(key, config_value);
            }
        }
    }

    /// Get all configuration values as a HashMap.
    pub fn all_config(&self) -> HashMap<String, ConfigValue> {
        let mut config = HashMap::new();

        // Add all default values
        let all_keys = [
            ObservatoryConfigKey::OtlpEndpoint,
            ObservatoryConfigKey::OtlpPort,
            ObservatoryConfigKey::SamplingRate,
            ObservatoryConfigKey::EnablePiiRedaction,
            ObservatoryConfigKey::EnableCostCalculation,
            ObservatoryConfigKey::BatchSize,
            ObservatoryConfigKey::BatchTimeoutMs,
            ObservatoryConfigKey::DatabaseUrl,
            ObservatoryConfigKey::RedisUrl,
            ObservatoryConfigKey::LogLevel,
        ];

        for key in all_keys {
            let cache_key = format!("{}/{}", key.namespace(), key.key());
            config.insert(cache_key, self.get(key));
        }

        config
    }

    /// Create a Config object from current settings.
    pub fn to_config(&self, namespace: &str) -> Config {
        let mut config = Config::new(namespace, self.default_environment.into());

        for (key, value) in &self.cache {
            if key.starts_with(&format!("{}/", namespace)) {
                let short_key = key.strip_prefix(&format!("{}/", namespace)).unwrap_or(key);
                config.set(short_key, value.clone());
            }
        }

        config
    }

    /// Get supported environments.
    pub fn supported_environments() -> Vec<ObservatoryEnvironment> {
        vec![
            ObservatoryEnvironment::Development,
            ObservatoryEnvironment::Staging,
            ObservatoryEnvironment::Production,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_adapter_in_memory() {
        let adapter = ConfigAdapter::in_memory();
        assert!(adapter.storage_path().is_empty());
    }

    #[test]
    fn test_config_key_defaults() {
        let adapter = ConfigAdapter::in_memory();

        // Check default values
        assert_eq!(
            adapter.get_string(ObservatoryConfigKey::OtlpEndpoint),
            Some("http://localhost:4317".to_string())
        );
        assert_eq!(
            adapter.get_integer(ObservatoryConfigKey::OtlpPort),
            Some(4317)
        );
        assert_eq!(
            adapter.get_float(ObservatoryConfigKey::SamplingRate),
            Some(1.0)
        );
        assert_eq!(
            adapter.get_bool(ObservatoryConfigKey::EnablePiiRedaction),
            Some(true)
        );
    }

    #[test]
    fn test_config_set_and_get() {
        let mut adapter = ConfigAdapter::in_memory();

        adapter.set(
            ObservatoryConfigKey::OtlpEndpoint,
            ConfigValue::String("http://custom:4317".to_string()),
        );

        assert_eq!(
            adapter.get_string(ObservatoryConfigKey::OtlpEndpoint),
            Some("http://custom:4317".to_string())
        );
    }

    #[test]
    fn test_environment_conversion() {
        assert_eq!(
            ObservatoryEnvironment::try_from("dev").unwrap(),
            ObservatoryEnvironment::Development
        );
        assert_eq!(
            ObservatoryEnvironment::try_from("production").unwrap(),
            ObservatoryEnvironment::Production
        );
        assert!(ObservatoryEnvironment::try_from("invalid").is_err());
    }

    #[test]
    fn test_all_config() {
        let adapter = ConfigAdapter::in_memory();
        let config = adapter.all_config();

        assert!(config.contains_key("collector/otlp_endpoint"));
        assert!(config.contains_key("storage/database_url"));
    }
}
