/**
 * Health Check Agent - Trend Analysis
 *
 * Analyzes health trends over time using linear regression
 * to determine if health is improving, stable, degrading, or volatile.
 */

import type {
  HealthState,
  HealthTrend,
  IndicatorType,
  HealthTrendAnalysis,
  TrendDataPoint,
} from '../contracts/schemas.js';

// ============================================================================
// LINEAR REGRESSION
// ============================================================================

interface LinearRegressionResult {
  slope: number;
  intercept: number;
  r_squared: number;
}

/**
 * Perform simple linear regression on a set of data points.
 * Returns slope, intercept, and R² (goodness of fit).
 */
export function linearRegression(
  xValues: number[],
  yValues: number[]
): LinearRegressionResult {
  if (xValues.length !== yValues.length || xValues.length < 2) {
    return { slope: 0, intercept: 0, r_squared: 0 };
  }

  const n = xValues.length;

  // Calculate means
  const xMean = xValues.reduce((a, b) => a + b, 0) / n;
  const yMean = yValues.reduce((a, b) => a + b, 0) / n;

  // Calculate slope and intercept
  let numerator = 0;
  let denominator = 0;
  let ssTotal = 0;

  for (let i = 0; i < n; i++) {
    const xDiff = xValues[i]! - xMean;
    const yDiff = yValues[i]! - yMean;
    numerator += xDiff * yDiff;
    denominator += xDiff * xDiff;
    ssTotal += yDiff * yDiff;
  }

  const slope = denominator !== 0 ? numerator / denominator : 0;
  const intercept = yMean - slope * xMean;

  // Calculate R² (coefficient of determination)
  let ssResidual = 0;
  for (let i = 0; i < n; i++) {
    const predicted = slope * xValues[i]! + intercept;
    const residual = yValues[i]! - predicted;
    ssResidual += residual * residual;
  }

  const r_squared = ssTotal !== 0 ? 1 - (ssResidual / ssTotal) : 0;

  return { slope, intercept, r_squared };
}

// ============================================================================
// TREND DETERMINATION
// ============================================================================

interface TrendDeterminationConfig {
  improving_slope_threshold: number;  // Slope must be less than this for improving (negative for "lower is better" metrics)
  degrading_slope_threshold: number;  // Slope must be greater than this for degrading
  stable_r_squared_min: number;       // R² must be at least this for stable classification
  volatile_r_squared_max: number;     // R² below this indicates volatility
}

const DEFAULT_TREND_CONFIG: TrendDeterminationConfig = {
  improving_slope_threshold: -0.01,  // 1% improvement per unit time
  degrading_slope_threshold: 0.01,   // 1% degradation per unit time
  stable_r_squared_min: 0.7,         // 70% of variance explained
  volatile_r_squared_max: 0.3,       // Less than 30% explained = volatile
};

/**
 * Determine health trend based on slope and R².
 *
 * @param slope - Rate of change (positive = increasing value)
 * @param r_squared - Goodness of fit (0-1)
 * @param higherIsBetter - If true, positive slope means improving; if false, negative slope means improving
 */
export function determineTrend(
  slope: number,
  r_squared: number,
  higherIsBetter: boolean,
  config: TrendDeterminationConfig = DEFAULT_TREND_CONFIG
): HealthTrend {
  // Check for volatility first
  if (r_squared < config.volatile_r_squared_max) {
    return 'volatile';
  }

  // Normalize slope based on whether higher is better
  const normalizedSlope = higherIsBetter ? slope : -slope;

  // Check for improvement
  if (normalizedSlope > Math.abs(config.improving_slope_threshold)) {
    return 'improving';
  }

  // Check for degradation
  if (normalizedSlope < -Math.abs(config.degrading_slope_threshold)) {
    return 'degrading';
  }

  // Stable if R² is high enough
  if (r_squared >= config.stable_r_squared_min) {
    return 'stable';
  }

  // Default to stable for small changes
  return 'stable';
}

// ============================================================================
// INDICATOR TREND DIRECTION
// ============================================================================

/**
 * Determine if higher values are better for a given indicator type.
 */
export function isHigherBetter(indicatorType: IndicatorType): boolean {
  switch (indicatorType) {
    case 'latency':
      return false; // Lower latency is better
    case 'error_rate':
      return false; // Lower error rate is better
    case 'throughput':
      return true;  // Higher throughput is better
    case 'saturation':
      return false; // Lower saturation is better
    case 'availability':
      return true;  // Higher availability is better
    default:
      return false;
  }
}

// ============================================================================
// TREND ANALYSIS
// ============================================================================

