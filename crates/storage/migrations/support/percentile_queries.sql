-- ============================================================================
-- Percentile Query Helpers
-- ============================================================================
-- Purpose: Helper queries for computing percentiles from LLM Observatory data
-- Since continuous aggregates don't support PERCENTILE_CONT, use these
-- queries for accurate percentile calculation.
--
-- Performance Note: These queries scan raw data, which is slower than
-- aggregates. Use time windows wisely.

-- ============================================================================
-- 1. Real-time Percentiles (Last 15 Minutes)
-- ============================================================================
-- Use Case: Live dashboards, real-time monitoring
-- Performance: Fast (small time window, indexed by ts)

CREATE OR REPLACE VIEW llm_percentiles_realtime AS
SELECT
    time_bucket('1 minute', ts) AS bucket,
    provider,
    model,
    status_code,
    COUNT(*) AS request_count,
    -- Duration percentiles
    PERCENTILE_CONT(0.50) WITHIN GROUP (ORDER BY duration_ms) AS p50_duration_ms,
    PERCENTILE_CONT(0.90) WITHIN GROUP (ORDER BY duration_ms) AS p90_duration_ms,
    PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY duration_ms) AS p95_duration_ms,
    PERCENTILE_CONT(0.99) WITHIN GROUP (ORDER BY duration_ms) AS p99_duration_ms,
    AVG(duration_ms) AS avg_duration_ms,
    MIN(duration_ms) AS min_duration_ms,
    MAX(duration_ms) AS max_duration_ms,
    -- Time to first token (TTFT) percentiles (for streaming)
    PERCENTILE_CONT(0.50) WITHIN GROUP (ORDER BY ttft_ms) AS p50_ttft_ms,
    PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY ttft_ms) AS p95_ttft_ms,
    PERCENTILE_CONT(0.99) WITHIN GROUP (ORDER BY ttft_ms) AS p99_ttft_ms,
    -- Token percentiles
    PERCENTILE_CONT(0.50) WITHIN GROUP (ORDER BY total_tokens) AS p50_tokens,
    PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY total_tokens) AS p95_tokens,
    -- Cost percentiles
    PERCENTILE_CONT(0.50) WITHIN GROUP (ORDER BY total_cost_usd) AS p50_cost_usd,
    PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY total_cost_usd) AS p95_cost_usd
FROM llm_traces
WHERE ts >= NOW() - INTERVAL '15 minutes'
    AND status_code = 'OK'  -- Only successful requests
GROUP BY bucket, provider, model, status_code;

COMMENT ON VIEW llm_percentiles_realtime IS 'Real-time percentiles for the last 15 minutes. Queries raw data directly.';

-- Example usage:
-- SELECT * FROM llm_percentiles_realtime
-- WHERE provider = 'openai'
-- ORDER BY bucket DESC
-- LIMIT 15;

-- ============================================================================
-- 2. Hourly Percentiles (Last 24 Hours)
-- ============================================================================
-- Use Case: Recent historical analysis, troubleshooting
-- Performance: Moderate (24 hours of data)

CREATE OR REPLACE VIEW llm_percentiles_hourly AS
SELECT
    time_bucket('1 hour', ts) AS bucket,
    provider,
    model,
    environment,
    COUNT(*) AS request_count,
    -- Duration percentiles
    PERCENTILE_CONT(0.50) WITHIN GROUP (ORDER BY duration_ms) AS p50_duration_ms,
    PERCENTILE_CONT(0.90) WITHIN GROUP (ORDER BY duration_ms) AS p90_duration_ms,
    PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY duration_ms) AS p95_duration_ms,
    PERCENTILE_CONT(0.99) WITHIN GROUP (ORDER BY duration_ms) AS p99_duration_ms,
    AVG(duration_ms) AS avg_duration_ms,
    -- Cost statistics
    SUM(total_cost_usd) AS total_cost_usd,
    AVG(total_cost_usd) AS avg_cost_usd,
    PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY total_cost_usd) AS p95_cost_usd,
    -- Token statistics
    SUM(total_tokens) AS total_tokens,
    AVG(total_tokens) AS avg_tokens,
    PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY total_tokens) AS p95_tokens,
    -- Error rate
    SUM(CASE WHEN status_code = 'ERROR' THEN 1 ELSE 0 END)::FLOAT / COUNT(*) AS error_rate
