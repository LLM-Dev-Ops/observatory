/**
 * Health Check Agent - Zod Validation Schemas
 *
 * CLASSIFICATION: ADVISORY, NON-ACTUATING
 * DECISION_TYPE: health_evaluation
 * CONFIDENCE: Statistical (0.0-1.0)
 * CONSTRAINTS_APPLIED: Always [] (empty)
 *
 * CONSTITUTIONAL CONSTRAINTS:
 * - This agent MUST NOT trigger alerts
 * - This agent MUST NOT initiate remediation
 * - This agent MUST NOT change execution behavior
 * - This agent MUST NOT access database directly
 * - All persistence via ruvector-service only
 */

import { z } from 'zod';

// ============================================================================
// CONSTANTS
// ============================================================================

export const AGENT_ID = 'health-check-agent' as const;
export const AGENT_VERSION = '1.0.0' as const;
export const AGENT_CLASSIFICATION = 'advisory' as const;
export const DECISION_TYPE = 'health_evaluation' as const;

// ============================================================================
// HEALTH STATE ENUMS
// ============================================================================

/**
 * Discrete health states for targets
 */
export const HealthStateSchema = z.enum(['healthy', 'degraded', 'unhealthy']);

/**
 * Health trend over time
 */
export const HealthTrendSchema = z.enum([
  'improving',  // Health state getting better
  'stable',     // Health state consistent
  'degrading',  // Health state getting worse
  'volatile',   // Health state fluctuating unpredictably
]);

/**
 * Types of health indicators measured
 */
export const IndicatorTypeSchema = z.enum([
  'latency',      // Response time metrics
  'error_rate',   // Error percentage
  'throughput',   // Requests per second
  'saturation',   // Resource utilization
  'availability', // Uptime percentage
]);

/**
 * Target types that can be evaluated
 */
export const TargetTypeSchema = z.enum([
  'service',
  'agent',
  'provider',
  'endpoint',
]);

/**
 * Evaluation window granularity
 */
export const EvaluationWindowSchema = z.enum([
  '1m',
  '5m',
  '15m',
  '1h',
  '6h',
  '24h',
]);

/**
 * Trend window for historical analysis
 */
export const TrendWindowSchema = z.enum([
  '1h',
  '6h',
  '24h',
  '7d',
]);

// ============================================================================
// HEALTH INDICATOR SCHEMAS
// ============================================================================

/**
 * Time window for measurement
 */
export const MeasurementWindowSchema = z.object({
  start: z.string().datetime(),
  end: z.string().datetime(),
  duration_seconds: z.number().int().positive(),
}).strict();

/**
 * Percentile values for latency/throughput metrics
 */
export const PercentilesSchema = z.object({
  p50: z.number().optional(),
  p90: z.number().optional(),
  p95: z.number().optional(),
  p99: z.number().optional(),
}).strict();

/**
 * Individual health indicator measurement
 */
export const HealthIndicatorSchema = z.object({
  indicator_type: IndicatorTypeSchema,

  // Current values
  current_value: z.number(),
  unit: z.string(), // "ms", "percentage", "req/s", etc.

  // Statistical context
  baseline_value: z.number().optional(),
  threshold_warning: z.number().optional(),
  threshold_critical: z.number().optional(),

  // Percentile values (for latency/throughput)
  percentiles: PercentilesSchema.optional(),

  // Indicator health state
  state: HealthStateSchema,
  state_reason: z.string(),

  // Statistical confidence for this indicator
  sample_size: z.number().int().nonnegative(),
  confidence: z.number().min(0).max(1),

  // Time window for measurement
  measurement_window: MeasurementWindowSchema,
}).strict();

/**
 * Latency-specific indicator with detailed breakdown
 */
export const LatencyBreakdownSchema = z.object({
  ttft_ms: z.number().optional(),      // Time to first token
  processing_ms: z.number().optional(),
  network_ms: z.number().optional(),
}).strict();

/**
 * Error rate breakdown by category
 */
export const ErrorBreakdownSchema = z.object({
  network_errors: z.number().int().nonnegative(),
  provider_errors: z.number().int().nonnegative(),
  client_errors: z.number().int().nonnegative(),
  timeout_errors: z.number().int().nonnegative(),
  unknown_errors: z.number().int().nonnegative(),
}).strict();

/**
 * Throughput breakdown
 */
export const ThroughputBreakdownSchema = z.object({
  successful_requests: z.number().int().nonnegative(),
  failed_requests: z.number().int().nonnegative(),
  total_requests: z.number().int().nonnegative(),
}).strict();

