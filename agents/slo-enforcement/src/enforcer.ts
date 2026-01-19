/**
 * SLO/SLA Enforcement Agent - Core Violation Detection Logic
 *
 * This module implements the core SLO/SLA violation detection algorithm.
 *
 * Classification: ENFORCEMENT-CLASS, NON-ACTUATING
 *
 * This agent:
 * - Evaluates telemetry metrics against SLO definitions
 * - Detects breaches and near-breaches
 * - Calculates confidence scores
 * - Generates structured violation events
 *
 * This agent MUST NOT:
 * - Trigger alerts directly
 * - Initiate remediation
 * - Change policies or thresholds at runtime
 * - Modify system state in any way
 */

import { randomUUID } from 'crypto';
import { loadConfig } from './config';
import type {
  SloDefinition,
  TelemetryMetric,
  SloViolation,
  SloStatus,
  MetricContext,
  ViolationSeverity,
  BreachType,
  EnforcementResult,
} from '../contracts';
import type {
  EvaluationContext,
  EvaluationResult,
  ConfidenceFactors,
  HistoricalContext,
} from '../types';

/**
 * SLO Enforcer - Core violation detection engine
 */
export class SloEnforcer {
  private readonly config = loadConfig();

  /**
   * Evaluate all SLOs against provided metrics
   */
  evaluateAll(
    sloDefinitions: SloDefinition[],
    metrics: TelemetryMetric[],
    evaluationTime: Date,
    historicalContext?: Map<string, HistoricalContext>
  ): EnforcementResult {
    const startTime = Date.now();
    const violations: SloViolation[] = [];
    const sloStatuses: SloStatus[] = [];
    let metricsEvaluated = 0;

    for (const slo of sloDefinitions) {
      if (!slo.enabled) continue;

      // Find matching metrics for this SLO
      const matchingMetrics = this.findMatchingMetrics(slo, metrics);
      metricsEvaluated += matchingMetrics.length;

      if (matchingMetrics.length === 0) {
        // No metrics available for this SLO
        sloStatuses.push(this.buildUnknownStatus(slo));
        continue;
      }

      // Evaluate each metric against the SLO
      for (const metric of matchingMetrics) {
        const context: EvaluationContext = {
          slo,
          metric,
          evaluation_time: evaluationTime,
          historical_context: historicalContext?.get(slo.slo_id),
        };

        const result = this.evaluateSingle(context);

        if (result.violation) {
          violations.push(result.violation);
        }

        // Update status (last metric wins for now)
        const existingStatusIdx = sloStatuses.findIndex(s => s.slo_id === slo.slo_id);
        if (existingStatusIdx >= 0) {
          sloStatuses[existingStatusIdx] = result.status;
        } else {
          sloStatuses.push(result.status);
        }
      }
    }

    return {
      violations,
      slo_statuses: sloStatuses,
      evaluation_time: evaluationTime.toISOString(),
      metrics_evaluated: metricsEvaluated,
      slos_evaluated: sloDefinitions.filter(s => s.enabled).length,
      processing_time_ms: Date.now() - startTime,
    };
  }

  /**
   * Evaluate a single SLO against a single metric
   */
  evaluateSingle(context: EvaluationContext): EvaluationResult {
    const { slo, metric, evaluation_time, historical_context } = context;

    // Check if threshold is breached
    const isBreach = this.isThresholdBreached(slo, metric.value);

    // Check if near breach (warning threshold)
    const warningThresholdPct = slo.warning_threshold_percentage ??
      this.config.evaluation.defaultWarningThresholdPct;
    const warningThreshold = this.calculateWarningThreshold(slo, warningThresholdPct);
    const isNearBreach = !isBreach && this.isThresholdBreached(
      { ...slo, threshold: warningThreshold },
      metric.value
    );

    // Calculate metric context
    const metricContext = this.buildMetricContext(slo, metric, historical_context);

    // Calculate confidence
    const confidence = this.calculateConfidence(metric, historical_context);

    // Determine breach type
    const breachType = this.determineBreachType(slo, isBreach, isNearBreach, historical_context);

    // Determine severity
    const severity = this.determineSeverity(slo, metricContext, breachType);

    // Build violation if applicable
    let violation: SloViolation | undefined;
    if (isBreach || isNearBreach) {
      violation = {
        violation_id: randomUUID(),
        slo_id: slo.slo_id,
        slo_name: slo.name,
        breach_type: breachType,
        severity,
        indicator: slo.indicator,
        metric_context: metricContext,
        is_sla: slo.is_sla,
        sla_penalty_tier: slo.is_sla ? slo.sla_penalty_tier : undefined,
        detected_at: evaluation_time.toISOString(),
        window: slo.window,
        provider: metric.provider ?? slo.provider,
        model: metric.model ?? slo.model,
        environment: metric.environment ?? slo.environment,
        recommendation: this.generateRecommendation(slo, metricContext, breachType),
      };
    }

    // Build status
    const status = this.buildStatus(slo, metric, isBreach, isNearBreach, historical_context);

    return {
      slo,
      metric,
      is_violated: isBreach,
      is_near_breach: isNearBreach,
      violation,
      status,
      confidence,
    };
  }

