/**
 * Failure Classification Agent - Contract Schemas
 *
 * CONSTITUTIONAL CLASSIFICATION: READ-ONLY, DIAGNOSTIC
 *
 * This agent classifies observed failures into deterministic categories.
 * It MUST NOT:
 * - Attempt remediation
 * - Correlate incidents
 * - Escalate events
 * - Trigger alerts
 * - Modify system state
 * - Execute SQL directly
 */

import { z } from 'zod';

// =============================================================================
// PROVIDER SCHEMA (matching Rust Provider enum)
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

// =============================================================================
// FAILURE CATEGORY SCHEMA
// =============================================================================
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

  // Unknown/Unclassified
  'unknown',
]);

export const FailureSeveritySchema = z.enum([
  'critical',    // Service-impacting, immediate attention required
  'high',        // Significant impact, urgent investigation needed
  'medium',      // Moderate impact, scheduled investigation
  'low',         // Minor impact, informational
  'informational', // No direct impact, diagnostic value only
]);

export const FailureCauseSchema = z.enum([
  'provider',        // Failure originated from LLM provider
  'network',         // Network infrastructure failure
  'client',          // Client-side error (invalid request)
  'configuration',   // Misconfiguration
  'resource',        // Resource exhaustion
  'policy',          // Policy enforcement (rate limits, content filters)
  'unknown',         // Cannot determine cause
]);

// =============================================================================
// INPUT SCHEMAS
// =============================================================================
export const SpanStatusSchema = z.enum(['OK', 'ERROR', 'UNSET']);

export const SpanEventSchema = z.object({
  name: z.string(),
  timestamp: z.string().datetime(),
  attributes: z.record(z.string(), z.unknown()).default({}),
});

export const LatencySchema = z.object({
  start_time: z.string().datetime(),
  end_time: z.string().datetime().optional(),
  duration_ms: z.number().nonnegative(),
  time_to_first_token_ms: z.number().nonnegative().optional(),
});

export const ErrorDetailsSchema = z.object({
  code: z.string().optional(),
  message: z.string(),
  type: z.string().optional(),
  http_status: z.number().int().min(100).max(599).optional(),
  retry_after_ms: z.number().nonnegative().optional(),
  raw_response: z.string().optional(),
});

/**
 * FailureEventSchema - Input for classification
 *
 * This represents a telemetry event that has been flagged as a failure.
 * The agent classifies this event but does NOT modify or act upon it.
 */
export const FailureEventSchema = z.object({
  // Core identifiers
  span_id: z.string().min(1),
  trace_id: z.string().min(1),
  parent_span_id: z.string().optional(),

  // Provider context
  provider: ProviderSchema,
  model: z.string().min(1),

  // Failure details
  status: z.literal('ERROR'),
  error: ErrorDetailsSchema,

  // Timing
  latency: LatencySchema,

  // Request context (for classification)
  request_size_bytes: z.number().int().nonnegative().optional(),
  request_tokens: z.number().int().nonnegative().optional(),

  // Metadata
  metadata: z.record(z.string(), z.unknown()).default({}),
  events: z.array(SpanEventSchema).default([]),
  attributes: z.record(z.string(), z.unknown()).default({}),

  // Temporal context
  timestamp: z.string().datetime(),
}).strict();

/**
 * BatchClassificationRequestSchema - Batch input for classification
 */
export const BatchClassificationRequestSchema = z.object({
  events: z.array(FailureEventSchema).min(1).max(1000),
  correlation_id: z.string().optional(),
}).strict();

// =============================================================================
// OUTPUT SCHEMAS
// =============================================================================

/**
 * FailureClassificationSchema - Classification output for a single event
 */
export const FailureClassificationSchema = z.object({
  // Source reference
  span_id: z.string().min(1),
  trace_id: z.string().min(1),

  // Classification results
  category: FailureCategorySchema,
  severity: FailureSeveritySchema,
  cause: FailureCauseSchema,

  // Classification confidence (analytical, not probabilistic)
  confidence: z.number().min(0).max(1),
  confidence_factors: z.array(z.string()).default([]),

  // Classification reasoning (for diagnostics)
  classification_signals: z.array(z.object({
    signal_type: z.string(),
    signal_value: z.string(),
    weight: z.number().min(0).max(1),
  })).default([]),

  // Recommendations (ADVISORY ONLY - no action taken)
  recommendations: z.array(z.string()).default([]),

  // Temporal metadata
  classified_at: z.string().datetime(),
  classification_latency_ms: z.number().nonnegative(),

  // Schema version
  schema_version: z.string().default('1.0.0'),
}).strict();

/**
 * BatchClassificationResultSchema - Batch output
 */
export const BatchClassificationResultSchema = z.object({
  classifications: z.array(FailureClassificationSchema).min(1),
  batch_id: z.string(),
  total_events: z.number().int().positive(),
  classified_count: z.number().int().nonnegative(),
  failed_count: z.number().int().nonnegative(),
  processing_time_ms: z.number().nonnegative(),
}).strict();

// =============================================================================
// HARDENED: Evidence Reference Schema
// =============================================================================

/**
 * EvidenceRefSchema - Reference to supporting evidence for audit trails
 */
export const EvidenceRefSchema = z.object({
  ref_type: z.enum(['span_id', 'trace_id', 'log_id', 'metric_id', 'external']),
  ref_value: z.string().min(1),
  timestamp: z.string().datetime().optional(),
  source: z.string().optional(),
});

