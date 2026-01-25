/**
 * Failure Classification Agent - Contract Exports
 *
 * This module exports all schemas, types, and validation utilities
 * for the Failure Classification Agent.
 *
 * CONSTITUTIONAL CLASSIFICATION: READ-ONLY, DIAGNOSTIC
 */

// =============================================================================
// SCHEMA EXPORTS
// =============================================================================
export {
  // Provider and classification enums
  ProviderSchema,
  FailureCategorySchema,
  FailureSeveritySchema,
  FailureCauseSchema,
  SpanStatusSchema,

  // Input schemas
  SpanEventSchema,
  LatencySchema,
  ErrorDetailsSchema,
  FailureEventSchema,
  BatchClassificationRequestSchema,

  // Output schemas
  FailureClassificationSchema,
  BatchClassificationResultSchema,

  // HARDENED: Evidence reference schema
  EvidenceRefSchema,

  // Decision event schema (constitutional + hardened)
  DecisionEventSchema,

  // Query schemas
  ClassificationQuerySchema,
  AnalysisQuerySchema,
  AnalysisResultSchema,
} from './schemas';

// =============================================================================
// TYPE EXPORTS
// =============================================================================
export type {
  Provider,
  FailureCategory,
  FailureSeverity,
  FailureCause,
  SpanStatus,
  SpanEvent,
  Latency,
  ErrorDetails,
  FailureEvent,
  BatchClassificationRequest,
  FailureClassification,
  BatchClassificationResult,
  DecisionEvent,
  ClassificationQuery,
  AnalysisQuery,
  AnalysisResult,
  // HARDENED: Evidence reference type
  EvidenceRef,
} from './schemas';

export type {
  AgentClassification,
  AgentCapability,
  ProhibitedOperation,
  ClassificationSignal,
  ClassificationRule,
  RuleCondition,
  HandlerContext,
  HandlerResponse,
  RuvectorConfig,
  RuvectorHealthStatus,
  DecisionQuery,
  CLIInspectResult,
  CLIReplayResult,
  CLIStatusResult,
  ClassificationMetrics,
  TelemetrySpan,
} from './types';

// =============================================================================
// VALIDATION EXPORTS
// =============================================================================
export {
  // Validation functions
  validateFailureEvent,
  validateBatchRequest,
  validateClassification,
  validateBatchResult,
  validateDecisionEvent,
  validateClassificationQuery,
  validateAnalysisQuery,

  // Constitutional validation
  validateConstitutionalOperation,
  assertConstitutionalCompliance,

  // Hashing utilities
  hashInput,
  hashInputs,

  // Error classes
  ConstitutionalViolationError,
  ValidationError,

  // Types
  type ValidationResult,
} from './validation';

// =============================================================================
// AGENT METADATA (HARDENED)
// =============================================================================
export const AGENT_METADATA = {
  id: 'failure-classification-agent',
  version: '1.0.0',
  classification: 'READ-ONLY' as const,
  decision_type: 'failure_classification',

  // HARDENED: Phase 1 Layer 1 identity
  hardened: {
    phase: 'phase1' as const,
    layer: 'layer1' as const,
    domain: 'diagnostics' as const,
    ruvector_required: true as const,
    min_decision_events_per_run: 1 as const,
  },

  capabilities: [
    'classify_failures',
    'aggregate_statistics',
    'generate_reports',
    'emit_decision_events',
  ] as const,

  prohibited_operations: [
    'sql_execute',
    'sql_write',
    'orchestration_trigger',
    'state_modify',
    'constraint_apply',
    'retry_trigger',
    'alert_trigger',
    'remediation_trigger',
    'incident_correlation',
    'escalation_trigger',
  ] as const,

  downstream_consumers: [
    'post-mortem-generator-agent',
    'incident-reporting-systems',
    'governance-audit-views',
    'llm-analytics-hub',
  ] as const,
} as const;