/**
 * Saturation breakdown by resource type
 */
export const SaturationBreakdownSchema = z.object({
  current_usage: z.number().nonnegative(),
  max_capacity: z.number().positive(),
  utilization_percentage: z.number().min(0).max(100),
}).strict();

// ============================================================================
// HEALTH TREND SCHEMAS
// ============================================================================

/**
 * Single data point in trend analysis
 */
export const TrendDataPointSchema = z.object({
  timestamp: z.string().datetime(),
  value: z.number(),
  state: HealthStateSchema,
}).strict();

/**
 * Health trend analysis over time
 */
export const HealthTrendAnalysisSchema = z.object({
  indicator_type: IndicatorTypeSchema,
  trend: HealthTrendSchema,

  // Trend metrics
  slope: z.number(),                    // Rate of change
  r_squared: z.number().min(0).max(1),  // Goodness of fit
  change_percentage: z.number(),        // Change over period

  // Time series data points
  data_points: z.array(TrendDataPointSchema),

  // Prediction (advisory only - no action taken)
  predicted_state_in_1h: HealthStateSchema.optional(),
  confidence: z.number().min(0).max(1),
}).strict();

// ============================================================================
// STATE TRANSITION SCHEMA
// ============================================================================

/**
 * State transition tracking with hysteresis
 */
export const StateTransitionSchema = z.object({
  previous_state: HealthStateSchema.optional(),
  current_state: HealthStateSchema,
  transition_time: z.string().datetime().optional(),
  time_in_current_state_seconds: z.number().int().nonnegative(),
  // Hysteresis: require N consecutive samples before state change
  consecutive_samples_in_state: z.number().int().nonnegative(),
  hysteresis_threshold: z.number().int().positive().default(3),
}).strict();

// ============================================================================
// TARGET IDENTIFICATION
// ============================================================================

/**
 * Target being evaluated
 */
export const TargetSchema = z.object({
  type: TargetTypeSchema,
  id: z.string().min(1),
  name: z.string().min(1),
  version: z.string().optional(),
}).strict();

// ============================================================================
// AGGREGATE STATISTICS
// ============================================================================

/**
 * Aggregate statistics for evaluation period
 */
export const AggregateStatisticsSchema = z.object({
  total_requests: z.number().int().nonnegative(),
  total_errors: z.number().int().nonnegative(),
  avg_latency_ms: z.number().nonnegative(),
  error_rate_percentage: z.number().min(0).max(100),
  availability_percentage: z.number().min(0).max(100),
  sample_size: z.number().int().nonnegative(),
}).strict();

/**
 * Evaluation window specification
 */
export const EvaluationWindowSpecSchema = z.object({
  start: z.string().datetime(),
  end: z.string().datetime(),
  granularity: EvaluationWindowSchema,
}).strict();

// ============================================================================
// HEALTH EVALUATION OUTPUT SCHEMA
// ============================================================================

/**
 * Complete health evaluation for a target (service, agent, or provider)
 */
export const HealthEvaluationSchema = z.object({
  // Evaluation metadata
  evaluation_id: z.string().uuid(),
  evaluated_at: z.string().datetime(),

  // Target identification
  target: TargetSchema,

  // Overall health state (composite)
  overall_state: HealthStateSchema,
  overall_trend: HealthTrendSchema,
  overall_confidence: z.number().min(0).max(1),

  // State transition tracking (hysteresis)
  state_transition: StateTransitionSchema,

  // Individual indicators
  indicators: z.array(HealthIndicatorSchema),

  // Trend analysis
  trends: z.array(HealthTrendAnalysisSchema).optional(),

  // Aggregate statistics
  statistics: AggregateStatisticsSchema,

  // Evaluation window
  evaluation_window: EvaluationWindowSpecSchema,

  // Schema versioning
  schema_version: z.string().default('1.0.0'),
}).strict();

// ============================================================================
// DECISION EVENT SCHEMA (Constitutional Compliance)
// ============================================================================

/**
 * Processing metrics for the evaluation
 */
export const ProcessingMetricsSchema = z.object({
  events_analyzed: z.number().int().nonnegative(),
  indicators_computed: z.number().int().nonnegative(),
  processing_time_ms: z.number().nonnegative(),
}).strict();