  /**
   * Find metrics that match an SLO's filters
   */
  private findMatchingMetrics(slo: SloDefinition, metrics: TelemetryMetric[]): TelemetryMetric[] {
    return metrics.filter(metric => {
      // Must match indicator type
      if (metric.indicator !== slo.indicator) return false;

      // Must match window
      if (metric.window !== slo.window) return false;

      // Optional filters
      if (slo.provider && metric.provider !== slo.provider) return false;
      if (slo.model && metric.model !== slo.model) return false;
      if (slo.environment && metric.environment !== slo.environment) return false;

      return true;
    });
  }

  /**
   * Check if a threshold is breached based on operator
   */
  private isThresholdBreached(slo: SloDefinition, value: number): boolean {
    switch (slo.operator) {
      case 'lt':
        return value >= slo.threshold;
      case 'lte':
        return value > slo.threshold;
      case 'gt':
        return value <= slo.threshold;
      case 'gte':
        return value < slo.threshold;
      case 'eq':
        return value !== slo.threshold;
      case 'neq':
        return value === slo.threshold;
      default:
        return false;
    }
  }

  /**
   * Calculate the warning threshold based on percentage
   */
  private calculateWarningThreshold(slo: SloDefinition, warningPct: number): number {
    // For upper-bound SLOs (lt, lte), warning is at X% of threshold
    // For lower-bound SLOs (gt, gte), warning is at (100 + (100 - X))% of threshold
    const factor = warningPct / 100;

    switch (slo.operator) {
      case 'lt':
      case 'lte':
        return slo.threshold * factor;
      case 'gt':
      case 'gte':
        return slo.threshold / factor;
      default:
        return slo.threshold;
    }
  }

  /**
   * Build metric context for a violation
   */
  private buildMetricContext(
    slo: SloDefinition,
    metric: TelemetryMetric,
    historical?: HistoricalContext
  ): MetricContext {
    const deviation = this.calculateDeviation(slo, metric.value);

    return {
      current_value: metric.value,
      threshold_value: slo.threshold,
      deviation_percentage: deviation,
      trend: historical?.trend ?? 'stable',
      samples_in_window: metric.sample_count ?? 1,
      historical_average: historical?.average,
      historical_p95: historical?.p95,
      previous_breaches_in_window: historical?.previous_breaches ?? 0,
    };
  }

  /**
   * Calculate deviation from threshold as percentage
   */
  private calculateDeviation(slo: SloDefinition, value: number): number {
    if (slo.threshold === 0) return value > 0 ? 100 : 0;

    switch (slo.operator) {
      case 'lt':
      case 'lte':
        // For upper bounds, positive deviation = over threshold
        return ((value - slo.threshold) / slo.threshold) * 100;
      case 'gt':
      case 'gte':
        // For lower bounds, positive deviation = under threshold
        return ((slo.threshold - value) / slo.threshold) * 100;
      default:
        return Math.abs((value - slo.threshold) / slo.threshold) * 100;
    }
  }

  /**
   * Calculate confidence score for the evaluation
   */
  calculateConfidence(
    metric: TelemetryMetric,
    historical?: HistoricalContext
  ): number {
    const factors = this.calculateConfidenceFactors(metric, historical);

    // Weighted average of factors
    const weights = {
      sample_size: 0.3,
      freshness: 0.3,
      consistency: 0.25,
      coverage: 0.15,
    };

    const confidence =
      factors.sample_size_factor * weights.sample_size +
      factors.data_freshness_factor * weights.freshness +
      factors.consistency_factor * weights.consistency +
      factors.coverage_factor * weights.coverage;

    // Clamp to configured minimum
    return Math.max(this.config.confidence.minConfidence, Math.min(1, confidence));
  }

  /**
   * Calculate individual confidence factors
   */
  private calculateConfidenceFactors(
    metric: TelemetryMetric,
    historical?: HistoricalContext
  ): ConfidenceFactors {
    const config = this.config.confidence;

    // Sample size factor
    const sampleCount = metric.sample_count ?? 1;
    const sample_size_factor = Math.min(1, sampleCount / config.minSampleSize);

    // Data freshness factor
    const metricAge = Date.now() - new Date(metric.timestamp).getTime();
    const data_freshness_factor = Math.max(0, 1 - metricAge / config.maxDataAgeMs);

    // Consistency factor (based on trend volatility)
    let consistency_factor = 0.8; // Default for no historical data
    if (historical) {
      consistency_factor = historical.trend === 'volatile'
        ? 0.5
        : historical.trend === 'stable'
          ? 1.0
          : 0.8;
    }

    // Coverage factor (do we have all expected data)
    const coverage_factor = sampleCount > 0 ? 1.0 : 0.5;

    return {
      sample_size_factor,
      data_freshness_factor,
      consistency_factor,
      coverage_factor,
    };
  }

