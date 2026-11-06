//! Metric writer for batch insertion of metric data.

use crate::error::{StorageError, StorageResult};
use crate::models::{Metric, MetricDataPoint};
use crate::pool::StoragePool;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Writer for batch insertion of metric data.
///
/// This writer buffers metrics and data points, inserting them in batches for improved performance.
#[derive(Clone)]
pub struct MetricWriter {
    pool: StoragePool,
    buffer: Arc<RwLock<MetricBuffer>>,
    config: WriterConfig,
}

/// Configuration for the metric writer.
#[derive(Debug, Clone)]
pub struct WriterConfig {
    /// Maximum number of data points to buffer before flushing
    pub batch_size: usize,

    /// Maximum time to wait before flushing (in seconds)
    pub flush_interval_secs: u64,

    /// Maximum number of concurrent insert operations
    pub max_concurrency: usize,
}

impl Default for WriterConfig {
    fn default() -> Self {
        Self {
            batch_size: 500,
            flush_interval_secs: 5,
            max_concurrency: 4,
        }
    }
}

/// Internal buffer for metric data.
struct MetricBuffer {
    metrics: Vec<Metric>,
    data_points: Vec<MetricDataPoint>,
}

impl Default for MetricBuffer {
    fn default() -> Self {
        Self {
            metrics: Vec::new(),
            data_points: Vec::new(),
        }
    }
}

impl MetricWriter {
    /// Create a new metric writer.
    pub fn new(pool: StoragePool) -> Self {
        Self::with_config(pool, WriterConfig::default())
    }

    /// Create a new metric writer with custom configuration.
    pub fn with_config(pool: StoragePool, config: WriterConfig) -> Self {
        Self {
            pool,
            buffer: Arc::new(RwLock::new(MetricBuffer::default())),
            config,
        }
    }

    /// Write a single metric definition.
    ///
    /// The metric will be buffered and inserted in the next batch.
    pub async fn write_metric(&self, metric: Metric) -> StorageResult<()> {
        let mut buffer = self.buffer.write().await;
        buffer.metrics.push(metric);

        // Auto-flush if batch size reached
        if buffer.metrics.len() >= self.config.batch_size {
            drop(buffer);
            self.flush().await?;
        }

        Ok(())
    }

    /// Write multiple metrics in a batch.
    pub async fn write_metrics(&self, metrics: Vec<Metric>) -> StorageResult<()> {
        let mut buffer = self.buffer.write().await;
        buffer.metrics.extend(metrics);

        // Auto-flush if batch size reached
        if buffer.metrics.len() >= self.config.batch_size {
            drop(buffer);
            self.flush().await?;
        }

        Ok(())
    }

    /// Write a single data point.
    pub async fn write_data_point(&self, data_point: MetricDataPoint) -> StorageResult<()> {
        let mut buffer = self.buffer.write().await;
        buffer.data_points.push(data_point);

        // Auto-flush if batch size reached
        if buffer.data_points.len() >= self.config.batch_size {
            drop(buffer);
            self.flush().await?;
        }

        Ok(())
    }

    /// Write multiple data points in a batch.
    pub async fn write_data_points(&self, data_points: Vec<MetricDataPoint>) -> StorageResult<()> {
        let mut buffer = self.buffer.write().await;
        buffer.data_points.extend(data_points);

        // Auto-flush if batch size reached
        if buffer.data_points.len() >= self.config.batch_size {
            drop(buffer);
            self.flush().await?;
        }

        Ok(())
    }

    /// Flush all buffered data to the database.
    pub async fn flush(&self) -> StorageResult<()> {
        let mut buffer = self.buffer.write().await;

        // Take all buffered data
        let metrics = std::mem::take(&mut buffer.metrics);
        let data_points = std::mem::take(&mut buffer.data_points);

        drop(buffer); // Release lock during insertion

        // Insert metrics
        if !metrics.is_empty() {
            self.insert_metrics(metrics).await?;
        }

        // Insert data points
        if !data_points.is_empty() {
            self.insert_data_points(data_points).await?;
        }

        Ok(())
    }

