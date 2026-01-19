/**
 * SLO/SLA Enforcement Agent - Contract Schemas
 *
 * Classification: ENFORCEMENT-CLASS, NON-ACTUATING
 * decision_type: "slo_violation_detection"
 *
 * This agent MUST:
 * - NOT trigger alerts directly
 * - NOT initiate remediation
 * - NOT change policies or thresholds at runtime
 *
 * Primary consumers:
 * - LLM-Governance-Dashboard
 * - LLM-Policy-Engine
 * - Incident review workflows
 */

import { z } from 'zod';

// ============================================================================
// SLO/SLA POLICY DEFINITIONS
// ============================================================================

/**
 * SLO Indicator Type - The metric being measured
 */
export const SloIndicatorTypeSchema = z.enum([
  'latency_p50',
  'latency_p95',
  'latency_p99',
  'error_rate',
  'availability',
  'throughput',
  'saturation',
  'ttft',          // Time to First Token
  'token_rate',    // Tokens per second
  'cost_per_request',
  'cost_per_1k_tokens',
]);

export type SloIndicatorType = z.infer<typeof SloIndicatorTypeSchema>;

/**
 * SLO Comparison Operator
 */
export const SloOperatorSchema = z.enum([
  'lt',   // Less than
  'lte',  // Less than or equal
  'gt',   // Greater than
  'gte',  // Greater than or equal
  'eq',   // Equal
  'neq',  // Not equal
]);

export type SloOperator = z.infer<typeof SloOperatorSchema>;

/**
 * Violation Severity Level
 */
export const ViolationSeveritySchema = z.enum([
  'critical',      // Immediate attention required, SLA breach
  'high',          // Near-breach, requires urgent attention
  'medium',        // Warning threshold exceeded
  'low',           // Minor deviation, informational
]);

export type ViolationSeverity = z.infer<typeof ViolationSeveritySchema>;

/**
 * Breach Type - Whether this is an SLO or SLA violation
 */
export const BreachTypeSchema = z.enum([
  'slo_breach',       // Internal objective breached
  'sla_breach',       // External agreement breached
  'near_breach',      // Approaching threshold (warning)
  'consecutive_breach', // Multiple consecutive breaches
]);

export type BreachType = z.infer<typeof BreachTypeSchema>;

/**
 * Time Window for aggregation
 */
export const TimeWindowSchema = z.enum([
  '1m',    // 1 minute
  '5m',    // 5 minutes
  '15m',   // 15 minutes
  '1h',    // 1 hour
  '6h',    // 6 hours
  '24h',   // 24 hours
  '7d',    // 7 days
  '30d',   // 30 days
]);

export type TimeWindow = z.infer<typeof TimeWindowSchema>;

// ============================================================================
// SLO DEFINITION SCHEMA
// ============================================================================

/**
 * SLO Definition - Defines an individual Service Level Objective
 */
export const SloDefinitionSchema = z.object({
  slo_id: z.string().min(1).max(128),
  name: z.string().min(1).max(256),
  description: z.string().max(1024).optional(),
  indicator: SloIndicatorTypeSchema,
  operator: SloOperatorSchema,
  threshold: z.number(),
  unit: z.string().max(32).optional(),           // e.g., 'ms', '%', 'rps'
  window: TimeWindowSchema,
  provider: z.string().max(64).optional(),       // Filter by provider
  model: z.string().max(128).optional(),         // Filter by model
  environment: z.string().max(64).optional(),    // Filter by environment
  tags: z.array(z.string().max(64)).max(20).optional(),
  is_sla: z.boolean().default(false),            // True if this is an SLA commitment
  sla_penalty_tier: z.number().min(1).max(5).optional(), // Penalty severity if SLA
  warning_threshold_percentage: z.number().min(0).max(100).default(80), // % of threshold for warning
  enabled: z.boolean().default(true),
});

export type SloDefinition = z.infer<typeof SloDefinitionSchema>;

// ============================================================================
// TELEMETRY METRIC INPUT
// ============================================================================

/**
 * Telemetry Metric - Input telemetry data for evaluation
 */
export const TelemetryMetricSchema = z.object({
  metric_id: z.string().uuid(),
  indicator: SloIndicatorTypeSchema,
  value: z.number(),
  unit: z.string().max(32).optional(),
  timestamp: z.string().datetime(),
  window: TimeWindowSchema,
  provider: z.string().max(64).optional(),
  model: z.string().max(128).optional(),
  environment: z.string().max(64).optional(),
  sample_count: z.number().int().min(1).optional(),  // Number of samples in aggregate
  metadata: z.record(z.string().max(64), z.unknown()).optional(),
});

export type TelemetryMetric = z.infer<typeof TelemetryMetricSchema>;

/**
 * SLO Enforcement Request - Input for single evaluation
 */
