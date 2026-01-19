// Copyright 2025 LLM Observatory Contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * Zod schemas for Usage Pattern Agent contracts.
 *
 * These schemas define the contract layer between:
 * - Normalized telemetry input (from collector or ruvector-service)
 * - Analytical output (usage pattern summaries)
 * - Decision events for ruvector-service persistence
 *
 * CONSTITUTION: This agent is READ-ONLY and ADVISORY.
 * - Classification: ADVISORY
 * - decision_type: "usage_pattern_analysis"
 * - confidence: STATISTICAL (0.0-1.0 based on sample size and variance)
 * - constraints_applied: ALWAYS [] (advisory agent applies no constraints)
 *
 * This agent MUST NOT:
 * - Classify failures
 * - Evaluate health
 * - Enforce thresholds
 * - Generate alerts
 * - Modify system behavior
 * - Trigger orchestration
 */

import { z } from 'zod';

// ============================================================================
// Re-export common schemas from telemetry-collector (agentics-contracts)
// ============================================================================

export const ProviderSchema = z.enum([
  'openai',
  'anthropic',
  'google',
  'mistral',
  'cohere',
  'self-hosted',
]).or(z.string());

export const SpanStatusSchema = z.enum(['OK', 'ERROR', 'UNSET']);

export const TokenUsageSchema = z.object({
  prompt_tokens: z.number().int().nonnegative(),
  completion_tokens: z.number().int().nonnegative(),
  total_tokens: z.number().int().nonnegative(),
}).strict();

export const CostSchema = z.object({
  amount_usd: z.number().nonnegative(),
  currency: z.string().default('USD'),
  prompt_cost: z.number().nonnegative().optional(),
  completion_cost: z.number().nonnegative().optional(),
}).strict();

export const LatencySchema = z.object({
  total_ms: z.number().int().nonnegative(),
  ttft_ms: z.number().int().nonnegative().optional(),
  start_time: z.string().datetime(),
  end_time: z.string().datetime(),
}).strict();

// ============================================================================
// Input Schema: Normalized Telemetry Events (from collector or storage)
// ============================================================================

/**
 * Normalized telemetry event - input to Usage Pattern Agent.
 * These events come from the telemetry-collector or from ruvector-service queries.
 */
export const NormalizedTelemetryInputSchema = z.object({
  span_id: z.string().min(1),
  trace_id: z.string().min(1),
  parent_span_id: z.string().optional(),
  name: z.string().min(1),
  provider: ProviderSchema,
  model: z.string().min(1),
  token_usage: TokenUsageSchema.optional(),
  cost: CostSchema.optional(),
  latency: LatencySchema,
  status: SpanStatusSchema,
  normalized_at: z.string().datetime(),
  metadata: z.object({
    user_id: z.string().optional(),
    session_id: z.string().optional(),
    environment: z.string().optional(),
    tags: z.array(z.string()).default([]),
    attributes: z.record(z.string(), z.string()).default({}),
  }).default({}),
}).strict();

/**
 * Analysis request - parameters for usage pattern analysis.
 */
export const AnalysisRequestSchema = z.object({
  // Time window for analysis
  time_window: z.object({
    start: z.string().datetime(),
    end: z.string().datetime(),
    granularity: z.enum(['minute', 'hour', 'day', 'week', 'month']).default('hour'),
  }),

  // Optional filters
  filters: z.object({
    providers: z.array(ProviderSchema).optional(),
    models: z.array(z.string()).optional(),
    environments: z.array(z.string()).optional(),
    user_ids: z.array(z.string()).optional(),
  }).default({}),

  // Analysis options
  options: z.object({
    include_trends: z.boolean().default(true),
    include_distributions: z.boolean().default(true),
    include_seasonality: z.boolean().default(false),
    include_forecasts: z.boolean().default(false),
    percentiles: z.array(z.number().min(0).max(100)).default([50, 90, 95, 99]),
  }).default({}),

  // Request metadata
  request_id: z.string().uuid().optional(),
}).strict();

// ============================================================================
// Output Schemas: Usage Pattern Analysis Results
// ============================================================================

/**
 * Time-bucketed aggregation for trend analysis.
 */
export const TimeBucketSchema = z.object({
  bucket_start: z.string().datetime(),
  bucket_end: z.string().datetime(),
  request_count: z.number().int().nonnegative(),
  total_tokens: z.number().int().nonnegative(),
  total_cost_usd: z.number().nonnegative(),
  avg_latency_ms: z.number().nonnegative(),
  error_count: z.number().int().nonnegative(),
  unique_users: z.number().int().nonnegative(),
  unique_sessions: z.number().int().nonnegative(),
}).strict();

/**
 * Distribution statistics for a numeric metric.
 */
export const DistributionStatsSchema = z.object({
  metric_name: z.string(),
  count: z.number().int().nonnegative(),
  sum: z.number(),
  min: z.number(),
  max: z.number(),
  mean: z.number(),
  median: z.number(),
  std_dev: z.number().nonnegative(),
  variance: z.number().nonnegative(),
  percentiles: z.record(z.string(), z.number()), // e.g., { "p50": 123, "p95": 456 }
}).strict();

