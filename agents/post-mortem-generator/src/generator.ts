/**
 * Post-Mortem Generator Agent - Core Generation Logic
 *
 * CONSTITUTIONAL CLASSIFICATION: READ-ONLY
 *
 * Generates structured, reproducible post-mortem reports from
 * historical telemetry, failure classifications, and health evaluations.
 *
 * This module MUST NOT:
 * - Influence live systems
 * - Write advisory constraints
 * - Recommend remediation actions
 */

import { randomUUID } from 'crypto';
import type {
  PostMortemRequest,
  PostMortemReport,
  TimelineEvent,
  ClassificationBreakdown,
  HealthTransition,
  ContributingFactor,
  StatisticalSummary,
  FailureCategory,
  FailureSeverity,
  FailureCause,
  Provider,
} from '../contracts/schemas.js';
import type {
  AggregatedFailureData,
  AggregatedHealthData,
  AggregatedTelemetryData,
  StoredFailureClassification,
  StoredHealthEvaluation,
} from './types/ruvector.js';
import { recordDataCompleteness } from './telemetry.js';

// =============================================================================
// POST-MORTEM GENERATION
// =============================================================================

export interface GeneratorInput {
  request: PostMortemRequest;
  failureData: AggregatedFailureData;
  healthData: AggregatedHealthData;
  telemetryData: AggregatedTelemetryData;
  failureClassifications: StoredFailureClassification[];
  healthEvaluations: StoredHealthEvaluation[];
}

export interface GeneratorResult {
  report: PostMortemReport;
  confidence: number;
  processingTimeMs: number;
}

/**
 * Generate a post-mortem report from aggregated data.
 */
export function generatePostMortem(input: GeneratorInput): GeneratorResult {
  const startTime = Date.now();
  const {
    request,
    failureData,
    healthData,
    telemetryData,
    failureClassifications,
    healthEvaluations,
  } = input;

  const options = request.options || {};
  const reportId = randomUUID();

  // Build timeline
  const timeline = options.include_timeline !== false
    ? buildTimeline(failureClassifications, healthEvaluations, options.max_timeline_events || 1000)
    : undefined;

  // Build classification breakdown
  const classificationBreakdown = options.include_classification_breakdown !== false
    ? buildClassificationBreakdown(failureData)
    : undefined;

  // Build health transitions
  const healthTransitions = options.include_health_transitions !== false
    ? buildHealthTransitions(healthData)
    : undefined;

  // Build contributing factors
  const contributingFactors = options.include_contributing_factors !== false
    ? identifyContributingFactors(failureData, healthData, telemetryData)
    : undefined;

  // Build statistics
  const statistics = options.include_statistics !== false
    ? buildStatistics(failureData, telemetryData, request.time_range)
    : undefined;

  // Calculate data quality
  const dataQuality = calculateDataQuality(failureData, healthData, telemetryData);
  recordDataCompleteness(dataQuality.completeness);

  // Generate summary
  const summary = generateSummary(failureData, healthData, telemetryData);

  // Determine overall confidence based on data quality
  const confidence = dataQuality.completeness;

  const processingTimeMs = Date.now() - startTime;

  const report: PostMortemReport = {
    report_id: reportId,
    generated_at: new Date().toISOString(),
    generation_latency_ms: processingTimeMs,
    schema_version: '1.0.0',
    time_range: request.time_range,
    incident_id: request.incident_id,
    summary,
    timeline,
    classification_breakdown: classificationBreakdown,
    health_transitions: healthTransitions,
    contributing_factors: contributingFactors,
    statistics,
    data_quality: dataQuality,
  };

  return {
    report,
    confidence,
    processingTimeMs,
  };
}

// =============================================================================
// TIMELINE BUILDING
// =============================================================================

