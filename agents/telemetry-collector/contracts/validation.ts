// Copyright 2025 LLM Observatory Contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * Validation utilities for Telemetry Collector Agent.
 *
 * Provides functions for:
 * - Validating raw telemetry events
 * - Validating decision events
 * - Hashing inputs for tracking
 * - Constitutional constraint validation
 */

import { createHash } from 'crypto';
import { ZodError } from 'zod';
import {
  TelemetryEventSchema,
  DecisionEventSchema,
  type TelemetryEvent,
  type DecisionEvent,
} from './schemas.js';
import {
  type ValidationResult,
  type ValidationError,
  type HashingOptions,
  TelemetryCollectorErrorCode,
  TelemetryCollectorError,
} from './types.js';

// ============================================================================
// Validation Functions
// ============================================================================

/**
 * Validate a telemetry event against the schema.
 *
 * @param input - Raw input to validate
 * @returns Validation result with parsed data or errors
 *
 * @example
 * ```typescript
 * const result = validateTelemetryEvent(rawInput);
 * if (result.success) {
 *   // Use result.data (typed as TelemetryEvent)
 *   console.log(result.data.span_id);
 * } else {
 *   // Handle result.errors
 *   console.error(result.errors);
 * }
 * ```
 */
export function validateTelemetryEvent(input: unknown): ValidationResult<TelemetryEvent> {
  const startTime = performance.now();

  try {
    const data = TelemetryEventSchema.parse(input);
    const validationTimeMs = performance.now() - startTime;

    return {
      success: true,
      data,
      metadata: {
        validationTimeMs,
        schemaVersion: '1.0.0',
        inputHash: hashInput(data),
      },
    };
  } catch (error) {
    const validationTimeMs = performance.now() - startTime;

    if (error instanceof ZodError) {
      const errors: ValidationError[] = error.issues.map((err) => ({
        path: err.path.map(String),
        message: err.message,
        code: err.code,
        expected: 'expected' in err ? String(err.expected) : undefined,
        received: 'received' in err ? String(err.received) : undefined,
      }));

      return {
        success: false,
        errors,
        metadata: {
          validationTimeMs,
          schemaVersion: '1.0.0',
        },
      };
    }

    // Unknown error
    return {
      success: false,
      errors: [{
        path: [],
        message: error instanceof Error ? error.message : 'Unknown validation error',
        code: TelemetryCollectorErrorCode.UNKNOWN_ERROR,
      }],
      metadata: {
        validationTimeMs,
        schemaVersion: '1.0.0',
      },
    };
  }
}

/**
 * Validate a decision event against the schema and constitutional constraints.
 *
 * This function enforces:
 * - Schema validation (structure, types, required fields)
 * - Constitutional constraints (decision_type, confidence, constraints_applied)
 *
 * @param event - Decision event to validate
 * @returns Validation result
 *
 * @example
 * ```typescript
 * const result = validateDecisionEvent(decisionEvent);
 * if (!result.success) {
 *   throw new Error(`Invalid decision event: ${result.errors?.[0]?.message}`);
 * }
 * ```
 */
export function validateDecisionEvent(event: DecisionEvent): ValidationResult<DecisionEvent> {
  const startTime = performance.now();

  try {
    // Schema validation
    const data = DecisionEventSchema.parse(event);

    // Constitutional validation (additional checks beyond schema)
    const constitutionalErrors: ValidationError[] = [];

    // Verify decision_type is exactly "telemetry_ingestion"
    if (data.decision_type !== 'telemetry_ingestion') {
      constitutionalErrors.push({
        path: ['decision_type'],
        message: 'decision_type MUST be "telemetry_ingestion" per constitution',
        code: TelemetryCollectorErrorCode.CONSTITUTIONAL_VIOLATION,
        expected: 'telemetry_ingestion',
        received: data.decision_type,
      });
    }

    // Verify confidence is exactly 1.0
    if (data.confidence !== 1.0) {
      constitutionalErrors.push({
        path: ['confidence'],
        message: 'confidence MUST be 1.0 for read-only ingestion per constitution',
        code: TelemetryCollectorErrorCode.CONSTITUTIONAL_VIOLATION,
        expected: '1.0',
        received: String(data.confidence),
      });
    }

    // Verify constraints_applied is empty array
    if (data.constraints_applied.length !== 0) {
      constitutionalErrors.push({
        path: ['constraints_applied'],
        message: 'constraints_applied MUST be empty array for read-only agent per constitution',
        code: TelemetryCollectorErrorCode.CONSTITUTIONAL_VIOLATION,
        expected: '[]',
        received: JSON.stringify(data.constraints_applied),
      });
    }

    // Verify inputs_hash is valid SHA256 (64 hex chars)
    if (!/^[a-f0-9]{64}$/i.test(data.inputs_hash)) {
      constitutionalErrors.push({
        path: ['inputs_hash'],
        message: 'inputs_hash MUST be a valid SHA256 hash (64 hex characters)',
        code: TelemetryCollectorErrorCode.INVALID_FIELD_VALUE,
        expected: '64 character SHA256 hex string',
        received: data.inputs_hash,
      });
    }

    // Verify agent_version is valid semantic version
    if (!/^\d+\.\d+\.\d+$/.test(data.agent_version)) {
      constitutionalErrors.push({
        path: ['agent_version'],
        message: 'agent_version MUST be valid semantic version (X.Y.Z)',
        code: TelemetryCollectorErrorCode.INVALID_FIELD_VALUE,
        expected: 'X.Y.Z format',
        received: data.agent_version,
      });
    }

    const validationTimeMs = performance.now() - startTime;

    if (constitutionalErrors.length > 0) {
      return {
        success: false,
        errors: constitutionalErrors,
        metadata: {
          validationTimeMs,
          schemaVersion: '1.0.0',
        },
      };
    }

    return {
      success: true,
      data,
      metadata: {
        validationTimeMs,
        schemaVersion: '1.0.0',
      },
    };
  } catch (error) {
    const validationTimeMs = performance.now() - startTime;

    if (error instanceof ZodError) {
      const errors: ValidationError[] = error.issues.map((err) => ({
        path: err.path.map(String),
        message: err.message,
        code: err.code,
        expected: 'expected' in err ? String(err.expected) : undefined,
        received: 'received' in err ? String(err.received) : undefined,
      }));

      return {
        success: false,
        errors,
        metadata: {
          validationTimeMs,
          schemaVersion: '1.0.0',
        },
      };
    }

    return {
      success: false,
      errors: [{
        path: [],
        message: error instanceof Error ? error.message : 'Unknown validation error',
        code: TelemetryCollectorErrorCode.UNKNOWN_ERROR,
      }],
      metadata: {
        validationTimeMs,
        schemaVersion: '1.0.0',
      },
    };
  }
}

