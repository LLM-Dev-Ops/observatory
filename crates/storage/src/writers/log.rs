//! Log writer for batch insertion of log data.

use crate::error::{StorageError, StorageResult};
use crate::models::LogRecord;
use crate::pool::StoragePool;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Writer for batch insertion of log data.
///
/// This writer buffers log records and inserts them in batches for improved performance.
#[derive(Clone)]
pub struct LogWriter {
    pool: StoragePool,
    buffer: Arc<RwLock<LogBuffer>>,
    config: WriterConfig,
}

/// Configuration for the log writer.
#[derive(Debug, Clone)]
pub struct WriterConfig {
    /// Maximum number of logs to buffer before flushing
    pub batch_size: usize,

    /// Maximum time to wait before flushing (in seconds)
    pub flush_interval_secs: u64,

    /// Maximum number of concurrent insert operations
    pub max_concurrency: usize,
}

impl Default for WriterConfig {
    fn default() -> Self {
        Self {
            batch_size: 1000,
            flush_interval_secs: 5,
            max_concurrency: 4,
        }
    }
}

/// Internal buffer for log data.
struct LogBuffer {
    logs: Vec<LogRecord>,
}

impl Default for LogBuffer {
    fn default() -> Self {
        Self { logs: Vec::new() }
    }
}

impl LogWriter {
    /// Create a new log writer.
    pub fn new(pool: StoragePool) -> Self {
        Self::with_config(pool, WriterConfig::default())
    }

    /// Create a new log writer with custom configuration.
    pub fn with_config(pool: StoragePool, config: WriterConfig) -> Self {
        Self {
            pool,
            buffer: Arc::new(RwLock::new(LogBuffer::default())),
            config,
        }
    }

    /// Write a single log record.
    ///
    /// The log will be buffered and inserted in the next batch.
    pub async fn write_log(&self, log: LogRecord) -> StorageResult<()> {
        let mut buffer = self.buffer.write().await;
        buffer.logs.push(log);

        // Auto-flush if batch size reached
        if buffer.logs.len() >= self.config.batch_size {
            drop(buffer);
            self.flush().await?;
        }

        Ok(())
    }

    /// Write multiple log records in a batch.
    pub async fn write_logs(&self, logs: Vec<LogRecord>) -> StorageResult<()> {
        let mut buffer = self.buffer.write().await;
        buffer.logs.extend(logs);

        // Auto-flush if batch size reached
        if buffer.logs.len() >= self.config.batch_size {
            drop(buffer);
            self.flush().await?;
        }

        Ok(())
    }

    /// Flush all buffered data to the database.
    pub async fn flush(&self) -> StorageResult<()> {
        let mut buffer = self.buffer.write().await;

        // Take all buffered data
        let logs = std::mem::take(&mut buffer.logs);

        drop(buffer); // Release lock during insertion

        // Insert logs
        if !logs.is_empty() {
            self.insert_logs(logs).await?;
        }

        Ok(())
    }

    /// Insert logs using batch insert.
    async fn insert_logs(&self, logs: Vec<LogRecord>) -> StorageResult<()> {
        if logs.is_empty() {
            return Ok(());
        }

        tracing::debug!("Inserting {} logs", logs.len());
        let start = std::time::Instant::now();

        let mut query_builder = sqlx::QueryBuilder::new(
            "INSERT INTO logs (id, timestamp, observed_timestamp, severity_number, severity_text, \
             body, service_name, trace_id, span_id, trace_flags, attributes, resource_attributes, \
             scope_name, scope_version, scope_attributes, created_at) ",
        );

        query_builder.push_values(logs, |mut b, log| {
            b.push_bind(log.id)
                .push_bind(log.timestamp)
                .push_bind(log.observed_timestamp)
                .push_bind(log.severity_number)
                .push_bind(log.severity_text)
                .push_bind(log.body)
                .push_bind(log.service_name)
                .push_bind(log.trace_id)
                .push_bind(log.span_id)
                .push_bind(log.trace_flags)
                .push_bind(log.attributes)
                .push_bind(log.resource_attributes)
                .push_bind(log.scope_name)
                .push_bind(log.scope_version)
                .push_bind(log.scope_attributes)
                .push_bind(log.created_at);
        });

        query_builder.build().execute(self.pool.postgres()).await?;

        let elapsed = start.elapsed();
        tracing::info!(
            "Inserted {} logs in {:?} ({:.0} logs/sec)",
            logs.len(),
            elapsed,
            logs.len() as f64 / elapsed.as_secs_f64()
        );

        Ok(())
    }

    /// Get current buffer statistics.
    pub async fn buffer_stats(&self) -> BufferStats {
        let buffer = self.buffer.read().await;
        BufferStats {
            logs_buffered: buffer.logs.len(),
        }
    }

    /// Start automatic flushing based on time interval.
    ///
    /// Returns a handle that can be used to stop the auto-flush task.
    pub fn start_auto_flush(&self) -> tokio::task::JoinHandle<()> {
        let writer = self.clone();
        let interval = std::time::Duration::from_secs(self.config.flush_interval_secs);

        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            loop {
                ticker.tick().await;
                if let Err(e) = writer.flush().await {
                    tracing::error!("Auto-flush error: {}", e);
                }
            }
        })
    }
}

/// Statistics about the writer's buffer.
#[derive(Debug, Clone)]
pub struct BufferStats {
    /// Number of logs currently buffered
    pub logs_buffered: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_writer_config_default() {
        let config = WriterConfig::default();
        assert_eq!(config.batch_size, 1000);
        assert_eq!(config.flush_interval_secs, 5);
    }

    // TODO: Add integration tests with test database
}
