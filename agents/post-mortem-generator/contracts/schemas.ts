/**
 * Post-Mortem Generator Agent - Contract Schemas
 *
 * CONSTITUTIONAL CLASSIFICATION: READ-ONLY
 *
 * This agent generates structured, reproducible post-mortem reports from
 * historical telemetry, failure classifications, and health evaluations.
 *
 * It MUST NOT:
 * - Influence live systems
 * - Write advisory constraints
 * - Recommend remediation actions
 * - Trigger alerts
 * - Modify system state
 * - Execute SQL directly
 * - Invoke other agents
 */

import { z } from 'zod';

// =============================================================================
// SHARED SCHEMAS (from agentics-contracts)
// =============================================================================

export const ProviderSchema = z.enum([
  'openai',
  'anthropic',
  'google',
  'mistral',
  'cohere',
  'self_hosted',
  'custom',
]);

export const FailureCategorySchema = z.enum([
  // Network & Connectivity
  'network_timeout',
  'network_connection_refused',
  'network_dns_resolution',
  'network_ssl_handshake',
  // Provider Errors
  'provider_rate_limit',
  'provider_quota_exceeded',
  'provider_service_unavailable',
  'provider_internal_error',
  'provider_model_overloaded',
  'provider_authentication',
  'provider_authorization',
  // Request Errors
  'request_invalid_payload',
  'request_payload_too_large',
  'request_unsupported_model',
  'request_context_length_exceeded',
  'request_content_filter',
  'request_malformed',
  // Response Errors
  'response_incomplete',
  'response_malformed',
  'response_empty',
  'response_parsing_error',
  // Token & Cost Errors
  'token_limit_exceeded',
  'cost_limit_exceeded',
  'billing_error',
  // Timeout Errors
  'timeout_request',
  'timeout_response',
  'timeout_streaming',
  // System Errors
  'system_memory_exhausted',
  'system_resource_unavailable',
  'system_configuration_error',
  // Unknown
  'unknown',
]);

export const FailureSeveritySchema = z.enum([
  'critical',
  'high',
  'medium',
  'low',
  'informational',
]);

export const FailureCauseSchema = z.enum([
  'provider',
  'network',
  'client',
  'configuration',
  'resource',
  'policy',
  'unknown',
]);

export const HealthStateSchema = z.enum([
  'healthy',
  'degraded',
  'unhealthy',
]);

// =============================================================================
// INPUT SCHEMAS
// =============================================================================

/**
 * Time range specification for post-mortem analysis
 */
export const TimeRangeSchema = z.object({
  start_time: z.string().datetime(),
  end_time: z.string().datetime(),
});

/**
 * Scope specification for post-mortem analysis
 */
export const PostMortemScopeSchema = z.object({
  // Optional filters to narrow the scope
  providers: z.array(ProviderSchema).optional(),
  models: z.array(z.string()).optional(),
  services: z.array(z.string()).optional(),
  trace_ids: z.array(z.string()).optional(),
  // Include specific failure categories
  include_categories: z.array(FailureCategorySchema).optional(),
  // Minimum severity to include
  min_severity: FailureSeveritySchema.optional(),
});

/**
 * Post-mortem generation options
 */
export const PostMortemOptionsSchema = z.object({
  // Include detailed timeline events
  include_timeline: z.boolean().default(true),
  // Include failure classification breakdown
  include_classification_breakdown: z.boolean().default(true),
  // Include health state transitions
  include_health_transitions: z.boolean().default(true),
  // Include contributing factors analysis
  include_contributing_factors: z.boolean().default(true),
  // Include statistical summary
  include_statistics: z.boolean().default(true),
  // Maximum number of timeline events to include
  max_timeline_events: z.number().int().positive().default(1000),
  // Output format
  format: z.enum(['json', 'structured']).default('structured'),
});

/**
 * PostMortemRequestSchema - Primary input for post-mortem generation
 */
export const PostMortemRequestSchema = z.object({
  // Required: Time range for analysis
  time_range: TimeRangeSchema,
  // Optional: Scope filtering
  scope: PostMortemScopeSchema.optional(),
  // Optional: Generation options
  options: PostMortemOptionsSchema.optional(),
  // Incident identifier (if this is for a specific incident)
  incident_id: z.string().optional(),
  // Correlation ID for tracing
  correlation_id: z.string().optional(),
}).strict();

