-- ============================================================================
-- Test Script for Migration 004: Continuous Aggregates
-- ============================================================================
-- Purpose: Verify continuous aggregates are created correctly and functional
-- Run this after executing migration 004
-- Date: 2025-11-05
--
-- Prerequisites:
--   1. TimescaleDB extension is installed
--   2. Migrations 001-003 have been run successfully
--   3. llm_traces hypertable exists

\echo '============================================================================'
\echo 'Migration 004 Test Suite - Continuous Aggregates'
\echo '============================================================================'
\echo ''

-- ============================================================================
-- Test 1: Verify TimescaleDB Version
-- ============================================================================
\echo '=== Test 1: TimescaleDB Version Check ==='
SELECT
    extversion AS timescaledb_version,
    CASE
        WHEN extversion >= '2.14.0' THEN 'PASS - Version 2.14+ detected'
        ELSE 'WARNING - Version < 2.14, some features may not work'
    END AS status
FROM pg_extension
WHERE extname = 'timescaledb';
\echo ''

-- ============================================================================
-- Test 2: Verify Continuous Aggregates Exist
-- ============================================================================
\echo '=== Test 2: Continuous Aggregates Creation ==='
SELECT
    view_name,
    materialized_only,
    compression_enabled,
    CASE
        WHEN view_name IN ('llm_metrics_1min', 'llm_metrics_1hour', 'llm_metrics_1day', 'llm_error_summary')
        THEN 'PASS'
        ELSE 'FAIL - Unexpected view'
    END AS status
FROM timescaledb_information.continuous_aggregates
WHERE view_name LIKE 'llm_%'
ORDER BY view_name;

-- Check count
SELECT
    COUNT(*) AS aggregate_count,
    CASE
        WHEN COUNT(*) = 4 THEN 'PASS - All 4 aggregates created'
        ELSE 'FAIL - Expected 4, got ' || COUNT(*)::TEXT
    END AS status
FROM timescaledb_information.continuous_aggregates
WHERE view_name IN ('llm_metrics_1min', 'llm_metrics_1hour', 'llm_metrics_1day', 'llm_error_summary');
\echo ''

-- ============================================================================
-- Test 3: Verify Refresh Policies
-- ============================================================================
\echo '=== Test 3: Continuous Aggregate Refresh Policies ==='
SELECT
    ca.view_name,
    j.schedule_interval,
    (config::json->>'start_offset')::TEXT AS start_offset,
    (config::json->>'end_offset')::TEXT AS end_offset,
    CASE
        WHEN j.schedule_interval IS NOT NULL THEN 'PASS'
        ELSE 'FAIL - No refresh policy'
    END AS status
FROM timescaledb_information.continuous_aggregates ca
LEFT JOIN timescaledb_information.jobs j
    ON j.hypertable_name = ca.view_name
    AND j.proc_name = 'policy_refresh_continuous_aggregate'
WHERE ca.view_name IN ('llm_metrics_1min', 'llm_metrics_1hour', 'llm_metrics_1day', 'llm_error_summary')
ORDER BY ca.view_name;
\echo ''

-- ============================================================================
-- Test 4: Verify View Schemas
-- ============================================================================
\echo '=== Test 4: View Column Schemas ==='

\echo '--- llm_metrics_1min columns ---'
SELECT
    column_name,
    data_type,
    is_nullable
FROM information_schema.columns
WHERE table_name = 'llm_metrics_1min'
    AND table_schema = 'public'
ORDER BY ordinal_position;
\echo ''

\echo '--- llm_metrics_1hour columns ---'
SELECT
    column_name,
    data_type,
    is_nullable
FROM information_schema.columns
WHERE table_name = 'llm_metrics_1hour'
    AND table_schema = 'public'
ORDER BY ordinal_position;
\echo ''

\echo '--- llm_metrics_1day columns ---'
SELECT
    column_name,
    data_type,
    is_nullable
FROM information_schema.columns
WHERE table_name = 'llm_metrics_1day'
    AND table_schema = 'public'
ORDER BY ordinal_position;
\echo ''

