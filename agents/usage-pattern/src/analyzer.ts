/**
 * Usage Pattern Analyzer - Core analytical logic.
 *
 * CONSTITUTION: This is a READ-ONLY, ADVISORY agent.
 * - Consumes normalized telemetry events
 * - Performs statistical aggregation across time windows
 * - Identifies trends, seasonality, and usage distributions
 * - Produces analytical summaries suitable for dashboards and forecasting
 *
 * This analyzer MUST NOT:
 * - Classify failures
 * - Evaluate health
 * - Enforce thresholds
 * - Generate alerts
 * - Modify system behavior
 * - Trigger orchestration
 */

import { createHash } from 'crypto';
import { v4 as uuidv4 } from 'uuid';
import { AgentConfig } from './config.js';
import { RuvectorClient } from './ruvector-client.js';
import { StoredTelemetryEvent } from './types/ruvector.js';
import {
  AnalysisRequest,
  UsagePatternAnalysis,
  TimeBucket,
  DistributionStats,
  ProviderUsage,
  TrendAnalysis,
  SeasonalityPattern,
  UsageHotspot,
  GrowthPattern,
  NormalizedTelemetryInput,
  AnalysisRequestSchema,
} from '../contracts/schemas.js';

/**
 * Statistical helper functions.
 */
class Statistics {
  /**
   * Calculate mean of an array of numbers.
   */
  static mean(values: number[]): number {
    if (values.length === 0) return 0;
    return values.reduce((sum, v) => sum + v, 0) / values.length;
  }

  /**
   * Calculate median of an array of numbers.
   */
  static median(values: number[]): number {
    if (values.length === 0) return 0;
    const sorted = [...values].sort((a, b) => a - b);
    const mid = Math.floor(sorted.length / 2);
    return sorted.length % 2 !== 0
      ? sorted[mid]
      : (sorted[mid - 1] + sorted[mid]) / 2;
  }

  /**
   * Calculate standard deviation.
   */
  static standardDeviation(values: number[]): number {
    if (values.length < 2) return 0;
    const avg = this.mean(values);
    const squareDiffs = values.map((v) => Math.pow(v - avg, 2));
    return Math.sqrt(this.mean(squareDiffs));
  }

  /**
   * Calculate variance.
   */
  static variance(values: number[]): number {
    const stdDev = this.standardDeviation(values);
    return stdDev * stdDev;
  }

  /**
   * Calculate percentile.
   */
  static percentile(values: number[], p: number): number {
    if (values.length === 0) return 0;
    const sorted = [...values].sort((a, b) => a - b);
    const index = (p / 100) * (sorted.length - 1);
    const lower = Math.floor(index);
    const upper = Math.ceil(index);
    if (lower === upper) return sorted[lower];
    return sorted[lower] + (sorted[upper] - sorted[lower]) * (index - lower);
  }

  /**
   * Calculate linear regression for trend analysis.
   * Returns slope, intercept, and R-squared.
   */
  static linearRegression(
    x: number[],
    y: number[]
  ): { slope: number; intercept: number; rSquared: number } {
    const n = x.length;
    if (n < 2) {
      return { slope: 0, intercept: 0, rSquared: 0 };
    }

    const sumX = x.reduce((a, b) => a + b, 0);
    const sumY = y.reduce((a, b) => a + b, 0);
    const sumXY = x.reduce((total, xi, i) => total + xi * y[i], 0);
    const sumX2 = x.reduce((total, xi) => total + xi * xi, 0);
    const sumY2 = y.reduce((total, yi) => total + yi * yi, 0);

    const slope = (n * sumXY - sumX * sumY) / (n * sumX2 - sumX * sumX);
    const intercept = (sumY - slope * sumX) / n;

    // Calculate R-squared
    const yMean = sumY / n;
    const ssTotal = y.reduce((total, yi) => total + Math.pow(yi - yMean, 2), 0);
    const ssResidual = y.reduce((total, yi, i) => {
      const predicted = slope * x[i] + intercept;
      return total + Math.pow(yi - predicted, 2);
    }, 0);
    const rSquared = ssTotal > 0 ? 1 - ssResidual / ssTotal : 0;

    return {
      slope: isNaN(slope) ? 0 : slope,
      intercept: isNaN(intercept) ? 0 : intercept,
      rSquared: isNaN(rSquared) ? 0 : Math.max(0, Math.min(1, rSquared)),
    };
  }