/**
 * Batch post-mortem request
 */
export const BatchPostMortemRequestSchema = z.object({
  requests: z.array(PostMortemRequestSchema).min(1).max(10),
  correlation_id: z.string().optional(),
}).strict();

// =============================================================================
// OUTPUT SCHEMAS
// =============================================================================

/**
 * Timeline event in the post-mortem report
 */
export const TimelineEventSchema = z.object({
  // Event timestamp
  timestamp: z.string().datetime(),
  // Event type
  event_type: z.enum([
    'failure',
    'health_transition',
    'recovery',
    'first_failure',
    'last_failure',
    'peak_error_rate',
    'service_degradation',
    'service_recovery',
  ]),
  // Event description
  description: z.string(),
  // Source data references
  span_id: z.string().optional(),
  trace_id: z.string().optional(),
  // Associated metadata
  provider: ProviderSchema.optional(),
  model: z.string().optional(),
  service: z.string().optional(),
  // Severity at this point
  severity: FailureSeveritySchema.optional(),
  // Health state at this point
  health_state: HealthStateSchema.optional(),
});

/**
 * Classification breakdown in the post-mortem
 */
export const ClassificationBreakdownSchema = z.object({
  // By category
  by_category: z.array(z.object({
    category: FailureCategorySchema,
    count: z.number().int().nonnegative(),
    percentage: z.number().min(0).max(100),
    first_occurrence: z.string().datetime(),
    last_occurrence: z.string().datetime(),
  })),
  // By severity
  by_severity: z.array(z.object({
    severity: FailureSeveritySchema,
    count: z.number().int().nonnegative(),
    percentage: z.number().min(0).max(100),
  })),
  // By cause
  by_cause: z.array(z.object({
    cause: FailureCauseSchema,
    count: z.number().int().nonnegative(),
    percentage: z.number().min(0).max(100),
  })),
  // By provider
  by_provider: z.array(z.object({
    provider: ProviderSchema,
    count: z.number().int().nonnegative(),
    percentage: z.number().min(0).max(100),
    models_affected: z.array(z.string()),
  })),
});

/**
 * Health state transition in the post-mortem
 */
export const HealthTransitionSchema = z.object({
  timestamp: z.string().datetime(),
  target_id: z.string(),
  target_type: z.string(),
  from_state: HealthStateSchema,
  to_state: HealthStateSchema,
  duration_in_state_ms: z.number().int().nonnegative().optional(),
  trigger_event: z.string().optional(),
});

/**
 * Contributing factor identified in the post-mortem
 */
export const ContributingFactorSchema = z.object({
  // Factor identifier
  factor_id: z.string(),
  // Factor type
  factor_type: z.enum([
    'primary_cause',
    'contributing_cause',
    'correlation',
    'environmental',
    'temporal',
  ]),
  // Description
  description: z.string(),
  // Confidence in this factor (analytical, not probabilistic)
  confidence: z.number().min(0).max(1),
  // Evidence supporting this factor
  evidence: z.array(z.object({
    type: z.string(),
    reference: z.string(),
    weight: z.number().min(0).max(1),
  })),
  // Time range when this factor was active
  active_period: TimeRangeSchema.optional(),
});

/**
 * Statistical summary in the post-mortem
 */
export const StatisticalSummarySchema = z.object({
  // Total counts
  total_failures: z.number().int().nonnegative(),
  total_requests: z.number().int().nonnegative(),
  error_rate: z.number().min(0).max(1),
  // Time-based metrics
  duration_ms: z.number().int().nonnegative(),
  time_to_first_failure_ms: z.number().int().nonnegative().optional(),
  time_to_recovery_ms: z.number().int().nonnegative().optional(),
  // Latency statistics during incident
  latency_p50_ms: z.number().nonnegative().optional(),
  latency_p95_ms: z.number().nonnegative().optional(),
  latency_p99_ms: z.number().nonnegative().optional(),
  // Affected entities
  affected_providers: z.number().int().nonnegative(),
  affected_models: z.number().int().nonnegative(),
  affected_services: z.number().int().nonnegative(),
  affected_users: z.number().int().nonnegative().optional(),
  // Peak metrics
  peak_error_rate: z.number().min(0).max(1),
  peak_error_rate_timestamp: z.string().datetime().optional(),
});