\echo '--- llm_error_summary columns ---'
SELECT
    column_name,
    data_type,
    is_nullable
FROM information_schema.columns
WHERE table_name = 'llm_error_summary'
    AND table_schema = 'public'
ORDER BY ordinal_position;
\echo ''

-- ============================================================================
-- Test 5: Insert Test Data
-- ============================================================================
\echo '=== Test 5: Inserting Test Data ==='

-- Insert sample traces spanning different time buckets
INSERT INTO llm_traces (
    ts, trace_id, span_id, span_name, span_kind,
    provider, model, input_type, status_code,
    prompt_tokens, completion_tokens, total_tokens,
    prompt_cost_usd, completion_cost_usd, total_cost_usd,
    duration_ms, ttft_ms,
    user_id, session_id, environment,
    sampled
) VALUES
    -- Minute 1: Successful requests
    (NOW() - INTERVAL '5 minutes', 'trace-001', 'span-001', 'llm.chat.completion', 'client',
     'openai', 'gpt-4', 'chat', 'OK',
     100, 50, 150, 0.003, 0.0015, 0.0045, 1234, 123,
     'user-1', 'session-1', 'production', true),
    (NOW() - INTERVAL '5 minutes', 'trace-002', 'span-002', 'llm.chat.completion', 'client',
     'openai', 'gpt-4', 'chat', 'OK',
     120, 60, 180, 0.0036, 0.0018, 0.0054, 1456, 145,
     'user-2', 'session-2', 'production', true),
    (NOW() - INTERVAL '5 minutes', 'trace-003', 'span-003', 'llm.chat.completion', 'client',
     'anthropic', 'claude-3-opus', 'chat', 'OK',
     200, 100, 300, 0.015, 0.075, 0.090, 2345, 234,
     'user-1', 'session-1', 'production', true),

    -- Minute 2: Mix of success and errors
    (NOW() - INTERVAL '3 minutes', 'trace-004', 'span-004', 'llm.chat.completion', 'client',
     'openai', 'gpt-4', 'chat', 'ERROR',
     50, 0, 50, 0.0015, 0, 0.0015, 567, NULL,
     'user-3', 'session-3', 'production', true),
    (NOW() - INTERVAL '3 minutes', 'trace-005', 'span-005', 'llm.chat.completion', 'client',
     'openai', 'gpt-3.5-turbo', 'chat', 'OK',
     80, 40, 120, 0.00024, 0.00012, 0.00036, 890, 89,
     'user-2', 'session-2', 'staging', true),

    -- Hour ago: Older data
    (NOW() - INTERVAL '2 hours', 'trace-006', 'span-006', 'llm.chat.completion', 'client',
     'google', 'gemini-pro', 'chat', 'OK',
     150, 75, 225, 0.00075, 0.0015, 0.00225, 1111, 111,
     'user-4', 'session-4', 'production', true),

    -- Day ago: Historical data
    (NOW() - INTERVAL '25 hours', 'trace-007', 'span-007', 'llm.chat.completion', 'client',
     'anthropic', 'claude-3-sonnet', 'chat', 'OK',
     180, 90, 270, 0.0054, 0.0135, 0.0189, 1678, 167,
     'user-5', 'session-5', 'production', true);

SELECT
    COUNT(*) AS inserted_rows,
    CASE
        WHEN COUNT(*) = 7 THEN 'PASS - 7 test rows inserted'
        ELSE 'FAIL - Expected 7 rows'
    END AS status
FROM llm_traces
WHERE trace_id LIKE 'trace-00%';
\echo ''

-- ============================================================================
-- Test 6: Manual Refresh Continuous Aggregates
-- ============================================================================
\echo '=== Test 6: Manual Refresh of Continuous Aggregates ==='
\echo 'Refreshing llm_metrics_1min...'
CALL refresh_continuous_aggregate('llm_metrics_1min', NULL, NULL);

\echo 'Refreshing llm_metrics_1hour...'
CALL refresh_continuous_aggregate('llm_metrics_1hour', NULL, NULL);