export interface TrendDataInput {
  timestamp: Date;
  value: number;
  state: HealthState;
}

export interface TrendAnalysisInput {
  indicator_type: IndicatorType;
  data_points: TrendDataInput[];
  predict_ahead_hours?: number;
}

/**
 * Analyze trend for a specific indicator.
 */
export function analyzeTrend(input: TrendAnalysisInput): HealthTrendAnalysis | null {
  const { indicator_type, data_points, predict_ahead_hours = 1 } = input;

  // Need at least 2 data points for trend analysis
  if (data_points.length < 2) {
    return null;
  }

  // Sort by timestamp
  const sortedPoints = [...data_points].sort(
    (a, b) => a.timestamp.getTime() - b.timestamp.getTime()
  );

  // Convert to x (time in hours) and y (value) arrays
  const firstTimestamp = sortedPoints[0]!.timestamp.getTime();
  const xValues = sortedPoints.map(p => (p.timestamp.getTime() - firstTimestamp) / (1000 * 60 * 60));
  const yValues = sortedPoints.map(p => p.value);

  // Perform regression
  const regression = linearRegression(xValues, yValues);

  // Calculate change percentage
  const firstValue = yValues[0]!;
  const lastValue = yValues[yValues.length - 1]!;
  const changePercentage = firstValue !== 0
    ? ((lastValue - firstValue) / Math.abs(firstValue)) * 100
    : 0;

  // Determine trend
  const higherIsBetter = isHigherBetter(indicator_type);
  const trend = determineTrend(regression.slope, regression.r_squared, higherIsBetter);

  // Predict state in 1 hour (if enough confidence)
  let predicted_state_in_1h: HealthState | undefined;
  if (regression.r_squared >= 0.5 && predict_ahead_hours > 0) {
    const lastX = xValues[xValues.length - 1]!;
    const predictedValue = regression.slope * (lastX + predict_ahead_hours) + regression.intercept;
    predicted_state_in_1h = predictStateFromValue(indicator_type, predictedValue);
  }

  // Convert to output format
  const trendDataPoints: TrendDataPoint[] = sortedPoints.map(p => ({
    timestamp: p.timestamp.toISOString(),
    value: p.value,
    state: p.state,
  }));

  return {
    indicator_type,
    trend,
    slope: Math.round(regression.slope * 10000) / 10000,
    r_squared: Math.round(regression.r_squared * 100) / 100,
    change_percentage: Math.round(changePercentage * 100) / 100,
    data_points: trendDataPoints,
    predicted_state_in_1h,
    confidence: regression.r_squared,
  };
}

// ============================================================================
// STATE PREDICTION
// ============================================================================

/**
 * Predict health state from a value (rough approximation).
 * Uses default thresholds for estimation.
 */
function predictStateFromValue(
  indicatorType: IndicatorType,
  value: number
): HealthState {
  // Rough thresholds for prediction (should match config)
  switch (indicatorType) {
    case 'latency':
      if (value <= 500) return 'healthy';
      if (value <= 2000) return 'degraded';
      return 'unhealthy';

    case 'error_rate':
      if (value <= 1) return 'healthy';
      if (value <= 5) return 'degraded';
      return 'unhealthy';

    case 'throughput':
      if (value >= 10) return 'healthy';
      if (value >= 1) return 'degraded';
      return 'unhealthy';

    case 'saturation':
      if (value <= 70) return 'healthy';
      if (value <= 90) return 'degraded';
      return 'unhealthy';

    case 'availability':
      if (value >= 99.9) return 'healthy';
      if (value >= 99.0) return 'degraded';
      return 'unhealthy';

    default:
      return 'healthy';
  }
}

// ============================================================================
// AGGREGATE TREND
// ============================================================================

/**
 * Determine overall trend from multiple indicator trends.
 */
export function aggregateTrends(trends: HealthTrendAnalysis[]): HealthTrend {
  if (trends.length === 0) return 'stable';

  // Count trend types weighted by confidence
  const trendScores: Record<HealthTrend, number> = {
    improving: 0,
    stable: 0,
    degrading: 0,
    volatile: 0,
  };

  for (const trend of trends) {
    trendScores[trend.trend] += trend.confidence;
  }

  // Find dominant trend
  let maxScore = 0;
  let dominantTrend: HealthTrend = 'stable';

  for (const [trend, score] of Object.entries(trendScores)) {
    if (score > maxScore) {
      maxScore = score;
      dominantTrend = trend as HealthTrend;
    }
  }

  return dominantTrend;
}
