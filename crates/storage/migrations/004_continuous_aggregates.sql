-- Migration: 004_continuous_aggregates.sql
-- Description: Create continuous aggregates (materialized views) for fast analytics
-- Date: 2025-11-05
-- Author: LLM Observatory Core Team
-- Sections: 5.1 (1-min), 5.2 (1-hour), 5.3 (1-day), 5.4 (error summary)
--
-- IMPORTANT: TimescaleDB 2.14+ Compatibility Notes
-- - PERCENTILE_CONT is NOT supported in continuous aggregates
-- - Use percentile_agg (two-step aggregation) instead
-- - All aggregates must be parallelizable (no ORDER BY in aggregate functions)
-- - For percentiles, compute from raw data or use approximation algorithms

BEGIN;

-- ============================================================================
-- Section 5.1: 1-Minute Rollups
-- ============================================================================
-- Purpose: Real-time monitoring dashboards
-- Refresh: Every 30 seconds
-- Retention: 37 days (set in 005_retention_policies.sql)
--
-- Note: Stores pre-aggregated data for percentile calculation via queries
-- Use MIN/MAX for range, and approximate percentiles via interpolation

CREATE MATERIALIZED VIEW IF NOT EXISTS llm_metrics_1min
WITH (timescaledb.continuous) AS
SELECT
    time_bucket('1 minute', ts) AS bucket,
    provider,
    model,
    status_code,
    -- Basic aggregates
    COUNT(*) AS request_count,
    SUM(total_tokens) AS total_tokens,
    SUM(total_cost_usd) AS total_cost_usd,
    -- Duration statistics (for percentile approximation)
    AVG(duration_ms) AS avg_duration_ms,
    MIN(duration_ms) AS min_duration_ms,
    MAX(duration_ms) AS max_duration_ms,
    -- Store sum of squares for stddev calculation
    SUM(duration_ms * duration_ms) AS sum_duration_ms_squared,
    -- Token statistics
    AVG(prompt_tokens) AS avg_prompt_tokens,
    AVG(completion_tokens) AS avg_completion_tokens
FROM llm_traces
GROUP BY bucket, provider, model, status_code
WITH NO DATA;

COMMENT ON MATERIALIZED VIEW llm_metrics_1min IS 'Real-time 1-minute rollups for monitoring dashboards. Note: Percentiles must be computed from raw data or via approximate algorithms.';

-- Refresh policy: Update every 30 seconds
-- Covers data from 1 hour ago to 1 minute ago
SELECT add_continuous_aggregate_policy('llm_metrics_1min',
    start_offset => INTERVAL '1 hour',
    end_offset => INTERVAL '1 minute',
    schedule_interval => INTERVAL '30 seconds');

-- ============================================================================
-- Section 5.2: 1-Hour Rollups
-- ============================================================================
-- Purpose: Historical analysis and trending
-- Refresh: Every 5 minutes
-- Retention: 210 days (set in 005_retention_policies.sql)

CREATE MATERIALIZED VIEW IF NOT EXISTS llm_metrics_1hour
WITH (timescaledb.continuous) AS
SELECT
    time_bucket('1 hour', ts) AS bucket,
    provider,
    model,
    environment,
    -- Request metrics
    COUNT(*) AS request_count,
    SUM(total_tokens) AS total_tokens,
    SUM(total_cost_usd) AS total_cost_usd,
    -- Duration statistics
    AVG(duration_ms) AS avg_duration_ms,
    MIN(duration_ms) AS min_duration_ms,
    MAX(duration_ms) AS max_duration_ms,
    SUM(duration_ms * duration_ms) AS sum_duration_ms_squared,
    -- Error metrics
    SUM(CASE WHEN status_code = 'ERROR' THEN 1 ELSE 0 END) AS error_count,
    SUM(CASE WHEN status_code = 'OK' THEN 1 ELSE 0 END) AS success_count,
    -- Token breakdown
    SUM(prompt_tokens) AS total_prompt_tokens,
    SUM(completion_tokens) AS total_completion_tokens,
    AVG(prompt_tokens) AS avg_prompt_tokens,
    AVG(completion_tokens) AS avg_completion_tokens,
    -- Cost breakdown
    SUM(prompt_cost_usd) AS total_prompt_cost,
    SUM(completion_cost_usd) AS total_completion_cost
FROM llm_traces
GROUP BY bucket, provider, model, environment
WITH NO DATA;

