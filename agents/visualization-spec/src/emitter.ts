/**
 * Visualization Spec Agent - Decision Event Emitter
 *
 * Creates constitutional decision events for audit and replay.
 * Every invocation MUST emit exactly ONE DecisionEvent.
 */

import { randomUUID } from 'crypto';
import {
  AGENT_ID,
  AGENT_VERSION,
  AGENT_CLASSIFICATION,
  DECISION_TYPE,
  VisualizationDecisionEventSchema,
  type VisualizationSpec,
  type VisualizationDecisionEvent,
  type ProcessingMetrics,
} from '../contracts/schemas.js';
import { computeInputHash } from '../contracts/validation.js';
import type { VisualizationRequest } from '../contracts/schemas.js';

// =============================================================================
// Decision Event Creation
// =============================================================================

export interface CreateDecisionEventInput {
  request: VisualizationRequest | VisualizationRequest[];
  specs: VisualizationSpec[];
  executionRef: string;
  processingMetrics: ProcessingMetrics;
}

/**
 * Creates a constitutional decision event for the visualization specification
 *
 * This is the primary audit record for this agent. It captures:
 * - Input hash for deterministic replay verification
 * - All generated visualization specs
 * - Processing metrics for performance analysis
 * - Confidence score based on spec completeness
 *
 * IMPORTANT: constraints_applied is ALWAYS empty for READ-ONLY agents
 */
export function createDecisionEvent(input: CreateDecisionEventInput): VisualizationDecisionEvent {
  const inputHash = computeInputHash(input.request);
  const confidence = calculateConfidence(input.specs);

  const event: VisualizationDecisionEvent = {
    agent_id: AGENT_ID,
    agent_version: AGENT_VERSION,
    decision_type: DECISION_TYPE,
    confidence,
    constraints_applied: [], // ALWAYS empty - this is a READ-ONLY agent
    classification: AGENT_CLASSIFICATION,
    inputs_hash: inputHash,
    outputs: input.specs,
    execution_ref: input.executionRef,
    timestamp: new Date().toISOString(),
    processing_metrics: input.processingMetrics,
  };

  // Validate the event against schema before returning
  const parseResult = VisualizationDecisionEventSchema.safeParse(event);
  if (!parseResult.success) {
    throw new Error(`Invalid decision event: ${parseResult.error.message}`);
  }

  return parseResult.data;
}

// =============================================================================
// Confidence Calculation
// =============================================================================

/**
 * Calculates confidence score for generated specifications
 *
 * Confidence is ANALYTICAL (not probabilistic) based on:
 * - Spec completeness (all required fields present)
 * - Styling completeness
 * - Threshold/alert zone coverage
 * - Metadata completeness
 *
 * Returns a value between 0 and 1
 */
function calculateConfidence(specs: VisualizationSpec[]): number {
  if (specs.length === 0) {
    return 0;
  }

  const specConfidences = specs.map(calculateSpecConfidence);
  const avgConfidence = specConfidences.reduce((a, b) => a + b, 0) / specConfidences.length;

  // Round to 3 decimal places
  return Math.round(avgConfidence * 1000) / 1000;
}

/**
 * Calculates confidence for a single visualization spec
 */
function calculateSpecConfidence(spec: VisualizationSpec): number {
  let score = 0;
  let maxScore = 0;

  // Core fields (weighted heavily)
  maxScore += 30;
  if (spec.spec_id) score += 10;
  if (spec.visualization_type) score += 10;
  if (spec.data_source) score += 10;

  // Series configuration
  maxScore += 20;
  if (spec.series && spec.series.length > 0) {
    score += 10;
    // Check series completeness
    const seriesComplete = spec.series.every(s =>
      s.id && s.name && s.field && s.aggregation && s.color
    );
    if (seriesComplete) score += 10;
  }

  // Axes configuration
  maxScore += 15;
  if (spec.axes && spec.axes.length > 0) {
    score += 7;
    const axesComplete = spec.axes.every(a => a.type && a.scale !== undefined);
    if (axesComplete) score += 8;
  }

  // Styling completeness
  maxScore += 20;
  if (spec.styling) {
    score += 5;
    if (spec.styling.theme) score += 3;
    if (spec.styling.color_scheme) score += 3;
    if (spec.styling.legend) score += 3;
    if (spec.styling.tooltip) score += 3;
    if (spec.styling.dimensions) score += 3;
  }

  // Metadata completeness
  maxScore += 15;
  if (spec.metadata) {
    score += 5;
    if (spec.metadata.generated_at) score += 3;
    if (spec.metadata.generator_version) score += 3;
    if (spec.metadata.input_hash) score += 2;
    if (spec.metadata.deterministic !== undefined) score += 2;
  }

  return score / maxScore;
}

// =============================================================================
// Processing Metrics
// =============================================================================

/**
 * Creates processing metrics for the decision event
 */
export function createProcessingMetrics(
  startTime: number,
  parsingMs: number,
  validationMs: number,
  generationMs: number,
  specsGenerated: number
): ProcessingMetrics {
  return {
    parsing_ms: parsingMs,
    validation_ms: validationMs,
    generation_ms: generationMs,
    total_ms: Date.now() - startTime,
    specs_generated: specsGenerated,
  };
}

// =============================================================================
// Replay Verification
// =============================================================================

/**
 * Verifies that a replay produces the same output hash
 *
 * Used for determinism verification via CLI replay command
 */
export function verifyReplayDeterminism(
  originalEvent: VisualizationDecisionEvent,
  replaySpecs: VisualizationSpec[]
): {
  deterministic: boolean;
  originalHash: string;
  replayHash: string;
} {
  const originalHash = originalEvent.inputs_hash;
  const replayHash = computeInputHash({ specs: replaySpecs });

  // For determinism, we compare the output specs hash
  const originalSpecsHash = computeInputHash(originalEvent.outputs);
  const replaySpecsHash = computeInputHash(replaySpecs);

  return {
    deterministic: originalSpecsHash === replaySpecsHash,
    originalHash: originalSpecsHash,
    replayHash: replaySpecsHash,
  };
}

/**
 * Helper to compute output hash for specs
 */
function computeOutputHash(specs: VisualizationSpec[]): string {
  // Normalize specs by removing non-deterministic fields
  const normalized = specs.map(spec => ({
    ...spec,
    metadata: {
      ...spec.metadata,
      generated_at: '', // Remove timestamp for comparison
    },
  }));

  return computeInputHash(normalized);
}
