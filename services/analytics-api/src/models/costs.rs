//! # Cost Analysis API Data Models (Phase 4)
//!
//! This module contains all data structures for the Phase 4 Cost Analysis API.
//!
//! ## Endpoints
//! - `GET /api/v1/costs/summary` - Comprehensive cost summary with trends
//! - `GET /api/v1/costs/attribution` - Cost attribution by user, team, tag
//! - `GET /api/v1/costs/forecast` - Cost forecasting with linear regression
//! - `GET /api/v1/costs/budgets` - Budget management and alerts
//! - `GET /api/v1/costs/budgets/{id}/history` - Budget alert history
//!
//! ## Features
//! - Detailed cost breakdowns (by provider, model, user, team, tag)
//! - Trend analysis (daily, weekly, monthly)
//! - Top expensive traces identification
//! - Linear regression-based forecasting
//! - Budget threshold monitoring
//! - Alert history tracking
//!
//! ## Security
//! - All endpoints require authentication
//! - Organization-level data isolation
//! - SQL injection prevention via parameterized queries

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Enums and Types
// ============================================================================

/// Time period for cost aggregation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CostPeriod {
    Daily,
    Weekly,
    Monthly,
    Quarterly,
    Yearly,
}

impl CostPeriod {
    pub fn to_pg_interval(&self) -> &'static str {
        match self {
            CostPeriod::Daily => "1 day",
            CostPeriod::Weekly => "7 days",
            CostPeriod::Monthly => "30 days",
            CostPeriod::Quarterly => "90 days",
            CostPeriod::Yearly => "365 days",
        }
    }
}

/// Attribution dimension for cost grouping
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum AttributionDimension {
    User,
    Team,
    Tag,
    Provider,
    Model,
    Environment,
}

impl AttributionDimension {
    pub fn to_column_name(&self) -> &'static str {
        match self {
            AttributionDimension::User => "user_id",
            AttributionDimension::Team => "team_id",
            AttributionDimension::Tag => "tags",
            AttributionDimension::Provider => "provider",
            AttributionDimension::Model => "model",
            AttributionDimension::Environment => "environment",
        }
    }
}

/// Forecast period for predictions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ForecastPeriod {
    /// Forecast next 7 days
    NextWeek,
    /// Forecast next 30 days
    NextMonth,
    /// Forecast next 90 days
    NextQuarter,
    /// Custom number of days
    Custom(i32),
}

impl ForecastPeriod {
    pub fn to_days(&self) -> i32 {
        match self {
            ForecastPeriod::NextWeek => 7,
            ForecastPeriod::NextMonth => 30,
            ForecastPeriod::NextQuarter => 90,
            ForecastPeriod::Custom(days) => *days,
        }
    }
}

/// Budget alert threshold type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ThresholdType {
    /// Absolute cost threshold in USD
    Absolute,
    /// Percentage of budget
    Percentage,
}

/// Budget alert severity
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

// ============================================================================
// Request Models
// ============================================================================

/// Request for GET /api/v1/costs/summary
#[derive(Debug, Deserialize, Clone)]
pub struct CostSummaryRequest {
    /// Start time (default: 30 days ago)
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

    /// Include trend analysis
    #[serde(default = "default_true")]
    pub include_trends: bool,

    /// Include top expensive traces
    #[serde(default = "default_true")]
    pub include_top_traces: bool,

    /// Number of top traces to return (max 100)
    #[serde(default = "default_top_limit")]
    pub top_limit: i32,
}

fn default_true() -> bool {
    true
}

fn default_top_limit() -> i32 {
    10
}

impl CostSummaryRequest {
    pub fn validate(&self) -> Result<(), String> {
        if let (Some(start), Some(end)) = (self.start_time, self.end_time) {
            if start >= end {
                return Err("Start time must be before end time".to_string());
            }

            let duration = end - start;
            if duration.num_days() > 365 {
                return Err("Maximum time range is 365 days".to_string());
            }
        }

        if self.top_limit < 1 || self.top_limit > 100 {
            return Err("top_limit must be between 1 and 100".to_string());
        }

        Ok(())
    }
}

