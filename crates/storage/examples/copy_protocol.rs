//! Example demonstrating PostgreSQL COPY protocol usage.
//!
//! This example shows how to use the high-performance COPY protocol
//! for bulk insertion of traces, spans, and logs.
//!
//! ## Running
//!
//! ```bash
//! export DATABASE_URL="postgres://postgres:password@localhost/llm_observatory"
//! cargo run --example copy_protocol
//! ```

use chrono::Utc;
use llm_observatory_storage::{
    models::{LogRecord, Trace, TraceSpan},
    writers::CopyWriter,
    StorageConfig, StoragePool,
};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("=== PostgreSQL COPY Protocol Example ===\n");

    // Load configuration from environment
    let config = StorageConfig::from_env()?;
    println!(
        "Connecting to PostgreSQL at {}:{}",
        config.postgres.host, config.postgres.port
    );

    // Create storage pool
    let pool = StoragePool::new(config).await?;
    println!("Storage pool created successfully\n");

    // Get a tokio-postgres client for COPY operations
    let (client, _handle) = pool.get_tokio_postgres_client().await?;
    println!("Got tokio-postgres client for COPY\n");

    // Example 1: Insert traces using COPY
    println!("--- Example 1: Bulk Insert Traces ---");
    let traces = generate_sample_traces(1000);
    println!("Generated {} sample traces", traces.len());

    let start = std::time::Instant::now();
    let rows = CopyWriter::write_traces(&client, traces).await?;
    let elapsed = start.elapsed();

    println!("Inserted {} traces in {:?}", rows, elapsed);
    println!(
        "Throughput: {:.0} traces/sec\n",
        rows as f64 / elapsed.as_secs_f64()
    );

    // Example 2: Insert spans using COPY
    println!("--- Example 2: Bulk Insert Spans ---");
    let spans = generate_sample_spans(5000);
    println!("Generated {} sample spans", spans.len());

    let start = std::time::Instant::now();
    let rows = CopyWriter::write_spans(&client, spans).await?;
    let elapsed = start.elapsed();

    println!("Inserted {} spans in {:?}", rows, elapsed);
    println!(
        "Throughput: {:.0} spans/sec\n",
        rows as f64 / elapsed.as_secs_f64()
    );

    // Example 3: Insert logs using COPY
    println!("--- Example 3: Bulk Insert Logs ---");
    let logs = generate_sample_logs(10000);
    println!("Generated {} sample logs", logs.len());

    let start = std::time::Instant::now();
    let rows = CopyWriter::write_logs(&client, logs).await?;
    let elapsed = start.elapsed();

    println!("Inserted {} logs in {:?}", rows, elapsed);
    println!(
        "Throughput: {:.0} logs/sec\n",
        rows as f64 / elapsed.as_secs_f64()
    );

    // Example 4: Comparison with INSERT
    println!("--- Example 4: Performance Comparison ---");
    compare_insert_vs_copy(&pool, &client).await?;

    println!("\nExample completed successfully!");

    Ok(())
}

/// Generate sample traces for testing
fn generate_sample_traces(count: usize) -> Vec<Trace> {
    (0..count)
        .map(|i| {
            let now = Utc::now();
            Trace {
                id: Uuid::new_v4(),
                trace_id: format!("trace_{}", i),
                service_name: "example-service".to_string(),
                start_time: now,
                end_time: Some(now + chrono::Duration::milliseconds(150)),
                duration_us: Some(150_000),
                status: if i % 10 == 0 { "error" } else { "ok" }.to_string(),
                status_message: if i % 10 == 0 {
                    Some("Example error".to_string())
                } else {
                    None
                },
                root_span_name: Some(format!("request_{}", i % 5)),
                attributes: serde_json::json!({
                    "http.method": "POST",
                    "http.route": "/api/v1/predict",
                    "request.id": i,
                }),
                resource_attributes: serde_json::json!({
                    "service.name": "example-service",
                    "service.version": "1.0.0",
                }),
                span_count: (i % 10) as i32 + 1,
                created_at: now,
                updated_at: now,
            }
        })
        .collect()
}

