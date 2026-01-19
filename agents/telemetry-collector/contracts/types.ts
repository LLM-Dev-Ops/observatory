// Copyright 2025 LLM Observatory Contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * TypeScript types for Telemetry Collector Agent.
 *
 * These types are exported from Zod schemas to ensure runtime validation
 * and compile-time type safety are always in sync.
 */

export type {
  TelemetryEvent,
  NormalizedTelemetry,
  DecisionEvent,
  TokenUsage,
  Cost,
  Latency,
  Metadata,
  LlmInput,
  LlmOutput,
  SpanStatus,
  Provider,
} from './schemas.js';

// ============================================================================
// Agent Metadata
// ============================================================================

/**
 * Telemetry Collector Agent metadata.
 */
export interface AgentMetadata {
  /** Agent identifier (e.g., "telemetry-collector") */
  agentId: string;

  /** Agent version (semantic versioning) */
  agentVersion: string;

  /** Agent instance ID (unique per deployment) */
  instanceId: string;

  /** Deployment environment (production, staging, development) */
  environment: string;

  /** Additional agent configuration */
  config?: Record<string, unknown>;
}

// ============================================================================
// Validation Results
// ============================================================================

/**
 * Validation result for telemetry events.
 */
export interface ValidationResult<T = unknown> {
  /** Whether validation succeeded */
  success: boolean;

  /** Parsed and validated data (if successful) */
  data?: T;

  /** Validation errors (if failed) */
  errors?: ValidationError[];

  /** Validation metadata */
  metadata?: {
    /** Time taken for validation (ms) */
    validationTimeMs: number;

    /** Schema version used */
    schemaVersion: string;

    /** Input hash for tracking */
    inputHash?: string;
  };
}

/**
 * Validation error details.
 */
export interface ValidationError {
  /** Field path where error occurred */
  path: string[];

  /** Error message */
  message: string;

  /** Error code (for programmatic handling) */
  code: string;

  /** Expected value/type */
  expected?: string;

  /** Received value/type */
  received?: string;
}

// ============================================================================
// Normalization Context
// ============================================================================

/**
 * Context for telemetry normalization operations.
 */
export interface NormalizationContext {
  /** Agent metadata */
  agent: AgentMetadata;

  /** Execution reference (unique per batch) */
  executionRef: string;

  /** Timestamp when normalization started */
  startedAt: Date;

  /** Additional context data */
  context?: Record<string, unknown>;
}

// ============================================================================
// Processing Results
// ============================================================================

/**
 * Result of processing a single telemetry event.
 */
export interface ProcessingResult {
  /** Original telemetry event */
  original: unknown;

  /** Normalized telemetry (if successful) */
  normalized?: import('./schemas.js').NormalizedTelemetry;

  /** Validation result */
  validation: ValidationResult;

  /** Processing metadata */
  metadata: {
    /** Time taken for processing (ms) */
    processingTimeMs: number;

    /** Whether the event was valid */
    isValid: boolean;

    /** Input hash */
    inputHash: string;
  };
}

/**
 * Batch processing result.
 */
export interface BatchProcessingResult {
  /** Total events processed */
  total: number;

  /** Successfully normalized events */
  successful: number;

  /** Failed events */
  failed: number;

  /** Individual processing results */
  results: ProcessingResult[];

  /** Decision event (if all successful) */
  decisionEvent?: import('./schemas.js').DecisionEvent;

  /** Batch metadata */
  metadata: {
    /** Total processing time (ms) */
    totalTimeMs: number;

    /** Average processing time per event (ms) */
    avgTimeMs: number;

    /** Execution reference */
    executionRef: string;

    /** Timestamp when batch started */
    startedAt: string;

    /** Timestamp when batch completed */
    completedAt: string;
  };
}

// ============================================================================
// Input Hashing
// ============================================================================

/**
 * Hash algorithm for input tracking.
 */
export type HashAlgorithm = 'sha256';

/**
 * Hashing options.
 */
export interface HashingOptions {
  /** Hash algorithm to use */
  algorithm?: HashAlgorithm;

  /** Whether to include metadata in hash */
  includeMetadata?: boolean;

  /** Fields to exclude from hash */
  excludeFields?: string[];
}

// ============================================================================
// Error Types
// ============================================================================

/**
 * Error codes for telemetry collector operations.
 */
export enum TelemetryCollectorErrorCode {
  // Validation errors
  INVALID_SCHEMA = 'INVALID_SCHEMA',
  MISSING_REQUIRED_FIELD = 'MISSING_REQUIRED_FIELD',
  INVALID_FIELD_TYPE = 'INVALID_FIELD_TYPE',
  INVALID_FIELD_VALUE = 'INVALID_FIELD_VALUE',

  // Constitutional violations
  CONSTITUTIONAL_VIOLATION = 'CONSTITUTIONAL_VIOLATION',

  // Processing errors
  NORMALIZATION_FAILED = 'NORMALIZATION_FAILED',
  HASHING_FAILED = 'HASHING_FAILED',

  // Decision event errors
  DECISION_EVENT_CREATION_FAILED = 'DECISION_EVENT_CREATION_FAILED',

  // Persistence errors (should never occur - read-only agent)
  PERSISTENCE_ATTEMPTED = 'PERSISTENCE_ATTEMPTED',

  // Unknown errors
  UNKNOWN_ERROR = 'UNKNOWN_ERROR',
}

/**
 * Custom error class for telemetry collector operations.
 */
export class TelemetryCollectorError extends Error {
  constructor(
    public readonly code: TelemetryCollectorErrorCode,
    message: string,
    public readonly details?: Record<string, unknown>,
    public readonly cause?: Error,
  ) {
    super(message);
    this.name = 'TelemetryCollectorError';
    Error.captureStackTrace(this, this.constructor);
  }
}

// ============================================================================
// Configuration Types
// ============================================================================

/**
 * Telemetry collector configuration.
 */
export interface TelemetryCollectorConfig {
  /** Agent metadata */
  agent: AgentMetadata;

  /** Schema version to use */
  schemaVersion?: string;

  /** Validation options */
  validation?: {
    /** Whether to perform strict validation */
    strict?: boolean;

    /** Whether to validate against constitutional constraints */
    validateConstitution?: boolean;

    /** Maximum number of validation errors to collect */
    maxErrors?: number;
  };

  /** Normalization options */
  normalization?: {
    /** Whether to add normalization metadata */
    addMetadata?: boolean;

    /** Schema version to tag normalized events with */
    schemaVersion?: string;
  };

  /** Hashing options */
  hashing?: HashingOptions;
}
