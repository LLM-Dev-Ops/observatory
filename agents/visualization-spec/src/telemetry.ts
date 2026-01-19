/**
 * Visualization Spec Agent - Telemetry Module
 *
 * Provides observability through spans, metrics, and logging.
 * Self-observation is allowed - the agent can emit telemetry about itself.
 */

import { randomUUID } from 'crypto';
import { getConfig } from './config.js';
import { AGENT_ID, AGENT_VERSION } from '../contracts/schemas.js';
import type { Span } from '../contracts/types.js';

// =============================================================================
// Span Management
// =============================================================================

/**
 * Starts a new telemetry span for operation tracking
 */
export function startSpan(operationName: string, parentSpan?: Span): Span {
  const config = getConfig();

  return {
    executionRef: parentSpan?.executionRef ?? randomUUID(),
    spanId: randomUUID(),
    startTime: Date.now(),
    operationName,
    attributes: {
      'agent.id': AGENT_ID,
      'agent.version': AGENT_VERSION,
      'span.prefix': config.telemetry.spanPrefix,
    },
  };
}

/**
 * Ends a span and records metrics
 */
export function endSpan(
  span: Span,
  success: boolean,
  error?: Error | unknown,
  additionalAttributes?: Record<string, unknown>
): void {
  const durationMs = Date.now() - span.startTime;

  // Record span completion
  recordSpanMetrics(span.operationName, durationMs, success);

  // Log span completion
  const logLevel = success ? 'info' : 'error';
  log(logLevel, `Span completed: ${span.operationName}`, {
    executionRef: span.executionRef,
    spanId: span.spanId,
    durationMs,
    success,
    error: error instanceof Error ? error.message : error,
    ...additionalAttributes,
  });
}

// =============================================================================
// Metrics Collection
// =============================================================================

interface MetricsBucket {
  counter: Map<string, number>;
  histogram: Map<string, number[]>;
  gauge: Map<string, number>;
}

const metrics: MetricsBucket = {
  counter: new Map(),
  histogram: new Map(),
  gauge: new Map(),
};

/**
 * Increments a counter metric
 */
export function incrementCounter(name: string, labels?: Record<string, string>, value: number = 1): void {
  const key = formatMetricKey(name, labels);
  const current = metrics.counter.get(key) ?? 0;
  metrics.counter.set(key, current + value);
}

/**
 * Records a histogram observation
 */
export function recordHistogram(name: string, value: number, labels?: Record<string, string>): void {
  const key = formatMetricKey(name, labels);
  const values = metrics.histogram.get(key) ?? [];
  values.push(value);
  metrics.histogram.set(key, values);
}

/**
 * Sets a gauge metric
 */
export function setGauge(name: string, value: number, labels?: Record<string, string>): void {
  const key = formatMetricKey(name, labels);
  metrics.gauge.set(key, value);
}

/**
 * Records span metrics
 */
function recordSpanMetrics(operationName: string, durationMs: number, success: boolean): void {
  const config = getConfig();
  const prefix = config.telemetry.metricsPrefix;

  incrementCounter(`${prefix}_operations_total`, {
    operation: operationName,
    success: String(success),
  });

  recordHistogram(`${prefix}_operation_duration_ms`, durationMs, {
    operation: operationName,
  });
}

/**
 * Records generation metrics
 */
export function recordGenerationMetrics(
  specsGenerated: number,
  durationMs: number,
  success: boolean
): void {
  const config = getConfig();
  const prefix = config.telemetry.metricsPrefix;

  incrementCounter(`${prefix}_specs_generated_total`, {
    success: String(success),
  }, specsGenerated);

  recordHistogram(`${prefix}_generation_duration_ms`, durationMs, {
    success: String(success),
  });
}

/**
 * Records persistence metrics
 */
export function recordPersistenceMetrics(success: boolean, latencyMs?: number): void {
  const config = getConfig();
  const prefix = config.telemetry.metricsPrefix;

  incrementCounter(`${prefix}_persistence_total`, {
    success: String(success),
  });

  if (latencyMs !== undefined) {
    recordHistogram(`${prefix}_persistence_latency_ms`, latencyMs, {
      success: String(success),
    });
  }
}

// =============================================================================
// Prometheus Export
// =============================================================================

/**
 * Generates Prometheus-formatted metrics output
 */