    /// Insert metrics using batch insert or upsert.
    async fn insert_metrics(&self, metrics: Vec<Metric>) -> StorageResult<()> {
        if metrics.is_empty() {
            return Ok(());
        }

        tracing::debug!("Inserting {} metrics", metrics.len());
        let start = std::time::Instant::now();

        let mut query_builder = sqlx::QueryBuilder::new(
            "INSERT INTO metrics (id, name, description, unit, metric_type, service_name, \
             attributes, resource_attributes, created_at, updated_at) ",
        );

        query_builder.push_values(metrics, |mut b, metric| {
            b.push_bind(metric.id)
                .push_bind(metric.name)
                .push_bind(metric.description)
                .push_bind(metric.unit)
                .push_bind(metric.metric_type)
                .push_bind(metric.service_name)
                .push_bind(metric.attributes)
                .push_bind(metric.resource_attributes)
                .push_bind(metric.created_at)
                .push_bind(metric.updated_at);
        });

        // Upsert: update metadata if metric already exists
        query_builder.push(
            " ON CONFLICT (name, service_name) DO UPDATE SET \
             description = EXCLUDED.description, \
             unit = EXCLUDED.unit, \
             metric_type = EXCLUDED.metric_type, \
             attributes = EXCLUDED.attributes, \
             resource_attributes = EXCLUDED.resource_attributes, \
             updated_at = EXCLUDED.updated_at",
        );

        query_builder.build().execute(self.pool.postgres()).await?;

        let elapsed = start.elapsed();
        tracing::info!(
            "Inserted {} metrics in {:?} ({:.0} metrics/sec)",
            metrics.len(),
            elapsed,
            metrics.len() as f64 / elapsed.as_secs_f64()
        );

        Ok(())
    }

    /// Insert data points using batch insert.
    async fn insert_data_points(&self, data_points: Vec<MetricDataPoint>) -> StorageResult<()> {
        if data_points.is_empty() {
            return Ok(());
        }

        tracing::debug!("Inserting {} data points", data_points.len());
        let start = std::time::Instant::now();

        let mut query_builder = sqlx::QueryBuilder::new(
            "INSERT INTO metric_data_points (id, metric_id, timestamp, value, count, sum, \
             min, max, buckets, quantiles, exemplars, attributes, created_at) ",
        );

        query_builder.push_values(data_points, |mut b, dp| {
            b.push_bind(dp.id)
                .push_bind(dp.metric_id)
                .push_bind(dp.timestamp)
                .push_bind(dp.value)
                .push_bind(dp.count)
                .push_bind(dp.sum)
                .push_bind(dp.min)
                .push_bind(dp.max)
                .push_bind(dp.buckets)
                .push_bind(dp.quantiles)
                .push_bind(dp.exemplars)
                .push_bind(dp.attributes)
                .push_bind(dp.created_at);
        });

        query_builder.build().execute(self.pool.postgres()).await?;

        let elapsed = start.elapsed();
        tracing::info!(
            "Inserted {} data points in {:?} ({:.0} points/sec)",
            data_points.len(),
            elapsed,
            data_points.len() as f64 / elapsed.as_secs_f64()
        );

        Ok(())
    }

    /// Get current buffer statistics.
    pub async fn buffer_stats(&self) -> BufferStats {
        let buffer = self.buffer.read().await;
        BufferStats {
            metrics_buffered: buffer.metrics.len(),
            data_points_buffered: buffer.data_points.len(),
        }
    }
}

/// Statistics about the writer's buffer.
#[derive(Debug, Clone)]
pub struct BufferStats {
    /// Number of metrics currently buffered
    pub metrics_buffered: usize,

    /// Number of data points currently buffered
    pub data_points_buffered: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_writer_config_default() {
        let config = WriterConfig::default();
        assert_eq!(config.batch_size, 500);
        assert_eq!(config.flush_interval_secs, 5);
    }

    // TODO: Add integration tests with test database
}
