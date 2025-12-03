// Copyright 2025 LLM Observatory Contributors
// SPDX-License-Identifier: Apache-2.0

//! Latency Lens adapter for Observatory.
//!
//! This module provides latency sampling and metrics hooks by consuming
//! the llm-latency-lens-core crate from the LLM-Dev-Ops ecosystem.
//!
//! # Features
//!
//! - High-precision timing with nanosecond resolution
//! - TTFT (Time to First Token) tracking
//! - Latency distribution analysis (percentiles)
//! - Metrics aggregation for reporting
//!
//! # Example
//!
//! ```ignore
//! use llm_observatory_adapters::upstream::latency::LatencyAdapter;
//!
//! let adapter = LatencyAdapter::new();
//!
//! // Start a timing measurement
//! let measurement = adapter.start_measurement();
//!
//! // Add checkpoints during operation
//! measurement.checkpoint("request_sent");
//! // ... do work ...
//! measurement.checkpoint("first_token");
//! // ... do more work ...
//!
//! // Finish and get results
//! let result = measurement.finish();
//! println!("Total duration: {:?}", result.total_duration);
//! ```

use llm_latency_lens_core::{
    Clock, RequestId, RequestMetadata, SessionId, Timestamp, TimingEngine, TimingMeasurement,
    TimingResult, TokenEvent,
};
use llm_observatory_core::types::Latency;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;

/// Errors that can occur during latency operations.
#[derive(Debug, Error)]
pub enum LatencyAdapterError {
    /// Invalid timing measurement
    #[error("Invalid timing measurement: {0}")]
    InvalidMeasurement(String),

    /// Missing checkpoint
    #[error("Missing checkpoint: {0}")]
    MissingCheckpoint(String),

    /// Calculation error
    #[error("Latency calculation error: {0}")]
    CalculationError(String),
}

/// Result type for latency operations.
pub type Result<T> = std::result::Result<T, LatencyAdapterError>;

/// Latency distribution statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyDistribution {
    /// Minimum latency
    pub min: Duration,
    /// Maximum latency
    pub max: Duration,
    /// Mean latency
    pub mean: Duration,
    /// Standard deviation
    pub std_dev: Duration,
    /// 50th percentile (median)
    pub p50: Duration,
    /// 90th percentile
    pub p90: Duration,
    /// 95th percentile
    pub p95: Duration,
    /// 99th percentile
    pub p99: Duration,
    /// Number of samples
    pub sample_count: usize,
}

impl Default for LatencyDistribution {
    fn default() -> Self {
        Self {
            min: Duration::ZERO,
            max: Duration::ZERO,
            mean: Duration::ZERO,
            std_dev: Duration::ZERO,
            p50: Duration::ZERO,
            p90: Duration::ZERO,
            p95: Duration::ZERO,
            p99: Duration::ZERO,
            sample_count: 0,
        }
    }
}

impl LatencyDistribution {
    /// Create a new distribution from a slice of durations.
    pub fn from_samples(samples: &[Duration]) -> Self {
        if samples.is_empty() {
            return Self::default();
        }

        let mut sorted: Vec<Duration> = samples.to_vec();
        sorted.sort();

        let n = sorted.len();
        let sum: Duration = sorted.iter().sum();
        let mean = sum / n as u32;

        // Calculate standard deviation
        let variance: f64 = sorted
            .iter()
            .map(|d| {
                let diff = d.as_nanos() as f64 - mean.as_nanos() as f64;
                diff * diff
            })
            .sum::<f64>()
            / n as f64;
        let std_dev = Duration::from_nanos(variance.sqrt() as u64);

        Self {
            min: sorted[0],
            max: sorted[n - 1],
            mean,
            std_dev,
            p50: sorted[n * 50 / 100],
            p90: sorted[n * 90 / 100],
            p95: sorted[n * 95 / 100],
            p99: sorted[(n * 99 / 100).min(n - 1)],
            sample_count: n,
        }
    }
}

/// Throughput statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThroughputStats {
    /// Mean tokens per second
    pub mean_tokens_per_second: f64,
    /// Minimum tokens per second
    pub min_tokens_per_second: f64,
    /// Maximum tokens per second
    pub max_tokens_per_second: f64,
    /// 95th percentile tokens per second
    pub p95_tokens_per_second: f64,
}

impl Default for ThroughputStats {
    fn default() -> Self {
        Self {
            mean_tokens_per_second: 0.0,
            min_tokens_per_second: 0.0,
            max_tokens_per_second: 0.0,
            p95_tokens_per_second: 0.0,
        }
    }
}

/// A wrapper around timing measurement that tracks Observatory-specific checkpoints.
pub struct ObservatoryMeasurement {
    /// Session ID for this measurement
    session_id: SessionId,
    /// Request ID for this measurement
    request_id: RequestId,
    /// Start timestamp
    start_time: std::time::Instant,
    /// Checkpoints recorded during measurement
    checkpoints: Vec<(String, std::time::Instant)>,
    /// Time to first token (if recorded)
    ttft: Option<Duration>,
}