export function getPrometheusMetrics(): string {
  const config = getConfig();

  if (!config.telemetry.prometheusEnabled) {
    return '';
  }

  const lines: string[] = [];
  const prefix = config.telemetry.metricsPrefix;

  // Counter metrics
  for (const [key, value] of metrics.counter.entries()) {
    const { name, labels } = parseMetricKey(key);
    lines.push(`# TYPE ${name} counter`);
    lines.push(`${name}${formatLabels(labels)} ${value}`);
  }

  // Histogram metrics (simplified - just sum and count)
  for (const [key, values] of metrics.histogram.entries()) {
    const { name, labels } = parseMetricKey(key);
    const sum = values.reduce((a, b) => a + b, 0);
    const count = values.length;

    lines.push(`# TYPE ${name} histogram`);
    lines.push(`${name}_sum${formatLabels(labels)} ${sum}`);
    lines.push(`${name}_count${formatLabels(labels)} ${count}`);

    // Calculate percentiles
    if (count > 0) {
      const sorted = [...values].sort((a, b) => a - b);
      const p50 = sorted[Math.floor(count * 0.5)] ?? 0;
      const p95 = sorted[Math.floor(count * 0.95)] ?? 0;
      const p99 = sorted[Math.floor(count * 0.99)] ?? 0;

      lines.push(`${name}{${formatLabelsInner(labels)},quantile="0.5"} ${p50}`);
      lines.push(`${name}{${formatLabelsInner(labels)},quantile="0.95"} ${p95}`);
      lines.push(`${name}{${formatLabelsInner(labels)},quantile="0.99"} ${p99}`);
    }
  }

  // Gauge metrics
  for (const [key, value] of metrics.gauge.entries()) {
    const { name, labels } = parseMetricKey(key);
    lines.push(`# TYPE ${name} gauge`);
    lines.push(`${name}${formatLabels(labels)} ${value}`);
  }

  return lines.join('\n');
}

/**
 * Resets all metrics (for testing)
 */
export function resetMetrics(): void {
  metrics.counter.clear();
  metrics.histogram.clear();
  metrics.gauge.clear();
}

// =============================================================================
// Logging
// =============================================================================

type LogLevel = 'debug' | 'info' | 'warn' | 'error';

const LOG_LEVELS: Record<LogLevel, number> = {
  debug: 0,
  info: 1,
  warn: 2,
  error: 3,
};

/**
 * Logs a message with structured data
 */
export function log(
  level: LogLevel,
  message: string,
  data?: Record<string, unknown>
): void {
  const config = getConfig();
  const configLevel = LOG_LEVELS[config.telemetry.logLevel];
  const messageLevel = LOG_LEVELS[level];

  if (messageLevel < configLevel) {
    return;
  }

  const logEntry = {
    timestamp: new Date().toISOString(),
    level,
    agent_id: AGENT_ID,
    message,
    ...data,
  };

  // Output as JSON for structured logging
  console[level === 'error' ? 'error' : level === 'warn' ? 'warn' : 'log'](
    JSON.stringify(logEntry)
  );
}

// =============================================================================
// Utility Functions
// =============================================================================

/**
 * Formats a metric key with labels
 */
function formatMetricKey(name: string, labels?: Record<string, string>): string {
  if (!labels || Object.keys(labels).length === 0) {
    return name;
  }

  const labelParts = Object.entries(labels)
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([k, v]) => `${k}=${v}`)
    .join(',');

  return `${name}|${labelParts}`;
}

/**
 * Parses a metric key back to name and labels
 */
function parseMetricKey(key: string): { name: string; labels: Record<string, string> } {
  const [name, labelStr] = key.split('|');
  const labels: Record<string, string> = {};

  if (labelStr) {
    for (const pair of labelStr.split(',')) {
      const [k, v] = pair.split('=');
      if (k && v) {
        labels[k] = v;
      }
    }
  }

  return { name, labels };
}

/**
 * Formats labels for Prometheus output
 */
function formatLabels(labels: Record<string, string>): string {
  if (Object.keys(labels).length === 0) {
    return '';
  }

  return `{${formatLabelsInner(labels)}}`;
}

/**
 * Formats labels inner content
 */
function formatLabelsInner(labels: Record<string, string>): string {
  return Object.entries(labels)
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([k, v]) => `${k}="${v}"`)
    .join(',');
}
