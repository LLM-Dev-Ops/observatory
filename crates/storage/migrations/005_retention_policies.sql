-- Migration: 005_retention_policies.sql
-- Description: Configure compression and retention policies for data lifecycle management
-- Date: 2025-11-05
-- Author: LLM Observatory Core Team
-- Sections: 6.2 (Compression), 6.3 (Retention)

BEGIN;

-- ============================================================================
-- Section 6.2: Compression Policies
-- ============================================================================
-- Compression reduces storage by 85-95% while maintaining query performance
-- Data older than the threshold will be automatically compressed

-- ============================================================================
-- Enable Compression on llm_traces
-- ============================================================================
-- Segment by: provider, model (high cardinality columns)
-- Order by: ts DESC (time-series ordering for optimal compression)

ALTER TABLE llm_traces SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'provider, model',
    timescaledb.compress_orderby = 'ts DESC'
);

-- Compress chunks older than 7 days (keep hot data uncompressed)
-- Expected compression ratio: 10:1 to 20:1 (90-95% reduction)
SELECT add_compression_policy('llm_traces', INTERVAL '7 days');

COMMENT ON TABLE llm_traces IS 'LLM traces - Hypertable with compression (7-day hot tier, 90%+ compression ratio)';

-- ============================================================================
-- Enable Compression on llm_metrics
-- ============================================================================
-- Segment by: provider, model, metric_name (high cardinality)
-- Order by: ts DESC

ALTER TABLE llm_metrics SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'provider, model, metric_name',
    timescaledb.compress_orderby = 'ts DESC'
);

-- Compress chunks older than 7 days
-- Expected compression ratio: 20:1 to 50:1 (95-98% reduction)
SELECT add_compression_policy('llm_metrics', INTERVAL '7 days');

COMMENT ON TABLE llm_metrics IS 'LLM metrics - Hypertable with compression (7-day hot tier, 95%+ compression ratio)';

-- ============================================================================
-- Enable Compression on llm_logs
-- ============================================================================
-- Segment by: provider, log_level
-- Order by: ts DESC

ALTER TABLE llm_logs SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'provider, log_level',
    timescaledb.compress_orderby = 'ts DESC'
);

-- Compress chunks older than 7 days
-- Expected compression ratio: 5:1 to 10:1 (80-90% reduction)
SELECT add_compression_policy('llm_logs', INTERVAL '7 days');

COMMENT ON TABLE llm_logs IS 'LLM logs - Hypertable with compression (7-day hot tier, 80%+ compression ratio)';

-- ============================================================================
-- Section 6.3: Retention Policies
-- ============================================================================
-- Automatically drop old chunks to manage storage costs
-- Retention strategy from Section 6.1:
-- - Traces: 90 days total (7d hot + 83d compressed)
-- - Metrics (1-min): 37 days
-- - Metrics (1-hour): 210 days
-- - Metrics (1-day): 1095 days (3 years)
-- - Logs: 37 days

-- ============================================================================
-- Retention for llm_traces
-- ============================================================================
-- Keep 90 days of raw trace data (7d hot + 83d compressed)
SELECT add_retention_policy('llm_traces', INTERVAL '90 days');

-- ============================================================================
-- Retention for llm_metrics
-- ============================================================================
-- Keep 37 days of raw metrics (shorter because aggregates exist)
SELECT add_retention_policy('llm_metrics', INTERVAL '37 days');

-- ============================================================================
-- Retention for llm_logs
-- ============================================================================
-- Keep 37 days of logs
SELECT add_retention_policy('llm_logs', INTERVAL '37 days');

-- ============================================================================
-- Retention for Continuous Aggregates
-- ============================================================================
-- These keep longer than raw data for historical analysis

-- 1-minute rollups: 37 days
SELECT add_retention_policy('llm_metrics_1min', INTERVAL '37 days');

-- 1-hour rollups: 210 days (7 months)
SELECT add_retention_policy('llm_metrics_1hour', INTERVAL '210 days');

-- 1-day rollups: 1095 days (3 years)
SELECT add_retention_policy('llm_metrics_1day', INTERVAL '1095 days');

-- Error summary created by migration 004: 210 days (7 months).
-- Cost totals are already included in the metrics rollups; there is no separate
-- cost_analysis_hourly relation in this migration chain.
SELECT add_retention_policy('llm_error_summary', INTERVAL '210 days');

COMMIT;

-- ============================================================================
-- Verification and Monitoring Queries
-- ============================================================================
-- Run these queries to verify policies and monitor compression:
--
-- -- View all compression policies
-- SELECT
--     hypertable_name,
--     compress_after
-- FROM timescaledb_information.compression_settings;
--
-- -- View all retention policies
-- SELECT
--     hypertable_name,
--     older_than AS retention_period
-- FROM timescaledb_information.jobs
-- WHERE proc_name = 'policy_retention';
--
-- -- Check compression statistics
-- SELECT
--     hypertable_name,
--     total_chunks,
--     number_compressed_chunks,
--     pg_size_pretty(uncompressed_heap_size) AS uncompressed_size,
--     pg_size_pretty(compressed_heap_size) AS compressed_size,
--     ROUND(
--         (1 - compressed_heap_size::NUMERIC / NULLIF(uncompressed_heap_size, 0)) * 100,
--         2
--     ) AS compression_ratio_pct
-- FROM timescaledb_information.hypertables
-- WHERE hypertable_name LIKE 'llm_%'
-- ORDER BY hypertable_name;
--
-- -- View policy execution history
-- SELECT
--     job_id,
--     hypertable_name,
--     last_run_started_at,
--     last_successful_finish,
--     next_start,
--     total_runs,
--     total_successes,
--     total_failures
-- FROM timescaledb_information.job_stats
-- ORDER BY last_run_started_at DESC;
--
-- -- Manually compress a specific chunk (if needed)
-- -- SELECT compress_chunk(c)
-- -- FROM show_chunks('llm_traces', older_than => INTERVAL '7 days') c;
--
-- -- Manually drop old chunks (if needed, normally automatic)
-- -- SELECT drop_chunks('llm_traces', INTERVAL '90 days');