/**
 * PostMortemReportSchema - Primary output for post-mortem generation
 */
export const PostMortemReportSchema = z.object({
  // Report metadata
  report_id: z.string().uuid(),
  generated_at: z.string().datetime(),
  generation_latency_ms: z.number().nonnegative(),
  schema_version: z.string().default('1.0.0'),
  // Input context
  time_range: TimeRangeSchema,
  incident_id: z.string().optional(),
  // Executive summary
  summary: z.object({
    title: z.string(),
    description: z.string(),
    impact_level: FailureSeveritySchema,
    status: z.enum(['resolved', 'ongoing', 'unknown']),
  }),
  // Timeline of events
  timeline: z.array(TimelineEventSchema).optional(),
  // Classification breakdown
  classification_breakdown: ClassificationBreakdownSchema.optional(),
  // Health transitions
  health_transitions: z.array(HealthTransitionSchema).optional(),
  // Contributing factors
  contributing_factors: z.array(ContributingFactorSchema).optional(),
  // Statistical summary
  statistics: StatisticalSummarySchema.optional(),
  // Data quality notes
  data_quality: z.object({
    completeness: z.number().min(0).max(1),
    notes: z.array(z.string()),
    gaps: z.array(z.object({
      start: z.string().datetime(),
      end: z.string().datetime(),
      reason: z.string(),
    })),
  }).optional(),
}).strict();

/**
 * Batch post-mortem result
 */
export const BatchPostMortemResultSchema = z.object({
  reports: z.array(PostMortemReportSchema),
  batch_id: z.string(),
  total_requested: z.number().int().positive(),
  successful_count: z.number().int().nonnegative(),
  failed_count: z.number().int().nonnegative(),
  processing_time_ms: z.number().nonnegative(),
}).strict();

// =============================================================================
// DECISION EVENT SCHEMA (Constitutional Constraints)
// =============================================================================

/**
 * DecisionEventSchema - Constitutional compliance schema
 *
 * CRITICAL: This schema enforces constitutional constraints:
 * - decision_type MUST be 'postmortem_generation'
 * - constraints_applied MUST be empty (read-only agent)
 * - confidence is analytical (based on data completeness), not probabilistic
 */
export const DecisionEventSchema = z.object({
  // Agent identification
  agent_id: z.literal('post-mortem-generator-agent'),
  agent_version: z.string().regex(/^\d+\.\d+\.\d+$/),
  // Decision type (MUST be this literal)
  decision_type: z.literal('postmortem_generation'),
  // Input hash for reproducibility
  inputs_hash: z.string().length(64), // SHA256 hex
  // Report outputs
  outputs: z.array(PostMortemReportSchema).min(1),
  // Confidence (analytical: based on data completeness and quality)
  confidence: z.number().min(0).max(1),
  // CONSTITUTIONAL CONSTRAINT: No constraints applied for read-only agent
  constraints_applied: z.array(z.never()).length(0),
  // Execution reference for tracing
  execution_ref: z.string().min(1),
  // Timestamp (UTC)
  timestamp: z.string().datetime(),
}).strict();

// =============================================================================
// ERROR SCHEMAS
// =============================================================================

export const ErrorCodeSchema = z.enum([
  'VALIDATION_FAILED',
  'INVALID_INPUT',
  'INTERNAL_ERROR',
  'PERSISTENCE_ERROR',
  'TIMEOUT_ERROR',
  'INSUFFICIENT_DATA',
  'CONSTITUTIONAL_VIOLATION',
]);

export const ErrorResponseSchema = z.object({
  success: z.literal(false),
  error: z.object({
    code: ErrorCodeSchema,
    message: z.string(),
    details: z.unknown().optional(),
    timestamp: z.string().datetime(),
    execution_ref: z.string().optional(),
  }),
});

export const SuccessResponseSchema = z.object({
  success: z.literal(true),
  report: PostMortemReportSchema,
  execution_ref: z.string(),
  processing_time_ms: z.number().nonnegative(),
});

// =============================================================================
// CLI QUERY SCHEMAS
// =============================================================================

