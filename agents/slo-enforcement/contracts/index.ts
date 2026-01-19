/**
 * SLO/SLA Enforcement Agent - Contract Exports
 *
 * Re-exports all schemas, types, and validation functions
 * from the contracts module.
 */

export {
  // Enums
  SloIndicatorTypeSchema,
  SloOperatorSchema,
  ViolationSeveritySchema,
  BreachTypeSchema,
  TimeWindowSchema,

  // Core schemas
  SloDefinitionSchema,
  TelemetryMetricSchema,
  SloEnforcementRequestSchema,
  BatchEnforcementRequestSchema,
  MetricContextSchema,
  SloViolationSchema,
  SloStatusSchema,
  EnforcementResultSchema,
  DecisionEventSchema,

  // Query schemas
  ViolationQuerySchema,
  ReplayRequestSchema,
  AnalysisRequestSchema,
  AnalysisResultSchema,

  // Constants
  AGENT_METADATA,

  // Validation functions
  validateSloEnforcementRequest,
  validateDecisionEvent,
  validateViolationQuery,

  // Type guards
  isSlaViolation,
  isCriticalViolation,
  isNearBreach,
} from './schemas';

export type {
  // Types
  SloIndicatorType,
  SloOperator,
  ViolationSeverity,
  BreachType,
  TimeWindow,
  SloDefinition,
  TelemetryMetric,
  SloEnforcementRequest,
  BatchEnforcementRequest,
  MetricContext,
  SloViolation,
  SloStatus,
  EnforcementResult,
  DecisionEvent,
  ViolationQuery,
  ReplayRequest,
  AnalysisRequest,
  AnalysisResult,
  ValidationResult,
} from './schemas';