  /**
   * Calculate coefficient of variation (relative variability).
   */
  static coefficientOfVariation(values: number[]): number {
    const mean = this.mean(values);
    if (mean === 0) return 0;
    return this.standardDeviation(values) / mean;
  }
}

/**
 * Main Usage Pattern Analyzer class.
 */
export class UsagePatternAnalyzer {
  private config: AgentConfig;
  private client: RuvectorClient;

  constructor(config: AgentConfig, client: RuvectorClient) {
    this.config = config;
    this.client = client;
  }

  /**
   * Perform usage pattern analysis.
   *
   * CONSTITUTION: This is the main entry point for analysis.
   * - READ-ONLY operation
   * - ADVISORY output only
   * - No enforcement, no alerts
   */
  async analyze(request: AnalysisRequest): Promise<UsagePatternAnalysis> {
    const startTime = Date.now();
    const analysisId = uuidv4();

    // Validate request
    const validatedRequest = AnalysisRequestSchema.parse(request);

    // Fetch telemetry events
    const events = await this.fetchTelemetryEvents(validatedRequest);

    // Perform analysis
    const timeSeries = this.computeTimeSeries(events, validatedRequest);
    const summary = this.computeSummary(events);
    const distributions = this.computeDistributions(events, validatedRequest.options.percentiles);
    const providerUsage = this.computeProviderUsage(events);
    const hotspots = this.computeHotspots(events);
    const growthPatterns = this.computeGrowthPatterns(timeSeries);

    // Optional analyses
    const trends = validatedRequest.options.include_trends
      ? this.computeTrends(timeSeries)
      : undefined;

    const seasonality = validatedRequest.options.include_seasonality
      ? this.detectSeasonality(timeSeries)
      : undefined;

    // Calculate overall confidence based on sample size
    const overallConfidence = this.calculateConfidence(events.length);

    const analysis: UsagePatternAnalysis = {
      analysis_id: analysisId,
      analyzed_at: new Date().toISOString(),
      time_window: {
        start: validatedRequest.time_window.start,
        end: validatedRequest.time_window.end,
        granularity: validatedRequest.time_window.granularity,
      },
      summary,
      time_series: timeSeries,
      distributions,
      provider_usage: providerUsage,
      trends,
      seasonality,
      hotspots,
      growth_patterns: growthPatterns,
      overall_confidence: overallConfidence,
      sample_size: events.length,
      schema_version: '1.0.0',
    };

    return analysis;
  }

  /**
   * Fetch telemetry events from ruvector-service.
   */
  private async fetchTelemetryEvents(
    request: AnalysisRequest
  ): Promise<StoredTelemetryEvent[]> {
    const events: StoredTelemetryEvent[] = [];

    const query = {
      startTime: new Date(request.time_window.start),
      endTime: new Date(request.time_window.end),
      providers: request.filters.providers,
      models: request.filters.models,
      environments: request.filters.environments,
      userIds: request.filters.user_ids,
      limit: this.config.maxEventsPerAnalysis,
    };

    // Stream events to handle large datasets
    for await (const batch of this.client.streamTelemetryEvents(query)) {
      events.push(...batch);
      if (events.length >= this.config.maxEventsPerAnalysis) {
        break;
      }
    }

    return events;
  }