impl ObservatoryMeasurement {
    /// Create a new measurement.
    fn new(session_id: SessionId, request_id: RequestId) -> Self {
        Self {
            session_id,
            request_id,
            start_time: std::time::Instant::now(),
            checkpoints: Vec::new(),
            ttft: None,
        }
    }

    /// Add a checkpoint.
    pub fn checkpoint(&mut self, label: impl Into<String>) {
        self.checkpoints.push((label.into(), std::time::Instant::now()));
    }

    /// Record first token arrival.
    pub fn record_first_token(&mut self) {
        if self.ttft.is_none() {
            self.ttft = Some(self.start_time.elapsed());
            self.checkpoint("first_token");
        }
    }

    /// Get the session ID.
    pub fn session_id(&self) -> &SessionId {
        &self.session_id
    }

    /// Get the request ID.
    pub fn request_id(&self) -> &RequestId {
        &self.request_id
    }

    /// Get time to first token (if recorded).
    pub fn ttft(&self) -> Option<Duration> {
        self.ttft
    }

    /// Get elapsed time since start.
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Finish the measurement and return results.
    pub fn finish(self) -> ObservatoryTimingResult {
        let total_duration = self.start_time.elapsed();
        let checkpoint_durations: Vec<(String, Duration)> = self
            .checkpoints
            .iter()
            .map(|(label, instant)| (label.clone(), instant.duration_since(self.start_time)))
            .collect();

        ObservatoryTimingResult {
            session_id: self.session_id,
            request_id: self.request_id,
            total_duration,
            ttft: self.ttft,
            checkpoints: checkpoint_durations,
        }
    }
}

/// Result of an Observatory timing measurement.
#[derive(Debug, Clone)]
pub struct ObservatoryTimingResult {
    /// Session ID
    pub session_id: SessionId,
    /// Request ID
    pub request_id: RequestId,
    /// Total duration
    pub total_duration: Duration,
    /// Time to first token
    pub ttft: Option<Duration>,
    /// Checkpoint durations
    pub checkpoints: Vec<(String, Duration)>,
}

impl ObservatoryTimingResult {
    /// Convert to Observatory Latency type.
    pub fn to_latency(&self, start_time: chrono::DateTime<chrono::Utc>) -> Latency {
        let end_time = start_time + chrono::Duration::from_std(self.total_duration).unwrap_or_default();

        let mut latency = Latency::new(start_time, end_time);
        if let Some(ttft) = self.ttft {
            latency = latency.with_ttft(ttft.as_millis() as u64);
        }
        latency
    }

    /// Get a specific checkpoint duration.
    pub fn get_checkpoint(&self, label: &str) -> Option<Duration> {
        self.checkpoints
            .iter()
            .find(|(l, _)| l == label)
            .map(|(_, d)| *d)
    }

    /// Get total duration in milliseconds.
    pub fn total_ms(&self) -> u64 {
        self.total_duration.as_millis() as u64
    }

    /// Get total duration in microseconds.
    pub fn total_micros(&self) -> u64 {
        self.total_duration.as_micros() as u64
    }

    /// Get total duration in nanoseconds.
    pub fn total_nanos(&self) -> u128 {
        self.total_duration.as_nanos()
    }
}

/// Adapter for consuming llm-latency-lens-core functionality.
///
/// Provides a simplified interface for Observatory to interact with
/// the LLM-Dev-Ops Latency Lens for latency sampling and metrics.
pub struct LatencyAdapter {
    /// Current session ID
    session_id: SessionId,
    /// Collected latency samples
    samples: Vec<Duration>,
    /// TTFT samples
    ttft_samples: Vec<Duration>,
    /// Inter-token latency samples
    inter_token_samples: Vec<Duration>,
}

impl Default for LatencyAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl LatencyAdapter {
    /// Create a new LatencyAdapter.
    pub fn new() -> Self {
        Self {
            session_id: SessionId::new(),
            samples: Vec::new(),
            ttft_samples: Vec::new(),
            inter_token_samples: Vec::new(),
        }
    }

    /// Create a new LatencyAdapter with a specific session ID.
    pub fn with_session(session_id: SessionId) -> Self {
        Self {
            session_id,
            samples: Vec::new(),
            ttft_samples: Vec::new(),
            inter_token_samples: Vec::new(),
        }
    }

    /// Get the current session ID.
    pub fn session_id(&self) -> &SessionId {
        &self.session_id
    }

    /// Start a new timing measurement.
    pub fn start_measurement(&self) -> ObservatoryMeasurement {
        ObservatoryMeasurement::new(self.session_id.clone(), RequestId::new())
    }

    /// Record a latency sample.
    pub fn record_sample(&mut self, duration: Duration) {
        self.samples.push(duration);
    }

    /// Record a TTFT sample.
    pub fn record_ttft(&mut self, duration: Duration) {
        self.ttft_samples.push(duration);
    }