/**
 * Health Check Agent Decision Event
 *
 * CONSTITUTIONAL CONSTRAINTS:
 * - decision_type: MUST be "health_evaluation"
 * - confidence: STATISTICAL (0.0-1.0) based on sample size and indicator consistency
 * - constraints_applied: ALWAYS [] (advisory agent applies no constraints)
 *
 * This agent MUST NOT:
 * - Trigger alerts
 * - Initiate remediation
 * - Change execution behavior
 * - Access database directly
 */
export const HealthCheckDecisionEventSchema = z.object({
  // Agent identification
  agent_id: z.literal(AGENT_ID),
  agent_version: z.string().regex(/^\d+\.\d+\.\d+$/),

  // Decision metadata (CONSTITUTIONAL CONSTRAINTS)
  decision_type: z.literal(DECISION_TYPE),
  confidence: z.number().min(0).max(1),
  constraints_applied: z.array(z.never()).length(0), // ALWAYS empty

  // Classification
  classification: z.literal(AGENT_CLASSIFICATION),

  // Input/Output tracking
  inputs_hash: z.string().length(64), // SHA256 hex
  outputs: z.array(HealthEvaluationSchema).min(1),

  // Execution tracking
  execution_ref: z.string().min(1),
  timestamp: z.string().datetime(),

  // Processing metrics
  processing_metrics: ProcessingMetricsSchema,

  // Optional metadata
  metadata: z.record(z.string(), z.unknown()).optional(),
}).strict();

// ============================================================================
// INPUT SCHEMAS
// ============================================================================

/**
 * Telemetry aggregates input (from Usage Pattern Agent or direct query)
 */
export const TelemetryAggregatesInputSchema = z.object({
  target_id: z.string().min(1),
  target_type: TargetTypeSchema,

  // Time window
  window_start: z.string().datetime(),
  window_end: z.string().datetime(),

  // Aggregated metrics
  request_count: z.number().int().nonnegative(),
  error_count: z.number().int().nonnegative(),

  // Latency statistics
  latency_avg_ms: z.number().nonnegative(),
  latency_p50_ms: z.number().nonnegative().optional(),
  latency_p90_ms: z.number().nonnegative().optional(),
  latency_p95_ms: z.number().nonnegative().optional(),
  latency_p99_ms: z.number().nonnegative().optional(),

  // Token statistics (for LLM services)
  total_tokens: z.number().int().nonnegative().optional(),
  avg_tokens_per_request: z.number().nonnegative().optional(),

  // Cost statistics
  total_cost_usd: z.number().nonnegative().optional(),

  // Error breakdown
  error_breakdown: z.record(z.string(), z.number().int().nonnegative()).optional(),
}).strict();

/**
 * Target specification in request
 */
export const TargetSpecSchema = z.object({
  type: TargetTypeSchema,
  id: z.string().min(1),
}).strict();

/**
 * Evaluation options
 */
export const EvaluationOptionsSchema = z.object({
  include_trends: z.boolean().default(true),
  include_predictions: z.boolean().default(false),
  evaluation_window: EvaluationWindowSchema.default('5m'),
  trend_window: TrendWindowSchema.default('1h'),
}).strict();

/**
 * Health evaluation request
 */
export const HealthEvaluationRequestSchema = z.object({
  // Target specification
  targets: z.array(TargetSpecSchema).min(1).max(100),

  // Evaluation options
  options: EvaluationOptionsSchema.default({}),

  // Request metadata
  request_id: z.string().uuid().optional(),
}).strict();

// ============================================================================
// RESPONSE SCHEMAS
// ============================================================================

/**
 * Successful evaluation response
 */
export const HealthEvaluationResponseSchema = z.object({
  success: z.literal(true),
  evaluations: z.array(HealthEvaluationSchema),
  execution_ref: z.string(),
  processing_time_ms: z.number().nonnegative(),
}).strict();

// ============================================================================
// ERROR SCHEMAS
// ============================================================================

/**
 * Error codes for the agent
 */
export const ErrorCodeSchema = z.enum([
  'INVALID_INPUT',
  'VALIDATION_FAILED',
  'INSUFFICIENT_DATA',
  'TARGET_NOT_FOUND',
  'RUVECTOR_CONNECTION_ERROR',
  'EVALUATION_TIMEOUT',
  'INTERNAL_ERROR',
]);

/**
 * Error details
 */
export const ErrorDetailsSchema = z.object({
  code: ErrorCodeSchema,
  message: z.string(),
  details: z.unknown().optional(),
  timestamp: z.string().datetime(),
  execution_ref: z.string().optional(),
}).strict();

/**
 * Error response
 */
