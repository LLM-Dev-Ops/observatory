//! Metric data models.
//!
//! This module defines the data structures for storing metrics
//! (counters, gauges, histograms, etc.).

use crate::error::{StorageError, StorageResult};
use crate::validation::{validate_finite_f64, validate_not_empty, Validate};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Type of metric
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "metric_type", rename_all = "lowercase")]
pub enum MetricType {
    /// Counter - monotonically increasing value
    Counter,
    /// Gauge - value that can go up or down
    Gauge,
    /// Histogram - distribution of values
    Histogram,
    /// Summary - similar to histogram with quantiles
    Summary,
}

/// A metric definition and metadata.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Metric {
    /// Unique metric identifier
    pub id: Uuid,

    /// Metric name
    pub name: String,

    /// Metric description
    pub description: Option<String>,

    /// Metric unit (e.g., "bytes", "ms", "requests")
    pub unit: Option<String>,

    /// Metric type
    pub metric_type: String, // Stored as string in DB, convert to/from MetricType

    /// Service name
    pub service_name: String,

    /// Metric attributes/labels as JSON
    pub attributes: serde_json::Value,

    /// Resource attributes as JSON
    pub resource_attributes: serde_json::Value,

    /// Created timestamp
    pub created_at: DateTime<Utc>,

    /// Updated timestamp
    pub updated_at: DateTime<Utc>,
}

/// A single data point for a metric.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MetricDataPoint {
    /// Unique data point identifier
    pub id: Uuid,

    /// Metric ID this data point belongs to
    pub metric_id: Uuid,

    /// Timestamp of the data point
    pub timestamp: DateTime<Utc>,

    /// Numeric value (for counter, gauge)
    pub value: Option<f64>,

    /// Count (for histogram, summary)
    pub count: Option<i64>,

    /// Sum (for histogram, summary)
    pub sum: Option<f64>,

    /// Minimum value (for histogram, summary)
    pub min: Option<f64>,

    /// Maximum value (for histogram, summary)
    pub max: Option<f64>,

    /// Histogram buckets as JSON
    /// Format: [{"boundary": 0.1, "count": 10}, ...]
    pub buckets: Option<serde_json::Value>,

    /// Summary quantiles as JSON
    /// Format: [{"quantile": 0.5, "value": 100}, ...]
    pub quantiles: Option<serde_json::Value>,

    /// Exemplar data as JSON (sample traces)
    pub exemplars: Option<serde_json::Value>,

    /// Data point attributes as JSON
    pub attributes: serde_json::Value,

    /// Created timestamp
    pub created_at: DateTime<Utc>,
}

/// Histogram bucket for distribution metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistogramBucket {
    /// Upper boundary of the bucket
    pub boundary: f64,

    /// Count of values in this bucket
    pub count: i64,
}

/// Summary quantile for summary metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryQuantile {
    /// Quantile (0.0 to 1.0, e.g., 0.5 for median)
    pub quantile: f64,

    /// Value at this quantile
    pub value: f64,
}

/// Exemplar - a sample trace associated with a metric.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Exemplar {
    /// Trace ID associated with this exemplar
    pub trace_id: String,

    /// Span ID associated with this exemplar
    pub span_id: String,

    /// Value of the exemplar
    pub value: f64,

    /// Timestamp of the exemplar
    pub timestamp: DateTime<Utc>,

    /// Additional attributes
    pub attributes: serde_json::Value,
}

// TODO: Implement methods for creating and querying metrics
impl Metric {
    /// Create a new metric.
    pub fn new(/* TODO: Add parameters */) -> Self {
        todo!("Implement Metric::new")
    }

    /// Parse metric type from string.
    pub fn parse_type(s: &str) -> Result<MetricType, String> {
        match s.to_lowercase().as_str() {
            "counter" => Ok(MetricType::Counter),
            "gauge" => Ok(MetricType::Gauge),
            "histogram" => Ok(MetricType::Histogram),
            "summary" => Ok(MetricType::Summary),
            _ => Err(format!("Unknown metric type: {}", s)),
        }
    }
}

impl MetricDataPoint {
    /// Create a new data point.
    pub fn new(/* TODO: Add parameters */) -> Self {
        todo!("Implement MetricDataPoint::new")
    }

    /// Check if this is a counter data point.
    pub fn is_counter(&self) -> bool {
        self.value.is_some() && self.buckets.is_none() && self.quantiles.is_none()
    }

    /// Check if this is a histogram data point.
    pub fn is_histogram(&self) -> bool {
        self.buckets.is_some()
    }
}

impl std::fmt::Display for MetricType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MetricType::Counter => write!(f, "counter"),
            MetricType::Gauge => write!(f, "gauge"),
            MetricType::Histogram => write!(f, "histogram"),
            MetricType::Summary => write!(f, "summary"),
        }
    }
}

