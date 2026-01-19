/**
 * Self-observation telemetry module
 * Tracks agent metrics using OpenTelemetry patterns
 * Copyright 2025 LLM Observatory Contributors
 * SPDX-License-Identifier: Apache-2.0
 */

import { AgentTelemetryEvent } from './types/schemas.js';
import { createAgentTelemetryEvent } from './emitter.js';

/**
 * Agent metrics tracker (in-memory)
 */
class AgentMetricsTracker {
  private ingestionCount: number = 0;
  private errorCount: number = 0;
  private totalLatencyMs: number = 0;
  private latencyMeasurements: number = 0;
  private startTime: Date = new Date();

  /**
   * Record successful ingestion
   */
  recordIngestion(latencyMs: number): void {
    this.ingestionCount++;
    this.totalLatencyMs += latencyMs;
    this.latencyMeasurements++;
  }

  /**
   * Record error
   */
  recordError(): void {
    this.errorCount++;
  }

  /**
   * Get current metrics
   */
  getMetrics(): {
    ingestionCount: number;
    errorCount: number;
    avgLatencyMs: number;
  } {
    const avgLatencyMs =
      this.latencyMeasurements > 0 ? this.totalLatencyMs / this.latencyMeasurements : 0;

    return {
      ingestionCount: this.ingestionCount,
      errorCount: this.errorCount,
      avgLatencyMs: Math.round(avgLatencyMs * 100) / 100,
    };
  }

  /**
   * Get uptime in seconds
   */
  getUptimeSeconds(): number {
    return Math.floor((Date.now() - this.startTime.getTime()) / 1000);
  }

  /**
   * Reset metrics
   */
  reset(): void {
    this.ingestionCount = 0;
    this.errorCount = 0;
    this.totalLatencyMs = 0;
    this.latencyMeasurements = 0;
    this.startTime = new Date();
  }

  /**
   * Get success rate
   */
  getSuccessRate(): number {
    if (this.ingestionCount === 0) {
      return 1.0;
    }
    return (this.ingestionCount - this.errorCount) / this.ingestionCount;
  }
}

/**
 * Global metrics tracker instance
 */
const metricsTracker = new AgentMetricsTracker();

/**
 * Emit agent telemetry event
 * Per constitution: self-observation is allowed
 */
export function emitAgentTelemetry(executionRef: string): AgentTelemetryEvent {
  const metrics = metricsTracker.getMetrics();
  return createAgentTelemetryEvent(metrics, executionRef);
}

/**
 * Record successful telemetry ingestion
 */
export function recordIngestion(latencyMs: number): void {
  metricsTracker.recordIngestion(latencyMs);
}

/**
 * Record error during ingestion
 */
export function recordError(): void {
  metricsTracker.recordError();
}

/**
 * Get current agent metrics
 */
export function getAgentMetrics(): {
  ingestionCount: number;
  errorCount: number;
  avgLatencyMs: number;
  successRate: number;
  uptimeSeconds: number;
} {
  return {
    ...metricsTracker.getMetrics(),
    successRate: metricsTracker.getSuccessRate(),
    uptimeSeconds: metricsTracker.getUptimeSeconds(),
  };
}

/**
 * Reset agent metrics (useful for testing)
 */
export function resetAgentMetrics(): void {
  metricsTracker.reset();
}

/**
 * OpenTelemetry-compatible span context
 * Simplified version for stateless edge function
 */
export interface SpanContext {
  traceId: string;
  spanId: string;
  executionRef: string;
  startTime: Date;
}

/**
 * Start a telemetry span
 */
export function startSpan(executionRef: string): SpanContext {
  return {
    traceId: generateTraceId(),
    spanId: generateSpanId(),
    executionRef,
    startTime: new Date(),
  };
}

/**
 * End a telemetry span and record metrics
 */
export function endSpan(span: SpanContext, success: boolean): void {
  const endTime = Date.now();
  const startTimeMs = span.startTime.getTime();
  const latencyMs = endTime - startTimeMs;

  if (success) {
    recordIngestion(latencyMs);
  } else {
    recordError();
  }
}

/**
 * Generate trace ID (128-bit hex string)
 */
function generateTraceId(): string {
  const timestamp = Date.now().toString(16).padStart(16, '0');
  const random = Math.random().toString(16).substring(2, 18).padStart(16, '0');
  return timestamp + random;
}

/**
 * Generate span ID (64-bit hex string)
 */
function generateSpanId(): string {
  return Math.random().toString(16).substring(2, 18).padStart(16, '0');
}

/**
 * Create structured log entry
 * Compatible with Cloud Functions logging
 */
export function createLogEntry(
  level: 'INFO' | 'WARN' | 'ERROR',
  message: string,
  context?: Record<string, any>
): Record<string, any> {
  return {
    severity: level,
    message,
    timestamp: new Date().toISOString(),
    ...context,
  };
}

/**
 * Log helper functions
 */
export const logger = {
  info: (message: string, context?: Record<string, any>) => {
    console.log(JSON.stringify(createLogEntry('INFO', message, context)));
  },
  warn: (message: string, context?: Record<string, any>) => {
    console.warn(JSON.stringify(createLogEntry('WARN', message, context)));
  },
  error: (message: string, context?: Record<string, any>) => {
    console.error(JSON.stringify(createLogEntry('ERROR', message, context)));
  },
};
