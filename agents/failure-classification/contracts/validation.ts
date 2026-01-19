/**
 * Failure Classification Agent - Validation Utilities
 *
 * Provides validation functions with constitutional constraint enforcement.
 */

import { createHash } from 'crypto';
import {
  FailureEventSchema,
  BatchClassificationRequestSchema,
  FailureClassificationSchema,
  BatchClassificationResultSchema,
  DecisionEventSchema,
  ClassificationQuerySchema,
  AnalysisQuerySchema,
  type FailureEvent,
  type BatchClassificationRequest,
  type FailureClassification,
  type BatchClassificationResult,
  type DecisionEvent,
  type ClassificationQuery,
  type AnalysisQuery,
} from './schemas';
import type { ProhibitedOperation } from './types';

// =============================================================================
// VALIDATION RESULT TYPE
// =============================================================================

export interface ValidationResult<T> {
  success: boolean;
  data?: T;
  errors?: Array<{
    path: string;
    message: string;
    code: string;
  }>;
}

// =============================================================================
// INPUT VALIDATION
// =============================================================================

/**
 * Validate a single failure event
 */
export function validateFailureEvent(input: unknown): ValidationResult<FailureEvent> {
  const result = FailureEventSchema.safeParse(input);
  if (result.success) {
    return { success: true, data: result.data };
  }
  return {
    success: false,
    errors: result.error.errors.map(e => ({
      path: e.path.join('.'),
      message: e.message,
      code: e.code,
    })),
  };
}

/**
 * Validate batch classification request
 */
export function validateBatchRequest(input: unknown): ValidationResult<BatchClassificationRequest> {
  const result = BatchClassificationRequestSchema.safeParse(input);
  if (result.success) {
    return { success: true, data: result.data };
  }
  return {
    success: false,
    errors: result.error.errors.map(e => ({
      path: e.path.join('.'),
      message: e.message,
      code: e.code,
    })),
  };
}

// =============================================================================
// OUTPUT VALIDATION
// =============================================================================

/**
 * Validate a single classification result
 */
export function validateClassification(input: unknown): ValidationResult<FailureClassification> {
  const result = FailureClassificationSchema.safeParse(input);
  if (result.success) {
    return { success: true, data: result.data };
  }
  return {
    success: false,
    errors: result.error.errors.map(e => ({
      path: e.path.join('.'),
      message: e.message,
      code: e.code,
    })),
  };
}

/**
 * Validate batch classification result
 */
export function validateBatchResult(input: unknown): ValidationResult<BatchClassificationResult> {
  const result = BatchClassificationResultSchema.safeParse(input);
  if (result.success) {
    return { success: true, data: result.data };
  }
  return {
    success: false,
    errors: result.error.errors.map(e => ({
      path: e.path.join('.'),
      message: e.message,
      code: e.code,
    })),
  };
}

/**
 * Validate decision event with constitutional constraints
 */
export function validateDecisionEvent(input: unknown): ValidationResult<DecisionEvent> {
  const result = DecisionEventSchema.safeParse(input);
  if (result.success) {
    // Additional constitutional constraint validation
    const event = result.data;

    // Verify agent_id is correct
    if (event.agent_id !== 'failure-classification-agent') {
      return {
        success: false,
        errors: [{
          path: 'agent_id',
          message: 'Agent ID must be failure-classification-agent',
          code: 'constitutional_violation',
        }],
      };
    }

    // Verify decision_type is correct
    if (event.decision_type !== 'failure_classification') {
      return {
        success: false,
        errors: [{
          path: 'decision_type',
          message: 'Decision type must be failure_classification',
          code: 'constitutional_violation',
        }],
      };
    }

    // Verify constraints_applied is empty (read-only agent)
    if (event.constraints_applied.length !== 0) {
      return {
        success: false,
        errors: [{
          path: 'constraints_applied',
          message: 'Read-only agent must not apply constraints',
          code: 'constitutional_violation',
        }],
      };
    }

    return { success: true, data: event };
  }

  return {
    success: false,
    errors: result.error.errors.map(e => ({
      path: e.path.join('.'),
      message: e.message,
      code: e.code,
    })),
  };
}

