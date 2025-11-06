// Copyright 2025 LLM Observatory Contributors
// SPDX-License-Identifier: Apache-2.0

//! LLM Observatory core implementation with OpenTelemetry integration.

use crate::{Error, Result};
use opentelemetry::{
    global,
    trace::TracerProvider as _,
    KeyValue,
};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    trace::{RandomIdGenerator, Sampler, TracerProvider},
    Resource,
};
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Central observatory for LLM instrumentation.
///
/// This struct manages the OpenTelemetry setup and provides tracing capabilities
/// for instrumented LLM clients.
///
/// # Example
///
/// ```rust,no_run
/// use llm_observatory_sdk::LLMObservatory;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let observatory = LLMObservatory::builder()
///         .with_service_name("my-llm-app")
///         .with_otlp_endpoint("http://localhost:4317")
///         .with_environment("production")
///         .build()?;
///
///     // Use observatory with your LLM clients...
///     Ok(())
/// }
/// ```
#[derive(Clone)]
pub struct LLMObservatory {
    tracer: Arc<opentelemetry::global::BoxedTracer>,
    service_name: String,
    environment: String,
}

impl LLMObservatory {
    /// Create a new builder for configuring the observatory.
    pub fn builder() -> ObservatoryBuilder {
        ObservatoryBuilder::default()
    }

    /// Get the tracer for creating spans.
    pub fn tracer(&self) -> &opentelemetry::global::BoxedTracer {
        &self.tracer
    }

    /// Get the service name.
    pub fn service_name(&self) -> &str {
        &self.service_name
    }

    /// Get the environment name.
    pub fn environment(&self) -> &str {
        &self.environment
    }

    /// Shutdown the observatory and flush all pending telemetry.
    pub async fn shutdown(&self) -> Result<()> {
        global::shutdown_tracer_provider();
        Ok(())
    }
}

/// Builder for configuring and creating an [`LLMObservatory`] instance.
///
/// # Example
///
/// ```rust,no_run
/// use llm_observatory_sdk::ObservatoryBuilder;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let observatory = ObservatoryBuilder::default()
///         .with_service_name("my-service")
///         .with_otlp_endpoint("http://collector:4317")
///         .with_environment("staging")
///         .with_sampling_rate(0.1) // Sample 10% of traces
///         .build()?;
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone)]
pub struct ObservatoryBuilder {
    service_name: Option<String>,
    service_version: Option<String>,
    otlp_endpoint: Option<String>,
    environment: String,
    sampling_rate: f64,
    enable_console_export: bool,
    additional_attributes: Vec<KeyValue>,
}

impl Default for ObservatoryBuilder {
    fn default() -> Self {
        Self {
            service_name: None,
            service_version: Some(crate::VERSION.to_string()),
            otlp_endpoint: Some("http://localhost:4317".to_string()),
            environment: "development".to_string(),
            sampling_rate: 1.0,
            enable_console_export: false,
            additional_attributes: Vec::new(),
        }
    }
}

impl ObservatoryBuilder {
    /// Set the service name for telemetry.
    ///
    /// This name will appear in your observability platform to identify traces
    /// from this service.
    pub fn with_service_name(mut self, name: impl Into<String>) -> Self {
        self.service_name = Some(name.into());
        self
    }

    /// Set the service version.
    pub fn with_service_version(mut self, version: impl Into<String>) -> Self {
        self.service_version = Some(version.into());
        self
    }