  /**
   * Compute time-bucketed aggregations.
   */
  private computeTimeSeries(
    events: StoredTelemetryEvent[],
    request: AnalysisRequest
  ): TimeBucket[] {
    const buckets = new Map<string, {
      events: StoredTelemetryEvent[];
      start: Date;
      end: Date;
    }>();

    const granularityMs = this.getGranularityMs(request.time_window.granularity);
    const startMs = new Date(request.time_window.start).getTime();
    const endMs = new Date(request.time_window.end).getTime();

    // Initialize empty buckets
    for (let t = startMs; t < endMs; t += granularityMs) {
      const bucketStart = new Date(t);
      const bucketEnd = new Date(t + granularityMs);
      const key = bucketStart.toISOString();
      buckets.set(key, { events: [], start: bucketStart, end: bucketEnd });
    }

    // Assign events to buckets
    for (const event of events) {
      const eventTime = new Date(event.latency.start_time).getTime();
      const bucketTime = Math.floor((eventTime - startMs) / granularityMs) * granularityMs + startMs;
      const key = new Date(bucketTime).toISOString();

      if (buckets.has(key)) {
        buckets.get(key)!.events.push(event);
      }
    }

    // Aggregate each bucket
    const timeSeries: TimeBucket[] = [];
    for (const [, bucket] of buckets) {
      const bucketEvents = bucket.events;
      const uniqueUsers = new Set(bucketEvents.map((e) => e.metadata.user_id).filter(Boolean));
      const uniqueSessions = new Set(bucketEvents.map((e) => e.metadata.session_id).filter(Boolean));

      timeSeries.push({
        bucket_start: bucket.start.toISOString(),
        bucket_end: bucket.end.toISOString(),
        request_count: bucketEvents.length,
        total_tokens: bucketEvents.reduce((sum, e) => sum + (e.token_usage?.total_tokens || 0), 0),
        total_cost_usd: bucketEvents.reduce((sum, e) => sum + (e.cost?.amount_usd || 0), 0),
        avg_latency_ms: Statistics.mean(bucketEvents.map((e) => e.latency.total_ms)),
        error_count: bucketEvents.filter((e) => e.status === 'ERROR').length,
        unique_users: uniqueUsers.size,
        unique_sessions: uniqueSessions.size,
      });
    }

    return timeSeries.sort((a, b) =>
      new Date(a.bucket_start).getTime() - new Date(b.bucket_start).getTime()
    );
  }

  /**
   * Compute summary statistics.
   */
  private computeSummary(events: StoredTelemetryEvent[]): UsagePatternAnalysis['summary'] {
    const uniqueUsers = new Set(events.map((e) => e.metadata.user_id).filter(Boolean));
    const uniqueSessions = new Set(events.map((e) => e.metadata.session_id).filter(Boolean));
    const uniqueProviders = new Set(events.map((e) => e.provider));
    const uniqueModels = new Set(events.map((e) => e.model));
    const errorCount = events.filter((e) => e.status === 'ERROR').length;

    return {
      total_requests: events.length,
      total_tokens: events.reduce((sum, e) => sum + (e.token_usage?.total_tokens || 0), 0),
      total_cost_usd: events.reduce((sum, e) => sum + (e.cost?.amount_usd || 0), 0),
      unique_users: uniqueUsers.size,
      unique_sessions: uniqueSessions.size,
      unique_providers: uniqueProviders.size,
      unique_models: uniqueModels.size,
      error_rate: events.length > 0 ? errorCount / events.length : 0,
      avg_requests_per_user: uniqueUsers.size > 0 ? events.length / uniqueUsers.size : 0,
    };
  }

  /**
   * Compute distribution statistics for numeric metrics.
   */
  private computeDistributions(
    events: StoredTelemetryEvent[],
    percentiles: number[]
  ): UsagePatternAnalysis['distributions'] {
    const latencies = events.map((e) => e.latency.total_ms);
    const tokens = events.map((e) => e.token_usage?.total_tokens || 0);
    const costs = events.map((e) => e.cost?.amount_usd || 0);

    const computeStats = (values: number[], name: string): DistributionStats => {
      const percentilesMap: Record<string, number> = {};
      for (const p of percentiles) {
        percentilesMap[`p${p}`] = Statistics.percentile(values, p);
      }

      return {
        metric_name: name,
        count: values.length,
        sum: values.reduce((a, b) => a + b, 0),
        min: values.length > 0 ? Math.min(...values) : 0,
        max: values.length > 0 ? Math.max(...values) : 0,
        mean: Statistics.mean(values),
        median: Statistics.median(values),
        std_dev: Statistics.standardDeviation(values),
        variance: Statistics.variance(values),
        percentiles: percentilesMap,
      };
    };

    return {
      latency: latencies.length > 0 ? computeStats(latencies, 'latency_ms') : undefined,
      tokens: tokens.some((t) => t > 0) ? computeStats(tokens, 'total_tokens') : undefined,
      cost: costs.some((c) => c > 0) ? computeStats(costs, 'cost_usd') : undefined,
    };
  }