COMMENT ON MATERIALIZED VIEW llm_metrics_1hour IS 'Hourly rollups for historical analysis and trending. Compute error_rate as error_count / request_count in queries.';

-- Refresh policy: Update every 5 minutes
-- Covers data from 1 day ago to 1 hour ago
SELECT add_continuous_aggregate_policy('llm_metrics_1hour',
    start_offset => INTERVAL '1 day',
    end_offset => INTERVAL '1 hour',
    schedule_interval => INTERVAL '5 minutes');

-- ============================================================================
-- Section 5.3: 1-Day Rollups
-- ============================================================================
-- Purpose: Long-term trends and capacity planning
-- Refresh: Every 1 hour
-- Retention: 3 years (1095 days, set in 005_retention_policies.sql)
--
-- Note: COUNT(DISTINCT) is supported in TimescaleDB 2.14+ continuous aggregates

CREATE MATERIALIZED VIEW IF NOT EXISTS llm_metrics_1day
WITH (timescaledb.continuous) AS
SELECT
    time_bucket('1 day', ts) AS bucket,
    provider,
    model,
    -- Request metrics
    COUNT(*) AS request_count,
    SUM(total_tokens) AS total_tokens,
    -- Cost metrics
    SUM(total_cost_usd) AS total_cost_usd,
    SUM(prompt_cost_usd) AS prompt_cost_usd,
    SUM(completion_cost_usd) AS completion_cost_usd,
    AVG(total_cost_usd) AS avg_cost_per_request,
    -- Duration statistics
    AVG(duration_ms) AS avg_duration_ms,
    MIN(duration_ms) AS min_duration_ms,
    MAX(duration_ms) AS max_duration_ms,
    -- Unique users and sessions (COUNT DISTINCT supported in TimescaleDB 2.7+)
    COUNT(DISTINCT user_id) AS unique_users,
    COUNT(DISTINCT session_id) AS unique_sessions,
    -- Token statistics
    SUM(prompt_tokens) AS total_prompt_tokens,
    SUM(completion_tokens) AS total_completion_tokens,
    AVG(prompt_tokens) AS avg_prompt_tokens,
    AVG(completion_tokens) AS avg_completion_tokens
FROM llm_traces
GROUP BY bucket, provider, model
WITH NO DATA;

COMMENT ON MATERIALIZED VIEW llm_metrics_1day IS 'Daily rollups for long-term trends and capacity planning. Includes unique user/session counts.';

-- Refresh policy: Update every 1 hour
-- Covers data from 7 days ago to 1 day ago
SELECT add_continuous_aggregate_policy('llm_metrics_1day',
    start_offset => INTERVAL '7 days',
    end_offset => INTERVAL '1 day',
    schedule_interval => INTERVAL '1 hour');

-- ============================================================================
-- Section 5.4: Error Summary (Hourly)
-- ============================================================================
-- Purpose: Detailed error tracking and alerting
-- Refresh: Every 5 minutes
-- Retention: 210 days (set in 005_retention_policies.sql)
--
-- Tracks errors by type and model for monitoring and debugging

CREATE MATERIALIZED VIEW IF NOT EXISTS llm_error_summary
WITH (timescaledb.continuous) AS
SELECT
    time_bucket('1 hour', ts) AS bucket,
    provider,
    model,
    status_code,
    environment,
    -- Error counts
    COUNT(*) AS error_count,
    -- Sample error messages (first occurrence)
    MIN(error_message) AS sample_error_message,
    -- Associated metrics
    AVG(duration_ms) AS avg_duration_ms,
    SUM(total_cost_usd) AS total_cost_usd,
    -- User impact
    COUNT(DISTINCT user_id) AS affected_users,
    COUNT(DISTINCT session_id) AS affected_sessions
FROM llm_traces
WHERE status_code != 'OK'  -- Only track errors and non-OK statuses
GROUP BY bucket, provider, model, status_code, environment
WITH NO DATA;

COMMENT ON MATERIALIZED VIEW llm_error_summary IS 'Hourly error aggregation for monitoring and alerting. Filters to non-OK status codes only.';

-- Refresh policy: Update every 5 minutes
-- Covers data from 1 day ago to 1 hour ago
SELECT add_continuous_aggregate_policy('llm_error_summary',
    start_offset => INTERVAL '1 day',
    end_offset => INTERVAL '1 hour',
    schedule_interval => INTERVAL '5 minutes');

COMMIT;