/// Request for GET /api/v1/costs/attribution
#[derive(Debug, Deserialize, Clone)]
pub struct CostAttributionRequest {
    /// Start time
    pub start_time: DateTime<Utc>,

    /// End time
    pub end_time: DateTime<Utc>,

    /// Attribution dimension (user, team, tag, provider, model, environment)
    pub dimension: AttributionDimension,

    /// Filter by provider
    pub provider: Option<String>,

    /// Filter by model
    pub model: Option<String>,

    /// Filter by environment
    pub environment: Option<String>,

    /// Limit number of results (max 1000)
    #[serde(default = "default_attribution_limit")]
    pub limit: i32,

    /// Minimum cost threshold (filter out items below this cost)
    pub min_cost: Option<f64>,
}

fn default_attribution_limit() -> i32 {
    100
}

impl CostAttributionRequest {
    pub fn validate(&self) -> Result<(), String> {
        if self.start_time >= self.end_time {
            return Err("Start time must be before end time".to_string());
        }

        let duration = self.end_time - self.start_time;
        if duration.num_days() > 365 {
            return Err("Maximum time range is 365 days".to_string());
        }

        if self.limit < 1 || self.limit > 1000 {
            return Err("Limit must be between 1 and 1000".to_string());
        }

        if let Some(min_cost) = self.min_cost {
            if min_cost < 0.0 {
                return Err("min_cost cannot be negative".to_string());
            }
        }

        Ok(())
    }
}

/// Request for GET /api/v1/costs/forecast
#[derive(Debug, Deserialize, Clone)]
pub struct CostForecastRequest {
    /// Historical data start time (default: 30 days ago)
    pub historical_start: Option<DateTime<Utc>>,

    /// Historical data end time (default: now)
    pub historical_end: Option<DateTime<Utc>>,

    /// Forecast period (next_week, next_month, next_quarter, or custom)
    #[serde(default = "default_forecast_period")]
    pub forecast_period: ForecastPeriod,

    /// Filter by provider
    pub provider: Option<String>,

    /// Filter by model
    pub model: Option<String>,

    /// Filter by environment
    pub environment: Option<String>,

    /// Include confidence intervals
    #[serde(default = "default_true")]
    pub include_confidence_intervals: bool,
}

fn default_forecast_period() -> ForecastPeriod {
    ForecastPeriod::NextMonth
}

impl CostForecastRequest {
    pub fn validate(&self) -> Result<(), String> {
        if let (Some(start), Some(end)) = (self.historical_start, self.historical_end) {
            if start >= end {
                return Err("Historical start must be before end".to_string());
            }

            let duration = end - start;
            if duration.num_days() < 7 {
                return Err(
                    "Historical data must span at least 7 days for accurate forecasting"
                        .to_string(),
                );
            }

            if duration.num_days() > 365 {
                return Err("Maximum historical range is 365 days".to_string());
            }
        }

        match &self.forecast_period {
            ForecastPeriod::Custom(days) => {
                if *days < 1 || *days > 365 {
                    return Err("Custom forecast period must be between 1 and 365 days".to_string());
                }
            }
            _ => {}
        }

        Ok(())
    }
}

// ============================================================================
// Response Models
// ============================================================================

/// Response for GET /api/v1/costs/summary
#[derive(Debug, Serialize)]
pub struct CostSummaryResponse {
    /// Summary metadata
    pub metadata: CostSummaryMetadata,

    /// Overall cost statistics
    pub overview: CostOverview,

    /// Cost breakdown by provider
    pub by_provider: Vec<CostBreakdownItem>,

    /// Cost breakdown by model
    pub by_model: Vec<CostBreakdownItem>,

    /// Cost breakdown by environment
    pub by_environment: Vec<CostBreakdownItem>,

    /// Trend analysis (if requested)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trends: Option<CostTrends>,

