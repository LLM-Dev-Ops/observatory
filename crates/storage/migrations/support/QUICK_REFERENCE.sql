-- ============================================================================
-- Quick Reference SQL Commands for LLM Observatory
-- ============================================================================
-- Purpose: Common queries and operations for managing the storage layer
-- Date: 2025-11-05

-- ============================================================================
-- HYPERTABLE MANAGEMENT
-- ============================================================================

-- View all hypertables
SELECT * FROM timescaledb_information.hypertables;

-- View chunks for a specific hypertable
SELECT
    chunk_name,
    range_start,
    range_end,
    is_compressed,
    pg_size_pretty(total_bytes) AS size
FROM timescaledb_information.chunks
WHERE hypertable_name = 'llm_traces'
ORDER BY range_start DESC
LIMIT 20;

-- View chunk statistics (summary)
SELECT
    hypertable_name,
    COUNT(*) AS total_chunks,
    SUM(CASE WHEN is_compressed THEN 1 ELSE 0 END) AS compressed_chunks,
    pg_size_pretty(SUM(total_bytes)) AS total_size,
    pg_size_pretty(SUM(CASE WHEN is_compressed THEN total_bytes ELSE 0 END)) AS compressed_size
FROM timescaledb_information.chunks
GROUP BY hypertable_name;

-- ============================================================================
-- COMPRESSION MANAGEMENT
-- ============================================================================

-- View compression settings
SELECT
    hypertable_name,
    attname AS column_name,
    segmentby_column_index,
    orderby_column_index
FROM timescaledb_information.compression_settings
ORDER BY hypertable_name, segmentby_column_index, orderby_column_index;

-- View compression statistics
SELECT
    hypertable_name,
    total_chunks,
    number_compressed_chunks,
    pg_size_pretty(uncompressed_heap_size) AS uncompressed_size,
    pg_size_pretty(compressed_heap_size) AS compressed_size,
    ROUND((1 - compressed_heap_size::NUMERIC / NULLIF(uncompressed_heap_size, 0)) * 100, 2) AS compression_pct
FROM timescaledb_information.hypertables
WHERE hypertable_name LIKE 'llm_%'
ORDER BY hypertable_name;

-- Manually compress old chunks (if needed)
SELECT compress_chunk(c)
FROM show_chunks('llm_traces', older_than => INTERVAL '7 days') c
WHERE NOT is_compressed;

-- Decompress a specific chunk (for updates)
SELECT decompress_chunk('_timescaledb_internal._hyper_1_1_chunk');

-- ============================================================================
-- RETENTION & CLEANUP
-- ============================================================================

-- View retention policies
SELECT
    hypertable_name,
    format_interval(config::json->>'drop_after') AS retention_period,
    schedule_interval
FROM timescaledb_information.jobs
WHERE proc_name = 'policy_retention'
ORDER BY hypertable_name;

-- Manually drop old chunks
SELECT drop_chunks('llm_traces', older_than => INTERVAL '90 days');

-- View chunks scheduled for deletion
SELECT
    chunk_name,
    range_start,
    range_end,
    pg_size_pretty(total_bytes) AS size
FROM timescaledb_information.chunks
WHERE hypertable_name = 'llm_traces'
    AND range_end < NOW() - INTERVAL '90 days'
ORDER BY range_start DESC;

-- ============================================================================
-- CONTINUOUS AGGREGATES
-- ============================================================================

-- View all continuous aggregates
SELECT
    view_name,
    format_interval(refresh_lag) AS refresh_lag,
    format_interval(refresh_interval) AS refresh_interval,
    materialized_only,
    compression_enabled
FROM timescaledb_information.continuous_aggregates
ORDER BY view_name;

-- Check materialization status
SELECT
    view_name,
    completed_threshold,
    invalidation_threshold,
    materialization_hypertable_name
FROM timescaledb_information.continuous_aggregate_stats
ORDER BY view_name;

-- Manually refresh a continuous aggregate
CALL refresh_continuous_aggregate('llm_metrics_1min', NULL, NULL);
CALL refresh_continuous_aggregate('llm_metrics_1hour', NULL, NULL);
CALL refresh_continuous_aggregate('llm_metrics_1day', NULL, NULL);
CALL refresh_continuous_aggregate('cost_analysis_hourly', NULL, NULL);

-- Refresh specific time range
CALL refresh_continuous_aggregate('llm_metrics_1hour',
    NOW() - INTERVAL '7 days',
    NOW() - INTERVAL '1 hour');

-- ============================================================================
-- JOB MONITORING
-- ============================================================================

-- View all background jobs
SELECT
    job_id,
    hypertable_name,
    proc_name,
    schedule_interval,
    config
FROM timescaledb_information.jobs
ORDER BY job_id;

-- View job execution statistics
SELECT
    job_id,
    hypertable_name,
    last_run_started_at,
    last_successful_finish,
    next_start,
    total_runs,
    total_successes,
    total_failures,
    last_run_status
FROM timescaledb_information.job_stats
ORDER BY last_run_started_at DESC;