// =============================================================================
// DECISION EVENT SCHEMA (Constitutional Constraints + HARDENED)
// =============================================================================

/**
 * DecisionEventSchema - Constitutional compliance schema
 *
 * CRITICAL: This schema enforces constitutional constraints:
 * - decision_type MUST be 'failure_classification'
 * - constraints_applied MUST be empty (read-only agent)
 * - confidence is analytical (based on signal matching), not probabilistic
 *
 * HARDENED: Phase 1 Layer 1 standardization:
 * - source_agent, domain, phase, layer REQUIRED
 * - event_type differentiates signal types
 * - evidence_refs for audit trails
 */
export const DecisionEventSchema = z.object({
  // HARDENED: Agent Identity (Phase 1 Layer 1 standardization)
  source_agent: z.string().min(1).describe('Agent name emitting this event'),
  domain: z.string().min(1).describe('Agent domain'),
  phase: z.literal('phase1').describe('Deployment phase'),
  layer: z.literal('layer1').describe('Architecture layer'),

  // HARDENED: Event type (signal, NOT conclusion)
  event_type: z.string().min(1).describe('Type of signal being emitted'),

  // Agent identification (backwards compatibility)
  agent_id: z.literal('failure-classification-agent'),
  agent_version: z.string().regex(/^\d+\.\d+\.\d+$/),

  // Decision type (MUST be this literal)
  decision_type: z.literal('failure_classification'),

  // Input hash for reproducibility
  inputs_hash: z.string().length(64), // SHA256 hex

  // Classification outputs
  outputs: z.array(FailureClassificationSchema).min(1),

  // Confidence (analytical, not probabilistic)
  confidence: z.number().min(0).max(1),

  // HARDENED: Evidence references for audit trails
  evidence_refs: z.array(EvidenceRefSchema).default([]),

  // CONSTITUTIONAL CONSTRAINT: No constraints applied for read-only agent
  constraints_applied: z.array(z.never()).length(0),

  // Execution reference for tracing
  execution_ref: z.string().min(1),

  // Timestamp (UTC)
  timestamp: z.string().datetime(),
}).strict();

// =============================================================================
// CLI QUERY SCHEMAS
// =============================================================================

export const ClassificationQuerySchema = z.object({
  // Filtering
  span_id: z.string().optional(),
  trace_id: z.string().optional(),
  provider: ProviderSchema.optional(),
  category: FailureCategorySchema.optional(),
  severity: FailureSeveritySchema.optional(),
  cause: FailureCauseSchema.optional(),

  // Time range
  start_time: z.string().datetime().optional(),
  end_time: z.string().datetime().optional(),

  // Pagination
  limit: z.number().int().min(1).max(1000).default(100),
  offset: z.number().int().nonnegative().default(0),

  // Sorting
  sort_by: z.enum(['timestamp', 'severity', 'category']).default('timestamp'),
  sort_order: z.enum(['asc', 'desc']).default('desc'),
}).strict();

export const AnalysisQuerySchema = z.object({
  // Grouping
  group_by: z.enum(['category', 'severity', 'cause', 'provider', 'model']).default('category'),

  // Time range
  start_time: z.string().datetime().optional(),
  end_time: z.string().datetime().optional(),
  time_window_hours: z.number().int().positive().default(24),

  // Filtering
  provider: ProviderSchema.optional(),
  min_severity: FailureSeveritySchema.optional(),
}).strict();

export const AnalysisResultSchema = z.object({
  query: AnalysisQuerySchema,
  aggregations: z.array(z.object({
    key: z.string(),
    count: z.number().int().nonnegative(),
    percentage: z.number().min(0).max(100),
    avg_confidence: z.number().min(0).max(1),
  })),
  total_failures: z.number().int().nonnegative(),
  time_range: z.object({
    start: z.string().datetime(),
    end: z.string().datetime(),
  }),
  generated_at: z.string().datetime(),
}).strict();

// =============================================================================
// TYPE EXPORTS
// =============================================================================

export type Provider = z.infer<typeof ProviderSchema>;
export type FailureCategory = z.infer<typeof FailureCategorySchema>;
export type FailureSeverity = z.infer<typeof FailureSeveritySchema>;
export type FailureCause = z.infer<typeof FailureCauseSchema>;
export type SpanStatus = z.infer<typeof SpanStatusSchema>;
export type SpanEvent = z.infer<typeof SpanEventSchema>;
export type Latency = z.infer<typeof LatencySchema>;
export type ErrorDetails = z.infer<typeof ErrorDetailsSchema>;
export type FailureEvent = z.infer<typeof FailureEventSchema>;
export type BatchClassificationRequest = z.infer<typeof BatchClassificationRequestSchema>;
export type FailureClassification = z.infer<typeof FailureClassificationSchema>;
export type BatchClassificationResult = z.infer<typeof BatchClassificationResultSchema>;
export type DecisionEvent = z.infer<typeof DecisionEventSchema>;
export type ClassificationQuery = z.infer<typeof ClassificationQuerySchema>;
export type AnalysisQuery = z.infer<typeof AnalysisQuerySchema>;
export type AnalysisResult = z.infer<typeof AnalysisResultSchema>;

// HARDENED: Evidence reference type
export type EvidenceRef = z.infer<typeof EvidenceRefSchema>;
