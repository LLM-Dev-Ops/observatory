/**
 * Post-Mortem Generator Agent - Telemetry & Metrics
 *
 * Self-observation and Prometheus metrics.
 */

import { randomUUID } from 'crypto';

// =============================================================================
// SPAN TRACKING
// =============================================================================

export interface Span {
  spanId: string;
  executionRef: string;
  startTime: number;
  endTime?: number;
  success?: boolean;
  attributes: Record<string, unknown>;
}

const activeSpans = new Map<string, Span>();

export function startSpan(name: string): Span {
  const span: Span = {
    spanId: randomUUID(),
    executionRef: randomUUID(),
    startTime: Date.now(),
    attributes: { name },
  };
  activeSpans.set(span.spanId, span);
  return span;
}

export function endSpan(span: Span, success: boolean, attributes?: Record<string, unknown>): void {
  span.endTime = Date.now();
  span.success = success;
  if (attributes) {
    span.attributes = { ...span.attributes, ...attributes };
  }
  activeSpans.delete(span.spanId);

  // Record metrics
  const durationMs = span.endTime - span.startTime;
  recordGenerationLatency(durationMs);
  if (success) {
    incrementGenerationCount(1);
  } else {
    incrementErrorCount();
  }
}

// =============================================================================
// PROMETHEUS METRICS
// =============================================================================

interface MetricCounter {
  name: string;
  help: string;
  labels: Record<string, number>;
}

interface MetricHistogram {
  name: string;
  help: string;
  buckets: number[];
  values: number[];
  sum: number;
  count: number;
}

const counters: Map<string, MetricCounter> = new Map();
const histograms: Map<string, MetricHistogram> = new Map();

// Initialize metrics
function initMetrics(): void {
  counters.set('postmortem_generations_total', {
    name: 'postmortem_generations_total',
    help: 'Total number of post-mortem reports generated',
    labels: { total: 0 },
  });

  counters.set('postmortem_errors_total', {
    name: 'postmortem_errors_total',
    help: 'Total number of post-mortem generation errors',
    labels: { total: 0 },
  });

  histograms.set('postmortem_generation_latency_ms', {
    name: 'postmortem_generation_latency_ms',
    help: 'Post-mortem generation latency in milliseconds',
    buckets: [10, 50, 100, 250, 500, 1000, 2500, 5000, 10000],
    values: [],
    sum: 0,
    count: 0,
  });

  histograms.set('postmortem_data_completeness', {
    name: 'postmortem_data_completeness',
    help: 'Data completeness score for generated post-mortems',
    buckets: [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0],
    values: [],
    sum: 0,
    count: 0,
  });
}

initMetrics();

export function incrementGenerationCount(count: number): void {
  const counter = counters.get('postmortem_generations_total');
  if (counter) {
    counter.labels.total += count;
  }
}

export function incrementErrorCount(): void {
  const counter = counters.get('postmortem_errors_total');
  if (counter) {
    counter.labels.total += 1;
  }
}

export function recordGenerationLatency(latencyMs: number): void {
  const histogram = histograms.get('postmortem_generation_latency_ms');
  if (histogram) {
    histogram.values.push(latencyMs);
    histogram.sum += latencyMs;
    histogram.count += 1;
  }
}

export function recordDataCompleteness(completeness: number): void {
  const histogram = histograms.get('postmortem_data_completeness');
  if (histogram) {
    histogram.values.push(completeness);
    histogram.sum += completeness;
    histogram.count += 1;
  }
}

export function recordPersistenceMetrics(success: boolean): void {
  // Track persistence success/failure
  if (!success) {
    incrementErrorCount();
  }
}

export function getPrometheusMetrics(): string {
  const lines: string[] = [];

  // Counters
  for (const counter of counters.values()) {
    lines.push(`# HELP ${counter.name} ${counter.help}`);
    lines.push(`# TYPE ${counter.name} counter`);
    for (const [label, value] of Object.entries(counter.labels)) {
      lines.push(`${counter.name}{label="${label}"} ${value}`);
    }
  }

  // Histograms
  for (const histogram of histograms.values()) {
    lines.push(`# HELP ${histogram.name} ${histogram.help}`);
    lines.push(`# TYPE ${histogram.name} histogram`);

    // Calculate bucket counts
    const bucketCounts = histogram.buckets.map((bucket) => {
      return histogram.values.filter((v) => v <= bucket).length;
    });

    for (let i = 0; i < histogram.buckets.length; i++) {
      lines.push(`${histogram.name}_bucket{le="${histogram.buckets[i]}"} ${bucketCounts[i]}`);
    }
    lines.push(`${histogram.name}_bucket{le="+Inf"} ${histogram.count}`);
    lines.push(`${histogram.name}_sum ${histogram.sum}`);
    lines.push(`${histogram.name}_count ${histogram.count}`);
  }

  return lines.join('\n');
}

// =============================================================================
// LOGGING
// =============================================================================

export function info(message: string, context?: Record<string, unknown>): void {
  console.log(JSON.stringify({
    level: 'info',
    message,
    ...context,
    timestamp: new Date().toISOString(),
  }));
}

export function error(message: string, context?: Record<string, unknown>): void {
  console.error(JSON.stringify({
    level: 'error',
    message,
    ...context,
    timestamp: new Date().toISOString(),
  }));
}

export function warn(message: string, context?: Record<string, unknown>): void {
  console.warn(JSON.stringify({
    level: 'warn',
    message,
    ...context,
    timestamp: new Date().toISOString(),
  }));
}