export const SloEnforcementRequestSchema = z.object({
  slo_definitions: z.array(SloDefinitionSchema).min(1).max(100),
  metrics: z.array(TelemetryMetricSchema).min(1).max(1000),
  evaluation_time: z.string().datetime(),
  include_historical_context: z.boolean().default(false),
  correlation_id: z.string().max(128).optional(),
});

export type SloEnforcementRequest = z.infer<typeof SloEnforcementRequestSchema>;

/**
 * Batch Enforcement Request - Multiple evaluations
 */
export const BatchEnforcementRequestSchema = z.object({
  requests: z.array(SloEnforcementRequestSchema).min(1).max(50),
});

export type BatchEnforcementRequest = z.infer<typeof BatchEnforcementRequestSchema>;

// ============================================================================
// VIOLATION OUTPUT
// ============================================================================

/**
 * Metric Context - Contextual information about the metric at violation time
 */
export const MetricContextSchema = z.object({
  current_value: z.number(),
  threshold_value: z.number(),
  deviation_percentage: z.number(),   // How far from threshold (positive = breach)
  trend: z.enum(['improving', 'stable', 'degrading', 'volatile']),
  samples_in_window: z.number().int().min(1),
  historical_average: z.number().optional(),
  historical_p95: z.number().optional(),
  previous_breaches_in_window: z.number().int().min(0).default(0),
});

export type MetricContext = z.infer<typeof MetricContextSchema>;

/**
 * SLO Violation - A detected violation of an SLO/SLA
 */
export const SloViolationSchema = z.object({
  violation_id: z.string().uuid(),
  slo_id: z.string().min(1).max(128),
  slo_name: z.string().min(1).max(256),
  breach_type: BreachTypeSchema,
  severity: ViolationSeveritySchema,
  indicator: SloIndicatorTypeSchema,
  metric_context: MetricContextSchema,
  is_sla: z.boolean(),
  sla_penalty_tier: z.number().min(1).max(5).optional(),
  detected_at: z.string().datetime(),
  window: TimeWindowSchema,
  provider: z.string().max(64).optional(),
  model: z.string().max(128).optional(),
  environment: z.string().max(64).optional(),
  recommendation: z.string().max(512).optional(),  // Advisory recommendation only
});

export type SloViolation = z.infer<typeof SloViolationSchema>;

/**
 * SLO Status - Current status of an SLO (even if not violated)
 */
export const SloStatusSchema = z.object({
  slo_id: z.string().min(1).max(128),
  slo_name: z.string().min(1).max(256),
  status: z.enum(['healthy', 'warning', 'breached', 'unknown']),
  current_value: z.number().optional(),
  threshold: z.number(),
  compliance_percentage: z.number().min(0).max(100).optional(),
  last_breach_at: z.string().datetime().optional(),
  consecutive_breach_count: z.number().int().min(0).default(0),
});

export type SloStatus = z.infer<typeof SloStatusSchema>;

/**
 * Enforcement Result - Output for single evaluation
 */
export const EnforcementResultSchema = z.object({
  violations: z.array(SloViolationSchema),
  slo_statuses: z.array(SloStatusSchema),
  evaluation_time: z.string().datetime(),
  metrics_evaluated: z.number().int().min(0),
  slos_evaluated: z.number().int().min(0),
  processing_time_ms: z.number().int().min(0),
});

export type EnforcementResult = z.infer<typeof EnforcementResultSchema>;

// ============================================================================
// DECISION EVENT (CONSTITUTIONAL COMPLIANCE)
// ============================================================================

/**
 * Agent Metadata Constants
 */
export const AGENT_METADATA = {
  id: 'slo-enforcement-agent',
  version: '1.0.0',
  decision_type: 'slo_violation_detection',
  classification: 'enforcement-class' as const,
  actuating: false,
} as const;

/**
 * DecisionEvent Schema - Constitutional compliance
 *
 * This schema MUST be validated for every decision event emitted.
 *
 * Required fields per PROMPT 0:
 * - agent_id: Literal agent identifier
 * - agent_version: Semantic version
 * - decision_type: Literal decision type
 * - inputs_hash: SHA256 hex (64 chars)
 * - outputs: Agent-specific outputs
 * - confidence: 0.0-1.0
 * - constraints_applied: ALWAYS [] (empty for non-actuating)
 * - execution_ref: UUID for tracing
 * - timestamp: ISO8601 UTC
 */
