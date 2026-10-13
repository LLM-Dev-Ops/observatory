//! Trace writer for batch insertion of trace data.

use crate::error::{StorageError, StorageResult};
use crate::models::{Trace, TraceEvent, TraceSpan};
use crate::pool::StoragePool;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Writer for batch insertion of trace data.
///
/// This writer buffers traces and inserts them in batches for improved performance.
#[derive(Clone)]
pub struct TraceWriter {
    pool: StoragePool,
    buffer: Arc<RwLock<TraceBuffer>>,
    config: WriterConfig,
    stats: Arc<RwLock<WriteStats>>,
}

/// Configuration for the trace writer.
#[derive(Debug, Clone)]
pub struct WriterConfig {
    /// Maximum number of traces to buffer before flushing
    pub batch_size: usize,

    /// Maximum time to wait before flushing (in seconds)
    pub flush_interval_secs: u64,

    /// Maximum number of concurrent insert operations
    pub max_concurrency: usize,
}

impl Default for WriterConfig {
    fn default() -> Self {
        Self {
            batch_size: 100,
            flush_interval_secs: 5,
            max_concurrency: 4,
        }
    }
}

/// Internal buffer for trace data.
struct TraceBuffer {
    traces: Vec<Trace>,
    spans: Vec<TraceSpan>,
    events: Vec<TraceEvent>,
}

impl Default for TraceBuffer {
    fn default() -> Self {
        Self {
            traces: Vec::new(),
            spans: Vec::new(),
            events: Vec::new(),
        }
    }
}

impl TraceWriter {
    /// Create a new trace writer.
    pub fn new(pool: StoragePool) -> Self {
        Self::with_config(pool, WriterConfig::default())
    }

    /// Create a new trace writer with custom configuration.
    pub fn with_config(pool: StoragePool, config: WriterConfig) -> Self {
        Self {
            pool,
            buffer: Arc::new(RwLock::new(TraceBuffer::default())),
            config,
            stats: Arc::new(RwLock::new(WriteStats::default())),
        }
    }

    /// Write a single trace.
    ///
    /// The trace will be buffered and inserted in the next batch.
    pub async fn write_trace(&self, trace: Trace) -> StorageResult<()> {
        let mut buffer = self.buffer.write().await;
        buffer.traces.push(trace);

        // Auto-flush if batch size reached
        if buffer.traces.len() >= self.config.batch_size {
            drop(buffer); // Release lock before flushing
            self.flush().await?;
        }

        Ok(())
    }

    /// Write multiple traces in a batch.
    pub async fn write_traces(&self, traces: Vec<Trace>) -> StorageResult<()> {
        let mut buffer = self.buffer.write().await;
        buffer.traces.extend(traces);

        // Auto-flush if batch size reached
        if buffer.traces.len() >= self.config.batch_size {
            drop(buffer);
            self.flush().await?;
        }

        Ok(())
    }

    /// Write a single span.
    pub async fn write_span(&self, span: TraceSpan) -> StorageResult<()> {
        let mut buffer = self.buffer.write().await;
        buffer.spans.push(span);

        // Auto-flush if batch size reached
        if buffer.spans.len() >= self.config.batch_size {
            drop(buffer);
            self.flush().await?;
        }

        Ok(())
    }

    /// Write multiple spans in a batch.
    pub async fn write_spans(&self, spans: Vec<TraceSpan>) -> StorageResult<()> {
        let mut buffer = self.buffer.write().await;
        buffer.spans.extend(spans);

        // Auto-flush if batch size reached
        if buffer.spans.len() >= self.config.batch_size {
            drop(buffer);
            self.flush().await?;
        }

        Ok(())
    }

    /// Write a single event.
    pub async fn write_event(&self, event: TraceEvent) -> StorageResult<()> {
        let mut buffer = self.buffer.write().await;
        buffer.events.push(event);

        Ok(())
    }

