//! Log repository for querying log data.

use crate::error::{StorageError, StorageResult};
use crate::models::{LogLevel, LogRecord};
use crate::pool::StoragePool;
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Repository for querying log data.
#[derive(Clone)]
pub struct LogRepository {
    pool: StoragePool,
}

impl LogRepository {
    /// Create a new log repository.
    pub fn new(pool: StoragePool) -> Self {
        Self { pool }
    }

    /// Get a log record by its ID.
    pub async fn get_by_id(&self, id: Uuid) -> StorageResult<LogRecord> {
        sqlx::query_as::<_, LogRecord>("SELECT * FROM log_records WHERE id = $1")
            .bind(id)
            .fetch_one(self.pool.postgres())
            .await
            .map_err(StorageError::from)
    }

    /// Get logs for a time range with filters.
    pub async fn get_logs(
        &self,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        filters: LogFilters,
    ) -> StorageResult<Vec<LogRecord>> {
        let mut filters = filters;
        filters.start_time = Some(start_time);
        filters.end_time = Some(end_time);
        self.list(filters).await
    }

    /// List log records with optional filters.
    pub async fn list(&self, filters: LogFilters) -> StorageResult<Vec<LogRecord>> {
        let mut query = String::from("SELECT * FROM log_records WHERE 1=1");
        let mut bind_index = 1;

        if filters.service_name.is_some() {
            query.push_str(&format!(" AND service_name = ${}", bind_index));
            bind_index += 1;
        }

        if filters.min_severity.is_some() {
            query.push_str(&format!(" AND severity_number >= ${}", bind_index));
            bind_index += 1;
        }

        if filters.trace_id.is_some() {
            query.push_str(&format!(" AND trace_id = ${}", bind_index));
            bind_index += 1;
        }

        if filters.span_id.is_some() {
            query.push_str(&format!(" AND span_id = ${}", bind_index));
            bind_index += 1;
        }

        if filters.start_time.is_some() {
            query.push_str(&format!(" AND timestamp >= ${}", bind_index));
            bind_index += 1;
        }

        if filters.end_time.is_some() {
            query.push_str(&format!(" AND timestamp <= ${}", bind_index));
            bind_index += 1;
        }

        if filters.search_query.is_some() {
            query.push_str(&format!(" AND body ILIKE ${}", bind_index));
            bind_index += 1;
        }

        // Sort order
        match filters.sort_order {
            SortOrder::Asc => query.push_str(" ORDER BY timestamp ASC"),
            SortOrder::Desc => query.push_str(" ORDER BY timestamp DESC"),
        }

        if let Some(limit) = filters.limit {
            query.push_str(&format!(" LIMIT ${}", bind_index));
            bind_index += 1;
        }

        if let Some(offset) = filters.offset {
            query.push_str(&format!(" OFFSET ${}", bind_index));
        }

        let mut q = sqlx::query_as::<_, LogRecord>(&query);

        if let Some(service_name) = &filters.service_name {
            q = q.bind(service_name);
        }
        if let Some(min_severity) = filters.min_severity {
            q = q.bind(min_severity);
        }
        if let Some(trace_id) = &filters.trace_id {
            q = q.bind(trace_id);
        }
        if let Some(span_id) = &filters.span_id {
            q = q.bind(span_id);
        }
        if let Some(start_time) = filters.start_time {
            q = q.bind(start_time);
        }
        if let Some(end_time) = filters.end_time {
            q = q.bind(end_time);
        }
        if let Some(search_query) = &filters.search_query {
            q = q.bind(format!("%{}%", search_query));
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

    /// Search logs by service name and time range.
    pub async fn search_by_service(
        &self,
        service_name: &str,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> StorageResult<Vec<LogRecord>> {
        sqlx::query_as::<_, LogRecord>(
            r#"
            SELECT * FROM log_records
            WHERE service_name = $1
              AND timestamp >= $2
              AND timestamp <= $3
            ORDER BY timestamp DESC
            LIMIT 1000
            "#,
        )
        .bind(service_name)
        .bind(start_time)
        .bind(end_time)
        .fetch_all(self.pool.postgres())
        .await
        .map_err(StorageError::from)
    }

    /// Search logs by trace ID (get all logs for a trace).
    pub async fn search_by_trace(&self, trace_id: &str) -> StorageResult<Vec<LogRecord>> {
        self.get_logs_by_trace(trace_id).await
    }

    /// Get all logs for a trace.
    pub async fn get_logs_by_trace(&self, trace_id: &str) -> StorageResult<Vec<LogRecord>> {
        sqlx::query_as::<_, LogRecord>(
            r#"
            SELECT * FROM log_records
            WHERE trace_id = $1
            ORDER BY timestamp ASC
            "#,
        )
        .bind(trace_id)
        .fetch_all(self.pool.postgres())
        .await
        .map_err(StorageError::from)
    }

    /// Search logs by severity level.
    pub async fn search_by_level(
        &self,
        min_level: LogLevel,
        filters: LogFilters,
    ) -> StorageResult<Vec<LogRecord>> {
        let mut filters = filters;
        filters.min_severity = Some(min_level.to_severity_number());
        self.list(filters).await
    }

    /// Full-text search in log messages.
    pub async fn search_text(
        &self,
        query: &str,
        filters: LogFilters,
    ) -> StorageResult<Vec<LogRecord>> {
        let mut filters = filters;
        filters.search_query = Some(query.to_string());
        self.list(filters).await
    }

    /// Advanced search with full-text query.
    pub async fn search_logs(
        &self,
        query: &str,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> StorageResult<Vec<LogRecord>> {
        sqlx::query_as::<_, LogRecord>(
            r#"
            SELECT * FROM log_records
            WHERE body ILIKE $1
              AND timestamp >= $2
              AND timestamp <= $3
            ORDER BY timestamp DESC
            LIMIT 1000
            "#,
        )
        .bind(format!("%{}%", query))
        .bind(start_time)
        .bind(end_time)
        .fetch_all(self.pool.postgres())
        .await
        .map_err(StorageError::from)
    }

    /// Get error logs for a time range.
    pub async fn get_errors(
        &self,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> StorageResult<Vec<LogRecord>> {
        let mut filters = LogFilters::default();
        filters.min_severity = Some(LogLevel::Error.to_severity_number());
        filters.start_time = Some(start_time);
        filters.end_time = Some(end_time);
        filters.limit = Some(1000);

        self.list(filters).await
    }

    /// Get log statistics for a time range.
    pub async fn get_stats(
        &self,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> StorageResult<LogStats> {
        let row = sqlx::query(
            r#"
            SELECT
                COUNT(*) as total_logs,
                COUNT(*) FILTER (WHERE severity_number >= 17) as error_count,
                COUNT(*) FILTER (WHERE severity_number >= 13 AND severity_number < 17) as warn_count,
                COUNT(*) FILTER (WHERE severity_number >= 9 AND severity_number < 13) as info_count,
                EXTRACT(EPOCH FROM ($2 - $1))::FLOAT as duration_seconds
            FROM log_records
            WHERE timestamp >= $1 AND timestamp <= $2
            "#
        )
        .bind(start_time)
        .bind(end_time)
        .fetch_one(self.pool.postgres())
        .await
        .map_err(StorageError::from)?;

        let total_logs: i64 = row.try_get("total_logs")?;
        let error_count: i64 = row.try_get("error_count")?;
        let warn_count: i64 = row.try_get("warn_count")?;
        let info_count: i64 = row.try_get("info_count")?;
        let duration_seconds: f64 = row.try_get("duration_seconds")?;

        let logs_per_second = if duration_seconds > 0.0 {
            Some(total_logs as f64 / duration_seconds)
        } else {
            None
        };

        Ok(LogStats {
            total_logs,
            error_count,
            warn_count,
            info_count,
            logs_per_second,
        })
    }

    /// Get log count by severity level.
    pub async fn count_by_level(
        &self,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> StorageResult<Vec<LogLevelCount>> {
        let rows = sqlx::query(
            r#"
            SELECT
                severity_number,
                severity_text,
                COUNT(*) as count
            FROM log_records
            WHERE timestamp >= $1 AND timestamp <= $2
            GROUP BY severity_number, severity_text
            ORDER BY severity_number ASC
            "#,
        )
        .bind(start_time)
        .bind(end_time)
        .fetch_all(self.pool.postgres())
        .await
        .map_err(StorageError::from)?;

        let mut counts = Vec::new();
        for row in rows {
            counts.push(LogLevelCount {
                severity_number: row.try_get("severity_number")?,
                severity_text: row.try_get("severity_text")?,
                count: row.try_get("count")?,
            });
        }

        Ok(counts)
    }

    /// Delete old logs (for data retention).
    pub async fn delete_before(&self, before: DateTime<Utc>) -> StorageResult<u64> {
        let result = sqlx::query!("DELETE FROM log_records WHERE timestamp < $1", before)
            .execute(self.pool.postgres())
            .await
            .map_err(StorageError::from)?;

        Ok(result.rows_affected())
    }

    /// Stream logs in real-time (tail functionality).
    ///
    /// Note: This is a simple polling-based implementation.
    /// For production use, consider using PostgreSQL LISTEN/NOTIFY.
    pub async fn stream_logs(
        &self,
        filters: LogFilters,
    ) -> StorageResult<impl futures::Stream<Item = StorageResult<LogRecord>>> {
        use futures::stream::{self, StreamExt};
        use std::time::Duration;

        let pool = self.pool.clone();
        let mut last_timestamp = filters.start_time.unwrap_or_else(Utc::now);

        let stream = stream::unfold(
            (pool, filters, last_timestamp),
            move |(pool, mut filters, mut last_ts)| async move {
                // Poll for new logs every second
                tokio::time::sleep(Duration::from_secs(1)).await;

                filters.start_time = Some(last_ts);
                filters.limit = Some(100);
                filters.sort_order = SortOrder::Asc;

                let repo = LogRepository { pool: pool.clone() };
                match repo.list(filters.clone()).await {
                    Ok(logs) => {
                        if !logs.is_empty() {
                            if let Some(last_log) = logs.last() {
                                last_ts = last_log.timestamp;
                            }

                            let items: Vec<_> = logs.into_iter().map(Ok).collect();
                            Some((stream::iter(items), (pool, filters, last_ts)))
                        } else {
                            Some((stream::iter(vec![]), (pool, filters, last_ts)))
                        }
                    }
                    Err(e) => Some((stream::iter(vec![Err(e)]), (pool, filters, last_ts))),
                }
            },
        )
        .flatten();

        Ok(stream)
    }
}

/// Filters for querying logs.
#[derive(Debug, Default, Clone)]
pub struct LogFilters {
    /// Filter by service name
    pub service_name: Option<String>,

    /// Filter by minimum severity level
    pub min_severity: Option<i32>,

    /// Filter by trace ID
    pub trace_id: Option<String>,

    /// Filter by span ID
    pub span_id: Option<String>,

    /// Start time range
    pub start_time: Option<DateTime<Utc>>,

    /// End time range
    pub end_time: Option<DateTime<Utc>>,

    /// Full-text search query
    pub search_query: Option<String>,

    /// Limit number of results
    pub limit: Option<i64>,

    /// Offset for pagination
    pub offset: Option<i64>,

    /// Sort order (asc or desc)
    pub sort_order: SortOrder,
}

/// Sort order for log queries.
#[derive(Debug, Clone, Copy, Default)]
pub enum SortOrder {
    /// Ascending (oldest first)
    Asc,
    /// Descending (newest first)
    #[default]
    Desc,
}

/// Statistics about logs.
#[derive(Debug, Clone)]
pub struct LogStats {
    /// Total number of logs
    pub total_logs: i64,

    /// Number of error logs
    pub error_count: i64,

    /// Number of warning logs
    pub warn_count: i64,

    /// Number of info logs
    pub info_count: i64,

    /// Logs per second (average)
    pub logs_per_second: Option<f64>,
}

/// Count of logs by severity level.
#[derive(Debug, Clone)]
pub struct LogLevelCount {
    /// Severity level
    pub severity_number: i32,

    /// Severity text
    pub severity_text: String,

    /// Count of logs at this level
    pub count: i64,
}

impl std::fmt::Display for SortOrder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SortOrder::Asc => write!(f, "asc"),
            SortOrder::Desc => write!(f, "desc"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO: Add integration tests with test database

    #[test]
    fn test_log_filters_default() {
        let filters = LogFilters::default();
        assert!(filters.service_name.is_none());
        assert_eq!(filters.sort_order.to_string(), "desc");
    }

    #[test]
    fn test_sort_order_display() {
        assert_eq!(SortOrder::Asc.to_string(), "asc");
        assert_eq!(SortOrder::Desc.to_string(), "desc");
    }
}
