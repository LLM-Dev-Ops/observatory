//! # Metrics API Data Models (Phase 3)
//!
//! This module contains all data structures for the Phase 3 Metrics API.
//!
//! ## Endpoints
//! - `GET /api/v1/metrics` - Time-series metrics query
//! - `GET /api/v1/metrics/summary` - Metrics summary with period comparison
//! - `POST /api/v1/metrics/query` - Custom metrics query with advanced features
//!
//! ## Features
//! - Multiple metric types (duration, cost, tokens, errors, throughput)
//! - Flexible time bucketing (1min, 5min, 1hour, 1day)
//! - Multiple aggregation functions (avg, sum, min, max, count, p50, p95, p99)
//! - Group by multiple dimensions
//! - HAVING clause support for filtering aggregated results
//! - Automatic continuous aggregate table selection
//!
//! ## Security
//! - All metric names validated against whitelist
//! - Dimension names validated against whitelist
//! - Query complexity limits enforced
//! - SQL injection prevention via parameterized queries

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Enums and Types
// ============================================================================

/// Supported metric types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum MetricType {
    /// Request count
    RequestCount,
    /// Duration in milliseconds (avg, min, max, percentiles)
    Duration,
    /// Total cost in USD
    TotalCost,
    /// Prompt cost in USD
    PromptCost,
    /// Completion cost in USD
    CompletionCost,
    /// Total tokens
    TotalTokens,
    /// Prompt tokens
    PromptTokens,
    /// Completion tokens
    CompletionTokens,
    /// Error count
    ErrorCount,
    /// Success count
    SuccessCount,
    /// Error rate (computed as error_count / request_count)
    ErrorRate,
    /// Success rate (computed as success_count / request_count)
    SuccessRate,
    /// Throughput (requests per second)
    Throughput,
    /// Time to first token (TTFT) in milliseconds
    TimeToFirstToken,
    /// Unique users
    UniqueUsers,
    /// Unique sessions
    UniqueSessions,
}

impl MetricType {
    /// Returns the SQL column name for this metric type
    pub fn to_column_name(&self) -> &'static str {
        match self {
            MetricType::RequestCount => "request_count",
            MetricType::Duration => "avg_duration_ms",
            MetricType::TotalCost => "total_cost_usd",
            MetricType::PromptCost => "total_prompt_cost",
            MetricType::CompletionCost => "total_completion_cost",
            MetricType::TotalTokens => "total_tokens",
            MetricType::PromptTokens => "total_prompt_tokens",
            MetricType::CompletionTokens => "total_completion_tokens",
            MetricType::ErrorCount => "error_count",
            MetricType::SuccessCount => "success_count",
            MetricType::ErrorRate => "error_rate",
            MetricType::SuccessRate => "success_rate",
            MetricType::Throughput => "throughput",
            MetricType::TimeToFirstToken => "avg_ttft_ms",
            MetricType::UniqueUsers => "unique_users",
            MetricType::UniqueSessions => "unique_sessions",
        }
    }

    /// Returns whether this metric requires raw data query (percentiles)
    pub fn requires_raw_data(&self) -> bool {
        false // Most metrics are available in aggregates
    }
}

/// Aggregation functions for metrics
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AggregationFunction {
    /// Average value
    Avg,
    /// Sum of values
    Sum,
    /// Minimum value
    Min,
    /// Maximum value
    Max,
    /// Count of values
    Count,
    /// 50th percentile (median)
    P50,
    /// 90th percentile
    P90,
    /// 95th percentile
    P95,
    /// 99th percentile
    P99,
}

impl AggregationFunction {
    /// Returns the SQL function for this aggregation
    pub fn to_sql(&self) -> &'static str {
        match self {
            AggregationFunction::Avg => "AVG",
            AggregationFunction::Sum => "SUM",
            AggregationFunction::Min => "MIN",
            AggregationFunction::Max => "MAX",
            AggregationFunction::Count => "COUNT",
            AggregationFunction::P50 => "PERCENTILE_CONT(0.50) WITHIN GROUP (ORDER BY",
            AggregationFunction::P90 => "PERCENTILE_CONT(0.90) WITHIN GROUP (ORDER BY",
            AggregationFunction::P95 => "PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY",
            AggregationFunction::P99 => "PERCENTILE_CONT(0.99) WITHIN GROUP (ORDER BY",
        }
    }

    /// Returns whether this aggregation requires raw data query
    pub fn requires_raw_data(&self) -> bool {
        matches!(
            self,
            AggregationFunction::P50
                | AggregationFunction::P90
                | AggregationFunction::P95
                | AggregationFunction::P99
        )
    }
}