    /// Flush all buffered data to the database.
    pub async fn flush(&self) -> StorageResult<()> {
        let mut buffer = self.buffer.write().await;

        // Take all buffered data
        let traces = std::mem::take(&mut buffer.traces);
        let spans = std::mem::take(&mut buffer.spans);
        let events = std::mem::take(&mut buffer.events);

        drop(buffer); // Release lock during insertion

        // Insert traces with retry logic
        if !traces.is_empty() {
            let count = traces.len();
            let traces_clone = traces.clone();
            self.with_retry(|| async { self.insert_traces(traces_clone.clone()).await })
                .await?;

            // Update stats
            let mut stats = self.stats.write().await;
            stats.traces_written += count as u64;
            drop(stats);
        }

        // Insert spans with retry logic
        if !spans.is_empty() {
            let count = spans.len();
            let spans_clone = spans.clone();
            self.with_retry(|| async { self.insert_spans(spans_clone.clone()).await })
                .await?;

            // Update stats
            let mut stats = self.stats.write().await;
            stats.spans_written += count as u64;
            drop(stats);
        }

        // Insert events with retry logic
        if !events.is_empty() {
            let count = events.len();
            let events_clone = events.clone();
            self.with_retry(|| async { self.insert_events(events_clone.clone()).await })
                .await?;

            // Update stats
            let mut stats = self.stats.write().await;
            stats.events_written += count as u64;
            drop(stats);
        }

        Ok(())
    }

    /// Insert traces using batch insert.
    async fn insert_traces(&self, traces: Vec<Trace>) -> StorageResult<()> {
        if traces.is_empty() {
            return Ok(());
        }

        tracing::debug!("Inserting {} traces", traces.len());
        let start = std::time::Instant::now();

        // Use QueryBuilder for batch inserts (more efficient than individual INSERTs)
        let mut query_builder = sqlx::QueryBuilder::new(
            "INSERT INTO traces (id, trace_id, service_name, start_time, end_time, duration_us, \
             status, status_message, root_span_name, attributes, resource_attributes, span_count, \
             created_at, updated_at) ",
        );

        query_builder.push_values(traces, |mut b, trace| {
            b.push_bind(trace.id)
                .push_bind(trace.trace_id)
                .push_bind(trace.service_name)
                .push_bind(trace.start_time)
                .push_bind(trace.end_time)
                .push_bind(trace.duration_us)
                .push_bind(trace.status)
                .push_bind(trace.status_message)
                .push_bind(trace.root_span_name)
                .push_bind(trace.attributes)
                .push_bind(trace.resource_attributes)
                .push_bind(trace.span_count)
                .push_bind(trace.created_at)
                .push_bind(trace.updated_at);
        });

        // Add ON CONFLICT clause to handle duplicates
        query_builder.push(
            " ON CONFLICT (trace_id) DO UPDATE SET \
             end_time = EXCLUDED.end_time, \
             duration_us = EXCLUDED.duration_us, \
             status = EXCLUDED.status, \
             status_message = EXCLUDED.status_message, \
             span_count = EXCLUDED.span_count, \
             updated_at = EXCLUDED.updated_at",
        );

        query_builder.build().execute(self.pool.postgres()).await?;

        let elapsed = start.elapsed();
        tracing::info!(
            "Inserted {} traces in {:?} ({:.0} traces/sec)",
            traces.len(),
            elapsed,
            traces.len() as f64 / elapsed.as_secs_f64()
        );

        Ok(())
    }

    /// Insert spans using batch insert.
    async fn insert_spans(&self, spans: Vec<TraceSpan>) -> StorageResult<()> {
        if spans.is_empty() {
            return Ok(());
        }

        tracing::debug!("Inserting {} spans", spans.len());
        let start = std::time::Instant::now();

        let mut query_builder = sqlx::QueryBuilder::new(
            "INSERT INTO trace_spans (id, trace_id, span_id, parent_span_id, name, kind, \
             service_name, start_time, end_time, duration_us, status, status_message, \
             attributes, events, links, created_at) ",
        );

        query_builder.push_values(spans, |mut b, span| {
            b.push_bind(span.id)
                .push_bind(span.trace_id)
                .push_bind(span.span_id)
                .push_bind(span.parent_span_id)
                .push_bind(span.name)
                .push_bind(span.kind)
                .push_bind(span.service_name)
                .push_bind(span.start_time)
                .push_bind(span.end_time)
                .push_bind(span.duration_us)
                .push_bind(span.status)
                .push_bind(span.status_message)
                .push_bind(span.attributes)
                .push_bind(span.events)
                .push_bind(span.links)
                .push_bind(span.created_at);
        });

        // Add ON CONFLICT clause to handle duplicates
        query_builder.push(
            " ON CONFLICT (span_id) DO UPDATE SET \
             end_time = EXCLUDED.end_time, \
             duration_us = EXCLUDED.duration_us, \
             status = EXCLUDED.status, \
             status_message = EXCLUDED.status_message, \
             attributes = EXCLUDED.attributes, \
             events = EXCLUDED.events",
        );

        query_builder.build().execute(self.pool.postgres()).await?;

        let elapsed = start.elapsed();
        tracing::info!(
            "Inserted {} spans in {:?} ({:.0} spans/sec)",
            spans.len(),
            elapsed,
            spans.len() as f64 / elapsed.as_secs_f64()
        );

        Ok(())
    }