    /// Record an inter-token latency sample.
    pub fn record_inter_token(&mut self, duration: Duration) {
        self.inter_token_samples.push(duration);
    }

    /// Record samples from a timing result.
    pub fn record_from_result(&mut self, result: &ObservatoryTimingResult) {
        self.record_sample(result.total_duration);
        if let Some(ttft) = result.ttft {
            self.record_ttft(ttft);
        }
    }

    /// Get total latency distribution.
    pub fn latency_distribution(&self) -> LatencyDistribution {
        LatencyDistribution::from_samples(&self.samples)
    }

    /// Get TTFT distribution.
    pub fn ttft_distribution(&self) -> LatencyDistribution {
        LatencyDistribution::from_samples(&self.ttft_samples)
    }

    /// Get inter-token latency distribution.
    pub fn inter_token_distribution(&self) -> LatencyDistribution {
        LatencyDistribution::from_samples(&self.inter_token_samples)
    }

    /// Get the number of samples collected.
    pub fn sample_count(&self) -> usize {
        self.samples.len()
    }

    /// Clear all samples.
    pub fn clear(&mut self) {
        self.samples.clear();
        self.ttft_samples.clear();
        self.inter_token_samples.clear();
    }

    /// Calculate throughput from token count and duration.
    pub fn calculate_throughput(tokens: u32, duration: Duration) -> f64 {
        if duration.is_zero() {
            return 0.0;
        }
        tokens as f64 / duration.as_secs_f64()
    }

    /// Check if a latency exceeds a threshold.
    pub fn exceeds_threshold(latency: Duration, threshold_ms: u64) -> bool {
        latency.as_millis() as u64 > threshold_ms
    }

    /// Convert Observatory Latency to Duration.
    pub fn latency_to_duration(latency: &Latency) -> Duration {
        Duration::from_millis(latency.total_ms)
    }

    /// Create a Latency from Duration and timestamps.
    pub fn duration_to_latency(
        duration: Duration,
        start_time: chrono::DateTime<chrono::Utc>,
    ) -> Latency {
        let end_time = start_time + chrono::Duration::from_std(duration).unwrap_or_default();
        Latency::new(start_time, end_time)
    }

    /// Get aggregated statistics.
    pub fn aggregate_stats(&self) -> AggregatedLatencyStats {
        AggregatedLatencyStats {
            total_latency: self.latency_distribution(),
            ttft: self.ttft_distribution(),
            inter_token: self.inter_token_distribution(),
            sample_count: self.sample_count(),
        }
    }
}

/// Aggregated latency statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedLatencyStats {
    /// Total latency distribution
    pub total_latency: LatencyDistribution,
    /// TTFT distribution
    pub ttft: LatencyDistribution,
    /// Inter-token latency distribution
    pub inter_token: LatencyDistribution,
    /// Total number of samples
    pub sample_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_latency_adapter_creation() {
        let adapter = LatencyAdapter::new();
        assert_eq!(adapter.sample_count(), 0);
    }

    #[test]
    fn test_measurement_flow() {
        let adapter = LatencyAdapter::new();
        let mut measurement = adapter.start_measurement();

        measurement.checkpoint("start");
        std::thread::sleep(Duration::from_millis(10));
        measurement.record_first_token();
        std::thread::sleep(Duration::from_millis(10));
        measurement.checkpoint("end");

        let result = measurement.finish();
        assert!(result.total_duration >= Duration::from_millis(20));
        assert!(result.ttft.is_some());
    }

    #[test]
    fn test_latency_distribution() {
        let samples = vec![
            Duration::from_millis(100),
            Duration::from_millis(150),
            Duration::from_millis(200),
            Duration::from_millis(250),
            Duration::from_millis(300),
        ];

        let dist = LatencyDistribution::from_samples(&samples);
        assert_eq!(dist.min, Duration::from_millis(100));
        assert_eq!(dist.max, Duration::from_millis(300));
        assert_eq!(dist.sample_count, 5);
    }

    #[test]
    fn test_record_samples() {
        let mut adapter = LatencyAdapter::new();

        adapter.record_sample(Duration::from_millis(100));
        adapter.record_sample(Duration::from_millis(200));
        adapter.record_ttft(Duration::from_millis(50));

        assert_eq!(adapter.sample_count(), 2);
        assert_eq!(adapter.ttft_samples.len(), 1);
    }

    #[test]
    fn test_calculate_throughput() {
        let throughput = LatencyAdapter::calculate_throughput(1000, Duration::from_secs(1));
        assert_eq!(throughput, 1000.0);

        let throughput = LatencyAdapter::calculate_throughput(500, Duration::from_millis(500));
        assert_eq!(throughput, 1000.0);
    }

    #[test]
    fn test_exceeds_threshold() {
        assert!(LatencyAdapter::exceeds_threshold(
            Duration::from_millis(5001),
            5000
        ));
        assert!(!LatencyAdapter::exceeds_threshold(
            Duration::from_millis(4999),
            5000
        ));
    }
}