function buildTimeline(
  failureClassifications: StoredFailureClassification[],
  healthEvaluations: StoredHealthEvaluation[],
  maxEvents: number
): TimelineEvent[] {
  const events: TimelineEvent[] = [];

  // Add failure events
  for (const classification of failureClassifications) {
    events.push({
      timestamp: classification.classified_at,
      event_type: 'failure',
      description: `${classification.category} failure from ${classification.provider}/${classification.model}`,
      span_id: classification.span_id,
      trace_id: classification.trace_id,
      provider: classification.provider,
      model: classification.model,
      severity: classification.severity,
    });
  }

  // Add health transition events
  for (const evaluation of healthEvaluations) {
    if (evaluation.previous_state && evaluation.previous_state !== evaluation.health_state) {
      events.push({
        timestamp: evaluation.evaluated_at,
        event_type: 'health_transition',
        description: `${evaluation.target_type}/${evaluation.target_id} transitioned from ${evaluation.previous_state} to ${evaluation.health_state}`,
        service: evaluation.target_id,
        health_state: evaluation.health_state,
      });
    }
  }

  // Sort by timestamp
  events.sort((a, b) => new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime());

  // Mark special events
  if (events.length > 0) {
    // First failure
    const firstFailure = events.find((e) => e.event_type === 'failure');
    if (firstFailure) {
      firstFailure.event_type = 'first_failure';
    }

    // Last failure (find from end)
    for (let i = events.length - 1; i >= 0; i--) {
      if (events[i].event_type === 'failure') {
        events[i].event_type = 'last_failure';
        break;
      }
    }
  }

  // Limit to max events
  return events.slice(0, maxEvents);
}

// =============================================================================
// CLASSIFICATION BREAKDOWN
// =============================================================================

function buildClassificationBreakdown(
  failureData: AggregatedFailureData
): ClassificationBreakdown {
  const totalFailures = failureData.total_failures || 1;

  // By category
  const byCategory: ClassificationBreakdown['by_category'] = [];
  for (const [category, count] of failureData.by_category) {
    const providerData = findFirstLastOccurrence(failureData, category);
    byCategory.push({
      category,
      count,
      percentage: (count / totalFailures) * 100,
      first_occurrence: providerData.first,
      last_occurrence: providerData.last,
    });
  }
  byCategory.sort((a, b) => b.count - a.count);

  // By severity
  const bySeverity: ClassificationBreakdown['by_severity'] = [];
  for (const [severity, count] of failureData.by_severity) {
    bySeverity.push({
      severity,
      count,
      percentage: (count / totalFailures) * 100,
    });
  }
  bySeverity.sort((a, b) => severityOrder(a.severity) - severityOrder(b.severity));

  // By cause
  const byCause: ClassificationBreakdown['by_cause'] = [];
  for (const [cause, count] of failureData.by_cause) {
    byCause.push({
      cause,
      count,
      percentage: (count / totalFailures) * 100,
    });
  }
  byCause.sort((a, b) => b.count - a.count);

  // By provider
  const byProvider: ClassificationBreakdown['by_provider'] = [];
  for (const [provider, data] of failureData.by_provider) {
    byProvider.push({
      provider,
      count: data.count,
      percentage: (data.count / totalFailures) * 100,
      models_affected: Array.from(data.models),
    });
  }
  byProvider.sort((a, b) => b.count - a.count);

  return {
    by_category: byCategory,
    by_severity: bySeverity,
    by_cause: byCause,
    by_provider: byProvider,
  };
}

function findFirstLastOccurrence(
  failureData: AggregatedFailureData,
  _category: FailureCategory
): { first: string; last: string } {
  // Get from time series if available
  if (failureData.time_series.length > 0) {
    return {
      first: failureData.time_series[0].timestamp,
      last: failureData.time_series[failureData.time_series.length - 1].timestamp,
    };
  }
  const now = new Date().toISOString();
  return { first: now, last: now };
}

function severityOrder(severity: FailureSeverity): number {
  const order: Record<FailureSeverity, number> = {
    critical: 0,
    high: 1,
    medium: 2,
    low: 3,
    informational: 4,
  };
  return order[severity];
}

// =============================================================================
// HEALTH TRANSITIONS
// =============================================================================

function buildHealthTransitions(
  healthData: AggregatedHealthData
): HealthTransition[] {
  return healthData.health_transitions.map((t) => ({
    timestamp: t.timestamp,
    target_id: t.target_id,
    target_type: t.target_type,
    from_state: t.from_state,
    to_state: t.to_state,
    duration_in_state_ms: t.duration_in_previous_state_ms,
  }));
}

// =============================================================================
// CONTRIBUTING FACTORS
// =============================================================================