/**
 * Provider usage breakdown.
 */
export const ProviderUsageSchema = z.object({
  provider: ProviderSchema,
  request_count: z.number().int().nonnegative(),
  total_tokens: z.number().int().nonnegative(),
  total_cost_usd: z.number().nonnegative(),
  avg_latency_ms: z.number().nonnegative(),
  error_rate: z.number().min(0).max(1),
  model_breakdown: z.array(z.object({
    model: z.string(),
    request_count: z.number().int().nonnegative(),
    total_tokens: z.number().int().nonnegative(),
    total_cost_usd: z.number().nonnegative(),
  })),
  percentage_of_total: z.number().min(0).max(100),
}).strict();

/**
 * Trend analysis result.
 */
export const TrendAnalysisSchema = z.object({
  metric_name: z.string(),
  direction: z.enum(['increasing', 'decreasing', 'stable', 'volatile']),
  slope: z.number(), // Rate of change per time unit
  r_squared: z.number().min(0).max(1), // Goodness of fit
  change_percentage: z.number(), // Percentage change over period
  confidence: z.number().min(0).max(1), // Statistical confidence
}).strict();

/**
 * Seasonality pattern detection.
 */
export const SeasonalityPatternSchema = z.object({
  pattern_type: z.enum(['hourly', 'daily', 'weekly', 'monthly']),
  detected: z.boolean(),
  strength: z.number().min(0).max(1), // 0 = no pattern, 1 = strong pattern
  peak_periods: z.array(z.string()), // e.g., ["09:00-12:00", "14:00-17:00"]
  trough_periods: z.array(z.string()),
  confidence: z.number().min(0).max(1),
}).strict();

/**
 * Usage hotspot identification.
 */
export const UsageHotspotSchema = z.object({
  dimension: z.enum(['time', 'provider', 'model', 'user', 'environment']),
  value: z.string(),
  intensity: z.number().min(0).max(1), // Normalized intensity
  request_count: z.number().int().nonnegative(),
  percentage_of_total: z.number().min(0).max(100),
}).strict();

/**
 * Growth pattern analysis.
 */
export const GrowthPatternSchema = z.object({
  metric_name: z.string(),
  period_over_period_growth: z.number(), // Percentage
  compound_growth_rate: z.number(), // CAGR-like metric
  growth_classification: z.enum([
    'rapid_growth',      // > 20% growth
    'moderate_growth',   // 5-20% growth
    'stable',            // -5% to 5%
    'moderate_decline',  // -20% to -5%
    'rapid_decline',     // < -20%
  ]),
  confidence: z.number().min(0).max(1),
}).strict();

/**
 * Complete usage pattern analysis output.
 */
export const UsagePatternAnalysisSchema = z.object({
  // Analysis metadata
  analysis_id: z.string().uuid(),
  analyzed_at: z.string().datetime(),
  time_window: z.object({
    start: z.string().datetime(),
    end: z.string().datetime(),
    granularity: z.enum(['minute', 'hour', 'day', 'week', 'month']),
  }),

  // Summary statistics
  summary: z.object({
    total_requests: z.number().int().nonnegative(),
    total_tokens: z.number().int().nonnegative(),
    total_cost_usd: z.number().nonnegative(),
    unique_users: z.number().int().nonnegative(),
    unique_sessions: z.number().int().nonnegative(),
    unique_providers: z.number().int().nonnegative(),
    unique_models: z.number().int().nonnegative(),
    error_rate: z.number().min(0).max(1),
    avg_requests_per_user: z.number().nonnegative(),
  }),

  // Time-series data
  time_series: z.array(TimeBucketSchema),

  // Distribution statistics
  distributions: z.object({
    latency: DistributionStatsSchema.optional(),
    tokens: DistributionStatsSchema.optional(),
    cost: DistributionStatsSchema.optional(),
  }),

  // Provider breakdown
  provider_usage: z.array(ProviderUsageSchema),

  // Trend analysis (optional)
  trends: z.array(TrendAnalysisSchema).optional(),

  // Seasonality patterns (optional)
  seasonality: z.array(SeasonalityPatternSchema).optional(),

  // Usage hotspots
  hotspots: z.array(UsageHotspotSchema),

  // Growth patterns
  growth_patterns: z.array(GrowthPatternSchema),

  // Statistical confidence of overall analysis
  overall_confidence: z.number().min(0).max(1),
  sample_size: z.number().int().nonnegative(),

  // Schema version for evolution
  schema_version: z.string().default('1.0.0'),
}).strict();

// ============================================================================
// Decision Event Schema (for ruvector-service persistence)
// ============================================================================

/**
 * Decision event schema for Usage Pattern Agent.
 *
 * CONSTITUTION CONSTRAINTS:
 * - decision_type: MUST be "usage_pattern_analysis" (literal)
 * - confidence: STATISTICAL (0.0-1.0) based on sample size and variance
 * - constraints_applied: ALWAYS [] (advisory agent applies no constraints)
 *
 * This agent is ADVISORY ONLY. It:
 * - Produces analytical summaries
 * - Does NOT classify failures
 * - Does NOT evaluate health
 * - Does NOT enforce thresholds
 * - Does NOT generate alerts
 */
