/**
 * SLO/SLA Enforcement Agent - Telemetry
 *
 * Self-observation metrics for the agent.
 * These metrics are compatible with LLM-Observatory ingestion.
 */

import { loadConfig } from './config';
import type { AgentMetrics } from '../types';

// Start time for uptime calculation
const startTime = Date.now();

// Metrics state
let evaluationsTotal = 0;
let violationsDetected = 0;
let errorsTotal = 0;
let totalLatencyMs = 0;
let latencyCount = 0;
let slosEvaluated = 0;
let slaBreachesDetected = 0;
let lastEvaluationAt: string | undefined;

/**
 * Record an evaluation
 */
export function recordEvaluation(
  sloCount: number,
  violationCount: number,
  latencyMs: number
): void {
  evaluationsTotal++;
  violationsDetected += violationCount;
  slosEvaluated += sloCount;
  totalLatencyMs += latencyMs;
  latencyCount++;
  lastEvaluationAt = new Date().toISOString();
}

/**
 * Record an SLA breach
 */
export function recordSlaBreaches(count: number): void {
  slaBreachesDetected += count;
}

/**
 * Record an error
 */
export function recordError(error: Error): void {
  errorsTotal++;
  console.error(`[${new Date().toISOString()}] Error:`, error.message);
}

/**
 * Get current metrics
 */
export function getMetrics(): AgentMetrics {
  const uptimeSeconds = Math.floor((Date.now() - startTime) / 1000);
  const avgLatencyMs = latencyCount > 0 ? totalLatencyMs / latencyCount : 0;

  return {
    evaluations_total: evaluationsTotal,
    violations_detected: violationsDetected,
    errors_total: errorsTotal,
    avg_latency_ms: Math.round(avgLatencyMs * 100) / 100,
    uptime_seconds: uptimeSeconds,
    last_evaluation_at: lastEvaluationAt,
    slos_evaluated: slosEvaluated,
    sla_breaches_detected: slaBreachesDetected,
  };
}

/**
 * Reset metrics (for testing)
 */
export function resetMetrics(): void {
  evaluationsTotal = 0;
  violationsDetected = 0;
  errorsTotal = 0;
  totalLatencyMs = 0;
  latencyCount = 0;
  slosEvaluated = 0;
  slaBreachesDetected = 0;
  lastEvaluationAt = undefined;
}

/**
 * Format metrics for logging
 */
export function formatMetrics(): string {
  const metrics = getMetrics();
  return [
    `evaluations=${metrics.evaluations_total}`,
    `violations=${metrics.violations_detected}`,
    `errors=${metrics.errors_total}`,
    `avg_latency_ms=${metrics.avg_latency_ms.toFixed(2)}`,
    `uptime_s=${metrics.uptime_seconds}`,
    `sla_breaches=${metrics.sla_breaches_detected}`,
  ].join(' ');
}

/**
 * Start periodic metrics logging
 */
export function startMetricsLogger(): NodeJS.Timer | undefined {
  const config = loadConfig();

  if (!config.telemetry.enabled) {
    return undefined;
  }

  return setInterval(() => {
    console.log(`[metrics] ${formatMetrics()}`);
  }, config.telemetry.metricsInterval);
}
