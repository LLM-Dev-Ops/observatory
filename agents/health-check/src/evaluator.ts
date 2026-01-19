/**
 * Health Check Agent - Main Evaluator
 *
 * Orchestrates the health evaluation process:
 * 1. Extract indicators from telemetry
 * 2. Compute composite health state
 * 3. Apply hysteresis for state stability
 * 4. Calculate confidence
 * 5. Analyze trends (optional)
 */

import { randomUUID } from 'crypto';
import type {
  HealthState,
  HealthTrend,
  HealthIndicator,
  HealthEvaluation,
  TelemetryAggregatesInput,
  TargetSpec,
  EvaluationOptions,
  AggregateStatistics,
  EvaluationWindowSpec,
  StateTransition,
  HealthTrendAnalysis,
} from '../contracts/schemas.js';
import type { Config, IndicatorWeights } from './config.js';
import { extractIndicatorsFromTelemetry } from './indicators.js';
import { evaluateWithHysteresis, type HysteresisState } from './hysteresis.js';
import { calculateConfidence } from './confidence.js';
import { analyzeTrend, aggregateTrends, type TrendDataInput } from './trends.js';

// ============================================================================
// COMPOSITE STATE CALCULATION
// ============================================================================

const STATE_SCORES: Record<HealthState, number> = {
  healthy: 0,
  degraded: 1,
  unhealthy: 2,
};

/**
 * Compute composite health state from individual indicators using weighted voting.
 */
export function computeCompositeState(
  indicators: HealthIndicator[],
  weights: IndicatorWeights
): { state: HealthState; weighted_score: number } {
  if (indicators.length === 0) {
    return { state: 'healthy', weighted_score: 0 };
  }

  let weightedScore = 0;
  let totalWeight = 0;

  for (const indicator of indicators) {
    const weight = weights[indicator.indicator_type] ?? 1.0;
    const score = STATE_SCORES[indicator.state];
    weightedScore += score * weight * indicator.confidence;
    totalWeight += weight * indicator.confidence;
  }

  const avgScore = totalWeight > 0 ? weightedScore / totalWeight : 0;

  // Convert back to state
  let state: HealthState;
  if (avgScore < 0.5) {
    state = 'healthy';
  } else if (avgScore < 1.5) {
    state = 'degraded';
  } else {
    state = 'unhealthy';
  }

  return { state, weighted_score: avgScore };
}

// ============================================================================
// AGGREGATE STATISTICS
// ============================================================================

/**
 * Build aggregate statistics from telemetry.
 */
export function buildAggregateStatistics(
  telemetry: TelemetryAggregatesInput
): AggregateStatistics {
  const errorRate = telemetry.request_count > 0
    ? (telemetry.error_count / telemetry.request_count) * 100
    : 0;

  const availability = 100 - errorRate;

  return {
    total_requests: telemetry.request_count,
    total_errors: telemetry.error_count,
    avg_latency_ms: telemetry.latency_avg_ms,
    error_rate_percentage: Math.round(errorRate * 100) / 100,
    availability_percentage: Math.round(availability * 100) / 100,
    sample_size: telemetry.request_count,
  };
}

// ============================================================================
// EVALUATION WINDOW
// ============================================================================

/**
 * Parse evaluation window string to seconds.
 */
export function parseWindowToSeconds(window: string): number {
  const match = window.match(/^(\d+)(m|h|d)$/);
  if (!match) return 300; // Default 5 minutes

  const value = parseInt(match[1]!, 10);
  const unit = match[2];

  switch (unit) {
    case 'm': return value * 60;
    case 'h': return value * 60 * 60;
    case 'd': return value * 60 * 60 * 24;
    default: return 300;
  }
}

/**
 * Build evaluation window specification.
 */
export function buildEvaluationWindowSpec(
  telemetry: TelemetryAggregatesInput,
  granularity: string
): EvaluationWindowSpec {
  return {
    start: telemetry.window_start,
    end: telemetry.window_end,
    granularity: granularity as EvaluationWindowSpec['granularity'],
  };
}

// ============================================================================
// EVALUATION CONTEXT
// ============================================================================

export interface EvaluationContext {
  target: TargetSpec;
  telemetry: TelemetryAggregatesInput;
  options: EvaluationOptions;
  config: Config;
  previous_hysteresis_state?: HysteresisState;
  historical_data?: TrendDataInput[];
}

// ============================================================================
// MAIN EVALUATION FUNCTION
// ============================================================================

/**
 * Evaluate health for a single target.
 *
 * Process:
 * 1. Extract indicators from telemetry
 * 2. Compute composite state using weighted voting
 * 3. Apply hysteresis to prevent state flapping
 * 4. Calculate statistical confidence
 * 5. Analyze trends (if requested)
 */
