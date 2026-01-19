/**
 * Failure Classification Agent - Telemetry (Self-Observation)
 *
 * CONSTITUTIONAL CLASSIFICATION: READ-ONLY, DIAGNOSTIC
 *
 * This module handles self-observation telemetry for the agent.
 * It uses OpenTelemetry for tracing and metrics.
 */

import type { TelemetrySpan } from '../contracts';
import { AGENT_METADATA } from '../contracts';
import { randomUUID } from 'crypto';

// =============================================================================
// SPAN MANAGEMENT
// =============================================================================

interface ActiveSpan {
  name: string;
  trace_id: string;
  span_id: string;
  parent_span_id?: string;
  start_time: string;
  attributes: Record<string, string | number | boolean>;
}

const activeSpans: Map<string, ActiveSpan> = new Map();

/**
 * Start a new telemetry span
 */
export function startSpan(
  name: string,
  traceId?: string,
  parentSpanId?: string
): ActiveSpan {
  const span: ActiveSpan = {
    name,
    trace_id: traceId || randomUUID(),
    span_id: randomUUID(),
    parent_span_id: parentSpanId,
    start_time: new Date().toISOString(),
    attributes: {
      'agent.id': AGENT_METADATA.id,
      'agent.version': AGENT_METADATA.version,
      'agent.classification': AGENT_METADATA.classification,
    },
  };

  activeSpans.set(span.span_id, span);
  return span;
}

/**
 * End a telemetry span
 */
export function endSpan(
  span: ActiveSpan,
  status: 'OK' | 'ERROR' | 'UNSET',
  error?: Error
): TelemetrySpan {
  activeSpans.delete(span.span_id);

  const completedSpan: TelemetrySpan = {
    name: span.name,
    trace_id: span.trace_id,
    span_id: span.span_id,
    parent_span_id: span.parent_span_id,
    start_time: span.start_time,
    end_time: new Date().toISOString(),
    status,
    attributes: {
      ...span.attributes,
      ...(error ? { 'error.message': error.message } : {}),
    },
  };

  // In production, this would send to an OpenTelemetry collector
  if (process.env.SELF_OBSERVATION_ENABLED === 'true') {
    emitSpan(completedSpan);
  }

  return completedSpan;
}

/**
 * Add attributes to a span
 */
export function addSpanAttributes(
  span: ActiveSpan,
  attributes: Record<string, string | number | boolean>
): void {
  span.attributes = { ...span.attributes, ...attributes };
}

// =============================================================================
// TELEMETRY EMISSION
// =============================================================================

interface TelemetryEvent {
  event_type: string;
  timestamp: string;
  agent_id: string;
  agent_version: string;
  data: Record<string, unknown>;
}

/**
 * Emit a telemetry event
 */
export function emitTelemetry(
  eventType: string,
  data: Record<string, unknown>
): void {
  if (process.env.SELF_OBSERVATION_ENABLED !== 'true') {
    return;
  }

  const event: TelemetryEvent = {
    event_type: eventType,
    timestamp: new Date().toISOString(),
    agent_id: AGENT_METADATA.id,
    agent_version: AGENT_METADATA.version,
    data,
  };

  // In production, this would be sent to the telemetry collector
  // For now, we log it in a structured format
  if (process.env.NODE_ENV !== 'test') {
    console.log(JSON.stringify({ telemetry: event }));
  }
}

/**
 * Emit a completed span
 */
function emitSpan(span: TelemetrySpan): void {
  // In production, this would send to OpenTelemetry collector
  // via OTLP HTTP/gRPC
  if (process.env.NODE_ENV !== 'test') {
    console.log(JSON.stringify({ span }));
  }
}

// =============================================================================
// METRICS
// =============================================================================

interface MetricPoint {
  name: string;
  type: 'counter' | 'gauge' | 'histogram';
  value: number;
  labels: Record<string, string>;
  timestamp: string;
}

const metricBuffer: MetricPoint[] = [];

/**
 * Record a counter metric
 */
export function incrementCounter(
  name: string,
  labels: Record<string, string> = {},
  value: number = 1
): void {
  metricBuffer.push({
    name,
    type: 'counter',
    value,
    labels: {
      ...labels,
      agent_id: AGENT_METADATA.id,
      agent_version: AGENT_METADATA.version,
    },
    timestamp: new Date().toISOString(),
  });

  maybeFlushMetrics();
}

/**
 * Record a histogram metric
 */
export function recordHistogram(
  name: string,
  value: number,
  labels: Record<string, string> = {}
): void {
  metricBuffer.push({
    name,
    type: 'histogram',
    value,
    labels: {
      ...labels,
      agent_id: AGENT_METADATA.id,
      agent_version: AGENT_METADATA.version,
    },
    timestamp: new Date().toISOString(),
  });

  maybeFlushMetrics();
}

/**
 * Flush metrics buffer
 */
function maybeFlushMetrics(): void {
  if (metricBuffer.length >= 100) {
    flushMetrics();
  }
}

/**
 * Force flush metrics
 */
export function flushMetrics(): void {
  if (metricBuffer.length === 0) return;

  const metrics = metricBuffer.splice(0, metricBuffer.length);

  // In production, this would send to metrics collector
  if (
    process.env.SELF_OBSERVATION_ENABLED === 'true' &&
    process.env.NODE_ENV !== 'test'
  ) {
    console.log(JSON.stringify({ metrics }));
  }
}

// =============================================================================
// STANDARD METRIC NAMES
// =============================================================================

export const METRICS = {
  CLASSIFICATIONS_TOTAL: 'failure_classifications_total',
  CLASSIFICATION_LATENCY: 'failure_classification_latency_ms',
  CLASSIFICATION_CONFIDENCE: 'failure_classification_confidence',
  CLASSIFICATION_ERRORS: 'failure_classification_errors_total',
  RUVECTOR_REQUESTS: 'ruvector_requests_total',
  RUVECTOR_LATENCY: 'ruvector_request_latency_ms',
  RUVECTOR_ERRORS: 'ruvector_errors_total',
} as const;
