/**
 * Health Check Agent - Indicator Evaluation
 *
 * Evaluates individual health indicators against thresholds.
 * Each indicator produces a discrete health state (healthy/degraded/unhealthy).
 */

import type {
  HealthState,
  IndicatorType,
  HealthIndicator,
  MeasurementWindow,
  TelemetryAggregatesInput,
} from '../contracts/schemas.js';
import type { ThresholdConfig } from './config.js';

// ============================================================================
// INDICATOR EVALUATION RESULT
// ============================================================================

export interface IndicatorEvaluationResult {
  state: HealthState;
  reason: string;
}

// ============================================================================
// LATENCY INDICATOR
// ============================================================================

/**
 * Evaluate latency indicator against thresholds.
 * Uses P95 latency as the primary metric.
 */
export function evaluateLatencyIndicator(
  p95_ms: number,
  thresholds: ThresholdConfig['latency']
): IndicatorEvaluationResult {
  if (p95_ms <= thresholds.healthy_max_p95_ms) {
    return {
      state: 'healthy',
      reason: `P95 latency ${p95_ms.toFixed(1)}ms within healthy threshold (≤${thresholds.healthy_max_p95_ms}ms)`,
    };
  }

  if (p95_ms <= thresholds.degraded_max_p95_ms) {
    return {
      state: 'degraded',
      reason: `P95 latency ${p95_ms.toFixed(1)}ms elevated but acceptable (≤${thresholds.degraded_max_p95_ms}ms)`,
    };
  }

  return {
    state: 'unhealthy',
    reason: `P95 latency ${p95_ms.toFixed(1)}ms exceeds critical threshold (>${thresholds.degraded_max_p95_ms}ms)`,
  };
}

// ============================================================================
// ERROR RATE INDICATOR
// ============================================================================

/**
 * Evaluate error rate indicator against thresholds.
 */
export function evaluateErrorRateIndicator(
  error_rate_percentage: number,
  thresholds: ThresholdConfig['error_rate']
): IndicatorEvaluationResult {
  if (error_rate_percentage <= thresholds.healthy_max_percentage) {
    return {
      state: 'healthy',
      reason: `Error rate ${error_rate_percentage.toFixed(2)}% within healthy threshold (≤${thresholds.healthy_max_percentage}%)`,
    };
  }

  if (error_rate_percentage <= thresholds.degraded_max_percentage) {
    return {
      state: 'degraded',
      reason: `Error rate ${error_rate_percentage.toFixed(2)}% elevated but acceptable (≤${thresholds.degraded_max_percentage}%)`,
    };
  }

  return {
    state: 'unhealthy',
    reason: `Error rate ${error_rate_percentage.toFixed(2)}% exceeds critical threshold (>${thresholds.degraded_max_percentage}%)`,
  };
}

// ============================================================================
// THROUGHPUT INDICATOR
// ============================================================================

/**
 * Evaluate throughput indicator against thresholds.
 * Lower throughput indicates potential issues.
 */
export function evaluateThroughputIndicator(
  rps: number,
  thresholds: ThresholdConfig['throughput']
): IndicatorEvaluationResult {
  if (rps >= thresholds.healthy_min_rps) {
    return {
      state: 'healthy',
      reason: `Throughput ${rps.toFixed(2)} req/s within healthy threshold (≥${thresholds.healthy_min_rps} req/s)`,
    };
  }

  if (rps >= thresholds.degraded_min_rps) {
    return {
      state: 'degraded',
      reason: `Throughput ${rps.toFixed(2)} req/s reduced but acceptable (≥${thresholds.degraded_min_rps} req/s)`,
    };
  }

  return {
    state: 'unhealthy',
    reason: `Throughput ${rps.toFixed(2)} req/s below critical threshold (<${thresholds.degraded_min_rps} req/s)`,
  };
}

// ============================================================================
// SATURATION INDICATOR
// ============================================================================

/**
 * Evaluate saturation indicator against thresholds.
 * Higher saturation indicates potential capacity issues.
 */
export function evaluateSaturationIndicator(
  saturation_percentage: number,
  thresholds: ThresholdConfig['saturation']
): IndicatorEvaluationResult {
  if (saturation_percentage <= thresholds.healthy_max_percentage) {
    return {
      state: 'healthy',
      reason: `Resource saturation ${saturation_percentage.toFixed(1)}% within healthy threshold (≤${thresholds.healthy_max_percentage}%)`,
    };
  }

  if (saturation_percentage <= thresholds.degraded_max_percentage) {
    return {
      state: 'degraded',
      reason: `Resource saturation ${saturation_percentage.toFixed(1)}% elevated but manageable (≤${thresholds.degraded_max_percentage}%)`,
    };
  }

  return {
    state: 'unhealthy',
    reason: `Resource saturation ${saturation_percentage.toFixed(1)}% exceeds critical threshold (>${thresholds.degraded_max_percentage}%)`,
  };
}

// ============================================================================
// AVAILABILITY INDICATOR
// ============================================================================

/**
 * Evaluate availability indicator against thresholds.
 * Lower availability indicates service degradation.
 */
export function evaluateAvailabilityIndicator(
  availability_percentage: number,
  thresholds: ThresholdConfig['availability']
): IndicatorEvaluationResult {
  if (availability_percentage >= thresholds.healthy_min_percentage) {
    return {
      state: 'healthy',
      reason: `Availability ${availability_percentage.toFixed(2)}% within healthy threshold (≥${thresholds.healthy_min_percentage}%)`,
    };
  }

  if (availability_percentage >= thresholds.degraded_min_percentage) {
    return {
      state: 'degraded',
      reason: `Availability ${availability_percentage.toFixed(2)}% reduced but acceptable (≥${thresholds.degraded_min_percentage}%)`,
    };
  }

  return {
    state: 'unhealthy',
    reason: `Availability ${availability_percentage.toFixed(2)}% below critical threshold (<${thresholds.degraded_min_percentage}%)`,
  };
}