  /**
   * Compute provider usage breakdown.
   */
  private computeProviderUsage(events: StoredTelemetryEvent[]): ProviderUsage[] {
    const providerMap = new Map<string, StoredTelemetryEvent[]>();

    for (const event of events) {
      const provider = event.provider;
      if (!providerMap.has(provider)) {
        providerMap.set(provider, []);
      }
      providerMap.get(provider)!.push(event);
    }

    const totalRequests = events.length;
    const result: ProviderUsage[] = [];

    for (const [provider, providerEvents] of providerMap) {
      const modelMap = new Map<string, StoredTelemetryEvent[]>();
      for (const event of providerEvents) {
        if (!modelMap.has(event.model)) {
          modelMap.set(event.model, []);
        }
        modelMap.get(event.model)!.push(event);
      }

      const modelBreakdown = Array.from(modelMap.entries()).map(([model, modelEvents]) => ({
        model,
        request_count: modelEvents.length,
        total_tokens: modelEvents.reduce((sum, e) => sum + (e.token_usage?.total_tokens || 0), 0),
        total_cost_usd: modelEvents.reduce((sum, e) => sum + (e.cost?.amount_usd || 0), 0),
      }));

      const errorCount = providerEvents.filter((e) => e.status === 'ERROR').length;

      result.push({
        provider,
        request_count: providerEvents.length,
        total_tokens: providerEvents.reduce((sum, e) => sum + (e.token_usage?.total_tokens || 0), 0),
        total_cost_usd: providerEvents.reduce((sum, e) => sum + (e.cost?.amount_usd || 0), 0),
        avg_latency_ms: Statistics.mean(providerEvents.map((e) => e.latency.total_ms)),
        error_rate: providerEvents.length > 0 ? errorCount / providerEvents.length : 0,
        model_breakdown: modelBreakdown,
        percentage_of_total: totalRequests > 0 ? (providerEvents.length / totalRequests) * 100 : 0,
      });
    }

    return result.sort((a, b) => b.request_count - a.request_count);
  }

  /**
   * Compute trend analysis for key metrics.
   */
  private computeTrends(timeSeries: TimeBucket[]): TrendAnalysis[] {
    if (timeSeries.length < this.config.minSampleSizeForTrends) {
      return [];
    }

    const metrics: Array<{ name: string; values: number[] }> = [
      { name: 'request_count', values: timeSeries.map((b) => b.request_count) },
      { name: 'total_tokens', values: timeSeries.map((b) => b.total_tokens) },
      { name: 'total_cost_usd', values: timeSeries.map((b) => b.total_cost_usd) },
      { name: 'avg_latency_ms', values: timeSeries.map((b) => b.avg_latency_ms) },
      { name: 'error_count', values: timeSeries.map((b) => b.error_count) },
    ];

    const trends: TrendAnalysis[] = [];
    const x = timeSeries.map((_, i) => i);

    for (const metric of metrics) {
      const { slope, rSquared } = Statistics.linearRegression(x, metric.values);

      // Determine direction
      const coeffVar = Statistics.coefficientOfVariation(metric.values);
      let direction: TrendAnalysis['direction'];

      if (coeffVar > 0.5) {
        direction = 'volatile';
      } else if (Math.abs(slope) < 0.01 * Statistics.mean(metric.values)) {
        direction = 'stable';
      } else if (slope > 0) {
        direction = 'increasing';
      } else {
        direction = 'decreasing';
      }

      // Calculate percentage change
      const firstValue = metric.values[0] || 1;
      const lastValue = metric.values[metric.values.length - 1] || 1;
      const changePercentage = firstValue !== 0 ? ((lastValue - firstValue) / firstValue) * 100 : 0;

      trends.push({
        metric_name: metric.name,
        direction,
        slope,
        r_squared: rSquared,
        change_percentage: changePercentage,
        confidence: rSquared, // R-squared as confidence measure
      });
    }

    return trends;
  }