  /**
   * Determine the breach type
   */
  private determineBreachType(
    slo: SloDefinition,
    isBreach: boolean,
    isNearBreach: boolean,
    historical?: HistoricalContext
  ): BreachType {
    if (!isBreach && isNearBreach) {
      return 'near_breach';
    }

    if (isBreach && historical && historical.previous_breaches > 0) {
      return 'consecutive_breach';
    }

    if (isBreach && slo.is_sla) {
      return 'sla_breach';
    }

    return 'slo_breach';
  }

  /**
   * Determine severity based on context
   */
  private determineSeverity(
    slo: SloDefinition,
    context: MetricContext,
    breachType: BreachType
  ): ViolationSeverity {
    // SLA breaches are always critical
    if (breachType === 'sla_breach') {
      return 'critical';
    }

    // Near breaches are low severity
    if (breachType === 'near_breach') {
      return 'low';
    }

    // Consecutive breaches escalate severity
    if (breachType === 'consecutive_breach') {
      if (context.previous_breaches_in_window >= 3) {
        return 'critical';
      }
      return 'high';
    }

    // Base severity on deviation
    const absDeviation = Math.abs(context.deviation_percentage);
    if (absDeviation > 50) {
      return 'critical';
    }
    if (absDeviation > 25) {
      return 'high';
    }
    if (absDeviation > 10) {
      return 'medium';
    }

    return 'low';
  }

  /**
   * Generate advisory recommendation (ADVISORY ONLY - NO ACTION)
   */
  private generateRecommendation(
    slo: SloDefinition,
    context: MetricContext,
    breachType: BreachType
  ): string {
    // Note: These are advisory recommendations only. This agent does NOT
    // trigger any actions, alerts, or remediation.

    if (breachType === 'near_breach') {
      return `Monitor ${slo.indicator} closely. Current value ${context.current_value} is approaching threshold ${context.threshold_value}.`;
    }

    if (breachType === 'consecutive_breach') {
      return `${slo.indicator} has breached threshold ${context.previous_breaches_in_window + 1} consecutive times. Investigation recommended.`;
    }

    if (breachType === 'sla_breach') {
      return `SLA breach detected for ${slo.name}. Current: ${context.current_value}, SLA: ${context.threshold_value}. Review for potential penalty tier ${slo.sla_penalty_tier ?? 'N/A'}.`;
    }

    return `SLO breach: ${slo.indicator} is ${context.deviation_percentage.toFixed(1)}% ${context.deviation_percentage > 0 ? 'over' : 'under'} threshold.`;
  }

  /**
   * Build SLO status
   */
  private buildStatus(
    slo: SloDefinition,
    metric: TelemetryMetric,
    isBreach: boolean,
    isNearBreach: boolean,
    historical?: HistoricalContext
  ): SloStatus {
    let status: 'healthy' | 'warning' | 'breached' | 'unknown';

    if (isBreach) {
      status = 'breached';
    } else if (isNearBreach) {
      status = 'warning';
    } else {
      status = 'healthy';
    }

    // Calculate compliance (simplified)
    let compliance: number | undefined;
    if (historical && historical.previous_values.length > 0) {
      const total = historical.previous_values.length + 1;
      const breaches = historical.previous_breaches + (isBreach ? 1 : 0);
      compliance = ((total - breaches) / total) * 100;
    }

    return {
      slo_id: slo.slo_id,
      slo_name: slo.name,
      status,
      current_value: metric.value,
      threshold: slo.threshold,
      compliance_percentage: compliance,
      last_breach_at: isBreach
        ? new Date().toISOString()
        : historical?.last_breach_at?.toISOString(),
      consecutive_breach_count: isBreach
        ? (historical?.previous_breaches ?? 0) + 1
        : 0,
    };
  }

  /**
   * Build unknown status when no metrics available
   */
  private buildUnknownStatus(slo: SloDefinition): SloStatus {
    return {
      slo_id: slo.slo_id,
      slo_name: slo.name,
      status: 'unknown',
      threshold: slo.threshold,
      consecutive_breach_count: 0,
    };
  }

  /**
   * Calculate overall confidence for a batch evaluation
   */
  calculateOverallConfidence(results: EvaluationResult[]): number {
    if (results.length === 0) return this.config.confidence.minConfidence;

    const sum = results.reduce((acc, r) => acc + r.confidence, 0);
    return sum / results.length;
  }
}

// Singleton instance
let enforcerInstance: SloEnforcer | null = null;

/**
 * Get the singleton SLO Enforcer instance
 */
export function getSloEnforcer(): SloEnforcer {
  if (enforcerInstance === null) {
    enforcerInstance = new SloEnforcer();
  }
  return enforcerInstance;
}

/**
 * Reset the enforcer instance (for testing)
 */
export function resetSloEnforcer(): void {
  enforcerInstance = null;
}

export default getSloEnforcer;