export function evaluateHealth(context: EvaluationContext): HealthEvaluation {
  const {
    target,
    telemetry,
    options,
    config,
    previous_hysteresis_state,
    historical_data,
  } = context;

  const currentTimestamp = new Date().toISOString();

  // Step 1: Extract indicators from telemetry
  const indicators = extractIndicatorsFromTelemetry(telemetry, config.thresholds);

  // Step 2: Compute composite state
  const { state: computedState } = computeCompositeState(
    indicators,
    config.indicator_weights
  );

  // Step 3: Apply hysteresis
  const evaluationIntervalSeconds = parseWindowToSeconds(options.evaluation_window);
  const hysteresisResult = evaluateWithHysteresis({
    computed_state: computedState,
    previous_state: previous_hysteresis_state ?? null,
    config: config.hysteresis,
    current_timestamp: currentTimestamp,
    evaluation_interval_seconds: evaluationIntervalSeconds,
  });

  // Step 4: Calculate confidence
  const confidenceResult = calculateConfidence({
    indicators,
    totalSampleSize: telemetry.request_count,
    evaluationWindowEnd: new Date(telemetry.window_end),
    currentTime: new Date(),
  });

  // Step 5: Analyze trends (if requested)
  let trends: HealthTrendAnalysis[] | undefined;
  let overallTrend: HealthTrend = 'stable';

  if (options.include_trends && historical_data && historical_data.length >= 2) {
    // Group historical data by indicator type and analyze each
    // For now, we'll use the provided historical data as-is
    // In production, this would group by indicator type

    const trendAnalysis = analyzeTrend({
      indicator_type: 'latency', // Default to latency for overall trend
      data_points: historical_data,
      predict_ahead_hours: options.include_predictions ? 1 : 0,
    });

    if (trendAnalysis) {
      trends = [trendAnalysis];
      overallTrend = trendAnalysis.trend;
    }
  }

  // If no historical data, infer trend from current state
  if (!trends) {
    overallTrend = inferTrendFromState(hysteresisResult.final_state, previous_hysteresis_state?.current_state);
  }

  // Build aggregate statistics
  const statistics = buildAggregateStatistics(telemetry);

  // Build evaluation window spec
  const evaluationWindow = buildEvaluationWindowSpec(telemetry, options.evaluation_window);

  // Build the evaluation
  const evaluation: HealthEvaluation = {
    evaluation_id: randomUUID(),
    evaluated_at: currentTimestamp,
    target: {
      type: target.type,
      id: target.id,
      name: target.id, // Use ID as name if not provided
    },
    overall_state: hysteresisResult.final_state,
    overall_trend: overallTrend,
    overall_confidence: confidenceResult.overall_confidence,
    state_transition: hysteresisResult.state_transition,
    indicators,
    trends,
    statistics,
    evaluation_window: evaluationWindow,
    schema_version: '1.0.0',
  };

  return evaluation;
}

// ============================================================================
// TREND INFERENCE
// ============================================================================

/**
 * Infer trend from state transition when no historical data is available.
 */
function inferTrendFromState(
  currentState: HealthState,
  previousState?: HealthState
): HealthTrend {
  if (!previousState) return 'stable';

  const currentScore = STATE_SCORES[currentState];
  const previousScore = STATE_SCORES[previousState];

  if (currentScore < previousScore) return 'improving';
  if (currentScore > previousScore) return 'degrading';
  return 'stable';
}

// ============================================================================
// BATCH EVALUATION
// ============================================================================

export interface BatchEvaluationInput {
  targets: TargetSpec[];
  telemetry_map: Map<string, TelemetryAggregatesInput>;
  options: EvaluationOptions;
  config: Config;
  hysteresis_states?: Map<string, HysteresisState>;
}

/**
 * Evaluate health for multiple targets.
 */
export function evaluateHealthBatch(input: BatchEvaluationInput): HealthEvaluation[] {
  const {
    targets,
    telemetry_map,
    options,
    config,
    hysteresis_states,
  } = input;

  const evaluations: HealthEvaluation[] = [];

  for (const target of targets) {
    const targetKey = `${target.type}:${target.id}`;
    const telemetry = telemetry_map.get(targetKey);

    if (!telemetry) {
      // Skip targets without telemetry
      continue;
    }

    const evaluation = evaluateHealth({
      target,
      telemetry,
      options,
      config,
      previous_hysteresis_state: hysteresis_states?.get(targetKey),
    });

    evaluations.push(evaluation);
  }

  return evaluations;
}

// ============================================================================
// EVALUATION SUMMARY
// ============================================================================

export interface EvaluationSummary {
  total_targets: number;
  healthy_count: number;
  degraded_count: number;
  unhealthy_count: number;
  average_confidence: number;
  dominant_trend: HealthTrend;
}

/**
 * Generate summary from multiple evaluations.
 */
export function summarizeEvaluations(evaluations: HealthEvaluation[]): EvaluationSummary {
  if (evaluations.length === 0) {
    return {
      total_targets: 0,
      healthy_count: 0,
      degraded_count: 0,
      unhealthy_count: 0,
      average_confidence: 0,
      dominant_trend: 'stable',
    };
  }

  const summary: EvaluationSummary = {
    total_targets: evaluations.length,
    healthy_count: 0,
    degraded_count: 0,
    unhealthy_count: 0,
    average_confidence: 0,
    dominant_trend: 'stable',
  };

  let totalConfidence = 0;
  const trendCounts: Record<HealthTrend, number> = {
    improving: 0,
    stable: 0,
    degrading: 0,
    volatile: 0,
  };

  for (const evaluation of evaluations) {
    // Count states
    switch (evaluation.overall_state) {
      case 'healthy':
        summary.healthy_count++;
        break;
      case 'degraded':
        summary.degraded_count++;
        break;
      case 'unhealthy':
        summary.unhealthy_count++;
        break;
    }

    // Sum confidence
    totalConfidence += evaluation.overall_confidence;

    // Count trends
    trendCounts[evaluation.overall_trend]++;
  }

  // Calculate average confidence
  summary.average_confidence = Math.round((totalConfidence / evaluations.length) * 100) / 100;

  // Find dominant trend
  let maxCount = 0;
  for (const [trend, count] of Object.entries(trendCounts)) {
    if (count > maxCount) {
      maxCount = count;
      summary.dominant_trend = trend as HealthTrend;
    }
  }

  return summary;
}