// ============================================================================
// GENERIC INDICATOR EVALUATION
// ============================================================================

/**
 * Evaluate any indicator type against thresholds.
 */
export function evaluateIndicator(
  type: IndicatorType,
  value: number,
  thresholds: ThresholdConfig
): IndicatorEvaluationResult {
  switch (type) {
    case 'latency':
      return evaluateLatencyIndicator(value, thresholds.latency);
    case 'error_rate':
      return evaluateErrorRateIndicator(value, thresholds.error_rate);
    case 'throughput':
      return evaluateThroughputIndicator(value, thresholds.throughput);
    case 'saturation':
      return evaluateSaturationIndicator(value, thresholds.saturation);
    case 'availability':
      return evaluateAvailabilityIndicator(value, thresholds.availability);
    default:
      // TypeScript exhaustiveness check
      const _exhaustive: never = type;
      throw new Error(`Unknown indicator type: ${_exhaustive}`);
  }
}

// ============================================================================
// CONFIDENCE CALCULATION FOR INDIVIDUAL INDICATOR
// ============================================================================

/**
 * Calculate confidence for an individual indicator based on sample size.
 * Uses logarithmic scaling that saturates around 1000 samples.
 */
export function calculateIndicatorConfidence(sampleSize: number): number {
  if (sampleSize <= 0) return 0;

  // Logarithmic scaling: log10(samples + 1) / 3
  // - 10 samples → ~0.33
  // - 100 samples → ~0.67
  // - 1000 samples → 1.0
  const confidence = Math.min(1.0, Math.log10(sampleSize + 1) / 3);
  return Math.round(confidence * 100) / 100;
}

// ============================================================================
// BUILD HEALTH INDICATOR FROM TELEMETRY
// ============================================================================

/**
 * Build a HealthIndicator object from telemetry aggregates.
 */
export function buildHealthIndicator(
  type: IndicatorType,
  value: number,
  unit: string,
  sampleSize: number,
  thresholds: ThresholdConfig,
  measurementWindow: MeasurementWindow
): HealthIndicator {
  const evaluation = evaluateIndicator(type, value, thresholds);
  const confidence = calculateIndicatorConfidence(sampleSize);

  return {
    indicator_type: type,
    current_value: value,
    unit,
    state: evaluation.state,
    state_reason: evaluation.reason,
    sample_size: sampleSize,
    confidence,
    measurement_window: measurementWindow,
  };
}

// ============================================================================
// EXTRACT INDICATORS FROM TELEMETRY AGGREGATES
// ============================================================================

/**
 * Extract all available health indicators from telemetry aggregates.
 */
export function extractIndicatorsFromTelemetry(
  telemetry: TelemetryAggregatesInput,
  thresholds: ThresholdConfig
): HealthIndicator[] {
  const indicators: HealthIndicator[] = [];

  // Calculate measurement window
  const windowStart = new Date(telemetry.window_start);
  const windowEnd = new Date(telemetry.window_end);
  const durationSeconds = Math.round((windowEnd.getTime() - windowStart.getTime()) / 1000);

  const measurementWindow: MeasurementWindow = {
    start: telemetry.window_start,
    end: telemetry.window_end,
    duration_seconds: durationSeconds,
  };

  // Latency indicator (use P95 if available, fall back to average)
  const latencyValue = telemetry.latency_p95_ms ?? telemetry.latency_avg_ms;
  indicators.push(
    buildHealthIndicator(
      'latency',
      latencyValue,
      'ms',
      telemetry.request_count,
      thresholds,
      measurementWindow
    )
  );

  // Error rate indicator
  const errorRate = telemetry.request_count > 0
    ? (telemetry.error_count / telemetry.request_count) * 100
    : 0;
  indicators.push(
    buildHealthIndicator(
      'error_rate',
      errorRate,
      'percentage',
      telemetry.request_count,
      thresholds,
      measurementWindow
    )
  );

  // Throughput indicator
  const throughput = durationSeconds > 0
    ? telemetry.request_count / durationSeconds
    : 0;
  indicators.push(
    buildHealthIndicator(
      'throughput',
      throughput,
      'req/s',
      telemetry.request_count,
      thresholds,
      measurementWindow
    )
  );

  // Availability indicator (calculated as 100% - error rate)
  const availability = 100 - errorRate;
  indicators.push(
    buildHealthIndicator(
      'availability',
      availability,
      'percentage',
      telemetry.request_count,
      thresholds,
      measurementWindow
    )
  );

  return indicators;
}

// ============================================================================
// STATE COMPARISON UTILITIES
// ============================================================================

const STATE_SEVERITY: Record<HealthState, number> = {
  healthy: 0,
  degraded: 1,
  unhealthy: 2,
};

/**
 * Check if state1 is worse than state2.
 */
export function isWorse(state1: HealthState, state2: HealthState): boolean {
  return STATE_SEVERITY[state1] > STATE_SEVERITY[state2];
}

/**
 * Check if state1 is better than state2.
 */
export function isBetter(state1: HealthState, state2: HealthState): boolean {
  return STATE_SEVERITY[state1] < STATE_SEVERITY[state2];
}

/**
 * Get the worst state from a list of states.
 */
export function getWorstState(states: HealthState[]): HealthState {
  if (states.length === 0) return 'healthy';

  let worst: HealthState = 'healthy';
  for (const state of states) {
    if (isWorse(state, worst)) {
      worst = state;
    }
  }
  return worst;
}
