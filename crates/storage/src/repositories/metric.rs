//! Metric repository for querying metric data.

use crate::error::{StorageError, StorageResult};
use crate::models::{Metric, MetricDataPoint, MetricType};
use crate::pool::StoragePool;
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Repository for querying metric data.
#[derive(Clone)]
pub struct MetricRepository {
    pool: StoragePool,
}

impl MetricRepository {
    /// Create a new metric repository.
    pub fn new(pool: StoragePool) -> Self {
        Self { pool }
    }

    /// Get a metric by its ID.
    pub async fn get_by_id(&self, id: Uuid) -> StorageResult<Metric> {
        sqlx::query_as::<_, Metric>("SELECT * FROM metrics WHERE id = $1")
            .bind(id)
            .fetch_one(self.pool.postgres())
            .await
            .map_err(StorageError::from)
    }

    /// Get a metric by name and service.
    pub async fn get_by_name(&self, name: &str, service_name: &str) -> StorageResult<Metric> {
        sqlx::query_as::<_, Metric>(
            "SELECT * FROM metrics WHERE name = $1 AND service_name = $2 LIMIT 1",
        )
        .bind(name)
        .bind(service_name)
        .fetch_one(self.pool.postgres())
        .await
        .map_err(StorageError::from)
    }

    /// List all metrics with optional filters.
    pub async fn list(&self, filters: MetricFilters) -> StorageResult<Vec<Metric>> {
        let mut query = String::from("SELECT * FROM metrics WHERE 1=1");
        let mut bind_index = 1;

        if filters.service_name.is_some() {
            query.push_str(&format!(" AND service_name = ${}", bind_index));
            bind_index += 1;
        }

        if filters.metric_type.is_some() {
            query.push_str(&format!(" AND metric_type = ${}", bind_index));
            bind_index += 1;
        }

        if filters.name_pattern.is_some() {
            query.push_str(&format!(" AND name LIKE ${}", bind_index));
            bind_index += 1;
        }

        query.push_str(" ORDER BY name ASC");

        if let Some(limit) = filters.limit {
            query.push_str(&format!(" LIMIT ${}", bind_index));
            bind_index += 1;
        }

        if let Some(offset) = filters.offset {
            query.push_str(&format!(" OFFSET ${}", bind_index));
        }

        let mut q = sqlx::query_as::<_, Metric>(&query);

        if let Some(service_name) = &filters.service_name {
            q = q.bind(service_name);
        }
        if let Some(metric_type) = &filters.metric_type {
            q = q.bind(metric_type);
        }
        if let Some(name_pattern) = &filters.name_pattern {
            q = q.bind(format!("%{}%", name_pattern));
        }
        if let Some(limit) = filters.limit {
            q = q.bind(limit);
        }
        if let Some(offset) = filters.offset {
            q = q.bind(offset);
        }

        q.fetch_all(self.pool.postgres())
            .await
            .map_err(StorageError::from)
    }