    /// Insert events using batch insert.
    async fn insert_events(&self, events: Vec<TraceEvent>) -> StorageResult<()> {
        if events.is_empty() {
            return Ok(());
        }

        tracing::debug!("Inserting {} events", events.len());
        let start = std::time::Instant::now();

        let mut query_builder = sqlx::QueryBuilder::new(
            "INSERT INTO trace_events (id, span_id, name, timestamp, attributes, created_at) ",
        );

        query_builder.push_values(events, |mut b, event| {
            b.push_bind(event.id)
                .push_bind(event.span_id)
                .push_bind(event.name)
                .push_bind(event.timestamp)
                .push_bind(event.attributes)
                .push_bind(event.created_at);
        });

        query_builder.build().execute(self.pool.postgres()).await?;

        let elapsed = start.elapsed();
        tracing::info!(
            "Inserted {} events in {:?} ({:.0} events/sec)",
            events.len(),
            elapsed,
            events.len() as f64 / elapsed.as_secs_f64()
        );

        Ok(())
    }

    /// Get current buffer statistics.
    pub async fn buffer_stats(&self) -> BufferStats {
        let buffer = self.buffer.read().await;
        BufferStats {
            traces_buffered: buffer.traces.len(),
            spans_buffered: buffer.spans.len(),
            events_buffered: buffer.events.len(),
        }
    }

    /// Get write statistics.
    pub async fn write_stats(&self) -> WriteStats {
        let stats = self.stats.read().await;
        stats.clone()
    }