    /// Top expensive traces (if requested)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_traces: Option<Vec<ExpensiveTrace>>,
}

#[derive(Debug, Serialize)]
pub struct CostSummaryMetadata {
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub period_days: i64,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct CostOverview {
    /// Total cost in USD
    pub total_cost: f64,

    /// Prompt cost in USD
    pub prompt_cost: f64,

    /// Completion cost in USD
    pub completion_cost: f64,

    /// Total number of requests
    pub total_requests: i64,

    /// Total tokens processed
    pub total_tokens: i64,

    /// Average cost per request
    pub avg_cost_per_request: f64,

    /// Average cost per 1000 tokens
    pub avg_cost_per_1k_tokens: f64,

    /// Day-over-day change percentage
    pub day_over_day_change: Option<f64>,

    /// Week-over-week change percentage
    pub week_over_week_change: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct CostBreakdownItem {
    /// Dimension value (e.g., "openai", "gpt-4")
    pub name: String,

    /// Total cost for this item
    pub cost: f64,

    /// Number of requests
    pub requests: i64,

    /// Percentage of total cost
    pub percentage: f64,

    /// Average cost per request for this item
    pub avg_cost_per_request: f64,
}

#[derive(Debug, Serialize)]
pub struct CostTrends {
    /// Daily cost trend
    pub daily: Vec<CostDataPoint>,

    /// Weekly cost trend
    pub weekly: Vec<CostDataPoint>,

    /// Growth rate (% per day)
    pub growth_rate_daily: f64,

    /// Growth rate (% per week)
    pub growth_rate_weekly: f64,
}

#[derive(Debug, Serialize)]
pub struct CostDataPoint {
    pub date: DateTime<Utc>,
    pub cost: f64,
    pub requests: i64,
}

#[derive(Debug, Serialize)]
pub struct ExpensiveTrace {
    pub trace_id: String,
    pub timestamp: DateTime<Utc>,
    pub provider: String,
    pub model: String,
    pub cost: f64,
    pub tokens: i64,
    pub duration_ms: i32,
    pub user_id: Option<String>,
}

/// Response for GET /api/v1/costs/attribution
#[derive(Debug, Serialize)]
pub struct CostAttributionResponse {
    /// Attribution metadata
    pub metadata: AttributionMetadata,

    /// Attribution items
    pub items: Vec<AttributionItem>,

    /// Summary statistics
    pub summary: AttributionSummary,
}

#[derive(Debug, Serialize)]
pub struct AttributionMetadata {
    pub dimension: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub total_items: usize,
}

#[derive(Debug, Serialize)]
pub struct AttributionItem {
    /// Dimension value (user_id, team_id, etc.)
    pub dimension_value: String,

    /// Total cost attributed to this item
    pub total_cost: f64,

    /// Prompt cost
    pub prompt_cost: f64,

    /// Completion cost
    pub completion_cost: f64,

    /// Number of requests
    pub request_count: i64,

    /// Total tokens
    pub total_tokens: i64,

    /// Percentage of total cost
    pub cost_percentage: f64,

    /// Average cost per request
    pub avg_cost_per_request: f64,

    /// Cost breakdown by provider
    pub by_provider: HashMap<String, f64>,

    /// Cost breakdown by model
    pub by_model: HashMap<String, f64>,
}

#[derive(Debug, Serialize)]
pub struct AttributionSummary {
    pub total_cost: f64,
    pub total_requests: i64,
    pub unique_items: i64,
    pub avg_cost_per_item: f64,
}

/// Response for GET /api/v1/costs/forecast
#[derive(Debug, Serialize)]
pub struct CostForecastResponse {
    /// Forecast metadata
    pub metadata: ForecastMetadata,

    /// Historical data used for forecasting
    pub historical: Vec<CostDataPoint>,

    /// Forecasted data points
    pub forecast: Vec<ForecastDataPoint>,

    /// Forecast summary
    pub summary: ForecastSummary,
}

#[derive(Debug, Serialize)]
pub struct ForecastMetadata {
    pub historical_start: DateTime<Utc>,
    pub historical_end: DateTime<Utc>,
    pub forecast_start: DateTime<Utc>,
    pub forecast_end: DateTime<Utc>,
    pub forecast_days: i32,
    pub model_type: String, // "linear_regression"
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ForecastDataPoint {
    pub date: DateTime<Utc>,
    pub forecasted_cost: f64,
    pub lower_bound: Option<f64>,
    pub upper_bound: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct ForecastSummary {
    /// Total forecasted cost for the period
    pub total_forecasted_cost: f64,

    /// Average daily forecasted cost
    pub avg_daily_cost: f64,

    /// Projected monthly cost
    pub projected_monthly_cost: f64,

    /// R-squared value (model accuracy)
    pub r_squared: f64,

    /// Mean absolute percentage error
    pub mape: Option<f64>,
}

/// Budget configuration
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Budget {
    pub id: String,
    pub org_id: String,
    pub name: String,
    pub amount: f64,
    pub period: CostPeriod,
    pub threshold_type: ThresholdType,
    pub warning_threshold: f64,
    pub critical_threshold: f64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_active: bool,
}

/// Budget alert
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BudgetAlert {
    pub id: String,
    pub budget_id: String,
    pub org_id: String,
    pub severity: AlertSeverity,
    pub current_cost: f64,
    pub threshold_value: f64,
    pub percentage_used: f64,
    pub message: String,
    pub triggered_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

// ============================================================================
// Internal Database Row Types
// ============================================================================

/// Row for cost overview query
#[derive(Debug, sqlx::FromRow)]
pub struct CostOverviewRow {
    pub total_cost_usd: Option<f64>,
    pub prompt_cost_usd: Option<f64>,
    pub completion_cost_usd: Option<f64>,
    pub total_requests: Option<i64>,
    pub total_tokens: Option<i64>,
}

/// Row for cost breakdown query
#[derive(Debug, sqlx::FromRow)]
pub struct CostBreakdownRow {
    pub name: String,
    pub cost: Option<f64>,
    pub requests: Option<i64>,
}

/// Row for cost trend query
#[derive(Debug, sqlx::FromRow)]
pub struct CostTrendRow {
    pub date: DateTime<Utc>,
    pub cost: Option<f64>,
    pub requests: Option<i64>,
}

/// Row for expensive trace query
#[derive(Debug, sqlx::FromRow)]
pub struct ExpensiveTraceRow {
    pub trace_id: String,
    pub ts: DateTime<Utc>,
    pub provider: String,
    pub model: String,
    pub total_cost_usd: f64,
    pub total_tokens: i64,
    pub duration_ms: i32,
    pub user_id: Option<String>,
}

/// Row for attribution query
#[derive(Debug, sqlx::FromRow)]
pub struct AttributionRow {
    pub dimension_value: String,
    pub total_cost: Option<f64>,
    pub prompt_cost: Option<f64>,
    pub completion_cost: Option<f64>,
    pub request_count: Option<i64>,
    pub total_tokens: Option<i64>,
}

/// Row for provider/model breakdown within attribution
#[derive(Debug, sqlx::FromRow)]
pub struct AttributionBreakdownRow {
    pub dimension_value: String,
    pub breakdown_key: String,
    pub breakdown_cost: Option<f64>,
}

/// Row for forecast historical data
#[derive(Debug, sqlx::FromRow)]
pub struct ForecastHistoricalRow {
    pub date: DateTime<Utc>,
    pub cost: Option<f64>,
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Calculate linear regression for cost forecasting
pub fn calculate_linear_regression(data: &[(f64, f64)]) -> (f64, f64, f64) {
    if data.is_empty() {
        return (0.0, 0.0, 0.0);
    }

    let n = data.len() as f64;

    // Calculate means
    let mean_x: f64 = data.iter().map(|(x, _)| x).sum::<f64>() / n;
    let mean_y: f64 = data.iter().map(|(_, y)| y).sum::<f64>() / n;

    // Calculate slope (b1) and intercept (b0)
    let numerator: f64 = data.iter().map(|(x, y)| (x - mean_x) * (y - mean_y)).sum();
    let denominator: f64 = data.iter().map(|(x, _)| (x - mean_x).powi(2)).sum();

    let slope = if denominator != 0.0 {
        numerator / denominator
    } else {
        0.0
    };

    let intercept = mean_y - slope * mean_x;

    // Calculate R-squared
    let ss_tot: f64 = data.iter().map(|(_, y)| (y - mean_y).powi(2)).sum();
    let ss_res: f64 = data
        .iter()
        .map(|(x, y)| {
            let predicted = intercept + slope * x;
            (y - predicted).powi(2)
        })
        .sum();

    let r_squared = if ss_tot != 0.0 {
        1.0 - (ss_res / ss_tot)
    } else {
        0.0
    };

    (slope, intercept, r_squared)
}

/// Calculate Mean Absolute Percentage Error (MAPE)
pub fn calculate_mape(actual: &[f64], predicted: &[f64]) -> Option<f64> {
    if actual.len() != predicted.len() || actual.is_empty() {
        return None;
    }

    let sum: f64 = actual
        .iter()
        .zip(predicted.iter())
        .filter(|(a, _)| **a != 0.0)
        .map(|(a, p)| ((a - p) / a).abs())
        .sum();

    let count = actual.iter().filter(|a| **a != 0.0).count() as f64;

    if count > 0.0 {
        Some((sum / count) * 100.0)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cost_period_to_interval() {
        assert_eq!(CostPeriod::Daily.to_pg_interval(), "1 day");
        assert_eq!(CostPeriod::Weekly.to_pg_interval(), "7 days");
        assert_eq!(CostPeriod::Monthly.to_pg_interval(), "30 days");
    }

    #[test]
    fn test_attribution_dimension_to_column() {
        assert_eq!(AttributionDimension::User.to_column_name(), "user_id");
        assert_eq!(AttributionDimension::Provider.to_column_name(), "provider");
        assert_eq!(AttributionDimension::Model.to_column_name(), "model");
    }

    #[test]
    fn test_forecast_period_to_days() {
        assert_eq!(ForecastPeriod::NextWeek.to_days(), 7);
        assert_eq!(ForecastPeriod::NextMonth.to_days(), 30);
        assert_eq!(ForecastPeriod::NextQuarter.to_days(), 90);
        assert_eq!(ForecastPeriod::Custom(15).to_days(), 15);
    }

    #[test]
    fn test_linear_regression() {
        let data = vec![(1.0, 2.0), (2.0, 4.0), (3.0, 6.0), (4.0, 8.0)];
        let (slope, intercept, r_squared) = calculate_linear_regression(&data);

        assert!((slope - 2.0).abs() < 0.001);
        assert!(intercept.abs() < 0.001);
        assert!((r_squared - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_mape_calculation() {
        let actual = vec![100.0, 200.0, 300.0];
        let predicted = vec![110.0, 190.0, 310.0];

        let mape = calculate_mape(&actual, &predicted);
        assert!(mape.is_some());
        assert!(mape.unwrap() > 0.0 && mape.unwrap() < 10.0);
    }

    #[test]
    fn test_cost_summary_validation() {
        let req = CostSummaryRequest {
            start_time: None,
            end_time: None,
            provider: None,
            model: None,
            environment: None,
            user_id: None,
            include_trends: true,
            include_top_traces: true,
            top_limit: 10,
        };

        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_cost_attribution_validation() {
        let now = Utc::now();
        let req = CostAttributionRequest {
            start_time: now - chrono::Duration::days(30),
            end_time: now,
            dimension: AttributionDimension::User,
            provider: None,
            model: None,
            environment: None,
            limit: 100,
            min_cost: None,
        };

        assert!(req.validate().is_ok());
    }
}
