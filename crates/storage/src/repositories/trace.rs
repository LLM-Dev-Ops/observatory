//! Trace repository for querying trace data.

use crate::error::{StorageError, StorageResult};
use crate::models::{Trace, TraceEvent, TraceSpan};
use crate::pool::StoragePool;
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Repository for querying trace data.
#[derive(Clone)]
pub struct TraceRepository {
    pool: StoragePool,
}

impl TraceRepository {
    /// Create a new trace repository.
    pub fn new(pool: StoragePool) -> Self {
        Self { pool }
    }

    /// Get a trace by its ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The unique trace identifier
    ///
    /// # Errors
    ///
    /// Returns `StorageError::NotFound` if the trace doesn't exist.
    pub async fn get_by_id(&self, id: Uuid) -> StorageResult<Trace> {
        sqlx::query_as::<_, Trace>("SELECT * FROM traces WHERE id = $1")
            .bind(id)
            .fetch_one(self.pool.postgres())
            .await
            .map_err(StorageError::from)
    }

    /// Get a trace by its trace ID (hex format).
    pub async fn get_by_trace_id(&self, trace_id: &str) -> StorageResult<Trace> {
        sqlx::query_as::<_, Trace>("SELECT * FROM traces WHERE trace_id = $1 LIMIT 1")
            .bind(trace_id)
            .fetch_one(self.pool.postgres())
            .await
            .map_err(StorageError::from)
    }

    /// Get a trace with all its spans.
    ///
    /// Returns a trace and all associated spans ordered by start time.
    pub async fn get_trace_by_id(&self, trace_id: &str) -> StorageResult<(Trace, Vec<TraceSpan>)> {
        // Get the trace
        let trace = self.get_by_trace_id(trace_id).await?;

        // Get all spans for this trace
        let spans = self.get_spans(trace.id).await?;

        Ok((trace, spans))
    }

