// Copyright 2025 LLM Observatory Contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * Telemetry Collector Agent Contracts.
 *
 * This module exports all contract-layer schemas, types, and validation utilities
 * for the Telemetry Collector Agent.
 *
 * CONSTITUTION: This agent is READ-ONLY, NON-ENFORCING, NON-ANALYTICAL.
 *
 * @module contracts
 */

// ============================================================================
// Schemas
// ============================================================================

export {
  // Schemas
  TelemetryEventSchema,
  NormalizedTelemetrySchema,
  DecisionEventSchema,
  TokenUsageSchema,
  CostSchema,
  LatencySchema,
  MetadataSchema,
  LlmInputSchema,
  LlmOutputSchema,
  SpanStatusSchema,
  ProviderSchema,

  // Schema-derived types (for re-export convenience)
  type TelemetryEvent,
  type NormalizedTelemetry,
  type DecisionEvent,
  type TokenUsage,
  type Cost,
  type Latency,
  type Metadata,
  type LlmInput,
  type LlmOutput,
  type SpanStatus,
  type Provider,
} from './schemas.js';

// ============================================================================
// Additional Types
// ============================================================================

export type {
  // Agent metadata
  AgentMetadata,

  // Validation
  ValidationResult,
  ValidationError,

  // Processing
  NormalizationContext,
  ProcessingResult,
  BatchProcessingResult,

  // Hashing
  HashAlgorithm,
  HashingOptions,

  // Configuration
  TelemetryCollectorConfig,
} from './types.js';

// ============================================================================
// Error Types
// ============================================================================

export {
  TelemetryCollectorErrorCode,
  TelemetryCollectorError,
} from './types.js';

// ============================================================================
// Validation Functions
// ============================================================================

export {
  validateTelemetryEvent,
  validateDecisionEvent,
  hashInput,
  hashInputs,
  validateConstitutionalOperation,
} from './validation.js';
