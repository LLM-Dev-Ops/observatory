/**
 * Health Check Agent - Confidence Calculation
 *
 * Calculates statistical confidence for health evaluations based on:
 * 1. Sample size (more samples = higher confidence)
 * 2. Indicator variance (lower variance = higher confidence)
 * 3. Indicator coverage (more indicators = higher confidence)
 * 4. Data freshness (recent data = higher confidence)
 */

import type { HealthIndicator } from '../contracts/schemas.js';

// ============================================================================
// CONFIDENCE FACTORS
// ============================================================================

export interface ConfidenceFactors {
  sample_factor: number;      // Based on total sample size
  coverage_factor: number;    // Based on indicator coverage
  indicator_factor: number;   // Average of individual indicator confidences
  freshness_factor: number;   // Based on data age
  variance_factor: number;    // Based on state consistency
}

// ============================================================================
// SAMPLE SIZE FACTOR
// ============================================================================

/**
 * Calculate confidence factor based on sample size.
 * Uses logarithmic scaling that saturates around 1000 samples.
 *
 * - 10 samples → ~0.33
 * - 100 samples → ~0.67
 * - 1000+ samples → 1.0
 */
export function calculateSampleFactor(totalSampleSize: number): number {
  if (totalSampleSize <= 0) return 0;
  return Math.min(1.0, Math.log10(totalSampleSize + 1) / 3);
}

// ============================================================================
// COVERAGE FACTOR
// ============================================================================

/**
 * Calculate confidence factor based on indicator coverage.
 * More indicators = higher confidence.
 *
 * Expected indicators: latency, error_rate, throughput, saturation, availability
 */
export function calculateCoverageFactor(
  indicatorCount: number,
  expectedIndicators: number = 5
): number {
  if (indicatorCount <= 0) return 0;
  return Math.min(1.0, indicatorCount / expectedIndicators);
}

// ============================================================================
// INDICATOR FACTOR
// ============================================================================

/**
 * Calculate confidence factor based on individual indicator confidences.
 * Uses weighted average of indicator confidences.
 */
export function calculateIndicatorFactor(indicators: HealthIndicator[]): number {
  if (indicators.length === 0) return 0;

  const totalConfidence = indicators.reduce((sum, ind) => sum + ind.confidence, 0);
  return totalConfidence / indicators.length;
}

// ============================================================================
// FRESHNESS FACTOR
// ============================================================================

/**
 * Calculate confidence factor based on data freshness.
 * Data older than maxAcceptableAge results in reduced confidence.
 *
 * @param dataEndTime - End time of the measurement window
 * @param currentTime - Current timestamp
 * @param maxAcceptableAgeMs - Maximum acceptable data age in milliseconds (default: 5 minutes)
 */
export function calculateFreshnessFactor(
  dataEndTime: Date,
  currentTime: Date = new Date(),
  maxAcceptableAgeMs: number = 5 * 60 * 1000
): number {
  const dataAgeMs = currentTime.getTime() - dataEndTime.getTime();

  if (dataAgeMs <= 0) return 1.0; // Data from future or current
  if (dataAgeMs >= maxAcceptableAgeMs) return 0; // Data too old

  return 1 - (dataAgeMs / maxAcceptableAgeMs);
}

// ============================================================================
// VARIANCE FACTOR
// ============================================================================

/**
 * Calculate confidence factor based on state consistency across indicators.
 * If all indicators agree on state, confidence is higher.
 */
export function calculateVarianceFactor(indicators: HealthIndicator[]): number {
  if (indicators.length <= 1) return 1.0;

  // Count states
  const stateCounts = {
    healthy: 0,
    degraded: 0,
    unhealthy: 0,
  };

  for (const ind of indicators) {
    stateCounts[ind.state]++;
  }

  // Find the dominant state
  const maxCount = Math.max(stateCounts.healthy, stateCounts.degraded, stateCounts.unhealthy);
  const agreementRatio = maxCount / indicators.length;

  // Agreement of 100% → 1.0, 50% → 0.5, etc.
  return agreementRatio;
}

// ============================================================================
// COMPOSITE CONFIDENCE CALCULATION
// ============================================================================

export interface ConfidenceInput {
  indicators: HealthIndicator[];
  totalSampleSize: number;
  evaluationWindowEnd: Date;
  currentTime?: Date;
  expectedIndicatorCount?: number;
  maxAcceptableDataAgeMs?: number;
}

export interface ConfidenceResult {
  overall_confidence: number;
  factors: ConfidenceFactors;
}

/**
 * Calculate composite confidence for a health evaluation.
 *
 * Weights:
 * - Sample factor: 25%
 * - Coverage factor: 15%
 * - Indicator factor: 30%
 * - Freshness factor: 15%
 * - Variance factor: 15%
 */
export function calculateConfidence(input: ConfidenceInput): ConfidenceResult {
  const {
    indicators,
    totalSampleSize,
    evaluationWindowEnd,
    currentTime = new Date(),
    expectedIndicatorCount = 5,
    maxAcceptableDataAgeMs = 5 * 60 * 1000,
  } = input;

  // Calculate individual factors
  const factors: ConfidenceFactors = {
    sample_factor: calculateSampleFactor(totalSampleSize),
    coverage_factor: calculateCoverageFactor(indicators.length, expectedIndicatorCount),
    indicator_factor: calculateIndicatorFactor(indicators),
    freshness_factor: calculateFreshnessFactor(evaluationWindowEnd, currentTime, maxAcceptableDataAgeMs),
    variance_factor: calculateVarianceFactor(indicators),
  };

  // Weighted combination
  const overallConfidence =
    factors.sample_factor * 0.25 +
    factors.coverage_factor * 0.15 +
    factors.indicator_factor * 0.30 +
    factors.freshness_factor * 0.15 +
    factors.variance_factor * 0.15;

  // Round to 2 decimal places
  const roundedConfidence = Math.round(overallConfidence * 100) / 100;

  return {
    overall_confidence: roundedConfidence,
    factors,
  };
}

// ============================================================================
// CONFIDENCE CLASSIFICATION
// ============================================================================

export type ConfidenceLevel = 'high' | 'medium' | 'low' | 'insufficient';

/**
 * Classify confidence level for human-readable output.
 */
export function classifyConfidence(confidence: number): ConfidenceLevel {
  if (confidence >= 0.8) return 'high';
  if (confidence >= 0.5) return 'medium';
  if (confidence >= 0.2) return 'low';
  return 'insufficient';
}

/**
 * Check if confidence is sufficient for reliable evaluation.
 */
export function isSufficientConfidence(
  confidence: number,
  threshold: number = 0.2
): boolean {
  return confidence >= threshold;
}
