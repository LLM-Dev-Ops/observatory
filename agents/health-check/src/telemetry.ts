/**
 * Health Check Agent - Telemetry & Self-Observation
 *
 * Provides self-observation capabilities for the agent itself.
 * This is allowed per constitution: "self-observing allowed"
 */

import { randomUUID } from 'crypto';
import { AGENT_ID, AGENT_VERSION } from '../contracts/schemas.js';

// ============================================================================
// METRICS TRACKING
// ============================================================================

interface Metrics {
  evaluation_count: number;
  evaluation_errors: number;
  total_processing_time_ms: number;
  total_targets_evaluated: number;
  decision_events_persisted: number;
  decision_events_failed: number;
}

const metrics: Metrics = {
  evaluation_count: 0,
  evaluation_errors: 0,
  total_processing_time_ms: 0,
  total_targets_evaluated: 0,
  decision_events_persisted: 0,
  decision_events_failed: 0,
};

// ============================================================================
// SPAN TRACKING
// ============================================================================

export interface Span {
  traceId: string;
  spanId: string;
  executionRef: string;
  startTime: number;
  operation: string;
  attributes: Record<string, unknown>;
}

const activeSpans = new Map<string, Span>();

/**
 * Start a new telemetry span.
 */
export function startSpan(operation: string, attributes?: Record<string, unknown>): Span {
  const spanId = randomUUID().split('-')[0]!;
  const traceId = randomUUID();
  const executionRef = `${AGENT_ID}:${Date.now()}:${spanId}`;

  const span: Span = {
    traceId,
    spanId,
    executionRef,
    startTime: Date.now(),
    operation,
    attributes: {
      'agent.id': AGENT_ID,
      'agent.version': AGENT_VERSION,
      ...attributes,
    },
  };

  activeSpans.set(spanId, span);
  return span;
}

/**
 * End a telemetry span.
 */
export function endSpan(span: Span, success: boolean, additionalAttributes?: Record<string, unknown>): void {
  const duration = Date.now() - span.startTime;

  // Update metrics
  metrics.evaluation_count++;
  metrics.total_processing_time_ms += duration;

  if (!success) {
    metrics.evaluation_errors++;
  }

  // Log the span (structured JSON for Cloud Functions)
  const logEntry = {
    severity: success ? 'INFO' : 'ERROR',
    message: `${span.operation} ${success ? 'completed' : 'failed'}`,
    'logging.googleapis.com/trace': span.traceId,
    'logging.googleapis.com/spanId': span.spanId,
    agent_id: AGENT_ID,
    agent_version: AGENT_VERSION,
    operation: span.operation,
    execution_ref: span.executionRef,
    duration_ms: duration,
    success,
    ...span.attributes,
    ...additionalAttributes,
  };

  console.log(JSON.stringify(logEntry));

  activeSpans.delete(span.spanId);
}

// ============================================================================
// METRICS RECORDING
// ============================================================================

/**
 * Record evaluation metrics.
 */
export function recordEvaluationMetrics(
  targetsEvaluated: number,
  processingTimeMs: number,
  success: boolean
): void {
  metrics.total_targets_evaluated += targetsEvaluated;
  metrics.total_processing_time_ms += processingTimeMs;

  if (!success) {
    metrics.evaluation_errors++;
  }
}

/**
 * Record decision event persistence metrics.
 */
export function recordPersistenceMetrics(success: boolean): void {
  if (success) {
    metrics.decision_events_persisted++;
  } else {
    metrics.decision_events_failed++;
  }
}

// ============================================================================
// METRICS EXPORT
// ============================================================================

/**
 * Get current metrics snapshot.
 */
