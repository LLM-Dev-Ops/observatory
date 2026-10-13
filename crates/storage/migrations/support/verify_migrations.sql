-- ============================================================================
-- Migration Verification Script
-- ============================================================================
-- Purpose: Verify all migrations have been applied correctly
-- Run this after executing all migrations to ensure schema is correct
-- Date: 2025-11-05

-- ============================================================================
-- 1. Verify TimescaleDB Extension
-- ============================================================================
\echo '=== Checking TimescaleDB Extension ==='
SELECT
    extname AS extension_name,
    extversion AS version,
    'Installed' AS status
FROM pg_extension
WHERE extname = 'timescaledb';

-- ============================================================================
-- 2. Verify Core Tables Exist
-- ============================================================================
\echo ''
\echo '=== Checking Core Tables ==='
SELECT
    tablename AS table_name,
    pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) AS total_size
FROM pg_tables
WHERE schemaname = 'public'
    AND tablename IN ('llm_traces', 'llm_metrics', 'llm_logs',
                      'model_pricing', 'api_keys', 'users', 'projects')
ORDER BY tablename;

-- ============================================================================
-- 3. Verify Hypertables Configuration
-- ============================================================================
\echo ''
\echo '=== Checking Hypertables ==='
SELECT
    hypertable_schema,
    hypertable_name,
    num_dimensions,
    num_chunks,
    compression_enabled,
    pg_size_pretty(total_bytes) AS total_size
FROM timescaledb_information.hypertables
WHERE hypertable_name IN ('llm_traces', 'llm_metrics', 'llm_logs')
ORDER BY hypertable_name;

-- ============================================================================
-- 4. Verify Chunk Intervals
-- ============================================================================
\echo ''
\echo '=== Checking Chunk Time Intervals ==='
SELECT
    hypertable_name,
    dimension_number,
    column_name,
    time_interval
FROM timescaledb_information.dimensions
WHERE hypertable_name IN ('llm_traces', 'llm_metrics', 'llm_logs')
ORDER BY hypertable_name, dimension_number;

-- ============================================================================
-- 5. Verify Indexes
-- ============================================================================
\echo ''
\echo '=== Checking Indexes (llm_traces) ==='
SELECT
    indexname AS index_name,
    indexdef AS definition,
    pg_size_pretty(pg_relation_size(indexname::regclass)) AS index_size
FROM pg_indexes
WHERE schemaname = 'public'
    AND tablename = 'llm_traces'
ORDER BY pg_relation_size(indexname::regclass) DESC;

\echo ''
\echo '=== Checking Indexes (llm_metrics) ==='
SELECT
    indexname AS index_name,
    pg_size_pretty(pg_relation_size(indexname::regclass)) AS index_size
FROM pg_indexes
WHERE schemaname = 'public'
    AND tablename = 'llm_metrics'
ORDER BY indexname;

\echo ''
\echo '=== Checking Indexes (llm_logs) ==='
SELECT
    indexname AS index_name,
    pg_size_pretty(pg_relation_size(indexname::regclass)) AS index_size
FROM pg_indexes
WHERE schemaname = 'public'
    AND tablename = 'llm_logs'
ORDER BY indexname;

-- ============================================================================
-- 6. Verify Continuous Aggregates
-- ============================================================================
\echo ''
\echo '=== Checking Continuous Aggregates ==='
SELECT
    view_name,
    format_interval(refresh_lag) AS refresh_lag,
    format_interval(refresh_interval) AS refresh_interval,
    materialized_only,
    compression_enabled
FROM timescaledb_information.continuous_aggregates
ORDER BY view_name;

-- ============================================================================
-- 7. Verify Compression Settings
-- ============================================================================
\echo ''
\echo '=== Checking Compression Settings ==='
SELECT
    hypertable_schema,
    hypertable_name,
    attname AS column_name,
    segmentby_column_index,
    orderby_column_index,
    compression_algorithm
FROM timescaledb_information.compression_settings
WHERE hypertable_name IN ('llm_traces', 'llm_metrics', 'llm_logs')
ORDER BY hypertable_name, segmentby_column_index, orderby_column_index;

-- ============================================================================
-- 8. Verify Compression Policies
-- ============================================================================
\echo ''
\echo '=== Checking Compression Policies ==='
SELECT
    j.hypertable_name,
    format_interval(config::json->>'compress_after') AS compress_after,
    j.schedule_interval,
    js.last_run_started_at,
    js.next_start
FROM timescaledb_information.jobs j
JOIN timescaledb_information.job_stats js ON j.job_id = js.job_id
WHERE j.proc_name = 'policy_compression'
ORDER BY j.hypertable_name;