    /// List traces with optional filters.
    pub async fn list(&self, filters: TraceFilters) -> StorageResult<Vec<Trace>> {
        let mut query = String::from("SELECT * FROM traces WHERE 1=1");
        let mut bind_index = 1;

        // Build dynamic query based on filters
        if filters.service_name.is_some() {
            query.push_str(&format!(" AND service_name = ${}", bind_index));
            bind_index += 1;
        }

        if filters.status.is_some() {
            query.push_str(&format!(" AND status = ${}", bind_index));
            bind_index += 1;
        }

        if filters.start_time.is_some() {
            query.push_str(&format!(" AND start_time >= ${}", bind_index));
            bind_index += 1;
        }

        if filters.end_time.is_some() {
            query.push_str(&format!(" AND start_time <= ${}", bind_index));
            bind_index += 1;
        }

        if filters.min_duration_us.is_some() {
            query.push_str(&format!(" AND duration_us >= ${}", bind_index));
            bind_index += 1;
        }

        if filters.max_duration_us.is_some() {
            query.push_str(&format!(" AND duration_us <= ${}", bind_index));
            bind_index += 1;
        }

        query.push_str(" ORDER BY start_time DESC");

        if let Some(limit) = filters.limit {
            query.push_str(&format!(" LIMIT ${}", bind_index));
            bind_index += 1;
        }

        if let Some(offset) = filters.offset {
            query.push_str(&format!(" OFFSET ${}", bind_index));
        }

        // Build and execute query
        let mut q = sqlx::query_as::<_, Trace>(&query);

        if let Some(service_name) = &filters.service_name {
            q = q.bind(service_name);
        }
        if let Some(status) = &filters.status {
            q = q.bind(status);
        }
        if let Some(start_time) = filters.start_time {
            q = q.bind(start_time);
        }
        if let Some(end_time) = filters.end_time {
            q = q.bind(end_time);
        }
        if let Some(min_duration) = filters.min_duration_us {
            q = q.bind(min_duration);
        }
        if let Some(max_duration) = filters.max_duration_us {
            q = q.bind(max_duration);
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

    /// Get traces for a time range with pagination.
    pub async fn get_traces(
        &self,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        limit: i64,
        filters: TraceFilters,
    ) -> StorageResult<Vec<Trace>> {
        let mut filters = filters;
        filters.start_time = Some(start_time);
        filters.end_time = Some(end_time);
        filters.limit = Some(limit);

        self.list(filters).await
    }

    /// Get all spans for a trace.
    pub async fn get_spans(&self, trace_id: Uuid) -> StorageResult<Vec<TraceSpan>> {
        sqlx::query_as::<_, TraceSpan>(
            "SELECT * FROM trace_spans WHERE trace_id = $1 ORDER BY start_time ASC",
        )
        .bind(trace_id)
        .fetch_all(self.pool.postgres())
        .await
        .map_err(StorageError::from)
    }

    /// Get a specific span by ID.
    pub async fn get_span_by_id(&self, span_id: Uuid) -> StorageResult<TraceSpan> {
        sqlx::query_as::<_, TraceSpan>("SELECT * FROM trace_spans WHERE id = $1")
            .bind(span_id)
            .fetch_one(self.pool.postgres())
            .await
            .map_err(StorageError::from)
    }

    /// Get all events for a span.
    pub async fn get_events(&self, span_id: Uuid) -> StorageResult<Vec<TraceEvent>> {
        sqlx::query_as::<_, TraceEvent>(
            "SELECT * FROM trace_events WHERE span_id = $1 ORDER BY timestamp ASC",
        )
        .bind(span_id)
        .fetch_all(self.pool.postgres())
        .await
        .map_err(StorageError::from)
    }

    /// Search traces by service name and time range.
    pub async fn search_by_service(
        &self,
        service_name: &str,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> StorageResult<Vec<Trace>> {
        sqlx::query_as::<_, Trace>(
            r#"
            SELECT * FROM traces
            WHERE service_name = $1
              AND start_time >= $2
              AND start_time <= $3
            ORDER BY start_time DESC
            LIMIT 100
            "#,
        )
        .bind(service_name)
        .bind(start_time)
        .bind(end_time)
        .fetch_all(self.pool.postgres())
        .await
        .map_err(StorageError::from)
    }

    /// Search traces with errors.
    pub async fn search_errors(&self, filters: TraceFilters) -> StorageResult<Vec<Trace>> {
        let mut filters = filters;
        filters.status = Some("error".to_string());
        self.list(filters).await
    }

    /// Search traces by criteria.
    ///
    /// Advanced search supporting multiple filters:
    /// - Service name
    /// - Status (ok, error, unset)
    /// - Time range
    /// - Duration range
    pub async fn search_traces(&self, filters: TraceFilters) -> StorageResult<Vec<Trace>> {
        self.list(filters).await
    }

    /// Get trace statistics for a time range.
    pub async fn get_trace_statistics(&self, trace_id: &str) -> StorageResult<TraceStats> {
        let trace = self.get_by_trace_id(trace_id).await?;
        let spans = self.get_spans(trace.id).await?;

        let total_spans = spans.len() as i64;
        let error_count = spans.iter().filter(|s| s.is_error()).count() as i64;

        let durations: Vec<i64> = spans.iter().filter_map(|s| s.duration_us).collect();

        let avg_duration_us = if !durations.is_empty() {
            Some(durations.iter().sum::<i64>() as f64 / durations.len() as f64)
        } else {
            None
        };

        let min_duration_us = durations.iter().min().copied();
        let max_duration_us = durations.iter().max().copied();

        Ok(TraceStats {
            total_traces: 1,
            total_spans,
            error_count,
            avg_duration_us,
            min_duration_us,
            max_duration_us,
        })
    }

    /// Get trace statistics for a time range.
    pub async fn get_stats(
        &self,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> StorageResult<TraceStats> {
        let row = sqlx::query!(
            r#"
            SELECT
                COUNT(*) as "total_traces!",
                COALESCE(SUM(span_count), 0) as "total_spans!",
                COUNT(*) FILTER (WHERE status = 'error') as "error_count!",
                AVG(duration_us) as avg_duration_us,
                MIN(duration_us) as min_duration_us,
                MAX(duration_us) as max_duration_us
            FROM traces
            WHERE start_time >= $1 AND start_time <= $2
            "#,
            start_time,
            end_time
        )
        .fetch_one(self.pool.postgres())
        .await
        .map_err(StorageError::from)?;

        Ok(TraceStats {
            total_traces: row.total_traces,
            total_spans: row.total_spans,
            error_count: row.error_count,
            avg_duration_us: row.avg_duration_us,
            min_duration_us: row.min_duration_us,
            max_duration_us: row.max_duration_us,
        })
    }

    /// Delete old traces (for data retention).
    pub async fn delete_before(&self, before: DateTime<Utc>) -> StorageResult<u64> {
        let result = sqlx::query!("DELETE FROM traces WHERE start_time < $1", before)
            .execute(self.pool.postgres())
            .await
            .map_err(StorageError::from)?;

        Ok(result.rows_affected())
    }
}

/// Filters for querying traces.
#[derive(Debug, Default, Clone)]
pub struct TraceFilters {
    /// Filter by service name
    pub service_name: Option<String>,

    /// Filter by status
    pub status: Option<String>,

    /// Start time range
    pub start_time: Option<DateTime<Utc>>,

    /// End time range
    pub end_time: Option<DateTime<Utc>>,

    /// Minimum duration in microseconds
    pub min_duration_us: Option<i64>,

    /// Maximum duration in microseconds
    pub max_duration_us: Option<i64>,

    /// Limit number of results
    pub limit: Option<i64>,

    /// Offset for pagination
    pub offset: Option<i64>,
}

/// Statistics about traces.
#[derive(Debug, Clone)]
pub struct TraceStats {
    /// Total number of traces
    pub total_traces: i64,

    /// Total number of spans
    pub total_spans: i64,

    /// Number of traces with errors
    pub error_count: i64,

    /// Average trace duration in microseconds
    pub avg_duration_us: Option<f64>,

    /// Minimum trace duration in microseconds
    pub min_duration_us: Option<i64>,

    /// Maximum trace duration in microseconds
    pub max_duration_us: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO: Add integration tests with test database

    #[test]
    fn test_trace_filters_default() {
        let filters = TraceFilters::default();
        assert!(filters.service_name.is_none());
        assert!(filters.limit.is_none());
    }
}