export const ErrorResponseSchema = z.object({
  success: z.literal(false),
  error: ErrorDetailsSchema,
}).strict();

// ============================================================================
// CLI SCHEMAS
// ============================================================================

/**
 * CLI command types
 */
export const CLICommandSchema = z.enum([
  'evaluate',
  'inspect',
  'replay',
  'status',
  'trends',
  'export',
]);

/**
 * CLI output format
 */
export const OutputFormatSchema = z.enum(['json', 'table', 'summary']);

/**
 * CLI invocation schema
 */
export const CLIInvocationSchema = z.discriminatedUnion('command', [
  z.object({
    command: z.literal('evaluate'),
    target_type: TargetTypeSchema,
    target_id: z.string(),
    window: EvaluationWindowSchema.default('5m'),
    output_format: OutputFormatSchema.default('json'),
  }),
  z.object({
    command: z.literal('inspect'),
    evaluation_id: z.string().uuid(),
    output_format: OutputFormatSchema.default('json'),
  }),
  z.object({
    command: z.literal('replay'),
    evaluation_id: z.string().uuid(),
    compare: z.boolean().default(true),
    output_format: OutputFormatSchema.default('json'),
  }),
  z.object({
    command: z.literal('status'),
    detailed: z.boolean().default(false),
    output_format: OutputFormatSchema.default('json'),
  }),
  z.object({
    command: z.literal('trends'),
    target_type: TargetTypeSchema,
    target_id: z.string(),
    window: TrendWindowSchema.default('24h'),
    indicator: IndicatorTypeSchema.optional(),
    output_format: OutputFormatSchema.default('json'),
  }),
  z.object({
    command: z.literal('export'),
    format: z.enum(['json', 'csv']).default('json'),
    output: z.string().optional(),
  }),
]);

// ============================================================================
// TYPE EXPORTS
// ============================================================================

export type HealthState = z.infer<typeof HealthStateSchema>;
export type HealthTrend = z.infer<typeof HealthTrendSchema>;
export type IndicatorType = z.infer<typeof IndicatorTypeSchema>;
export type TargetType = z.infer<typeof TargetTypeSchema>;
export type EvaluationWindow = z.infer<typeof EvaluationWindowSchema>;
export type TrendWindow = z.infer<typeof TrendWindowSchema>;

export type MeasurementWindow = z.infer<typeof MeasurementWindowSchema>;
export type Percentiles = z.infer<typeof PercentilesSchema>;
export type HealthIndicator = z.infer<typeof HealthIndicatorSchema>;
export type LatencyBreakdown = z.infer<typeof LatencyBreakdownSchema>;
export type ErrorBreakdown = z.infer<typeof ErrorBreakdownSchema>;
export type ThroughputBreakdown = z.infer<typeof ThroughputBreakdownSchema>;
export type SaturationBreakdown = z.infer<typeof SaturationBreakdownSchema>;

export type TrendDataPoint = z.infer<typeof TrendDataPointSchema>;
export type HealthTrendAnalysis = z.infer<typeof HealthTrendAnalysisSchema>;
export type StateTransition = z.infer<typeof StateTransitionSchema>;
export type Target = z.infer<typeof TargetSchema>;
export type AggregateStatistics = z.infer<typeof AggregateStatisticsSchema>;
export type EvaluationWindowSpec = z.infer<typeof EvaluationWindowSpecSchema>;

export type HealthEvaluation = z.infer<typeof HealthEvaluationSchema>;
export type ProcessingMetrics = z.infer<typeof ProcessingMetricsSchema>;
export type HealthCheckDecisionEvent = z.infer<typeof HealthCheckDecisionEventSchema>;

export type TelemetryAggregatesInput = z.infer<typeof TelemetryAggregatesInputSchema>;
export type TargetSpec = z.infer<typeof TargetSpecSchema>;
export type EvaluationOptions = z.infer<typeof EvaluationOptionsSchema>;
export type HealthEvaluationRequest = z.infer<typeof HealthEvaluationRequestSchema>;
export type HealthEvaluationResponse = z.infer<typeof HealthEvaluationResponseSchema>;

export type ErrorCode = z.infer<typeof ErrorCodeSchema>;
export type ErrorDetails = z.infer<typeof ErrorDetailsSchema>;
export type ErrorResponse = z.infer<typeof ErrorResponseSchema>;

export type CLICommand = z.infer<typeof CLICommandSchema>;
export type OutputFormat = z.infer<typeof OutputFormatSchema>;
export type CLIInvocation = z.infer<typeof CLIInvocationSchema>;