  /**
   * Detect seasonality patterns.
   */
  private detectSeasonality(timeSeries: TimeBucket[]): SeasonalityPattern[] {
    if (timeSeries.length < this.config.minSampleSizeForSeasonality) {
      return [];
    }

    const patterns: SeasonalityPattern[] = [];

    // Check for hourly patterns (if granularity allows)
    const hourlyPattern = this.detectHourlyPattern(timeSeries);
    if (hourlyPattern) {
      patterns.push(hourlyPattern);
    }

    // Check for daily patterns
    const dailyPattern = this.detectDailyPattern(timeSeries);
    if (dailyPattern) {
      patterns.push(dailyPattern);
    }

    // Check for weekly patterns
    const weeklyPattern = this.detectWeeklyPattern(timeSeries);
    if (weeklyPattern) {
      patterns.push(weeklyPattern);
    }

    return patterns;
  }

  /**
   * Detect hourly patterns in the data.
   */
  private detectHourlyPattern(timeSeries: TimeBucket[]): SeasonalityPattern | null {
    // Group by hour of day
    const hourlyBuckets = new Map<number, number[]>();

    for (const bucket of timeSeries) {
      const hour = new Date(bucket.bucket_start).getUTCHours();
      if (!hourlyBuckets.has(hour)) {
        hourlyBuckets.set(hour, []);
      }
      hourlyBuckets.get(hour)!.push(bucket.request_count);
    }

    if (hourlyBuckets.size < 12) {
      return null; // Not enough hours represented
    }

    // Calculate variance between hours
    const hourlyMeans = new Map<number, number>();
    for (const [hour, values] of hourlyBuckets) {
      hourlyMeans.set(hour, Statistics.mean(values));
    }

    const meansArray = Array.from(hourlyMeans.values());
    const overallMean = Statistics.mean(meansArray);
    const coeffVar = Statistics.coefficientOfVariation(meansArray);

    // Identify peaks and troughs
    const sortedHours = Array.from(hourlyMeans.entries()).sort((a, b) => b[1] - a[1]);
    const peakPeriods: string[] = [];
    const troughPeriods: string[] = [];

    for (const [hour, mean] of sortedHours.slice(0, 3)) {
      if (mean > overallMean * 1.2) {
        peakPeriods.push(`${hour.toString().padStart(2, '0')}:00-${((hour + 1) % 24).toString().padStart(2, '0')}:00`);
      }
    }

    for (const [hour, mean] of sortedHours.slice(-3)) {
      if (mean < overallMean * 0.8) {
        troughPeriods.push(`${hour.toString().padStart(2, '0')}:00-${((hour + 1) % 24).toString().padStart(2, '0')}:00`);
      }
    }

    const detected = coeffVar > 0.2;
    const strength = Math.min(1, coeffVar);

    return {
      pattern_type: 'hourly',
      detected,
      strength,
      peak_periods: peakPeriods,
      trough_periods: troughPeriods,
      confidence: detected ? Math.min(1, hourlyBuckets.size / 24) : 0,
    };
  }

  /**
   * Detect daily patterns in the data.
   */
  private detectDailyPattern(timeSeries: TimeBucket[]): SeasonalityPattern | null {
    // Group by day of week
    const dailyBuckets = new Map<number, number[]>();

    for (const bucket of timeSeries) {
      const dayOfWeek = new Date(bucket.bucket_start).getUTCDay();
      if (!dailyBuckets.has(dayOfWeek)) {
        dailyBuckets.set(dayOfWeek, []);
      }
      dailyBuckets.get(dayOfWeek)!.push(bucket.request_count);
    }

    if (dailyBuckets.size < 5) {
      return null; // Not enough days represented
    }

    // Calculate variance between days
    const dailyMeans = new Map<number, number>();
    const dayNames = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];

    for (const [day, values] of dailyBuckets) {
      dailyMeans.set(day, Statistics.mean(values));
    }

    const meansArray = Array.from(dailyMeans.values());
    const overallMean = Statistics.mean(meansArray);
    const coeffVar = Statistics.coefficientOfVariation(meansArray);

    // Identify peaks and troughs
    const sortedDays = Array.from(dailyMeans.entries()).sort((a, b) => b[1] - a[1]);
    const peakPeriods: string[] = [];
    const troughPeriods: string[] = [];