-- View failed jobs
SELECT
    job_id,
    hypertable_name,
    last_run_started_at,
    last_run_status,
    total_failures
FROM timescaledb_information.job_stats
WHERE last_run_status = 'failed'
ORDER BY last_run_started_at DESC;

-- ============================================================================
-- INDEX MANAGEMENT
-- ============================================================================

-- View all indexes and their sizes
SELECT
    schemaname,
    tablename,
    indexname,
    pg_size_pretty(pg_relation_size(indexname::regclass)) AS index_size,
    idx_scan AS times_used,
    idx_tup_read AS tuples_read,
    idx_tup_fetch AS tuples_fetched
FROM pg_indexes
JOIN pg_stat_user_indexes USING (schemaname, tablename, indexname)
WHERE schemaname = 'public'
    AND tablename IN ('llm_traces', 'llm_metrics', 'llm_logs')
ORDER BY pg_relation_size(indexname::regclass) DESC;

-- Find unused indexes
SELECT
    schemaname,
    tablename,
    indexname,
    pg_size_pretty(pg_relation_size(indexname::regclass)) AS index_size,
    idx_scan AS times_used
FROM pg_stat_user_indexes
WHERE schemaname = 'public'
    AND tablename IN ('llm_traces', 'llm_metrics', 'llm_logs')
    AND idx_scan = 0
ORDER BY pg_relation_size(indexrelname::regclass) DESC;

-- ============================================================================
-- QUERY PERFORMANCE
-- ============================================================================

-- View slow queries (requires pg_stat_statements)
SELECT
    SUBSTRING(query, 1, 100) AS short_query,
    calls,
    ROUND(mean_exec_time::numeric, 2) AS avg_time_ms,
    ROUND(total_exec_time::numeric, 2) AS total_time_ms,
    ROUND((100 * total_exec_time / SUM(total_exec_time) OVER ())::numeric, 2) AS pct_total
FROM pg_stat_statements
WHERE query NOT LIKE '%pg_stat_statements%'
ORDER BY mean_exec_time DESC
LIMIT 20;

-- Reset query statistics
SELECT pg_stat_statements_reset();

-- Explain a query (check if indexes are used)
EXPLAIN (ANALYZE, BUFFERS)
SELECT * FROM llm_traces
WHERE provider = 'openai'
    AND ts > NOW() - INTERVAL '1 hour'
ORDER BY ts DESC
LIMIT 100;

-- ============================================================================
-- DATABASE SIZE & STATISTICS
-- ============================================================================

-- Database size
SELECT
    pg_database.datname,
    pg_size_pretty(pg_database_size(pg_database.datname)) AS size
FROM pg_database
WHERE datname = current_database();

-- Table sizes (including indexes and TOAST)
SELECT
    schemaname,
    tablename,
    pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) AS total_size,
    pg_size_pretty(pg_relation_size(schemaname||'.'||tablename)) AS table_size,
    pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename) -
                   pg_relation_size(schemaname||'.'||tablename)) AS indexes_size
FROM pg_tables
WHERE schemaname = 'public'
ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC;

-- Row counts (approximate, fast)
SELECT
    schemaname,
    relname AS tablename,
    n_live_tup AS estimated_rows
FROM pg_stat_user_tables
WHERE schemaname = 'public'
ORDER BY n_live_tup DESC;

-- Row counts (exact, slow for large tables)
SELECT
    'llm_traces' AS table_name,
    COUNT(*) AS exact_count
FROM llm_traces
UNION ALL
SELECT 'llm_metrics', COUNT(*) FROM llm_metrics
UNION ALL
SELECT 'llm_logs', COUNT(*) FROM llm_logs;

-- ============================================================================
-- CONNECTION MANAGEMENT
-- ============================================================================

-- View active connections
SELECT
    datname,
    usename,
    application_name,
    client_addr,
    state,
    query_start,
    state_change,
    SUBSTRING(query, 1, 100) AS current_query
FROM pg_stat_activity
WHERE datname = current_database()
ORDER BY query_start DESC;

-- Kill a specific connection
SELECT pg_terminate_backend(pid)
FROM pg_stat_activity
WHERE pid = <pid>;

-- Kill all idle connections
SELECT pg_terminate_backend(pid)
FROM pg_stat_activity
WHERE datname = current_database()
    AND state = 'idle'
    AND query_start < NOW() - INTERVAL '1 hour';

-- ============================================================================
-- VACUUM & MAINTENANCE
-- ============================================================================

-- View autovacuum statistics
SELECT
    schemaname,
    relname,
    last_vacuum,
    last_autovacuum,
    vacuum_count,
    autovacuum_count,
    n_dead_tup AS dead_tuples
FROM pg_stat_user_tables
WHERE schemaname = 'public'
ORDER BY n_dead_tup DESC;

-- Manually vacuum
VACUUM ANALYZE llm_traces;
VACUUM ANALYZE llm_metrics;
VACUUM ANALYZE llm_logs;