function identifyContributingFactors(
  failureData: AggregatedFailureData,
  healthData: AggregatedHealthData,
  telemetryData: AggregatedTelemetryData
): ContributingFactor[] {
  const factors: ContributingFactor[] = [];

  // Identify primary cause based on highest failure category
  const topCategory = findTopEntry(failureData.by_category);
  if (topCategory) {
    const [category, count] = topCategory;
    factors.push({
      factor_id: randomUUID(),
      factor_type: 'primary_cause',
      description: `Primary failure category: ${category} (${count} occurrences)`,
      confidence: Math.min(1, count / (failureData.total_failures || 1)),
      evidence: [
        {
          type: 'failure_classification',
          reference: category,
          weight: count / (failureData.total_failures || 1),
        },
      ],
    });
  }

  // Identify provider-related issues
  const topProvider = findTopEntry(failureData.by_provider);
  if (topProvider) {
    const [provider, data] = topProvider;
    if (data.count > failureData.total_failures * 0.5) {
      factors.push({
        factor_id: randomUUID(),
        factor_type: 'contributing_cause',
        description: `Provider ${provider} accounted for ${Math.round((data.count / failureData.total_failures) * 100)}% of failures`,
        confidence: data.count / failureData.total_failures,
        evidence: [
          {
            type: 'provider_analysis',
            reference: provider,
            weight: data.count / failureData.total_failures,
          },
        ],
      });
    }
  }

  // Identify error rate spike correlation
  if (telemetryData.peak_error_rate.value > 0.5) {
    factors.push({
      factor_id: randomUUID(),
      factor_type: 'correlation',
      description: `Peak error rate of ${Math.round(telemetryData.peak_error_rate.value * 100)}% detected`,
      confidence: telemetryData.peak_error_rate.value,
      evidence: [
        {
          type: 'error_rate_analysis',
          reference: telemetryData.peak_error_rate.timestamp,
          weight: telemetryData.peak_error_rate.value,
        },
      ],
    });
  }

  // Identify health degradation correlation
  const degradations = healthData.health_transitions.filter(
    (t) => t.to_state === 'degraded' || t.to_state === 'unhealthy'
  );
  if (degradations.length > 0) {
    factors.push({
      factor_id: randomUUID(),
      factor_type: 'correlation',
      description: `${degradations.length} health degradation events detected`,
      confidence: Math.min(1, degradations.length / 10),
      evidence: degradations.slice(0, 5).map((d) => ({
        type: 'health_transition',
        reference: `${d.target_type}/${d.target_id}`,
        weight: 1 / degradations.length,
      })),
    });
  }

  return factors;
}

function findTopEntry<K, V>(map: Map<K, V>): [K, V] | null {
  let topEntry: [K, V] | null = null;
  let topCount = 0;

  for (const entry of map.entries()) {
    const count = typeof entry[1] === 'number' ? entry[1] : (entry[1] as { count?: number }).count || 0;
    if (count > topCount) {
      topCount = count;
      topEntry = entry;
    }
  }

  return topEntry;
}

// =============================================================================
// STATISTICS
// =============================================================================

function buildStatistics(
  failureData: AggregatedFailureData,
  telemetryData: AggregatedTelemetryData,
  timeRange: { start_time: string; end_time: string }
): StatisticalSummary {
  const durationMs =
    new Date(timeRange.end_time).getTime() - new Date(timeRange.start_time).getTime();

  // Count affected entities
  let affectedModels = 0;
  for (const [, data] of failureData.by_provider) {
    affectedModels += data.models.size;
  }

  // Find time to first failure
  let timeToFirstFailureMs: number | undefined;
  if (failureData.time_series.length > 0) {
    const firstFailureTime = new Date(failureData.time_series[0].timestamp).getTime();
    const startTime = new Date(timeRange.start_time).getTime();
    timeToFirstFailureMs = Math.max(0, firstFailureTime - startTime);
  }

  return {
    total_failures: failureData.total_failures,
    total_requests: telemetryData.total_requests,
    error_rate: telemetryData.error_rate,
    duration_ms: durationMs,
    time_to_first_failure_ms: timeToFirstFailureMs,
    latency_p50_ms: telemetryData.latency_stats.p50_ms,
    latency_p95_ms: telemetryData.latency_stats.p95_ms,
    latency_p99_ms: telemetryData.latency_stats.p99_ms,
    affected_providers: failureData.by_provider.size,
    affected_models: affectedModels,
    affected_services: 0, // Would need to aggregate from health data
    peak_error_rate: telemetryData.peak_error_rate.value,
    peak_error_rate_timestamp: telemetryData.peak_error_rate.timestamp,
  };
}