/// Time bucket intervals
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TimeInterval {
    /// 1 minute buckets
    OneMinute,
    /// 5 minute buckets
    FiveMinutes,
    /// 1 hour buckets
    OneHour,
    /// 1 day buckets
    OneDay,
}

impl TimeInterval {
    /// Returns the PostgreSQL interval string
    pub fn to_pg_interval(&self) -> &'static str {
        match self {
            TimeInterval::OneMinute => "1 minute",
            TimeInterval::FiveMinutes => "5 minutes",
            TimeInterval::OneHour => "1 hour",
            TimeInterval::OneDay => "1 day",
        }
    }

    /// Returns the appropriate aggregate table for this interval
    pub fn to_aggregate_table(&self) -> &'static str {
        match self {
            TimeInterval::OneMinute => "llm_metrics_1min",
            TimeInterval::FiveMinutes => "llm_metrics_1min", // Use 1min and aggregate
            TimeInterval::OneHour => "llm_metrics_1hour",
            TimeInterval::OneDay => "llm_metrics_1day",
        }
    }

    /// Returns whether this interval should use aggregates
    pub fn use_aggregates(&self) -> bool {
        true // All intervals have corresponding aggregates
    }
}

/// Supported dimension names for grouping
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum DimensionName {
    Provider,
    Model,
    Environment,
    StatusCode,
    UserId,
    SessionId,
}

impl DimensionName {
    /// Returns the SQL column name for this dimension
    pub fn to_column_name(&self) -> &'static str {
        match self {
            DimensionName::Provider => "provider",
            DimensionName::Model => "model",
            DimensionName::Environment => "environment",
            DimensionName::StatusCode => "status_code",
            DimensionName::UserId => "user_id",
            DimensionName::SessionId => "session_id",
        }
    }

    /// Returns whether this dimension is available in aggregate tables
    pub fn available_in_aggregates(&self) -> bool {
        matches!(
            self,
            DimensionName::Provider
                | DimensionName::Model
                | DimensionName::Environment
                | DimensionName::StatusCode
        )
    }
}

// ============================================================================
// Request Models
// ============================================================================

/// Request for GET /api/v1/metrics
#[derive(Debug, Deserialize, Clone)]
pub struct MetricsQueryRequest {
    /// Metric names to query (e.g., ["request_count", "duration", "total_cost"])
    pub metrics: Vec<MetricType>,

    /// Time interval for bucketing (default: 1hour)
    #[serde(default = "default_interval")]
    pub interval: TimeInterval,

    /// Start time (default: 24 hours ago)
    pub start_time: Option<DateTime<Utc>>,

    /// End time (default: now)
    pub end_time: Option<DateTime<Utc>>,

    /// Filter by provider
    pub provider: Option<String>,

    /// Filter by model
    pub model: Option<String>,

    /// Filter by environment
    pub environment: Option<String>,

    /// Filter by user ID
    pub user_id: Option<String>,

    /// Group by dimensions (e.g., ["provider", "model"])
    #[serde(default)]
    pub group_by: Vec<DimensionName>,

    /// Aggregation function (default: avg for duration/cost, sum for counts)
    pub aggregation: Option<AggregationFunction>,

    /// Include percentiles (requires raw data query, slower)
    #[serde(default)]
    pub include_percentiles: bool,
}

fn default_interval() -> TimeInterval {
    TimeInterval::OneHour
}

/// Request for POST /api/v1/metrics/query
#[derive(Debug, Deserialize, Clone)]
pub struct CustomMetricsQueryRequest {
    /// Metrics to query with their aggregation functions
    pub metrics: Vec<MetricAggregation>,

    /// Time interval for bucketing
    pub interval: TimeInterval,