FROM llm_traces
WHERE ts >= NOW() - INTERVAL '24 hours'
GROUP BY bucket, provider, model, environment;

COMMENT ON VIEW llm_percentiles_hourly IS 'Hourly percentiles for the last 24 hours. Queries raw data directly.';

-- Example usage:
-- SELECT
--     bucket,
--     model,
--     request_count,
--     ROUND(p50_duration_ms::NUMERIC, 2) AS p50_ms,
--     ROUND(p95_duration_ms::NUMERIC, 2) AS p95_ms,
--     ROUND(p99_duration_ms::NUMERIC, 2) AS p99_ms
-- FROM llm_percentiles_hourly
-- WHERE provider = 'openai'
-- ORDER BY bucket DESC;

-- ============================================================================
-- 3. Approximate Percentiles from Aggregates (Fast!)
-- ============================================================================
-- Use Case: Historical dashboards, large time windows
-- Performance: Very fast (uses pre-computed aggregates)
-- Accuracy: Approximate (assumes normal distribution)

CREATE OR REPLACE FUNCTION approximate_percentile(
    avg_value DOUBLE PRECISION,
    stddev_value DOUBLE PRECISION,
    percentile DOUBLE PRECISION
) RETURNS DOUBLE PRECISION AS $$
BEGIN
    -- Approximate percentiles using normal distribution assumption
    -- Uses z-scores for common percentiles
    RETURN CASE
        WHEN percentile = 0.50 THEN avg_value  -- Median ≈ Mean (normal dist)
        WHEN percentile = 0.75 THEN avg_value + (0.674 * stddev_value)  -- P75
        WHEN percentile = 0.90 THEN avg_value + (1.282 * stddev_value)  -- P90
        WHEN percentile = 0.95 THEN avg_value + (1.645 * stddev_value)  -- P95
        WHEN percentile = 0.99 THEN avg_value + (2.326 * stddev_value)  -- P99
        ELSE avg_value  -- Default to mean
    END;
END;
$$ LANGUAGE plpgsql IMMUTABLE;

COMMENT ON FUNCTION approximate_percentile IS 'Approximate percentile calculation from mean and stddev (assumes normal distribution)';

-- Create view using approximate percentiles from 1-min aggregates
CREATE OR REPLACE VIEW llm_percentiles_approximate AS
SELECT
    bucket,
    provider,
    model,
    status_code,
    request_count,
    avg_duration_ms,
    min_duration_ms,
    max_duration_ms,
    -- Calculate stddev from sum of squares
    SQRT(
        GREATEST(
            (sum_duration_ms_squared / NULLIF(request_count, 0)) -
            (avg_duration_ms * avg_duration_ms),
            0
        )
    ) AS stddev_duration_ms,
    -- Approximate percentiles
    approximate_percentile(
        avg_duration_ms,
        SQRT(GREATEST((sum_duration_ms_squared / NULLIF(request_count, 0)) - (avg_duration_ms * avg_duration_ms), 0)),
        0.50
    ) AS approx_p50_duration_ms,
    approximate_percentile(
        avg_duration_ms,
        SQRT(GREATEST((sum_duration_ms_squared / NULLIF(request_count, 0)) - (avg_duration_ms * avg_duration_ms), 0)),
        0.90
    ) AS approx_p90_duration_ms,
    approximate_percentile(
        avg_duration_ms,
        SQRT(GREATEST((sum_duration_ms_squared / NULLIF(request_count, 0)) - (avg_duration_ms * avg_duration_ms), 0)),
        0.95
    ) AS approx_p95_duration_ms,
    approximate_percentile(
        avg_duration_ms,
        SQRT(GREATEST((sum_duration_ms_squared / NULLIF(request_count, 0)) - (avg_duration_ms * avg_duration_ms), 0)),
        0.99
    ) AS approx_p99_duration_ms
FROM llm_metrics_1min;