// =============================================================================
// DATA QUALITY
// =============================================================================

function calculateDataQuality(
  failureData: AggregatedFailureData,
  healthData: AggregatedHealthData,
  telemetryData: AggregatedTelemetryData
): {
  completeness: number;
  notes: string[];
  gaps: Array<{ start: string; end: string; reason: string }>;
} {
  const notes: string[] = [];
  const gaps: Array<{ start: string; end: string; reason: string }> = [];

  let completenessScore = 0;
  let components = 0;

  // Check failure data completeness
  if (failureData.total_failures > 0) {
    completenessScore += 1;
    if (failureData.by_category.size > 0) completenessScore += 0.5;
    if (failureData.by_severity.size > 0) completenessScore += 0.5;
    if (failureData.by_cause.size > 0) completenessScore += 0.5;
    if (failureData.time_series.length > 0) completenessScore += 0.5;
    components += 3;
  } else {
    notes.push('No failure classification data available');
    components += 1;
  }

  // Check health data completeness
  if (healthData.health_transitions.length > 0) {
    completenessScore += 1;
    components += 1;
  } else {
    notes.push('No health evaluation data available');
    components += 1;
  }

  // Check telemetry data completeness
  if (telemetryData.total_requests > 0) {
    completenessScore += 1;
    if (telemetryData.latency_stats.avg_ms > 0) completenessScore += 0.5;
    components += 1.5;
  } else {
    notes.push('No telemetry data available');
    components += 1;
  }

  const completeness = components > 0 ? completenessScore / components : 0;

  if (completeness < 0.5) {
    notes.push('Limited data available - report may be incomplete');
  }

  return {
    completeness: Math.min(1, completeness),
    notes,
    gaps,
  };
}

// =============================================================================
// SUMMARY GENERATION
// =============================================================================

function generateSummary(
  failureData: AggregatedFailureData,
  healthData: AggregatedHealthData,
  telemetryData: AggregatedTelemetryData
): {
  title: string;
  description: string;
  impact_level: FailureSeverity;
  status: 'resolved' | 'ongoing' | 'unknown';
} {
  // Determine impact level from highest severity
  let impactLevel: FailureSeverity = 'informational';
  for (const severity of ['critical', 'high', 'medium', 'low', 'informational'] as FailureSeverity[]) {
    if (failureData.by_severity.has(severity) && failureData.by_severity.get(severity)! > 0) {
      impactLevel = severity;
      break;
    }
  }

  // Generate title
  const topCategory = findTopEntry(failureData.by_category);
  const categoryName = topCategory ? topCategory[0] : 'unknown';
  const title = `${impactLevel.charAt(0).toUpperCase() + impactLevel.slice(1)} Incident: ${categoryName.replace(/_/g, ' ')}`;

  // Generate description
  const parts: string[] = [];
  parts.push(`${failureData.total_failures} failures detected`);
  if (telemetryData.error_rate > 0) {
    parts.push(`${Math.round(telemetryData.error_rate * 100)}% error rate`);
  }
  if (failureData.by_provider.size > 0) {
    parts.push(`${failureData.by_provider.size} provider(s) affected`);
  }
  const description = parts.join(', ') + '.';

  // Determine status
  let status: 'resolved' | 'ongoing' | 'unknown' = 'unknown';
  if (healthData.health_transitions.length > 0) {
    const lastTransition = healthData.health_transitions[healthData.health_transitions.length - 1];
    if (lastTransition && lastTransition.to_state === 'healthy') {
      status = 'resolved';
    } else if (lastTransition && (lastTransition.to_state === 'degraded' || lastTransition.to_state === 'unhealthy')) {
      status = 'ongoing';
    }
  }

  return {
    title,
    description,
    impact_level: impactLevel,
    status,
  };
}