    /// Start time
    pub start_time: DateTime<Utc>,

    /// End time
    pub end_time: DateTime<Utc>,

    /// Group by dimensions
    #[serde(default)]
    pub group_by: Vec<DimensionName>,

    /// Filter conditions (AND logic)
    #[serde(default)]
    pub filters: Vec<MetricFilter>,

    /// HAVING clause conditions for filtering aggregated results
    #[serde(default)]
    pub having: Vec<HavingCondition>,

    /// Sort by field and direction
    pub sort_by: Option<SortConfig>,

    /// Limit number of results (max 10000)
    #[serde(default = "default_limit")]
    pub limit: i32,
}

fn default_limit() -> i32 {
    1000
}

/// Metric with its aggregation function
#[derive(Debug, Deserialize, Clone)]
pub struct MetricAggregation {
    pub metric: MetricType,
    pub aggregation: AggregationFunction,
    #[serde(default)]
    pub alias: Option<String>,
}

/// Filter condition for metrics query
#[derive(Debug, Deserialize, Clone)]
pub struct MetricFilter {
    pub dimension: DimensionName,
    pub operator: FilterOperator,
    pub value: FilterValue,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum FilterOperator {
    Eq,
    Ne,
    In,
    NotIn,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum FilterValue {
    String(String),
    Array(Vec<String>),
}

/// HAVING clause condition for filtering aggregated results
#[derive(Debug, Deserialize, Clone)]
pub struct HavingCondition {
    pub metric: MetricType,
    pub aggregation: AggregationFunction,
    pub operator: ComparisonOperator,
    pub value: f64,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonOperator {
    Gt,
    Gte,
    Lt,
    Lte,
    Eq,
}

impl ComparisonOperator {
    pub fn to_sql(&self) -> &'static str {
        match self {
            ComparisonOperator::Gt => ">",
            ComparisonOperator::Gte => ">=",
            ComparisonOperator::Lt => "<",
            ComparisonOperator::Lte => "<=",
            ComparisonOperator::Eq => "=",
        }
    }
}

/// Sort configuration
#[derive(Debug, Deserialize, Clone)]
pub struct SortConfig {
    pub field: MetricType,
    #[serde(default)]
    pub descending: bool,
}

// ============================================================================
// Response Models
// ============================================================================

/// Response for GET /api/v1/metrics
#[derive(Debug, Serialize)]
pub struct MetricsResponse {
    /// Query metadata
    pub metadata: MetricsMetadata,
    /// Time-series data points
    pub data: Vec<MetricDataPoint>,
}

#[derive(Debug, Serialize)]
pub struct MetricsMetadata {
    pub interval: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub metrics: Vec<String>,
    pub group_by: Vec<String>,
    pub data_source: String, // "aggregate" or "raw"
    pub total_points: usize,
}

#[derive(Debug, Serialize)]
pub struct MetricDataPoint {
    pub timestamp: DateTime<Utc>,
    #[serde(flatten)]
    pub dimensions: HashMap<String, String>,
    #[serde(flatten)]
    pub metrics: HashMap<String, MetricValue>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum MetricValue {
    Integer(i64),
    Float(f64),
    Null,
}

/// Response for GET /api/v1/metrics/summary
#[derive(Debug, Serialize)]
pub struct MetricsSummaryResponse {
    /// Current period summary
    pub current_period: PeriodSummary,
    /// Previous period summary (for comparison)
    pub previous_period: Option<PeriodSummary>,
    /// Period-over-period changes
    pub changes: Option<PeriodChanges>,
    /// Top items by various dimensions
    pub top_items: TopItems,
    /// Quality metrics
    pub quality: QualitySummary,
}

#[derive(Debug, Serialize)]
pub struct PeriodSummary {
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub total_requests: i64,
    pub total_cost_usd: f64,
    pub total_tokens: i64,
    pub avg_duration_ms: f64,
    pub p95_duration_ms: Option<f64>,
    pub error_rate: f64,
    pub success_rate: f64,
    pub unique_users: i64,
    pub unique_sessions: i64,
}

#[derive(Debug, Serialize)]
pub struct PeriodChanges {
    pub requests_change_pct: f64,
    pub cost_change_pct: f64,
    pub duration_change_pct: f64,
    pub error_rate_change_pct: f64,
}

#[derive(Debug, Serialize)]
pub struct TopItems {
    pub by_cost: Vec<TopItem>,
    pub by_requests: Vec<TopItem>,
    pub by_duration: Vec<TopItem>,
    pub by_errors: Vec<TopItem>,
}

#[derive(Debug, Serialize)]
pub struct TopItem {
    pub provider: String,
    pub model: String,
    pub value: f64,
    pub percentage: f64,
}

#[derive(Debug, Serialize)]
pub struct QualitySummary {
    pub error_count: i64,
    pub success_count: i64,
    pub error_rate: f64,
    pub success_rate: f64,
    pub most_common_errors: Vec<ErrorSummaryItem>,
}

#[derive(Debug, Serialize)]
pub struct ErrorSummaryItem {
    pub status_code: String,
    pub count: i64,
    pub percentage: f64,
    pub sample_message: Option<String>,
}

/// Response for POST /api/v1/metrics/query
#[derive(Debug, Serialize)]
pub struct CustomMetricsResponse {
    pub metadata: CustomMetricsMetadata,
    pub data: Vec<CustomMetricDataPoint>,
}

#[derive(Debug, Serialize)]
pub struct CustomMetricsMetadata {
    pub interval: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub group_by: Vec<String>,
    pub filters_applied: usize,
    pub having_conditions: usize,
    pub total_rows: usize,
}

#[derive(Debug, Serialize)]
pub struct CustomMetricDataPoint {
    pub timestamp: DateTime<Utc>,
    #[serde(flatten)]
    pub dimensions: HashMap<String, String>,
    #[serde(flatten)]
    pub metrics: HashMap<String, MetricValue>,
}

// ============================================================================
// Internal Database Row Types
// ============================================================================

/// Row from aggregate tables
#[derive(Debug, sqlx::FromRow)]
pub struct AggregateMetricRow {
    pub bucket: DateTime<Utc>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub environment: Option<String>,
    pub status_code: Option<String>,
    pub request_count: Option<i64>,
    pub total_tokens: Option<i64>,
    pub total_cost_usd: Option<f64>,
    pub avg_duration_ms: Option<f64>,
    pub min_duration_ms: Option<i32>,
    pub max_duration_ms: Option<i32>,
    pub error_count: Option<i64>,
    pub success_count: Option<i64>,
    pub total_prompt_tokens: Option<i64>,
    pub total_completion_tokens: Option<i64>,
    pub total_prompt_cost: Option<f64>,
    pub total_completion_cost: Option<f64>,
    pub unique_users: Option<i64>,
    pub unique_sessions: Option<i64>,
}

/// Row from percentile queries on raw data
#[derive(Debug, sqlx::FromRow)]
pub struct PercentileMetricRow {
    pub bucket: DateTime<Utc>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub p50_duration_ms: Option<f64>,
    pub p90_duration_ms: Option<f64>,
    pub p95_duration_ms: Option<f64>,
    pub p99_duration_ms: Option<f64>,
}

/// Summary aggregation row
#[derive(Debug, sqlx::FromRow)]
pub struct SummaryRow {
    pub total_requests: Option<i64>,
    pub total_cost_usd: Option<f64>,
    pub total_tokens: Option<i64>,
    pub avg_duration_ms: Option<f64>,
    pub error_count: Option<i64>,
    pub success_count: Option<i64>,
    pub unique_users: Option<i64>,
    pub unique_sessions: Option<i64>,
}

/// Top item row
#[derive(Debug, sqlx::FromRow)]
pub struct TopItemRow {
    pub provider: String,
    pub model: String,
    pub value: Option<f64>,
}

/// Error summary row
#[derive(Debug, sqlx::FromRow)]
pub struct ErrorSummaryRow {
    pub status_code: String,
    pub error_count: i64,
    pub sample_error_message: Option<String>,
}

// ============================================================================
// Validation
// ============================================================================

impl MetricsQueryRequest {
    /// Validates the request
    pub fn validate(&self) -> Result<(), String> {
        if self.metrics.is_empty() {
            return Err("At least one metric must be specified".to_string());
        }

        if self.metrics.len() > 20 {
            return Err("Maximum 20 metrics allowed per query".to_string());
        }

        if self.group_by.len() > 5 {
            return Err("Maximum 5 group by dimensions allowed".to_string());
        }

        // Validate time range
        if let (Some(start), Some(end)) = (self.start_time, self.end_time) {
            if start >= end {
                return Err("Start time must be before end time".to_string());
            }

            let duration = end - start;
            if duration.num_days() > 90 {
                return Err("Maximum time range is 90 days".to_string());
            }
        }

        Ok(())
    }
}

impl CustomMetricsQueryRequest {
    /// Validates the request
    pub fn validate(&self) -> Result<(), String> {
        if self.metrics.is_empty() {
            return Err("At least one metric must be specified".to_string());
        }

        if self.metrics.len() > 20 {
            return Err("Maximum 20 metrics allowed per query".to_string());
        }

        if self.group_by.len() > 5 {
            return Err("Maximum 5 group by dimensions allowed".to_string());
        }

        if self.having.len() > 10 {
            return Err("Maximum 10 HAVING conditions allowed".to_string());
        }

        if self.limit < 1 || self.limit > 10000 {
            return Err("Limit must be between 1 and 10000".to_string());
        }

        // Validate time range
        if self.start_time >= self.end_time {
            return Err("Start time must be before end time".to_string());
        }

        let duration = self.end_time - self.start_time;
        if duration.num_days() > 90 {
            return Err("Maximum time range is 90 days".to_string());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metric_type_to_column_name() {
        assert_eq!(MetricType::RequestCount.to_column_name(), "request_count");
        assert_eq!(MetricType::Duration.to_column_name(), "avg_duration_ms");
        assert_eq!(MetricType::TotalCost.to_column_name(), "total_cost_usd");
    }

    #[test]
    fn test_aggregation_function_requires_raw_data() {
        assert!(!AggregationFunction::Avg.requires_raw_data());
        assert!(!AggregationFunction::Sum.requires_raw_data());
        assert!(AggregationFunction::P95.requires_raw_data());
        assert!(AggregationFunction::P99.requires_raw_data());
    }

    #[test]
    fn test_time_interval_to_aggregate_table() {
        assert_eq!(
            TimeInterval::OneMinute.to_aggregate_table(),
            "llm_metrics_1min"
        );
        assert_eq!(
            TimeInterval::OneHour.to_aggregate_table(),
            "llm_metrics_1hour"
        );
        assert_eq!(
            TimeInterval::OneDay.to_aggregate_table(),
            "llm_metrics_1day"
        );
    }

    #[test]
    fn test_dimension_available_in_aggregates() {
        assert!(DimensionName::Provider.available_in_aggregates());
        assert!(DimensionName::Model.available_in_aggregates());
        assert!(!DimensionName::UserId.available_in_aggregates());
        assert!(!DimensionName::SessionId.available_in_aggregates());
    }

    #[test]
    fn test_metrics_query_validation() {
        let mut req = MetricsQueryRequest {
            metrics: vec![MetricType::RequestCount],
            interval: TimeInterval::OneHour,
            start_time: None,
            end_time: None,
            provider: None,
            model: None,
            environment: None,
            user_id: None,
            group_by: vec![],
            aggregation: None,
            include_percentiles: false,
        };

        assert!(req.validate().is_ok());

        // Test empty metrics
        req.metrics = vec![];
        assert!(req.validate().is_err());

        // Test too many metrics
        req.metrics = vec![MetricType::RequestCount; 21];
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_custom_metrics_query_validation() {
        let start = Utc::now() - chrono::Duration::days(1);
        let end = Utc::now();

        let req = CustomMetricsQueryRequest {
            metrics: vec![MetricAggregation {
                metric: MetricType::RequestCount,
                aggregation: AggregationFunction::Sum,
                alias: None,
            }],
            interval: TimeInterval::OneHour,
            start_time: start,
            end_time: end,
            group_by: vec![],
            filters: vec![],
            having: vec![],
            sort_by: None,
            limit: 100,
        };

        assert!(req.validate().is_ok());
    }
}