    for (const [day, mean] of sortedDays.slice(0, 2)) {
      if (mean > overallMean * 1.1) {
        peakPeriods.push(dayNames[day]);
      }
    }

    for (const [day, mean] of sortedDays.slice(-2)) {
      if (mean < overallMean * 0.9) {
        troughPeriods.push(dayNames[day]);
      }
    }

    const detected = coeffVar > 0.15;
    const strength = Math.min(1, coeffVar * 2);

    return {
      pattern_type: 'daily',
      detected,
      strength,
      peak_periods: peakPeriods,
      trough_periods: troughPeriods,
      confidence: detected ? Math.min(1, dailyBuckets.size / 7) : 0,
    };
  }

  /**
   * Detect weekly patterns in the data.
   */
  private detectWeeklyPattern(timeSeries: TimeBucket[]): SeasonalityPattern | null {
    // Group by week number
    const weeklyBuckets = new Map<number, number[]>();

    for (const bucket of timeSeries) {
      const date = new Date(bucket.bucket_start);
      const weekNumber = this.getWeekNumber(date);
      if (!weeklyBuckets.has(weekNumber)) {
        weeklyBuckets.set(weekNumber, []);
      }
      weeklyBuckets.get(weekNumber)!.push(bucket.request_count);
    }

    if (weeklyBuckets.size < 4) {
      return null; // Need at least 4 weeks
    }

    // Calculate weekly totals
    const weeklyTotals = new Map<number, number>();
    for (const [week, values] of weeklyBuckets) {
      weeklyTotals.set(week, values.reduce((a, b) => a + b, 0));
    }

    const totalsArray = Array.from(weeklyTotals.values());
    const coeffVar = Statistics.coefficientOfVariation(totalsArray);

    const detected = coeffVar > 0.1;
    const strength = Math.min(1, coeffVar * 3);

    return {
      pattern_type: 'weekly',
      detected,
      strength,
      peak_periods: [],
      trough_periods: [],
      confidence: detected ? Math.min(1, weeklyBuckets.size / 8) : 0,
    };
  }

  /**
   * Compute usage hotspots.
   */
  private computeHotspots(events: StoredTelemetryEvent[]): UsageHotspot[] {
    const hotspots: UsageHotspot[] = [];
    const totalEvents = events.length;

    if (totalEvents === 0) {
      return hotspots;
    }

    // Provider hotspots
    const providerCounts = new Map<string, number>();
    for (const event of events) {
      providerCounts.set(event.provider, (providerCounts.get(event.provider) || 0) + 1);
    }

    const topProviders = Array.from(providerCounts.entries())
      .sort((a, b) => b[1] - a[1])
      .slice(0, 5);

    for (const [provider, count] of topProviders) {
      hotspots.push({
        dimension: 'provider',
        value: provider,
        intensity: count / totalEvents,
        request_count: count,
        percentage_of_total: (count / totalEvents) * 100,
      });
    }

    // Model hotspots
    const modelCounts = new Map<string, number>();
    for (const event of events) {
      modelCounts.set(event.model, (modelCounts.get(event.model) || 0) + 1);
    }

    const topModels = Array.from(modelCounts.entries())
      .sort((a, b) => b[1] - a[1])
      .slice(0, 5);

    for (const [model, count] of topModels) {
      hotspots.push({
        dimension: 'model',
        value: model,
        intensity: count / totalEvents,
        request_count: count,
        percentage_of_total: (count / totalEvents) * 100,
      });
    }

    // User hotspots (if data available)
    const userCounts = new Map<string, number>();
    for (const event of events) {
      if (event.metadata.user_id) {
        userCounts.set(event.metadata.user_id, (userCounts.get(event.metadata.user_id) || 0) + 1);
      }
    }

    const topUsers = Array.from(userCounts.entries())
      .sort((a, b) => b[1] - a[1])
      .slice(0, 5);

    for (const [userId, count] of topUsers) {
      hotspots.push({
        dimension: 'user',
        value: userId,
        intensity: count / totalEvents,
        request_count: count,
        percentage_of_total: (count / totalEvents) * 100,
      });
    }

    return hotspots.sort((a, b) => b.intensity - a.intensity);
  }

  /**
   * Compute growth patterns.
   */
  private computeGrowthPatterns(timeSeries: TimeBucket[]): GrowthPattern[] {
    const patterns: GrowthPattern[] = [];

    if (timeSeries.length < 2) {
      return patterns;
    }

    const metrics: Array<{ name: string; values: number[] }> = [
      { name: 'request_count', values: timeSeries.map((b) => b.request_count) },
      { name: 'total_tokens', values: timeSeries.map((b) => b.total_tokens) },
      { name: 'total_cost_usd', values: timeSeries.map((b) => b.total_cost_usd) },
      { name: 'unique_users', values: timeSeries.map((b) => b.unique_users) },
    ];

    for (const metric of metrics) {
      const firstHalf = metric.values.slice(0, Math.floor(metric.values.length / 2));
      const secondHalf = metric.values.slice(Math.floor(metric.values.length / 2));

      const firstHalfSum = firstHalf.reduce((a, b) => a + b, 0);
      const secondHalfSum = secondHalf.reduce((a, b) => a + b, 0);

      const periodOverPeriodGrowth =
        firstHalfSum > 0 ? ((secondHalfSum - firstHalfSum) / firstHalfSum) * 100 : 0;

      // Compound growth rate approximation
      const n = metric.values.length;
      const firstValue = metric.values[0] || 1;
      const lastValue = metric.values[n - 1] || 1;
      const compoundGrowthRate =
        firstValue > 0 ? Math.pow(lastValue / firstValue, 1 / n) - 1 : 0;

      // Classify growth
      let growthClassification: GrowthPattern['growth_classification'];
      if (periodOverPeriodGrowth > 20) {
        growthClassification = 'rapid_growth';
      } else if (periodOverPeriodGrowth > 5) {
        growthClassification = 'moderate_growth';
      } else if (periodOverPeriodGrowth >= -5) {
        growthClassification = 'stable';
      } else if (periodOverPeriodGrowth >= -20) {
        growthClassification = 'moderate_decline';
      } else {
        growthClassification = 'rapid_decline';
      }

      // Calculate confidence based on data consistency
      const coeffVar = Statistics.coefficientOfVariation(metric.values);
      const confidence = Math.max(0, 1 - coeffVar);

      patterns.push({
        metric_name: metric.name,
        period_over_period_growth: periodOverPeriodGrowth,
        compound_growth_rate: compoundGrowthRate * 100,
        growth_classification: growthClassification,
        confidence,
      });
    }

    return patterns;
  }

  /**
   * Calculate overall confidence based on sample size.
   */
  private calculateConfidence(sampleSize: number): number {
    // Confidence increases with sample size, asymptotically approaching 1
    // Using a sigmoid-like function: 1 - e^(-k*n)
    const k = 0.001; // Controls the rate of confidence increase
    return Math.min(1, 1 - Math.exp(-k * sampleSize));
  }

  /**
   * Get granularity in milliseconds.
   */
  private getGranularityMs(granularity: string): number {
    switch (granularity) {
      case 'minute':
        return 60 * 1000;
      case 'hour':
        return 60 * 60 * 1000;
      case 'day':
        return 24 * 60 * 60 * 1000;
      case 'week':
        return 7 * 24 * 60 * 60 * 1000;
      case 'month':
        return 30 * 24 * 60 * 60 * 1000;
      default:
        return 60 * 60 * 1000;
    }
  }

  /**
   * Get ISO week number for a date.
   */
  private getWeekNumber(date: Date): number {
    const d = new Date(Date.UTC(date.getUTCFullYear(), date.getUTCMonth(), date.getUTCDate()));
    const dayNum = d.getUTCDay() || 7;
    d.setUTCDate(d.getUTCDate() + 4 - dayNum);
    const yearStart = new Date(Date.UTC(d.getUTCFullYear(), 0, 1));
    return Math.ceil((((d.getTime() - yearStart.getTime()) / 86400000) + 1) / 7);
  }

  /**
   * Compute inputs hash for decision event.
   */
  computeInputsHash(request: AnalysisRequest): string {
    const input = JSON.stringify(request);
    return createHash('sha256').update(input).digest('hex');
  }
}