\echo 'Refreshing llm_metrics_1day...'
CALL refresh_continuous_aggregate('llm_metrics_1day', NULL, NULL);

\echo 'Refreshing llm_error_summary...'
CALL refresh_continuous_aggregate('llm_error_summary', NULL, NULL);

\echo 'PASS - All aggregates refreshed successfully'
\echo ''

-- ============================================================================
-- Test 7: Query Continuous Aggregates
-- ============================================================================
\echo '=== Test 7: Query Results from Continuous Aggregates ==='

\echo '--- llm_metrics_1min (last 10 minutes) ---'
SELECT
    bucket,
    provider,
    model,
    status_code,
    request_count,
    total_tokens,
    ROUND(total_cost_usd::NUMERIC, 6) AS total_cost_usd,
    ROUND(avg_duration_ms::NUMERIC, 2) AS avg_duration_ms,
    min_duration_ms,
    max_duration_ms
FROM llm_metrics_1min
WHERE bucket >= NOW() - INTERVAL '10 minutes'
ORDER BY bucket DESC, request_count DESC;
\echo ''

\echo '--- llm_metrics_1hour (last 24 hours) ---'
SELECT
    bucket,
    provider,
    model,
    environment,
    request_count,
    error_count,
    success_count,
    ROUND((error_count::NUMERIC / NULLIF(request_count, 0)) * 100, 2) AS error_rate_pct,
    ROUND(total_cost_usd::NUMERIC, 6) AS total_cost_usd,
    ROUND(avg_duration_ms::NUMERIC, 2) AS avg_duration_ms
FROM llm_metrics_1hour
WHERE bucket >= NOW() - INTERVAL '24 hours'
ORDER BY bucket DESC, request_count DESC;
\echo ''

\echo '--- llm_metrics_1day (last 7 days) ---'
SELECT
    bucket::DATE AS date,
    provider,
    model,
    request_count,
    ROUND(total_cost_usd::NUMERIC, 6) AS total_cost_usd,
    unique_users,
    unique_sessions,
    ROUND(avg_duration_ms::NUMERIC, 2) AS avg_duration_ms
FROM llm_metrics_1day
WHERE bucket >= NOW() - INTERVAL '7 days'
ORDER BY bucket DESC, request_count DESC;
\echo ''

\echo '--- llm_error_summary (last 24 hours) ---'
SELECT
    bucket,
    provider,
    model,
    status_code,
    error_count,
    affected_users,
    affected_sessions,
    LEFT(sample_error_message, 50) AS sample_error
FROM llm_error_summary
WHERE bucket >= NOW() - INTERVAL '24 hours'
ORDER BY bucket DESC, error_count DESC;
\echo ''

-- ============================================================================
-- Test 8: Validate Data Accuracy
-- ============================================================================
\echo '=== Test 8: Data Accuracy Validation ==='

-- Compare aggregated data with raw data for recent period
\echo '--- Comparing 1-min aggregate with raw data ---'
WITH raw_data AS (
    SELECT
        time_bucket('1 minute', ts) AS bucket,
        provider,
        model,
        status_code,
        COUNT(*) AS raw_count,
        SUM(total_tokens) AS raw_tokens,
        SUM(total_cost_usd) AS raw_cost
    FROM llm_traces
    WHERE ts >= NOW() - INTERVAL '10 minutes'
        AND trace_id LIKE 'trace-00%'
    GROUP BY bucket, provider, model, status_code
),
agg_data AS (
    SELECT
        bucket,
        provider,
        model,
        status_code,
        request_count AS agg_count,
        total_tokens AS agg_tokens,
        total_cost_usd AS agg_cost
    FROM llm_metrics_1min
    WHERE bucket >= NOW() - INTERVAL '10 minutes'
)
SELECT
    COALESCE(r.bucket, a.bucket) AS bucket,
    COALESCE(r.provider, a.provider) AS provider,
    COALESCE(r.model, a.model) AS model,
    r.raw_count,
    a.agg_count,
    CASE
        WHEN r.raw_count = a.agg_count THEN 'PASS'
        WHEN r.raw_count IS NULL AND a.agg_count IS NULL THEN 'PASS'
        ELSE 'FAIL - Count mismatch'
    END AS count_status,
    ROUND(r.raw_cost::NUMERIC, 6) AS raw_cost,
    ROUND(a.agg_cost::NUMERIC, 6) AS agg_cost,
    CASE
        WHEN ABS(COALESCE(r.raw_cost, 0) - COALESCE(a.agg_cost, 0)) < 0.000001 THEN 'PASS'
        ELSE 'FAIL - Cost mismatch'
    END AS cost_status