    /// Set the OTLP gRPC endpoint for exporting traces.
    ///
    /// # Arguments
    ///
    /// * `endpoint` - OTLP gRPC endpoint (e.g., "http://localhost:4317")
    pub fn with_otlp_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.otlp_endpoint = Some(endpoint.into());
        self
    }

    /// Set the deployment environment (e.g., "production", "staging", "development").
    pub fn with_environment(mut self, env: impl Into<String>) -> Self {
        self.environment = env.into();
        self
    }

    /// Set the sampling rate for traces (0.0 to 1.0).
    ///
    /// A sampling rate of 1.0 means all traces are captured.
    /// A rate of 0.1 means 10% of traces are captured.
    pub fn with_sampling_rate(mut self, rate: f64) -> Self {
        self.sampling_rate = rate.clamp(0.0, 1.0);
        self
    }

    /// Enable console exporter for debugging (logs spans to stdout).
    pub fn with_console_export(mut self, enable: bool) -> Self {
        self.enable_console_export = enable;
        self
    }

    /// Add a custom resource attribute.
    pub fn with_attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.additional_attributes
            .push(KeyValue::new(key.into(), value.into()));
        self
    }

    /// Build the observatory instance.
    pub fn build(self) -> Result<LLMObservatory> {
        let service_name = self
            .service_name
            .ok_or_else(|| Error::config("service_name is required"))?;

        // Build resource attributes
        let mut resource_attrs = vec![
            KeyValue::new("service.name", service_name.clone()),
            KeyValue::new("deployment.environment", self.environment.clone()),
            KeyValue::new("telemetry.sdk.name", "llm-observatory-rust"),
            KeyValue::new("telemetry.sdk.language", "rust"),
            KeyValue::new("telemetry.sdk.version", crate::VERSION),
        ];

        if let Some(version) = &self.service_version {
            resource_attrs.push(KeyValue::new("service.version", version.clone()));
        }

        resource_attrs.extend(self.additional_attributes);

        let resource = Resource::new(resource_attrs);

        // Configure sampler based on sampling rate
        let sampler = if self.sampling_rate >= 1.0 {
            Sampler::AlwaysOn
        } else if self.sampling_rate <= 0.0 {
            Sampler::AlwaysOff
        } else {
            Sampler::TraceIdRatioBased(self.sampling_rate)
        };

        // Setup OTLP exporter
        let otlp_endpoint = self
            .otlp_endpoint
            .ok_or_else(|| Error::config("otlp_endpoint is required"))?;

        let exporter = opentelemetry_otlp::SpanExporter::builder()
            .with_tonic()
            .with_endpoint(&otlp_endpoint)
            .build()
            .map_err(|e| Error::OpenTelemetry(e.to_string()))?;

        // Create tracer provider
        let provider = TracerProvider::builder()
            .with_sampler(sampler)
            .with_id_generator(RandomIdGenerator::default())
            .with_resource(resource)
            .with_batch_exporter(exporter, opentelemetry_sdk::runtime::Tokio)
            .build();

        // Set global tracer provider
        let _ = global::set_tracer_provider(provider.clone());

        // Get tracer
        let tracer = provider.tracer("llm-observatory");

        // Setup tracing subscriber for console logging if enabled
        if self.enable_console_export {
            let filter = EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info"));

            tracing_subscriber::registry()
                .with(filter)
                .with(tracing_subscriber::fmt::layer())
                .try_init()
                .ok(); // Ignore if already initialized
        }

        Ok(LLMObservatory {
            tracer: Arc::new(tracer),
            service_name,
            environment: self.environment,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_defaults() {
        let builder = ObservatoryBuilder::default();
        assert_eq!(builder.environment, "development");
        assert_eq!(builder.sampling_rate, 1.0);
    }

    #[test]
    fn test_builder_configuration() {
        let builder = ObservatoryBuilder::default()
            .with_service_name("test-service")
            .with_environment("production")
            .with_sampling_rate(0.5);

        assert_eq!(builder.service_name, Some("test-service".to_string()));
        assert_eq!(builder.environment, "production");
        assert_eq!(builder.sampling_rate, 0.5);
    }

    #[test]
    fn test_sampling_rate_clamping() {
        let builder = ObservatoryBuilder::default().with_sampling_rate(1.5);
        assert_eq!(builder.sampling_rate, 1.0);

        let builder = ObservatoryBuilder::default().with_sampling_rate(-0.5);
        assert_eq!(builder.sampling_rate, 0.0);
    }

    #[test]
    fn test_build_without_service_name() {
        let result = ObservatoryBuilder::default().build();
        assert!(result.is_err());
    }
}