export const DecisionEventSchema = z.object({
  agent_id: z.literal(AGENT_METADATA.id),
  agent_version: z.string().regex(/^\d+\.\d+\.\d+$/),
  decision_type: z.literal(AGENT_METADATA.decision_type),
  inputs_hash: z.string().length(64).regex(/^[a-f0-9]{64}$/),
  outputs: z.object({
    violations: z.array(SloViolationSchema),
    slo_statuses: z.array(SloStatusSchema),
    metrics_evaluated: z.number().int().min(0),
    slos_evaluated: z.number().int().min(0),
  }),
  confidence: z.number().min(0).max(1),
  constraints_applied: z.array(z.never()).length(0), // MUST be empty
  execution_ref: z.string().uuid(),
  timestamp: z.string().datetime(),
}).strict();

export type DecisionEvent = z.infer<typeof DecisionEventSchema>;

// ============================================================================
// CLI / QUERY SCHEMAS
// ============================================================================

/**
 * Query for retrieving violations
 */
export const ViolationQuerySchema = z.object({
  slo_id: z.string().optional(),
  breach_type: BreachTypeSchema.optional(),
  severity: ViolationSeveritySchema.optional(),
  provider: z.string().optional(),
  model: z.string().optional(),
  environment: z.string().optional(),
  start_time: z.string().datetime().optional(),
  end_time: z.string().datetime().optional(),
  is_sla: z.boolean().optional(),
  limit: z.number().int().min(1).max(1000).default(100),
  offset: z.number().int().min(0).default(0),
  sort_by: z.enum(['detected_at', 'severity', 'slo_id']).default('detected_at'),
  sort_order: z.enum(['asc', 'desc']).default('desc'),
});

export type ViolationQuery = z.infer<typeof ViolationQuerySchema>;

/**
 * Replay Request - For replaying historical evaluations
 */
export const ReplayRequestSchema = z.object({
  execution_ref: z.string().uuid(),
  replay_id: z.string().uuid().optional(),
  dry_run: z.boolean().default(true), // If true, don't persist results
});

export type ReplayRequest = z.infer<typeof ReplayRequestSchema>;

/**
 * Analysis Request - For aggregated analysis
 */
export const AnalysisRequestSchema = z.object({
  start_time: z.string().datetime(),
  end_time: z.string().datetime(),
  group_by: z.array(z.enum(['slo_id', 'provider', 'model', 'severity', 'breach_type'])).min(1).max(3),
  filter_slo_ids: z.array(z.string()).optional(),
  filter_providers: z.array(z.string()).optional(),
  filter_severities: z.array(ViolationSeveritySchema).optional(),
});

export type AnalysisRequest = z.infer<typeof AnalysisRequestSchema>;

/**
 * Analysis Result - Aggregated violation analysis
 */
export const AnalysisResultSchema = z.object({
  time_range: z.object({
    start: z.string().datetime(),
    end: z.string().datetime(),
  }),
  total_violations: z.number().int().min(0),
  total_evaluations: z.number().int().min(0),
  violation_rate: z.number().min(0).max(1),
  by_severity: z.record(ViolationSeveritySchema, z.number().int().min(0)).optional(),
  by_breach_type: z.record(BreachTypeSchema, z.number().int().min(0)).optional(),
  by_slo: z.record(z.string(), z.number().int().min(0)).optional(),
  by_provider: z.record(z.string(), z.number().int().min(0)).optional(),
  sla_breaches: z.number().int().min(0),
  top_violating_slos: z.array(z.object({
    slo_id: z.string(),
    slo_name: z.string(),
    violation_count: z.number().int().min(0),
    compliance_rate: z.number().min(0).max(1),
  })).max(10).optional(),
});

export type AnalysisResult = z.infer<typeof AnalysisResultSchema>;

// ============================================================================
// VALIDATION HELPERS
// ============================================================================

export interface ValidationResult<T> {
  success: boolean;
  data?: T;
  errors?: z.ZodError['errors'];
}

export function validateSloEnforcementRequest(input: unknown): ValidationResult<SloEnforcementRequest> {
  const result = SloEnforcementRequestSchema.safeParse(input);
  if (result.success) {
    return { success: true, data: result.data };
  }
  return { success: false, errors: result.error.errors };
}

export function validateDecisionEvent(input: unknown): ValidationResult<DecisionEvent> {
  const result = DecisionEventSchema.safeParse(input);
  if (result.success) {
    return { success: true, data: result.data };
  }
  return { success: false, errors: result.error.errors };
}

export function validateViolationQuery(input: unknown): ValidationResult<ViolationQuery> {
  const result = ViolationQuerySchema.safeParse(input);
  if (result.success) {
    return { success: true, data: result.data };
  }
  return { success: false, errors: result.error.errors };
}

// ============================================================================
// TYPE GUARDS
// ============================================================================

export function isSlaViolation(violation: SloViolation): boolean {
  return violation.is_sla === true && violation.breach_type === 'sla_breach';
}

export function isCriticalViolation(violation: SloViolation): boolean {
  return violation.severity === 'critical';
}

export function isNearBreach(violation: SloViolation): boolean {
  return violation.breach_type === 'near_breach';
}