    /// Get metric time series by name.
    pub async fn get_metrics(
        &self,
        name: &str,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> StorageResult<Vec<MetricDataPoint>> {
        sqlx::query_as::<_, MetricDataPoint>(
            r#"
            SELECT mdp.*
            FROM metric_data_points mdp
            JOIN metrics m ON mdp.metric_id = m.id
            WHERE m.name = $1
              AND mdp.timestamp >= $2
              AND mdp.timestamp <= $3
            ORDER BY mdp.timestamp ASC
            "#,
        )
        .bind(name)
        .bind(start_time)
        .bind(end_time)
        .fetch_all(self.pool.postgres())
        .await
        .map_err(StorageError::from)
    }

    /// Get data points for a metric.
    pub async fn get_data_points(
        &self,
        metric_id: Uuid,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> StorageResult<Vec<MetricDataPoint>> {
        sqlx::query_as::<_, MetricDataPoint>(
            r#"
            SELECT * FROM metric_data_points
            WHERE metric_id = $1
              AND timestamp >= $2
              AND timestamp <= $3
            ORDER BY timestamp ASC
            "#,
        )
        .bind(metric_id)
        .bind(start_time)
        .bind(end_time)
        .fetch_all(self.pool.postgres())
        .await
        .map_err(StorageError::from)
    }

    /// Get latest data point for a metric.
    pub async fn get_latest_data_point(&self, metric_id: Uuid) -> StorageResult<MetricDataPoint> {
        sqlx::query_as::<_, MetricDataPoint>(
            "SELECT * FROM metric_data_points WHERE metric_id = $1 ORDER BY timestamp DESC LIMIT 1",
        )
        .bind(metric_id)
        .fetch_one(self.pool.postgres())
        .await
        .map_err(StorageError::from)
    }

    /// Query time series data with aggregation.
    pub async fn query_time_series(
        &self,
        query: TimeSeriesQuery,
    ) -> StorageResult<Vec<TimeSeriesPoint>> {
        let agg_func = match query.aggregation {
            Aggregation::Avg => "AVG(value)",
            Aggregation::Sum => "SUM(value)",
            Aggregation::Min => "MIN(value)",
            Aggregation::Max => "MAX(value)",
            Aggregation::Count => "COUNT(*)",
        };

        let sql = format!(
            r#"
            SELECT
                time_bucket($1, timestamp) AS bucket,
                {} AS value,
                COUNT(*) AS count
            FROM metric_data_points
            WHERE metric_id = $2
              AND timestamp >= $3
              AND timestamp <= $4
            GROUP BY bucket
            ORDER BY bucket ASC
            "#,
            agg_func
        );

        let bucket_interval = format!("{} seconds", query.bucket_size_secs);

        let rows = sqlx::query(&sql)
            .bind(bucket_interval)
            .bind(query.metric_id)
            .bind(query.start_time)
            .bind(query.end_time)
            .fetch_all(self.pool.postgres())
            .await
            .map_err(StorageError::from)?;

        let mut points = Vec::new();
        for row in rows {
            points.push(TimeSeriesPoint {
                timestamp: row.try_get("bucket")?,
                value: row.try_get::<f64, _>("value").unwrap_or(0.0),
                count: row.try_get::<i64, _>("count").unwrap_or(0),
            });
        }

        Ok(points)
    }

    /// Get metric aggregates with time bucketing.
    pub async fn get_metric_aggregates(
        &self,
        name: &str,
        bucket_seconds: i64,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> StorageResult<Vec<TimeSeriesPoint>> {
        // First get the metric
        let metric = sqlx::query_as::<_, Metric>("SELECT * FROM metrics WHERE name = $1 LIMIT 1")
            .bind(name)
            .fetch_one(self.pool.postgres())
            .await
            .map_err(StorageError::from)?;

        // Query with aggregation
        let query = TimeSeriesQuery {
            metric_id: metric.id,
            start_time,
            end_time,
            aggregation: Aggregation::Avg,
            bucket_size_secs: bucket_seconds,
        };

        self.query_time_series(query).await
    }

    /// Get cost summary for a time range.
    pub async fn get_cost_summary(
        &self,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> StorageResult<Vec<CostSummary>> {
        let rows = sqlx::query(
            r#"
            SELECT
                m.service_name,
                m.name as metric_name,
                COUNT(*) as data_point_count,
                SUM(mdp.value) as total_value,
                AVG(mdp.value) as avg_value
            FROM metric_data_points mdp
            JOIN metrics m ON mdp.metric_id = m.id
            WHERE mdp.timestamp >= $1
              AND mdp.timestamp <= $2
              AND m.name LIKE 'cost%'
            GROUP BY m.service_name, m.name
            ORDER BY total_value DESC
            "#,
        )
        .bind(start_time)
        .bind(end_time)
        .fetch_all(self.pool.postgres())
        .await
        .map_err(StorageError::from)?;

        let mut summaries = Vec::new();
        for row in rows {
            summaries.push(CostSummary {
                service_name: row.try_get("service_name")?,
                metric_name: row.try_get("metric_name")?,
                data_point_count: row.try_get("data_point_count")?,
                total_value: row.try_get("total_value")?,
                avg_value: row.try_get("avg_value")?,
            });
        }

        Ok(summaries)
    }

    /// Get latency percentiles for a service.
    pub async fn get_latency_percentiles(
        &self,
        service_name: &str,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> StorageResult<LatencyPercentiles> {
        let row = sqlx::query(
            r#"
            SELECT
                PERCENTILE_CONT(0.50) WITHIN GROUP (ORDER BY mdp.value) AS p50,
                PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY mdp.value) AS p95,
                PERCENTILE_CONT(0.99) WITHIN GROUP (ORDER BY mdp.value) AS p99,
                AVG(mdp.value) AS avg,
                MIN(mdp.value) AS min,
                MAX(mdp.value) AS max
            FROM metric_data_points mdp
            JOIN metrics m ON mdp.metric_id = m.id
            WHERE m.service_name = $1
              AND m.name LIKE '%latency%'
              AND mdp.timestamp >= $2
              AND mdp.timestamp <= $3
            "#,
        )
        .bind(service_name)
        .bind(start_time)
        .bind(end_time)
        .fetch_one(self.pool.postgres())
        .await
        .map_err(StorageError::from)?;

        Ok(LatencyPercentiles {
            p50: row.try_get("p50")?,
            p95: row.try_get("p95")?,
            p99: row.try_get("p99")?,
            avg: row.try_get("avg")?,
            min: row.try_get("min")?,
            max: row.try_get("max")?,
        })
    }

    /// Search metrics by name pattern.
    pub async fn search_by_name(&self, pattern: &str) -> StorageResult<Vec<Metric>> {
        sqlx::query_as::<_, Metric>("SELECT * FROM metrics WHERE name LIKE $1 ORDER BY name ASC")
            .bind(format!("%{}%", pattern))
            .fetch_all(self.pool.postgres())
            .await
            .map_err(StorageError::from)
    }

    /// Get metric statistics for a time range.
    pub async fn get_stats(
        &self,
        metric_id: Uuid,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> StorageResult<MetricStats> {
        let row = sqlx::query!(
            r#"
            SELECT
                COUNT(*) as "total_points!",
                AVG(value) as avg_value,
                MIN(value) as min_value,
                MAX(value) as max_value,
                SUM(value) as sum_value
            FROM metric_data_points
            WHERE metric_id = $1
              AND timestamp >= $2
              AND timestamp <= $3
            "#,
            metric_id,
            start_time,
            end_time
        )
        .fetch_one(self.pool.postgres())
        .await
        .map_err(StorageError::from)?;

        Ok(MetricStats {
            total_points: row.total_points,
            avg_value: row.avg_value,
            min_value: row.min_value,
            max_value: row.max_value,
            sum_value: row.sum_value,
        })
    }

    /// Delete old data points (for data retention).
    pub async fn delete_before(&self, before: DateTime<Utc>) -> StorageResult<u64> {
        let result = sqlx::query!(
            "DELETE FROM metric_data_points WHERE timestamp < $1",
            before
        )
        .execute(self.pool.postgres())
        .await
        .map_err(StorageError::from)?;

        Ok(result.rows_affected())
    }
}

/// Filters for querying metrics.
#[derive(Debug, Default, Clone)]
pub struct MetricFilters {
    /// Filter by service name
    pub service_name: Option<String>,

    /// Filter by metric type
    pub metric_type: Option<String>,

    /// Filter by name pattern
    pub name_pattern: Option<String>,

    /// Limit number of results
    pub limit: Option<i64>,

    /// Offset for pagination
    pub offset: Option<i64>,
}

/// Query parameters for time series data.
#[derive(Debug, Clone)]
pub struct TimeSeriesQuery {
    /// Metric ID
    pub metric_id: Uuid,

    /// Start time
    pub start_time: DateTime<Utc>,

    /// End time
    pub end_time: DateTime<Utc>,

    /// Aggregation function (avg, sum, min, max, count)
    pub aggregation: Aggregation,

    /// Bucket size in seconds
    pub bucket_size_secs: i64,
}

/// Aggregation function for time series queries.
#[derive(Debug, Clone, Copy)]
pub enum Aggregation {
    /// Average value
    Avg,
    /// Sum of values
    Sum,
    /// Minimum value
    Min,
    /// Maximum value
    Max,
    /// Count of data points
    Count,
}

/// A point in a time series.
#[derive(Debug, Clone)]
pub struct TimeSeriesPoint {
    /// Timestamp (bucket start)
    pub timestamp: DateTime<Utc>,

    /// Aggregated value
    pub value: f64,

    /// Number of data points in this bucket
    pub count: i64,
}

/// Statistics about a metric.
#[derive(Debug, Clone)]
pub struct MetricStats {
    /// Total number of data points
    pub total_points: i64,

    /// Average value
    pub avg_value: Option<f64>,

    /// Minimum value
    pub min_value: Option<f64>,

    /// Maximum value
    pub max_value: Option<f64>,

    /// Sum of all values
    pub sum_value: Option<f64>,
}

/// Cost summary for a time period.
#[derive(Debug, Clone)]
pub struct CostSummary {
    /// Service name
    pub service_name: String,

    /// Metric name
    pub metric_name: String,

    /// Number of data points
    pub data_point_count: i64,

    /// Total cost value
    pub total_value: Option<f64>,

    /// Average cost per data point
    pub avg_value: Option<f64>,
}

/// Latency percentile statistics.
#[derive(Debug, Clone)]
pub struct LatencyPercentiles {
    /// 50th percentile (median)
    pub p50: Option<f64>,

    /// 95th percentile
    pub p95: Option<f64>,

    /// 99th percentile
    pub p99: Option<f64>,

    /// Average latency
    pub avg: Option<f64>,

    /// Minimum latency
    pub min: Option<f64>,

    /// Maximum latency
    pub max: Option<f64>,
}

impl std::fmt::Display for Aggregation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Aggregation::Avg => write!(f, "avg"),
            Aggregation::Sum => write!(f, "sum"),
            Aggregation::Min => write!(f, "min"),
            Aggregation::Max => write!(f, "max"),
            Aggregation::Count => write!(f, "count"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO: Add integration tests with test database

    #[test]
    fn test_metric_filters_default() {
        let filters = MetricFilters::default();
        assert!(filters.service_name.is_none());
        assert!(filters.limit.is_none());
    }

    #[test]
    fn test_aggregation_display() {
        assert_eq!(Aggregation::Avg.to_string(), "avg");
        assert_eq!(Aggregation::Sum.to_string(), "sum");
    }
}