    /// Write a span from an LlmSpan, ensuring the trace exists.
    ///
    /// This method properly resolves the trace UUID from the string trace_id,
    /// creating the trace if it doesn't exist. This solves the UUID resolution
    /// problem in the From<LlmSpan> conversion.
    ///
    /// # Arguments
    ///
    /// * `llm_span` - The LLM span to convert and write
    ///
    /// # Returns
    ///
    /// The converted TraceSpan with the proper trace_id UUID
    ///
    /// # Errors
    ///
    /// Returns `StorageError` if:
    /// - Database query fails
    /// - Trace creation fails
    /// - Span insertion fails
    ///
    /// # Example
    ///
    /// ```no_run
    /// use llm_observatory_storage::writers::TraceWriter;
    /// use llm_observatory_core::span::LlmSpan;
    ///
    /// # async fn example(writer: TraceWriter, llm_span: LlmSpan) -> Result<(), Box<dyn std::error::Error>> {
    /// let trace_span = writer.write_span_from_llm(llm_span).await?;
    /// println!("Written span with trace UUID: {}", trace_span.trace_id);
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "llm-span-conversion")]
    pub async fn write_span_from_llm(
        &self,
        llm_span: llm_observatory_core::span::LlmSpan,
    ) -> StorageResult<TraceSpan> {
        use llm_observatory_core::span::LlmSpan;

        // 1. Ensure trace exists and get its UUID
        let trace = self.ensure_trace(&llm_span.trace_id, &llm_span).await?;

        // 2. Convert span to TraceSpan
        let mut span = TraceSpan::from(llm_span);

        // 3. Set the proper trace_id UUID (this was previously Uuid::default())
        span.trace_id = trace.id;

        // 4. Write the span
        self.write_span(span.clone()).await?;

        Ok(span)
    }

    /// Ensure a trace exists, creating it if necessary.
    ///
    /// This method queries the database for an existing trace by its string trace_id.
    /// If the trace doesn't exist, it creates a new one with information from the span.
    ///
    /// This handles concurrent writes correctly through PostgreSQL's ON CONFLICT clause,
    /// so multiple writers can safely call this method for the same trace_id.
    ///
    /// # Arguments
    ///
    /// * `trace_id` - The string trace ID to look up
    /// * `llm_span` - The span to extract trace metadata from (if creating)
    ///
    /// # Returns
    ///
    /// The existing or newly created Trace with its UUID
    ///
    /// # Errors
    ///
    /// Returns `StorageError` if database queries fail
    #[cfg(feature = "llm-span-conversion")]
    async fn ensure_trace(
        &self,
        trace_id: &str,
        llm_span: &llm_observatory_core::span::LlmSpan,
    ) -> StorageResult<Trace> {
        // Try to get existing trace first (most common case - trace already exists)
        let existing =
            sqlx::query_as::<_, Trace>("SELECT * FROM traces WHERE trace_id = $1 LIMIT 1")
                .bind(trace_id)
                .fetch_optional(self.pool.postgres())
                .await?;

        if let Some(trace) = existing {
            return Ok(trace);
        }

        // Trace doesn't exist, create it
        let service_name = llm_span
            .metadata
            .environment
            .clone()
            .unwrap_or_else(|| format!("llm-{}", llm_span.provider.as_str()));

        let trace = Trace::new(
            trace_id.to_string(),
            service_name,
            llm_span.latency.start_time,
        );

        // Insert the trace (with ON CONFLICT to handle race conditions)
        // If another writer creates the same trace concurrently, we'll use theirs
        let inserted = sqlx::query_as::<_, Trace>(
            "INSERT INTO traces (id, trace_id, service_name, start_time, end_time, duration_us, \
             status, status_message, root_span_name, attributes, resource_attributes, span_count, \
             created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14) \
             ON CONFLICT (trace_id) DO UPDATE SET updated_at = EXCLUDED.updated_at \
             RETURNING *",
        )
        .bind(trace.id)
        .bind(&trace.trace_id)
        .bind(&trace.service_name)
        .bind(trace.start_time)
        .bind(trace.end_time)
        .bind(trace.duration_us)
        .bind(&trace.status)
        .bind(&trace.status_message)
        .bind(&trace.root_span_name)
        .bind(&trace.attributes)
        .bind(&trace.resource_attributes)
        .bind(trace.span_count)
        .bind(trace.created_at)
        .bind(trace.updated_at)
        .fetch_one(self.pool.postgres())
        .await?;

        Ok(inserted)
    }

    /// Execute an operation with retry logic.
    async fn with_retry<F, Fut, T>(&self, op: F) -> StorageResult<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = StorageResult<T>>,
    {
        let max_retries = 3;
        let mut attempt = 0;

        loop {
            match op().await {
                Ok(result) => return Ok(result),
                Err(e) if e.is_retryable() && attempt < max_retries => {
                    attempt += 1;
                    let delay = std::time::Duration::from_millis(100 * (1 << attempt));
                    tracing::warn!(
                        "Operation failed (attempt {}/{}), retrying in {:?}: {}",
                        attempt,
                        max_retries,
                        delay,
                        e
                    );

                    // Update retry stats
                    let mut stats = self.stats.write().await;
                    stats.retries += 1;
                    drop(stats);

                    tokio::time::sleep(delay).await;
                }
                Err(e) => {
                    // Update failure stats
                    let mut stats = self.stats.write().await;
                    stats.write_failures += 1;
                    drop(stats);

                    return Err(e);
                }
            }
        }
    }
}

/// Statistics about the writer's buffer.
#[derive(Debug, Clone)]
pub struct BufferStats {
    /// Number of traces currently buffered
    pub traces_buffered: usize,

    /// Number of spans currently buffered
    pub spans_buffered: usize,

    /// Number of events currently buffered
    pub events_buffered: usize,
}

/// Statistics about write operations.
#[derive(Debug, Clone, Default)]
pub struct WriteStats {
    /// Total number of traces written
    pub traces_written: u64,

    /// Total number of spans written
    pub spans_written: u64,

    /// Total number of events written
    pub events_written: u64,

    /// Number of failed writes
    pub write_failures: u64,