COMMENT ON VIEW llm_percentiles_approximate IS 'Fast approximate percentiles from 1-min aggregates. Good for historical dashboards.';

-- Example usage:
-- SELECT
--     bucket,
--     model,
--     request_count,
--     ROUND(avg_duration_ms::NUMERIC, 2) AS avg_ms,
--     ROUND(approx_p50_duration_ms::NUMERIC, 2) AS p50_ms,
--     ROUND(approx_p95_duration_ms::NUMERIC, 2) AS p95_ms,
--     ROUND(approx_p99_duration_ms::NUMERIC, 2) AS p99_ms
-- FROM llm_percentiles_approximate
-- WHERE bucket >= NOW() - INTERVAL '24 hours'
--     AND provider = 'openai'
-- ORDER BY bucket DESC;

-- ============================================================================
-- 4. Model Comparison (Percentiles by Model)
-- ============================================================================
-- Use Case: Compare performance across different models
-- Performance: Moderate (depends on time window)

CREATE OR REPLACE FUNCTION compare_model_percentiles(
    time_window INTERVAL DEFAULT INTERVAL '1 hour'
)
RETURNS TABLE (
    model TEXT,
    provider TEXT,
    request_count BIGINT,
    p50_duration_ms DOUBLE PRECISION,
    p95_duration_ms DOUBLE PRECISION,
    p99_duration_ms DOUBLE PRECISION,
    avg_cost_usd DOUBLE PRECISION,
    p95_cost_usd DOUBLE PRECISION,
    error_rate DOUBLE PRECISION
) AS $$
BEGIN
    RETURN QUERY
    SELECT
        t.model,
        t.provider,
        COUNT(*) AS request_count,
        PERCENTILE_CONT(0.50) WITHIN GROUP (ORDER BY t.duration_ms) AS p50_duration_ms,
        PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY t.duration_ms) AS p95_duration_ms,
        PERCENTILE_CONT(0.99) WITHIN GROUP (ORDER BY t.duration_ms) AS p99_duration_ms,
        AVG(t.total_cost_usd) AS avg_cost_usd,
        PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY t.total_cost_usd) AS p95_cost_usd,
        SUM(CASE WHEN t.status_code = 'ERROR' THEN 1 ELSE 0 END)::FLOAT / COUNT(*) AS error_rate
    FROM llm_traces t
    WHERE t.ts >= NOW() - time_window
    GROUP BY t.model, t.provider
    ORDER BY request_count DESC;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION compare_model_percentiles IS 'Compare percentile performance across models for a given time window';

-- Example usage:
-- SELECT * FROM compare_model_percentiles(INTERVAL '1 hour');
-- SELECT * FROM compare_model_percentiles(INTERVAL '24 hours');

-- ============================================================================
-- 5. SLA Monitoring (Percentile-based)
-- ============================================================================
-- Use Case: Track SLA compliance based on percentile targets
-- Example: P95 latency should be < 2000ms, P99 < 5000ms

CREATE OR REPLACE VIEW llm_sla_monitoring AS
SELECT
    time_bucket('5 minutes', ts) AS bucket,
    provider,
    model,
    COUNT(*) AS request_count,
    PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY duration_ms) AS p95_duration_ms,
    PERCENTILE_CONT(0.99) WITHIN GROUP (ORDER BY duration_ms) AS p99_duration_ms,
    -- SLA compliance (example targets)
    CASE
        WHEN PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY duration_ms) < 2000 THEN 'PASS'
        ELSE 'FAIL'
    END AS p95_sla_2s,
    CASE
        WHEN PERCENTILE_CONT(0.99) WITHIN GROUP (ORDER BY duration_ms) < 5000 THEN 'PASS'
        ELSE 'FAIL'
    END AS p99_sla_5s,
    -- Error rate SLA (example: < 1%)
    SUM(CASE WHEN status_code = 'ERROR' THEN 1 ELSE 0 END)::FLOAT / COUNT(*) AS error_rate,
    CASE
        WHEN SUM(CASE WHEN status_code = 'ERROR' THEN 1 ELSE 0 END)::FLOAT / COUNT(*) < 0.01 THEN 'PASS'
        ELSE 'FAIL'
    END AS error_rate_sla_1pct