export const PostMortemQuerySchema = z.object({
  // Filtering
  report_id: z.string().uuid().optional(),
  incident_id: z.string().optional(),
  // Time range
  start_time: z.string().datetime().optional(),
  end_time: z.string().datetime().optional(),
  // Pagination
  limit: z.number().int().min(1).max(100).default(10),
  offset: z.number().int().nonnegative().default(0),
  // Sorting
  sort_by: z.enum(['generated_at', 'time_range_start', 'impact_level']).default('generated_at'),
  sort_order: z.enum(['asc', 'desc']).default('desc'),
}).strict();

export const CLIInspectResultSchema = z.object({
  report: PostMortemReportSchema.optional(),
  decision_event: DecisionEventSchema.optional(),
  metadata: z.object({
    retrieved_at: z.string().datetime(),
    source: z.enum(['ruvector', 'cache']),
  }),
});

export const CLIReplayResultSchema = z.object({
  original_report: PostMortemReportSchema,
  replayed_report: PostMortemReportSchema,
  match: z.boolean(),
  differences: z.array(z.object({
    path: z.string(),
    original: z.unknown(),
    replayed: z.unknown(),
  })),
});

export const CLIStatusResultSchema = z.object({
  agent_id: z.string(),
  agent_version: z.string(),
  classification: z.object({
    type: z.string(),
    subtype: z.string(),
  }),
  status: z.enum(['healthy', 'degraded', 'unhealthy']),
  uptime_seconds: z.number().nonnegative(),
  last_generation_at: z.string().datetime().optional(),
  metrics: z.object({
    total_reports_generated: z.number().int().nonnegative(),
    reports_last_hour: z.number().int().nonnegative(),
    avg_generation_latency_ms: z.number().nonnegative(),
    error_rate: z.number().min(0).max(1),
  }),
  ruvector_status: z.object({
    healthy: z.boolean(),
    latencyMs: z.number().nonnegative(),
    error: z.string().optional(),
  }),
});

// =============================================================================
// AGENT METADATA
// =============================================================================

export const AGENT_METADATA = {
  id: 'post-mortem-generator-agent',
  version: '1.0.0',
  name: 'Post-Mortem Generator Agent',
  classification: {
    type: 'READ-ONLY',
    subtype: 'ANALYTICAL',
    enforcement: false,
    advisory: false,
  },
  decision_type: 'postmortem_generation',
} as const;

// =============================================================================
// TYPE EXPORTS
// =============================================================================

export type Provider = z.infer<typeof ProviderSchema>;
export type FailureCategory = z.infer<typeof FailureCategorySchema>;
export type FailureSeverity = z.infer<typeof FailureSeveritySchema>;
export type FailureCause = z.infer<typeof FailureCauseSchema>;
export type HealthState = z.infer<typeof HealthStateSchema>;
export type TimeRange = z.infer<typeof TimeRangeSchema>;
export type PostMortemScope = z.infer<typeof PostMortemScopeSchema>;
export type PostMortemOptions = z.infer<typeof PostMortemOptionsSchema>;
export type PostMortemRequest = z.infer<typeof PostMortemRequestSchema>;
export type BatchPostMortemRequest = z.infer<typeof BatchPostMortemRequestSchema>;
export type TimelineEvent = z.infer<typeof TimelineEventSchema>;
export type ClassificationBreakdown = z.infer<typeof ClassificationBreakdownSchema>;
export type HealthTransition = z.infer<typeof HealthTransitionSchema>;
export type ContributingFactor = z.infer<typeof ContributingFactorSchema>;
export type StatisticalSummary = z.infer<typeof StatisticalSummarySchema>;
export type PostMortemReport = z.infer<typeof PostMortemReportSchema>;
export type BatchPostMortemResult = z.infer<typeof BatchPostMortemResultSchema>;
export type DecisionEvent = z.infer<typeof DecisionEventSchema>;
export type ErrorCode = z.infer<typeof ErrorCodeSchema>;
export type ErrorResponse = z.infer<typeof ErrorResponseSchema>;
export type SuccessResponse = z.infer<typeof SuccessResponseSchema>;
export type PostMortemQuery = z.infer<typeof PostMortemQuerySchema>;
export type CLIInspectResult = z.infer<typeof CLIInspectResultSchema>;
export type CLIReplayResult = z.infer<typeof CLIReplayResultSchema>;
export type CLIStatusResult = z.infer<typeof CLIStatusResultSchema>;