export const UsagePatternDecisionEventSchema = z.object({
  // Agent metadata
  agent_id: z.literal('usage-pattern-agent'),
  agent_version: z.string().regex(/^\d+\.\d+\.\d+$/),

  // Decision metadata (CONSTITUTIONAL CONSTRAINTS)
  decision_type: z.literal('usage_pattern_analysis'),
  confidence: z.number().min(0).max(1), // Statistical confidence based on analysis
  constraints_applied: z.array(z.never()).length(0), // ALWAYS empty - advisory only

  // Classification
  classification: z.literal('advisory'), // Constitutional classification

  // Input/Output tracking
  inputs_hash: z.string().length(64), // SHA256 hash (hex)
  outputs: z.array(UsagePatternAnalysisSchema).min(1),

  // Execution tracking
  execution_ref: z.string().min(1),
  timestamp: z.string().datetime(),

  // Processing metrics
  processing_metrics: z.object({
    events_analyzed: z.number().int().nonnegative(),
    processing_time_ms: z.number().nonnegative(),
    memory_used_bytes: z.number().int().nonnegative().optional(),
  }),

  // Optional metadata
  metadata: z.record(z.string(), z.unknown()).optional(),
}).strict();

// ============================================================================
// CLI Contract Schemas
// ============================================================================

/**
 * CLI invocation parameters for inspection/replay/analysis.
 */
export const CLIInvocationSchema = z.object({
  command: z.enum(['analyze', 'inspect', 'replay', 'status']),

  // For analyze command
  analyze: z.object({
    start_time: z.string().datetime(),
    end_time: z.string().datetime(),
    granularity: z.enum(['minute', 'hour', 'day', 'week', 'month']).default('hour'),
    providers: z.array(z.string()).optional(),
    models: z.array(z.string()).optional(),
    output_format: z.enum(['json', 'table', 'csv']).default('json'),
  }).optional(),

  // For inspect command
  inspect: z.object({
    analysis_id: z.string().uuid(),
  }).optional(),

  // For replay command
  replay: z.object({
    analysis_id: z.string().uuid(),
    dry_run: z.boolean().default(true),
  }).optional(),
}).strict();

/**
 * CLI output format.
 */
export const CLIOutputSchema = z.object({
  success: z.boolean(),
  command: z.string(),
  timestamp: z.string().datetime(),
  result: z.union([
    UsagePatternAnalysisSchema,
    z.object({
      status: z.string(),
      message: z.string(),
    }),
  ]).optional(),
  error: z.object({
    code: z.string(),
    message: z.string(),
    details: z.unknown().optional(),
  }).optional(),
}).strict();

// ============================================================================
// Error Schemas
// ============================================================================

/**
 * Agent error codes.
 */
export const ErrorCodeSchema = z.enum([
  'INVALID_INPUT',
  'VALIDATION_FAILED',
  'INSUFFICIENT_DATA',
  'TIME_WINDOW_TOO_LARGE',
  'RUVECTOR_CONNECTION_ERROR',
  'ANALYSIS_TIMEOUT',
  'INTERNAL_ERROR',
]);

/**
 * Agent error response.
 */
export const ErrorResponseSchema = z.object({
  error: z.object({
    code: ErrorCodeSchema,
    message: z.string(),
    details: z.unknown().optional(),
    timestamp: z.string().datetime(),
    execution_ref: z.string().optional(),
  }),
}).strict();

// ============================================================================
// Type Exports
// ============================================================================

export type NormalizedTelemetryInput = z.infer<typeof NormalizedTelemetryInputSchema>;
export type AnalysisRequest = z.infer<typeof AnalysisRequestSchema>;
export type TimeBucket = z.infer<typeof TimeBucketSchema>;
export type DistributionStats = z.infer<typeof DistributionStatsSchema>;
export type ProviderUsage = z.infer<typeof ProviderUsageSchema>;
export type TrendAnalysis = z.infer<typeof TrendAnalysisSchema>;
export type SeasonalityPattern = z.infer<typeof SeasonalityPatternSchema>;
export type UsageHotspot = z.infer<typeof UsageHotspotSchema>;
export type GrowthPattern = z.infer<typeof GrowthPatternSchema>;
export type UsagePatternAnalysis = z.infer<typeof UsagePatternAnalysisSchema>;
export type UsagePatternDecisionEvent = z.infer<typeof UsagePatternDecisionEventSchema>;
export type CLIInvocation = z.infer<typeof CLIInvocationSchema>;
export type CLIOutput = z.infer<typeof CLIOutputSchema>;
export type ErrorCode = z.infer<typeof ErrorCodeSchema>;
export type ErrorResponse = z.infer<typeof ErrorResponseSchema>;
export type Provider = z.infer<typeof ProviderSchema>;