// ============================================================================
// Hashing Functions
// ============================================================================

/**
 * Hash a telemetry event input for tracking.
 *
 * Uses SHA256 to create a deterministic hash of the input data.
 * This hash is used for:
 * - Input deduplication
 * - Audit trail tracking
 * - Decision event correlation
 *
 * @param input - Telemetry event to hash
 * @param options - Hashing options
 * @returns SHA256 hash (64 character hex string)
 *
 * @example
 * ```typescript
 * const hash = hashInput(telemetryEvent);
 * // hash = "a3c5d7e9f1b2d4c6e8f0a2b4c6d8e0f2a4b6c8d0e2f4a6b8c0d2e4f6a8b0c2d4"
 * ```
 */
export function hashInput(
  input: TelemetryEvent,
  options: HashingOptions = {},
): string {
  const {
    algorithm = 'sha256',
    includeMetadata = true,
    excludeFields = [],
  } = options;

  try {
    // Create a shallow copy to avoid mutating input
    const inputCopy: Record<string, unknown> = { ...input };

    // Exclude specified fields
    for (const field of excludeFields) {
      delete inputCopy[field];
    }

    // Optionally exclude metadata
    if (!includeMetadata) {
      delete inputCopy.metadata;
      delete inputCopy.attributes;
      delete inputCopy.events;
    }

    // Create deterministic JSON string
    // Sort keys to ensure consistent hash for same data
    const jsonString = JSON.stringify(inputCopy, Object.keys(inputCopy).sort());

    // Hash the JSON string
    const hash = createHash(algorithm);
    hash.update(jsonString, 'utf8');
    return hash.digest('hex');
  } catch (error) {
    throw new TelemetryCollectorError(
      TelemetryCollectorErrorCode.HASHING_FAILED,
      'Failed to hash telemetry event input',
      {
        algorithm,
        includeMetadata,
        excludeFields,
      },
      error instanceof Error ? error : undefined,
    );
  }
}

/**
 * Hash multiple telemetry events together.
 *
 * Useful for batch processing to create a single hash for all inputs.
 *
 * @param inputs - Array of telemetry events
 * @param options - Hashing options
 * @returns SHA256 hash of all inputs combined
 */
export function hashInputs(
  inputs: TelemetryEvent[],
  options: HashingOptions = {},
): string {
  try {
    // Hash each input individually
    const hashes = inputs.map((input) => hashInput(input, options));

    // Combine hashes and hash again
    const combinedHash = createHash('sha256');
    for (const hash of hashes) {
      combinedHash.update(hash, 'utf8');
    }

    return combinedHash.digest('hex');
  } catch (error) {
    throw new TelemetryCollectorError(
      TelemetryCollectorErrorCode.HASHING_FAILED,
      'Failed to hash multiple telemetry event inputs',
      {
        count: inputs.length,
        options,
      },
      error instanceof Error ? error : undefined,
    );
  }
}

// ============================================================================
// Constitutional Validation Helpers
// ============================================================================

/**
 * Validate that an operation is constitutional (read-only, non-enforcing).
 *
 * This function checks that the agent is not attempting to:
 * - Execute SQL directly
 * - Trigger orchestration
 * - Modify system state
 * - Apply constraints
 *
 * @param operation - Operation to validate
 * @returns Validation result
 */
export function validateConstitutionalOperation(
  operation: {
    type: string;
    metadata?: Record<string, unknown>;
  },
): ValidationResult<void> {
  const prohibitedOperations = [
    'sql_execute',
    'sql_write',
    'orchestration_trigger',
    'state_modify',
    'constraint_apply',
    'retry_trigger',
    'alert_trigger',
  ];

  if (prohibitedOperations.includes(operation.type)) {
    return {
      success: false,
      errors: [{
        path: ['type'],
        message: `Operation type "${operation.type}" violates constitutional constraint: agent is READ-ONLY, NON-ENFORCING`,
        code: TelemetryCollectorErrorCode.CONSTITUTIONAL_VIOLATION,
        expected: 'read-only operation',
        received: operation.type,
      }],
      metadata: {
        validationTimeMs: 0,
        schemaVersion: '1.0.0',
      },
    };
  }

  return {
    success: true,
    metadata: {
      validationTimeMs: 0,
      schemaVersion: '1.0.0',
    },
  };
}