export function getMetrics(): Metrics & { derived: Record<string, number> } {
  const avgProcessingTime = metrics.evaluation_count > 0
    ? metrics.total_processing_time_ms / metrics.evaluation_count
    : 0;

  const errorRate = metrics.evaluation_count > 0
    ? (metrics.evaluation_errors / metrics.evaluation_count) * 100
    : 0;

  const persistenceSuccessRate = (metrics.decision_events_persisted + metrics.decision_events_failed) > 0
    ? (metrics.decision_events_persisted / (metrics.decision_events_persisted + metrics.decision_events_failed)) * 100
    : 100;

  return {
    ...metrics,
    derived: {
      avg_processing_time_ms: Math.round(avgProcessingTime * 100) / 100,
      error_rate_percentage: Math.round(errorRate * 100) / 100,
      persistence_success_rate_percentage: Math.round(persistenceSuccessRate * 100) / 100,
    },
  };
}

/**
 * Export metrics in Prometheus format.
 */
export function getPrometheusMetrics(): string {
  const m = metrics;
  const prefix = 'health_check_agent';

  return `
# HELP ${prefix}_evaluations_total Total number of health evaluations
# TYPE ${prefix}_evaluations_total counter
${prefix}_evaluations_total ${m.evaluation_count}

# HELP ${prefix}_evaluation_errors_total Total number of evaluation errors
# TYPE ${prefix}_evaluation_errors_total counter
${prefix}_evaluation_errors_total ${m.evaluation_errors}

# HELP ${prefix}_targets_evaluated_total Total number of targets evaluated
# TYPE ${prefix}_targets_evaluated_total counter
${prefix}_targets_evaluated_total ${m.total_targets_evaluated}

# HELP ${prefix}_processing_time_ms_total Total processing time in milliseconds
# TYPE ${prefix}_processing_time_ms_total counter
${prefix}_processing_time_ms_total ${m.total_processing_time_ms}

# HELP ${prefix}_decision_events_persisted_total Total decision events persisted
# TYPE ${prefix}_decision_events_persisted_total counter
${prefix}_decision_events_persisted_total ${m.decision_events_persisted}

# HELP ${prefix}_decision_events_failed_total Total decision events that failed to persist
# TYPE ${prefix}_decision_events_failed_total counter
${prefix}_decision_events_failed_total ${m.decision_events_failed}

# HELP ${prefix}_active_spans Current number of active spans
# TYPE ${prefix}_active_spans gauge
${prefix}_active_spans ${activeSpans.size}
`.trim();
}

// ============================================================================
// RESET METRICS (for testing)
// ============================================================================

/**
 * Reset all metrics (for testing purposes).
 */
export function resetMetrics(): void {
  metrics.evaluation_count = 0;
  metrics.evaluation_errors = 0;
  metrics.total_processing_time_ms = 0;
  metrics.total_targets_evaluated = 0;
  metrics.decision_events_persisted = 0;
  metrics.decision_events_failed = 0;
  activeSpans.clear();
}

// ============================================================================
// STRUCTURED LOGGING
// ============================================================================

type LogLevel = 'DEBUG' | 'INFO' | 'WARNING' | 'ERROR';

interface LogEntry {
  severity: LogLevel;
  message: string;
  agent_id: string;
  agent_version: string;
  [key: string]: unknown;
}

/**
 * Log a structured message (compatible with Cloud Functions).
 */
export function log(
  level: LogLevel,
  message: string,
  attributes?: Record<string, unknown>
): void {
  const entry: LogEntry = {
    severity: level,
    message,
    agent_id: AGENT_ID,
    agent_version: AGENT_VERSION,
    timestamp: new Date().toISOString(),
    ...attributes,
  };

  console.log(JSON.stringify(entry));
}

/**
 * Log at DEBUG level.
 */
export function debug(message: string, attributes?: Record<string, unknown>): void {
  log('DEBUG', message, attributes);
}

/**
 * Log at INFO level.
 */
export function info(message: string, attributes?: Record<string, unknown>): void {
  log('INFO', message, attributes);
}

/**
 * Log at WARNING level.
 */
export function warn(message: string, attributes?: Record<string, unknown>): void {
  log('WARNING', message, attributes);
}

/**
 * Log at ERROR level.
 */
export function error(message: string, attributes?: Record<string, unknown>): void {
  log('ERROR', message, attributes);
}