-- ============================================================================
-- 9. Verify Retention Policies
-- ============================================================================
\echo ''
\echo '=== Checking Retention Policies ==='
SELECT
    j.hypertable_name,
    format_interval(config::json->>'drop_after') AS retention_period,
    j.schedule_interval,
    js.last_run_started_at,
    js.next_start
FROM timescaledb_information.jobs j
JOIN timescaledb_information.job_stats js ON j.job_id = js.job_id
WHERE j.proc_name = 'policy_retention'
ORDER BY j.hypertable_name;

-- ============================================================================
-- 10. Verify Foreign Keys
-- ============================================================================
\echo ''
\echo '=== Checking Foreign Key Constraints ==='
SELECT
    conname AS constraint_name,
    conrelid::regclass AS table_name,
    confrelid::regclass AS referenced_table,
    pg_get_constraintdef(oid) AS constraint_definition
FROM pg_constraint
WHERE contype = 'f'
    AND connamespace = 'public'::regnamespace
ORDER BY conrelid::regclass::text;

-- ============================================================================
-- 11. Verify Column Counts
-- ============================================================================
\echo ''
\echo '=== Checking Column Counts ==='
SELECT
    table_name,
    COUNT(*) AS column_count
FROM information_schema.columns
WHERE table_schema = 'public'
    AND table_name IN ('llm_traces', 'llm_metrics', 'llm_logs',
                       'model_pricing', 'api_keys', 'users', 'projects')
GROUP BY table_name
ORDER BY table_name;

-- Expected counts:
-- llm_traces: 33 columns
-- llm_metrics: 12 columns
-- llm_logs: 9 columns
-- model_pricing: 7 columns
-- api_keys: 11 columns
-- users: 5 columns
-- projects: 6 columns

-- ============================================================================
-- 12. Verify Table Comments
-- ============================================================================
\echo ''
\echo '=== Checking Table Comments ==='
SELECT
    c.relname AS table_name,
    d.description AS comment
FROM pg_class c
JOIN pg_namespace n ON n.oid = c.relnamespace
LEFT JOIN pg_description d ON d.objoid = c.oid AND d.objsubid = 0
WHERE n.nspname = 'public'
    AND c.relkind = 'r'
    AND c.relname IN ('llm_traces', 'llm_metrics', 'llm_logs',
                      'model_pricing', 'api_keys', 'users', 'projects')
ORDER BY c.relname;

-- ============================================================================
-- 13. Database Size Summary
-- ============================================================================
\echo ''
\echo '=== Database Size Summary ==='
SELECT
    pg_database.datname AS database_name,
    pg_size_pretty(pg_database_size(pg_database.datname)) AS size
FROM pg_database
WHERE datname = current_database();

-- ============================================================================
-- 14. Schema Validation Summary
-- ============================================================================
\echo ''
\echo '=== Migration Validation Summary ==='
SELECT
    'Tables' AS component,
    COUNT(*) AS count,
    CASE
        WHEN COUNT(*) = 7 THEN 'PASS'
        ELSE 'FAIL'
    END AS status
FROM pg_tables
WHERE schemaname = 'public'
    AND tablename IN ('llm_traces', 'llm_metrics', 'llm_logs',
                      'model_pricing', 'api_keys', 'users', 'projects')

UNION ALL

SELECT
    'Hypertables' AS component,
    COUNT(*) AS count,
    CASE
        WHEN COUNT(*) = 3 THEN 'PASS'
        ELSE 'FAIL'
    END AS status
FROM timescaledb_information.hypertables
WHERE hypertable_name IN ('llm_traces', 'llm_metrics', 'llm_logs')

UNION ALL

SELECT
    'Continuous Aggregates' AS component,
    COUNT(*) AS count,
    CASE
        WHEN COUNT(*) = 4 THEN 'PASS'
        ELSE 'FAIL'
    END AS status
FROM timescaledb_information.continuous_aggregates
WHERE view_name IN ('llm_metrics_1min', 'llm_metrics_1hour',
                    'llm_metrics_1day', 'cost_analysis_hourly')

UNION ALL

SELECT
    'Compression Policies' AS component,
    COUNT(*) AS count,
    CASE
        WHEN COUNT(*) >= 3 THEN 'PASS'
        ELSE 'FAIL'
    END AS status
FROM timescaledb_information.jobs
WHERE proc_name = 'policy_compression'

UNION ALL

SELECT
    'Retention Policies' AS component,
    COUNT(*) AS count,
    CASE
        WHEN COUNT(*) >= 3 THEN 'PASS'
        ELSE 'FAIL'
    END AS status
FROM timescaledb_information.jobs
WHERE proc_name = 'policy_retention';

\echo ''
\echo '=== Verification Complete ==='
\echo 'All checks passed if all statuses show PASS'