FROM llm_traces
WHERE ts >= NOW() - INTERVAL '1 hour'
GROUP BY bucket, provider, model
ORDER BY bucket DESC;

COMMENT ON VIEW llm_sla_monitoring IS 'SLA monitoring with percentile-based targets. Adjust thresholds as needed.';

-- Example usage:
-- SELECT
--     bucket,
--     model,
--     request_count,
--     ROUND(p95_duration_ms::NUMERIC, 0) AS p95_ms,
--     p95_sla_2s,
--     ROUND(p99_duration_ms::NUMERIC, 0) AS p99_ms,
--     p99_sla_5s,
--     ROUND(error_rate::NUMERIC * 100, 2) AS error_rate_pct,
--     error_rate_sla_1pct
-- FROM llm_sla_monitoring
-- WHERE p95_sla_2s = 'FAIL' OR p99_sla_5s = 'FAIL' OR error_rate_sla_1pct = 'FAIL'
-- ORDER BY bucket DESC;

-- ============================================================================
-- 6. User-specific Percentiles
-- ============================================================================
-- Use Case: Analyze latency/cost percentiles per user or session
-- Performance: Depends on cardinality of user_id

CREATE OR REPLACE FUNCTION user_percentiles(
    p_user_id TEXT,
    time_window INTERVAL DEFAULT INTERVAL '24 hours'
)
RETURNS TABLE (
    time_bucket TIMESTAMPTZ,
    model TEXT,
    request_count BIGINT,
    p50_duration_ms DOUBLE PRECISION,
    p95_duration_ms DOUBLE PRECISION,
    total_cost_usd DOUBLE PRECISION,
    avg_cost_per_request DOUBLE PRECISION
) AS $$
BEGIN
    RETURN QUERY
    SELECT
        time_bucket('1 hour', ts) AS time_bucket,
        t.model,
        COUNT(*) AS request_count,
        PERCENTILE_CONT(0.50) WITHIN GROUP (ORDER BY t.duration_ms) AS p50_duration_ms,
        PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY t.duration_ms) AS p95_duration_ms,
        SUM(t.total_cost_usd) AS total_cost_usd,
        AVG(t.total_cost_usd) AS avg_cost_per_request
    FROM llm_traces t
    WHERE t.user_id = p_user_id
        AND t.ts >= NOW() - time_window
    GROUP BY time_bucket, t.model
    ORDER BY time_bucket DESC;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION user_percentiles IS 'Get percentile statistics for a specific user';

-- Example usage:
-- SELECT * FROM user_percentiles('user-123', INTERVAL '7 days');

-- ============================================================================
-- Usage Recommendations
-- ============================================================================
--
-- 1. Real-time monitoring (< 15 min):
--    Use llm_percentiles_realtime view
--    Performance: Fast (small dataset)
--
-- 2. Recent analysis (< 24 hours):
--    Use llm_percentiles_hourly view
--    Performance: Moderate
--
-- 3. Historical dashboards (> 24 hours):
--    Use llm_percentiles_approximate view
--    Performance: Very fast (uses aggregates)
--    Trade-off: Approximate values
--
-- 4. Ad-hoc analysis:
--    Query llm_traces directly with specific time windows
--    Most flexible, performance depends on window size
--
-- 5. SLA monitoring:
--    Use llm_sla_monitoring view
--    Customize thresholds for your requirements
--
-- ============================================================================
-- Performance Tips
-- ============================================================================
--
-- 1. Always use time-based WHERE clauses (leverages hypertable partitioning)
-- 2. Smaller time windows = faster queries
-- 3. Add additional indexes if filtering by specific dimensions frequently:
--    CREATE INDEX idx_traces_user_ts ON llm_traces(user_id, ts DESC);
--    CREATE INDEX idx_traces_session_ts ON llm_traces(session_id, ts DESC);
-- 4. For very large time windows, use approximate percentiles
-- 5. Consider creating materialized views for specific percentile queries
--    that run frequently
