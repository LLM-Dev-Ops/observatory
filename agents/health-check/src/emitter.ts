/**
 * Health Check Agent - Decision Event Emitter
 *
 * Creates DecisionEvent objects that comply with constitutional constraints:
 * - agent_id: "health-check-agent"
 * - decision_type: "health_evaluation"
 * - confidence: Statistical (0.0-1.0)
 * - constraints_applied: Always [] (empty)
 */

import { createHash, randomUUID } from 'crypto';
import type {
  HealthCheckDecisionEvent,
  HealthEvaluation,
  HealthEvaluationRequest,
  ProcessingMetrics,
} from '../contracts/schemas.js';
import {
  AGENT_ID,
  AGENT_VERSION,
  AGENT_CLASSIFICATION,
  DECISION_TYPE,
} from '../contracts/schemas.js';

// ============================================================================
// EXECUTION REFERENCE GENERATION
// ============================================================================

/**
 * Generate a unique execution reference.
 * Format: agent_id:timestamp:random_suffix
 */
export function generateExecutionRef(): string {
  const timestamp = Date.now();
  const random = randomUUID().split('-')[0];
  return `${AGENT_ID}:${timestamp}:${random}`;
}

// ============================================================================
// INPUTS HASH CALCULATION
// ============================================================================

/**
 * Calculate SHA256 hash of inputs for audit trail.
 */
export function calculateInputsHash(request: HealthEvaluationRequest): string {
  const content = JSON.stringify({
    targets: request.targets,
    options: request.options,
  });

  return createHash('sha256').update(content).digest('hex');
}

// ============================================================================
// CONFIDENCE CALCULATION
// ============================================================================

/**
 * Calculate overall confidence from multiple evaluations.
 */
export function calculateOverallConfidence(evaluations: HealthEvaluation[]): number {
  if (evaluations.length === 0) return 0;

  const totalConfidence = evaluations.reduce(
    (sum, eval_) => sum + eval_.overall_confidence,
    0
  );

  return Math.round((totalConfidence / evaluations.length) * 100) / 100;
}

// ============================================================================
// PROCESSING METRICS
// ============================================================================

/**
 * Build processing metrics from evaluation data.
 */
export function buildProcessingMetrics(
  evaluations: HealthEvaluation[],
  processingTimeMs: number
): ProcessingMetrics {
  const indicatorsComputed = evaluations.reduce(
    (sum, eval_) => sum + eval_.indicators.length,
    0
  );

  const eventsAnalyzed = evaluations.reduce(
    (sum, eval_) => sum + eval_.statistics.total_requests,
    0
  );

  return {
    events_analyzed: eventsAnalyzed,
    indicators_computed: indicatorsComputed,
    processing_time_ms: processingTimeMs,
  };
}

// ============================================================================
// DECISION EVENT CREATION
// ============================================================================

export interface DecisionEventInput {
  request: HealthEvaluationRequest;
  evaluations: HealthEvaluation[];
  processingTimeMs: number;
  executionRef?: string;
}

/**
 * Create a DecisionEvent for health evaluation.
 *
 * CONSTITUTIONAL CONSTRAINTS ENFORCED:
 * - decision_type is locked to "health_evaluation"
 * - constraints_applied is always empty array
 * - classification is locked to "advisory"
 */
export function createDecisionEvent(input: DecisionEventInput): HealthCheckDecisionEvent {
  const {
    request,
    evaluations,
    processingTimeMs,
    executionRef = generateExecutionRef(),
  } = input;

  const inputsHash = calculateInputsHash(request);
  const confidence = calculateOverallConfidence(evaluations);
  const processingMetrics = buildProcessingMetrics(evaluations, processingTimeMs);

  // CONSTITUTIONAL CONSTRAINTS - These values are locked
  const decisionEvent: HealthCheckDecisionEvent = {
    // Agent identification
    agent_id: AGENT_ID,
    agent_version: AGENT_VERSION,

    // Decision metadata (CONSTITUTIONAL - DO NOT MODIFY)
    decision_type: DECISION_TYPE,
    confidence,
    constraints_applied: [], // ALWAYS empty for advisory agents

    // Classification (CONSTITUTIONAL)
    classification: AGENT_CLASSIFICATION,

    // Input/Output tracking
    inputs_hash: inputsHash,
    outputs: evaluations,

    // Execution tracking
    execution_ref: executionRef,
    timestamp: new Date().toISOString(),

    // Processing metrics
    processing_metrics: processingMetrics,

    // Metadata (optional)
    metadata: {
      targets_requested: request.targets.length,
      targets_evaluated: evaluations.length,
      options: request.options,
    },
  };

  return decisionEvent;
}

// ============================================================================
// VALIDATION
// ============================================================================

/**
 * Validate that a DecisionEvent complies with constitutional constraints.
 * This is a runtime check to ensure no code path violates the constraints.
 */
export function validateDecisionEventCompliance(
  event: HealthCheckDecisionEvent
): { valid: boolean; violations: string[] } {
  const violations: string[] = [];

  // Check agent_id
  if (event.agent_id !== AGENT_ID) {
    violations.push(`Invalid agent_id: expected "${AGENT_ID}", got "${event.agent_id}"`);
  }

  // Check decision_type
  if (event.decision_type !== DECISION_TYPE) {
    violations.push(`Invalid decision_type: expected "${DECISION_TYPE}", got "${event.decision_type}"`);
  }

  // Check classification
  if (event.classification !== AGENT_CLASSIFICATION) {
    violations.push(`Invalid classification: expected "${AGENT_CLASSIFICATION}", got "${event.classification}"`);
  }

  // Check constraints_applied is empty
  if (event.constraints_applied.length !== 0) {
    violations.push(`Invalid constraints_applied: must be empty array for advisory agents, got ${event.constraints_applied.length} items`);
  }

  // Check confidence is in valid range
  if (event.confidence < 0 || event.confidence > 1) {
    violations.push(`Invalid confidence: must be 0.0-1.0, got ${event.confidence}`);
  }

  // Check inputs_hash is valid SHA256
  if (event.inputs_hash.length !== 64 || !/^[0-9a-f]+$/i.test(event.inputs_hash)) {
    violations.push(`Invalid inputs_hash: must be 64 character hex string`);
  }

  // Check outputs is not empty
  if (event.outputs.length === 0) {
    violations.push(`Invalid outputs: must contain at least one HealthEvaluation`);
  }

  return {
    valid: violations.length === 0,
    violations,
  };
}