-- ============================================================================
-- DATA QUALITY CHECKS
-- ============================================================================

-- Check for NULL values in important columns
SELECT
    'llm_traces' AS table_name,
    COUNT(*) FILTER (WHERE provider IS NULL) AS null_providers,
    COUNT(*) FILTER (WHERE model IS NULL) AS null_models,
    COUNT(*) FILTER (WHERE status_code IS NULL) AS null_status,
    COUNT(*) FILTER (WHERE total_cost_usd IS NULL) AS null_costs
FROM llm_traces;

-- Check data freshness
SELECT
    'llm_traces' AS table_name,
    MAX(ts) AS latest_timestamp,
    NOW() - MAX(ts) AS time_since_latest
FROM llm_traces
UNION ALL
SELECT
    'llm_metrics' AS table_name,
    MAX(ts),
    NOW() - MAX(ts)
FROM llm_metrics
UNION ALL
SELECT
    'llm_logs' AS table_name,
    MAX(ts),
    NOW() - MAX(ts)
FROM llm_logs;

-- Check for duplicate traces
SELECT
    trace_id,
    span_id,
    COUNT(*) AS duplicate_count
FROM llm_traces
GROUP BY trace_id, span_id
HAVING COUNT(*) > 1;

-- ============================================================================
-- SAMPLE QUERIES (From Section 7 of the Plan)
-- ============================================================================

-- Recent traces by provider
SELECT
    ts,
    trace_id,
    model,
    duration_ms,
    total_cost_usd,
    status_code
FROM llm_traces
WHERE provider = 'openai'
    AND ts > NOW() - INTERVAL '1 hour'
ORDER BY ts DESC
LIMIT 100;

-- Top 10 most expensive requests (last 24 hours)
SELECT
    ts,
    trace_id,
    provider,
    model,
    total_cost_usd,
    total_tokens,
    duration_ms
FROM llm_traces
WHERE ts > NOW() - INTERVAL '24 hours'
    AND total_cost_usd IS NOT NULL
ORDER BY total_cost_usd DESC
LIMIT 10;

-- Error rate by model (last hour)
SELECT
    provider,
    model,
    COUNT(*) AS total_requests,
    SUM(CASE WHEN status_code = 'ERROR' THEN 1 ELSE 0 END) AS errors,
    ROUND((SUM(CASE WHEN status_code = 'ERROR' THEN 1 ELSE 0 END)::NUMERIC / COUNT(*) * 100), 2) AS error_rate_pct
FROM llm_traces
WHERE ts > NOW() - INTERVAL '1 hour'
GROUP BY provider, model
ORDER BY error_rate_pct DESC;

-- Cost by user (last 7 days)
SELECT
    user_id,
    COUNT(*) AS request_count,
    SUM(total_cost_usd) AS total_cost,
    AVG(total_cost_usd) AS avg_cost_per_request,
    SUM(total_tokens) AS total_tokens
FROM llm_traces
WHERE ts > NOW() - INTERVAL '7 days'
    AND user_id IS NOT NULL
GROUP BY user_id
ORDER BY total_cost DESC
LIMIT 20;

-- Latency percentiles (from 1-hour aggregate)
SELECT
    bucket,
    provider,
    model,
    ROUND(p50_duration_ms::numeric, 2) AS p50_ms,
    ROUND(p95_duration_ms::numeric, 2) AS p95_ms,
    ROUND(p99_duration_ms::numeric, 2) AS p99_ms
FROM llm_metrics_1hour
WHERE bucket > NOW() - INTERVAL '24 hours'
ORDER BY bucket DESC, provider, model;

-- ============================================================================
-- BACKUP & RESTORE
-- ============================================================================

-- Backup database (run from shell)
-- pg_dump -Fc llm_observatory > backup_$(date +%Y%m%d_%H%M%S).dump

-- Restore database (run from shell)
-- pg_restore -d llm_observatory backup_20251105_120000.dump

-- Backup specific tables
-- pg_dump -t llm_traces -t llm_metrics -t llm_logs llm_observatory > traces_backup.sql

-- ============================================================================
-- HEALTH CHECK
-- ============================================================================

-- Quick health check
SELECT
    'Database' AS component,
    current_database() AS name,
    pg_size_pretty(pg_database_size(current_database())) AS size,
    'UP' AS status
UNION ALL
SELECT
    'TimescaleDB',
    extversion,
    NULL,
    'ENABLED'
FROM pg_extension
WHERE extname = 'timescaledb'
UNION ALL
SELECT
    'Hypertables',
    COUNT(*)::text,
    NULL,
    CASE WHEN COUNT(*) = 3 THEN 'OK' ELSE 'WARN' END
FROM timescaledb_information.hypertables
WHERE hypertable_name IN ('llm_traces', 'llm_metrics', 'llm_logs')
UNION ALL
SELECT
    'Continuous Aggregates',
    COUNT(*)::text,
    NULL,
    CASE WHEN COUNT(*) = 4 THEN 'OK' ELSE 'WARN' END
FROM timescaledb_information.continuous_aggregates;