// =============================================================================
// QUERY VALIDATION
// =============================================================================

/**
 * Validate classification query
 */
export function validateClassificationQuery(input: unknown): ValidationResult<ClassificationQuery> {
  const result = ClassificationQuerySchema.safeParse(input);
  if (result.success) {
    return { success: true, data: result.data };
  }
  return {
    success: false,
    errors: result.error.errors.map(e => ({
      path: e.path.join('.'),
      message: e.message,
      code: e.code,
    })),
  };
}

/**
 * Validate analysis query
 */
export function validateAnalysisQuery(input: unknown): ValidationResult<AnalysisQuery> {
  const result = AnalysisQuerySchema.safeParse(input);
  if (result.success) {
    return { success: true, data: result.data };
  }
  return {
    success: false,
    errors: result.error.errors.map(e => ({
      path: e.path.join('.'),
      message: e.message,
      code: e.code,
    })),
  };
}

// =============================================================================
// CONSTITUTIONAL CONSTRAINT VALIDATION
// =============================================================================

/**
 * List of prohibited operations for this read-only agent
 */
const PROHIBITED_OPERATIONS: ProhibitedOperation[] = [
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
];

/**
 * Validate that an operation is not prohibited
 */
export function validateConstitutionalOperation(operation: string): ValidationResult<void> {
  if (PROHIBITED_OPERATIONS.includes(operation as ProhibitedOperation)) {
    return {
      success: false,
      errors: [{
        path: 'operation',
        message: `Operation '${operation}' is prohibited for read-only diagnostic agent`,
        code: 'constitutional_violation',
      }],
    };
  }
  return { success: true };
}

/**
 * Assert that the agent is operating within constitutional bounds
 */
export function assertConstitutionalCompliance(context: {
  operation: string;
  modifiesState: boolean;
  triggersAction: boolean;
  accessesStorage: 'read' | 'write' | 'none';
}): void {
  if (context.modifiesState) {
    throw new ConstitutionalViolationError(
      'Read-only agent cannot modify state',
      'state_modification'
    );
  }

  if (context.triggersAction) {
    throw new ConstitutionalViolationError(
      'Diagnostic agent cannot trigger actions',
      'action_trigger'
    );
  }

  if (context.accessesStorage === 'write') {
    // Only ruvector-service writes are allowed
    throw new ConstitutionalViolationError(
      'Direct storage writes are prohibited - use ruvector-service only',
      'direct_storage_write'
    );
  }

  const opValidation = validateConstitutionalOperation(context.operation);
  if (!opValidation.success) {
    throw new ConstitutionalViolationError(
      opValidation.errors![0].message,
      context.operation
    );
  }
}

// =============================================================================
// HASHING UTILITIES
// =============================================================================

/**
 * Generate SHA256 hash of input for reproducibility tracking
 */
export function hashInput(input: unknown): string {
  const normalized = JSON.stringify(input, Object.keys(input as object).sort());
  return createHash('sha256').update(normalized).digest('hex');
}

/**
 * Generate hash for batch inputs
 */
export function hashInputs(inputs: unknown[]): string {
  const combined = inputs.map(i => hashInput(i)).join('');
  return createHash('sha256').update(combined).digest('hex');
}

// =============================================================================
// ERROR CLASSES
// =============================================================================

/**
 * Error thrown when a constitutional constraint is violated
 */
export class ConstitutionalViolationError extends Error {
  public readonly code: string = 'CONSTITUTIONAL_VIOLATION';
  public readonly operation: string;

  constructor(message: string, operation: string) {
    super(`Constitutional violation: ${message}`);
    this.name = 'ConstitutionalViolationError';
    this.operation = operation;
  }
}

/**
 * Error thrown when validation fails
 */
export class ValidationError extends Error {
  public readonly code: string = 'VALIDATION_ERROR';
  public readonly errors: Array<{ path: string; message: string; code: string }>;

  constructor(errors: Array<{ path: string; message: string; code: string }>) {
    super(`Validation failed: ${errors.map(e => e.message).join(', ')}`);
    this.name = 'ValidationError';
    this.errors = errors;
  }
}