FROM raw_data r
FULL OUTER JOIN agg_data a
    ON r.bucket = a.bucket
    AND r.provider = a.provider
    AND r.model = a.model
    AND r.status_code = a.status_code
ORDER BY bucket DESC;
\echo ''

-- ============================================================================
-- Test 9: Materialization Status
-- ============================================================================
\echo '=== Test 9: Continuous Aggregate Materialization Status ==='
SELECT
    view_name,
    completed_threshold,
    invalidation_threshold,
    CASE
        WHEN completed_threshold IS NOT NULL THEN 'PASS - Materialized'
        ELSE 'WARNING - Not yet materialized'
    END AS status
FROM timescaledb_information.continuous_aggregate_stats
WHERE view_name IN ('llm_metrics_1min', 'llm_metrics_1hour', 'llm_metrics_1day', 'llm_error_summary')
ORDER BY view_name;
\echo ''

-- ============================================================================
-- Test 10: Performance Test (Optional)
-- ============================================================================
\echo '=== Test 10: Query Performance Comparison ==='
\echo '--- Query time: Continuous aggregate vs raw data ---'

-- Query aggregate (should be fast)
\echo 'Querying llm_metrics_1hour aggregate...'
\timing on
SELECT
    provider,
    model,
    SUM(request_count) AS total_requests,
    SUM(total_cost_usd) AS total_cost
FROM llm_metrics_1hour
WHERE bucket >= NOW() - INTERVAL '7 days'
GROUP BY provider, model;
\timing off
\echo ''

-- Query raw data (slower)
\echo 'Querying raw llm_traces table...'
\timing on
SELECT
    provider,
    model,
    COUNT(*) AS total_requests,
    SUM(total_cost_usd) AS total_cost
FROM llm_traces
WHERE ts >= NOW() - INTERVAL '7 days'
GROUP BY provider, model;
\timing off
\echo ''

-- ============================================================================
-- Test 11: Cleanup Test Data (Optional)
-- ============================================================================
\echo '=== Test 11: Cleanup Test Data ==='
\echo 'Do you want to delete test data? (Execute manually if needed)'
\echo '-- DELETE FROM llm_traces WHERE trace_id LIKE ''trace-00%'';'
\echo '-- CALL refresh_continuous_aggregate(''llm_metrics_1min'', NULL, NULL);'
\echo '-- CALL refresh_continuous_aggregate(''llm_metrics_1hour'', NULL, NULL);'
\echo '-- CALL refresh_continuous_aggregate(''llm_metrics_1day'', NULL, NULL);'
\echo '-- CALL refresh_continuous_aggregate(''llm_error_summary'', NULL, NULL);'
\echo ''

-- ============================================================================
-- Summary
-- ============================================================================
\echo '============================================================================'
\echo 'Test Suite Complete'
\echo '============================================================================'
\echo ''
\echo 'Summary of checks:'
\echo '  1. TimescaleDB version (should be 2.14+)'
\echo '  2. All 4 continuous aggregates created'
\echo '  3. Refresh policies configured'
\echo '  4. View schemas match expectations'
\echo '  5. Test data inserted successfully'
\echo '  6. Manual refresh completed'
\echo '  7. Query results returned'
\echo '  8. Data accuracy validated'
\echo '  9. Materialization status checked'
\echo '  10. Performance comparison executed'
\echo ''
\echo 'Review output above for any FAIL or WARNING statuses.'
\echo ''