impl Validate for Metric {
    fn validate(&self) -> StorageResult<()> {
        // Validate name is not empty
        validate_not_empty(&self.name, "name").map_err(|e| StorageError::validation(e))?;

        // Validate service_name is not empty
        validate_not_empty(&self.service_name, "service_name")
            .map_err(|e| StorageError::validation(e))?;

        // Validate metric_type is one of the allowed values
        if Metric::parse_type(&self.metric_type).is_err() {
            return Err(StorageError::validation(format!(
                "metric_type must be one of [counter, gauge, histogram, summary], got: {}",
                self.metric_type
            )));
        }

        Ok(())
    }
}

impl Validate for MetricDataPoint {
    fn validate(&self) -> StorageResult<()> {
        // Validate value if present (must be finite)
        if let Some(value) = self.value {
            validate_finite_f64(value, "value").map_err(|e| StorageError::validation(e))?;
        }

        // Validate count if present (must be non-negative)
        if let Some(count) = self.count {
            if count < 0 {
                return Err(StorageError::validation(format!(
                    "count must be non-negative, got: {}",
                    count
                )));
            }
        }

        // Validate sum if present (must be finite)
        if let Some(sum) = self.sum {
            validate_finite_f64(sum, "sum").map_err(|e| StorageError::validation(e))?;
        }

        // Validate min if present (must be finite)
        if let Some(min) = self.min {
            validate_finite_f64(min, "min").map_err(|e| StorageError::validation(e))?;
        }

        // Validate max if present (must be finite)
        if let Some(max) = self.max {
            validate_finite_f64(max, "max").map_err(|e| StorageError::validation(e))?;
        }

        // Validate min <= max if both present
        if let (Some(min), Some(max)) = (self.min, self.max) {
            if min > max {
                return Err(StorageError::validation(format!(
                    "min ({}) must be <= max ({})",
                    min, max
                )));
            }
        }

        // Validate buckets if present
        if let Some(ref buckets) = self.buckets {
            if let Some(buckets_array) = buckets.as_array() {
                for (i, bucket) in buckets_array.iter().enumerate() {
                    if let Some(obj) = bucket.as_object() {
                        // Validate boundary is finite
                        if let Some(boundary) = obj.get("boundary").and_then(|v| v.as_f64()) {
                            validate_finite_f64(boundary, &format!("buckets[{}].boundary", i))
                                .map_err(|e| StorageError::validation(e))?;
                        } else {
                            return Err(StorageError::validation(format!(
                                "buckets[{}].boundary must be a valid number",
                                i
                            )));
                        }

                        // Validate count is non-negative
                        if let Some(count) = obj.get("count").and_then(|v| v.as_i64()) {
                            if count < 0 {
                                return Err(StorageError::validation(format!(
                                    "buckets[{}].count must be non-negative, got: {}",
                                    i, count
                                )));
                            }
                        } else {
                            return Err(StorageError::validation(format!(
                                "buckets[{}].count must be a valid integer",
                                i
                            )));
                        }
                    }
                }
            }
        }

        // Validate quantiles if present
        if let Some(ref quantiles) = self.quantiles {
            if let Some(quantiles_array) = quantiles.as_array() {
                for (i, quantile) in quantiles_array.iter().enumerate() {
                    if let Some(obj) = quantile.as_object() {
                        // Validate quantile is in range [0, 1]
                        if let Some(q) = obj.get("quantile").and_then(|v| v.as_f64()) {
                            if q < 0.0 || q > 1.0 {
                                return Err(StorageError::validation(format!(
                                    "quantiles[{}].quantile must be between 0 and 1, got: {}",
                                    i, q
                                )));
                            }
                        } else {
                            return Err(StorageError::validation(format!(
                                "quantiles[{}].quantile must be a valid number",
                                i
                            )));
                        }

                        // Validate value is finite
                        if let Some(value) = obj.get("value").and_then(|v| v.as_f64()) {
                            validate_finite_f64(value, &format!("quantiles[{}].value", i))
                                .map_err(|e| StorageError::validation(e))?;
                        } else {
                            return Err(StorageError::validation(format!(
                                "quantiles[{}].value must be a valid number",
                                i
                            )));
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metric_type_display() {
        assert_eq!(MetricType::Counter.to_string(), "counter");
        assert_eq!(MetricType::Gauge.to_string(), "gauge");
    }

    #[test]
    fn test_parse_metric_type() {
        assert_eq!(Metric::parse_type("counter").unwrap(), MetricType::Counter);
        assert_eq!(Metric::parse_type("GAUGE").unwrap(), MetricType::Gauge);
        assert!(Metric::parse_type("unknown").is_err());
    }

    // TODO: Add more comprehensive tests
}