    /// Number of retries
    pub retries: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_writer_config_default() {
        let config = WriterConfig::default();
        assert_eq!(config.batch_size, 100);
        assert_eq!(config.flush_interval_secs, 5);
    }

    // Unit tests for UUID resolution functionality
    // Note: These are unit tests that don't require a database.
    // Integration tests with a real database should be added separately.

    #[cfg(feature = "llm-span-conversion")]
    mod llm_span_conversion_tests {
        use super::*;
        use chrono::Utc;
        use llm_observatory_core::{
            span::{LlmInput, LlmSpan, SpanStatus},
            types::{Latency, Metadata, Provider},
        };

        fn create_test_llm_span() -> LlmSpan {
            let now = Utc::now();
            LlmSpan::builder()
                .span_id("span_123")
                .trace_id("trace_456")
                .name("llm.completion")
                .provider(Provider::OpenAI)
                .model("gpt-4")
                .input(LlmInput::Text {
                    prompt: "Hello".to_string(),
                })
                .latency(Latency::new(now, now))
                .status(SpanStatus::Ok)
                .build()
                .expect("Failed to build test span")
        }

        #[test]
        fn test_from_llm_span_creates_trace_span() {
            let llm_span = create_test_llm_span();
            let trace_span = TraceSpan::from(llm_span.clone());

            // Verify basic fields are converted correctly
            assert_eq!(trace_span.span_id, llm_span.span_id);
            assert_eq!(trace_span.name, llm_span.name);
            assert_eq!(trace_span.status, "ok");
            assert_eq!(trace_span.service_name, "llm-service");

            // Verify trace_id is a valid UUID (even if placeholder)
            assert_ne!(trace_span.trace_id, Uuid::nil());
        }

        #[test]
        fn test_from_llm_span_converts_status_correctly() {
            let now = Utc::now();

            // Test Ok status
            let mut span = create_test_llm_span();
            span.status = SpanStatus::Ok;
            let trace_span = TraceSpan::from(span);
            assert_eq!(trace_span.status, "ok");

            // Test Error status
            let mut span = create_test_llm_span();
            span.status = SpanStatus::Error;
            let trace_span = TraceSpan::from(span);
            assert_eq!(trace_span.status, "error");

            // Test Unset status
            let mut span = create_test_llm_span();
            span.status = SpanStatus::Unset;
            let trace_span = TraceSpan::from(span);
            assert_eq!(trace_span.status, "unset");
        }

        #[test]
        fn test_from_llm_span_includes_llm_attributes() {
            let llm_span = create_test_llm_span();
            let trace_span = TraceSpan::from(llm_span.clone());

            // Verify LLM attributes are added
            let attrs = trace_span.attributes.as_object().unwrap();
            assert_eq!(
                attrs.get("llm.provider").unwrap().as_str().unwrap(),
                "openai"
            );
            assert_eq!(attrs.get("llm.model").unwrap().as_str().unwrap(), "gpt-4");
            assert!(attrs.contains_key("llm.latency.total_ms"));
        }

        #[test]
        fn test_from_llm_span_with_token_usage() {
            use llm_observatory_core::types::TokenUsage;

            let mut llm_span = create_test_llm_span();
            llm_span.token_usage = Some(TokenUsage::new(100, 50));

            let trace_span = TraceSpan::from(llm_span);
            let attrs = trace_span.attributes.as_object().unwrap();

            assert_eq!(
                attrs
                    .get("llm.usage.prompt_tokens")
                    .unwrap()
                    .as_u64()
                    .unwrap(),
                100
            );
            assert_eq!(
                attrs
                    .get("llm.usage.completion_tokens")
                    .unwrap()
                    .as_u64()
                    .unwrap(),
                50
            );
            assert_eq!(
                attrs
                    .get("llm.usage.total_tokens")
                    .unwrap()
                    .as_u64()
                    .unwrap(),
                150
            );
        }