/// Generate sample spans for testing
fn generate_sample_spans(count: usize) -> Vec<TraceSpan> {
    (0..count)
        .map(|i| {
            let now = Utc::now();
            TraceSpan {
                id: Uuid::new_v4(),
                trace_id: Uuid::new_v4(),
                span_id: format!("span_{}", i),
                parent_span_id: if i % 3 == 0 {
                    Some(format!("parent_{}", i / 3))
                } else {
                    None
                },
                name: format!("llm.{}", ["openai", "anthropic", "cohere"][i % 3]),
                kind: "internal".to_string(),
                service_name: "example-service".to_string(),
                start_time: now,
                end_time: Some(now + chrono::Duration::milliseconds(80)),
                duration_us: Some(80_000),
                status: "ok".to_string(),
                status_message: None,
                attributes: serde_json::json!({
                    "llm.provider": ["openai", "anthropic", "cohere"][i % 3],
                    "llm.model": "gpt-4",
                    "llm.usage.prompt_tokens": 100,
                    "llm.usage.completion_tokens": 50,
                }),
                events: Some(serde_json::json!([
                    {
                        "name": "llm.request.start",
                        "timestamp": now.to_rfc3339(),
                    }
                ])),
                links: None,
                created_at: now,
            }
        })
        .collect()
}

/// Generate sample logs for testing
fn generate_sample_logs(count: usize) -> Vec<LogRecord> {
    (0..count)
        .map(|i| {
            let now = Utc::now();
            let severity = match i % 5 {
                0 => (5, "DEBUG"),
                1 => (9, "INFO"),
                2 => (13, "WARN"),
                3 => (17, "ERROR"),
                _ => (9, "INFO"),
            };

            LogRecord {
                id: Uuid::new_v4(),
                timestamp: now,
                observed_timestamp: now,
                severity_number: severity.0,
                severity_text: severity.1.to_string(),
                body: format!("Example log message {} with severity {}", i, severity.1),
                service_name: "example-service".to_string(),
                trace_id: if i % 2 == 0 {
                    Some(format!("trace_{}", i / 2))
                } else {
                    None
                },
                span_id: if i % 2 == 0 {
                    Some(format!("span_{}", i / 2))
                } else {
                    None
                },
                trace_flags: None,
                attributes: serde_json::json!({
                    "log.index": i,
                    "log.category": ["request", "response", "error", "metric"][i % 4],
                }),
                resource_attributes: serde_json::json!({
                    "service.name": "example-service",
                }),
                scope_name: Some("example".to_string()),
                scope_version: Some("1.0.0".to_string()),
                scope_attributes: None,
                created_at: now,
            }
        })
        .collect()
}

/// Compare INSERT vs COPY performance
async fn compare_insert_vs_copy(
    pool: &StoragePool,
    client: &tokio_postgres::Client,
) -> Result<(), Box<dyn std::error::Error>> {
    const BATCH_SIZE: usize = 1000;

    // Test INSERT performance
    println!("Testing INSERT with QueryBuilder ({} rows)...", BATCH_SIZE);
    let traces = generate_sample_traces(BATCH_SIZE);

    let start = std::time::Instant::now();
    let mut query_builder = sqlx::QueryBuilder::new(
        "INSERT INTO traces (id, trace_id, service_name, start_time, end_time, duration_us, \
         status, status_message, root_span_name, attributes, resource_attributes, span_count, \
         created_at, updated_at) ",
    );

    query_builder.push_values(&traces, |mut b, trace| {
        b.push_bind(trace.id)
            .push_bind(&trace.trace_id)
            .push_bind(&trace.service_name)
            .push_bind(trace.start_time)
            .push_bind(trace.end_time)
            .push_bind(trace.duration_us)
            .push_bind(&trace.status)
            .push_bind(&trace.status_message)
            .push_bind(&trace.root_span_name)
            .push_bind(&trace.attributes)
            .push_bind(&trace.resource_attributes)
            .push_bind(trace.span_count)
            .push_bind(trace.created_at)
            .push_bind(trace.updated_at);
    });

    query_builder.build().execute(pool.postgres()).await?;
    let insert_elapsed = start.elapsed();
    let insert_throughput = BATCH_SIZE as f64 / insert_elapsed.as_secs_f64();

    println!(
        "INSERT: {:?} ({:.0} rows/sec)",
        insert_elapsed, insert_throughput
    );

    // Test COPY performance
    println!("Testing COPY protocol ({} rows)...", BATCH_SIZE);
    let traces = generate_sample_traces(BATCH_SIZE);

    let start = std::time::Instant::now();
    CopyWriter::write_traces(client, traces).await?;
    let copy_elapsed = start.elapsed();
    let copy_throughput = BATCH_SIZE as f64 / copy_elapsed.as_secs_f64();

    println!("COPY: {:?} ({:.0} rows/sec)", copy_elapsed, copy_throughput);

    // Calculate speedup
    let speedup = copy_throughput / insert_throughput;
    println!("\nSpeedup: {:.1}x faster with COPY", speedup);

    if speedup > 5.0 {
        println!("✓ COPY is significantly faster!");
    } else if speedup > 2.0 {
        println!("✓ COPY is moderately faster");
    } else {
        println!("⚠ COPY speedup is minimal (batch may be too small)");
    }

    Ok(())
}