-- ============================================================================
-- Verification Queries
-- ============================================================================
-- Run these queries to verify continuous aggregates were created:
--
-- -- List all continuous aggregates
-- SELECT view_name, refresh_lag, refresh_interval
-- FROM timescaledb_information.continuous_aggregates;
--
-- -- Check materialization status
-- SELECT view_name, completed_threshold, invalidation_threshold
-- FROM timescaledb_information.continuous_aggregate_stats;
--
-- -- Manual refresh (if needed)
-- CALL refresh_continuous_aggregate('llm_metrics_1min', NULL, NULL);
-- CALL refresh_continuous_aggregate('llm_metrics_1hour', NULL, NULL);
-- CALL refresh_continuous_aggregate('llm_metrics_1day', NULL, NULL);
-- CALL refresh_continuous_aggregate('llm_error_summary', NULL, NULL);

-- ============================================================================
-- Usage Examples: Querying the Continuous Aggregates
-- ============================================================================
--
-- Example 1: Get error rate for the last hour
-- SELECT
--     bucket,
--     provider,
--     model,
--     environment,
--     request_count,
--     error_count,
--     ROUND(error_count::NUMERIC / NULLIF(request_count, 0) * 100, 2) AS error_rate_pct
-- FROM llm_metrics_1hour
-- WHERE bucket >= NOW() - INTERVAL '1 hour'
-- ORDER BY bucket DESC, error_rate_pct DESC;
--
-- Example 2: Compute approximate percentiles from aggregated data
-- For more accurate percentiles, query raw llm_traces table directly
-- SELECT
--     bucket,
--     provider,
--     model,
--     avg_duration_ms,
--     min_duration_ms,
--     max_duration_ms,
--     -- Approximate stddev using Welford's method
--     SQRT(
--         GREATEST(
--             (sum_duration_ms_squared / request_count) -
--             (avg_duration_ms * avg_duration_ms),
--             0
--         )
--     ) AS stddev_duration_ms
-- FROM llm_metrics_1min
-- WHERE bucket >= NOW() - INTERVAL '15 minutes'
-- ORDER BY bucket DESC;
--
-- Example 3: Daily cost trends
-- SELECT
--     bucket::DATE AS date,
--     provider,
--     model,
--     request_count,
--     ROUND(total_cost_usd::NUMERIC, 4) AS total_cost_usd,
--     ROUND(avg_cost_per_request::NUMERIC, 6) AS avg_cost_per_request,
--     unique_users,
--     unique_sessions
-- FROM llm_metrics_1day
-- WHERE bucket >= NOW() - INTERVAL '30 days'
-- ORDER BY bucket DESC, total_cost_usd DESC;
--
-- Example 4: Error analysis
-- SELECT
--     bucket,
--     provider,
--     model,
--     status_code,
--     error_count,
--     affected_users,
--     affected_sessions,
--     sample_error_message
-- FROM llm_error_summary
-- WHERE bucket >= NOW() - INTERVAL '24 hours'
--     AND error_count > 10  -- Filter to significant error counts
-- ORDER BY bucket DESC, error_count DESC;

-- ============================================================================
-- Important Notes for Percentile Queries
-- ============================================================================
--
-- TimescaleDB 2.14 continuous aggregates do NOT support PERCENTILE_CONT
-- or other ordered-set aggregates because they cannot be computed incrementally.
--
-- Options for percentile calculation:
--
-- 1. Query raw data directly (best for accuracy, slower):
--    SELECT
--        time_bucket('1 minute', ts) AS bucket,
--        provider,
--        model,
--        PERCENTILE_CONT(0.50) WITHIN GROUP (ORDER BY duration_ms) AS p50,
--        PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY duration_ms) AS p95,
--        PERCENTILE_CONT(0.99) WITHIN GROUP (ORDER BY duration_ms) AS p99
--    FROM llm_traces
--    WHERE ts >= NOW() - INTERVAL '15 minutes'
--    GROUP BY bucket, provider, model;
--
-- 2. Use approximate algorithms (e.g., T-Digest, HyperLogLog):
--    -- Requires additional extensions like timescaledb-toolkit
--    -- Not included in base TimescaleDB
--
-- 3. Pre-compute percentiles via custom jobs:
--    -- Create a separate table for percentile storage
--    -- Run periodic jobs to compute and insert percentiles
--
-- 4. Use statistical approximations:
--    -- Estimate percentiles from mean and stddev (assumes normal distribution)
--    -- Less accurate but computationally cheap