        #[test]
        fn test_from_llm_span_with_cost() {
            use llm_observatory_core::types::Cost;

            let mut llm_span = create_test_llm_span();
            llm_span.cost = Some(Cost::with_breakdown(0.001, 0.002));

            let trace_span = TraceSpan::from(llm_span);
            let attrs = trace_span.attributes.as_object().unwrap();

            assert_eq!(
                attrs.get("llm.cost.amount_usd").unwrap().as_f64().unwrap(),
                0.003
            );
            assert_eq!(
                attrs.get("llm.cost.prompt_usd").unwrap().as_f64().unwrap(),
                0.001
            );
            assert_eq!(
                attrs
                    .get("llm.cost.completion_usd")
                    .unwrap()
                    .as_f64()
                    .unwrap(),
                0.002
            );
        }

        #[test]
        fn test_from_llm_span_with_metadata() {
            let mut llm_span = create_test_llm_span();
            llm_span.metadata = Metadata {
                user_id: Some("user123".to_string()),
                session_id: Some("session456".to_string()),
                environment: Some("production".to_string()),
                ..Default::default()
            };

            let trace_span = TraceSpan::from(llm_span);
            let attrs = trace_span.attributes.as_object().unwrap();

            assert_eq!(attrs.get("user.id").unwrap().as_str().unwrap(), "user123");
            assert_eq!(
                attrs.get("session.id").unwrap().as_str().unwrap(),
                "session456"
            );
            assert_eq!(
                attrs
                    .get("deployment.environment")
                    .unwrap()
                    .as_str()
                    .unwrap(),
                "production"
            );
            assert_eq!(trace_span.service_name, "production");
        }

        #[test]
        fn test_from_llm_span_duration_conversion() {
            let now = Utc::now();
            let later = now + chrono::Duration::milliseconds(500);

            let mut llm_span = create_test_llm_span();
            llm_span.latency = Latency::new(now, later);

            let trace_span = TraceSpan::from(llm_span);

            // Duration should be converted from ms to us
            assert!(trace_span.duration_us.is_some());
            let duration_us = trace_span.duration_us.unwrap();
            // Should be approximately 500000 microseconds (500ms)
            assert!(duration_us >= 490000 && duration_us <= 510000);
        }

        #[test]
        fn test_from_llm_span_with_events() {
            use llm_observatory_core::span::SpanEvent;

            let mut llm_span = create_test_llm_span();
            llm_span.events = vec![SpanEvent {
                name: "test_event".to_string(),
                timestamp: Utc::now(),
                attributes: Default::default(),
            }];

            let trace_span = TraceSpan::from(llm_span);

            assert!(trace_span.events.is_some());
            let events = trace_span.events.unwrap();
            let events_array = events.as_array().unwrap();
            assert_eq!(events_array.len(), 1);
        }

        #[test]
        fn test_from_llm_span_custom_attributes() {
            let mut llm_span = create_test_llm_span();
            llm_span
                .attributes
                .insert("custom.key".to_string(), serde_json::json!("custom_value"));

            let trace_span = TraceSpan::from(llm_span);
            let attrs = trace_span.attributes.as_object().unwrap();

            assert_eq!(
                attrs.get("custom.key").unwrap().as_str().unwrap(),
                "custom_value"
            );
        }

        #[test]
        fn test_trace_new() {
            let now = Utc::now();
            let trace = Trace::new("trace_abc123".to_string(), "test-service".to_string(), now);

            assert_eq!(trace.trace_id, "trace_abc123");
            assert_eq!(trace.service_name, "test-service");
            assert_eq!(trace.start_time, now);
            assert_eq!(trace.status, "unset");
            assert_eq!(trace.span_count, 0);
            assert!(trace.end_time.is_none());
        }

        #[test]
        fn test_trace_span_new() {
            let trace_id = Uuid::new_v4();
            let now = Utc::now();

            let span = TraceSpan::new(
                trace_id,
                "span_123".to_string(),
                "test-operation".to_string(),
                "test-service".to_string(),
                now,
            );

            assert_eq!(span.trace_id, trace_id);
            assert_eq!(span.span_id, "span_123");
            assert_eq!(span.name, "test-operation");
            assert_eq!(span.service_name, "test-service");
            assert_eq!(span.start_time, now);
            assert_eq!(span.kind, "internal");
            assert_eq!(span.status, "unset");
        }
    }

    // TODO: Add integration tests with test database for:
    // - write_span_from_llm() with real database
    // - ensure_trace() concurrent writes
    // - ensure_trace() race condition handling
    // - Full end-to-end test with LlmSpan -> DB -> query
}
