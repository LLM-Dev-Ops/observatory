/**
 * Hardened Decision Event Schema
 *
 * Phase 1 Layer 1 standardized schema for all agents.
 * Includes mandatory identity fields: source_agent, domain, phase, layer
 */

import { z } from 'zod';

// =============================================================================
// AGENT IDENTITY SCHEMAS
// =============================================================================

export const AgentPhaseSchema = z.literal('phase1');
export const AgentLayerSchema = z.literal('layer1');

export const AgentIdentitySchema = z.object({
  source_agent: z.string().min(1).describe('Agent name emitting the event'),
  domain: z.string().min(1).describe('Agent domain (e.g., diagnostics, observability)'),
  phase: AgentPhaseSchema.describe('Deployment phase (phase1)'),
  layer: AgentLayerSchema.describe('Architecture layer (layer1)'),
});

// =============================================================================
// DECISION EVENT SCHEMAS (Hardened)
// =============================================================================

/**
 * Evidence reference for audit trails
 */
export const EvidenceRefSchema = z.object({
  ref_type: z.enum(['span_id', 'trace_id', 'log_id', 'metric_id', 'external']),
  ref_value: z.string().min(1),
  timestamp: z.string().datetime().optional(),
  source: z.string().optional(),
});

/**
 * Base Hardened DecisionEvent Schema
 *
 * All agents MUST use this schema for their DecisionEvents.
 * Enforces:
 * - Agent identity fields (source_agent, domain, phase, layer)
 * - Signals, NOT conclusions
 * - Evidence references
 * - Confidence scores
 */
export const HardenedDecisionEventBaseSchema = z.object({
  // MANDATORY: Agent Identity (Phase 1 Layer 1 standardization)
  source_agent: z.string().min(1).describe('Agent name emitting this event'),
  domain: z.string().min(1).describe('Agent domain'),
  phase: AgentPhaseSchema,
  layer: AgentLayerSchema,

  // Event classification
  event_type: z.string().min(1).describe('Type of signal being emitted'),

  // Standard decision event fields
  agent_id: z.string().min(1).describe('Agent identifier (for backwards compatibility)'),
  agent_version: z.string().regex(/^\d+\.\d+\.\d+$/).describe('Semantic version'),
  decision_type: z.string().min(1).describe('Decision type for routing'),

  // Input tracking
  inputs_hash: z.string().length(64).describe('SHA256 hash of inputs'),

  // Output (signals, NOT conclusions)
  outputs: z.array(z.unknown()).min(0).describe('Signal outputs'),

  // Confidence (analytical, not probabilistic)
  confidence: z.number().min(0).max(1).describe('Confidence score 0-1'),

  // Evidence references (for audit trails)
  evidence_refs: z.array(EvidenceRefSchema).default([]).describe('References to supporting evidence'),

  // CONSTITUTIONAL: No constraints applied for Phase 1 agents
  constraints_applied: z.array(z.never()).length(0).describe('Must be empty for Phase 1'),

  // Execution tracking
  execution_ref: z.string().min(1).describe('Unique execution reference'),
  timestamp: z.string().datetime().describe('Event timestamp (UTC)'),
}).strict();

// =============================================================================
// TYPE EXPORTS
// =============================================================================

export type AgentPhase = z.infer<typeof AgentPhaseSchema>;
export type AgentLayer = z.infer<typeof AgentLayerSchema>;
export type AgentIdentity = z.infer<typeof AgentIdentitySchema>;
export type EvidenceRef = z.infer<typeof EvidenceRefSchema>;
export type HardenedDecisionEventBase = z.infer<typeof HardenedDecisionEventBaseSchema>;

// =============================================================================
// VALIDATION HELPERS
// =============================================================================

/**
 * Validate a hardened decision event
 */
export function validateHardenedDecisionEvent(
  event: unknown
): { success: true; data: HardenedDecisionEventBase } | { success: false; errors: string[] } {
  const result = HardenedDecisionEventBaseSchema.safeParse(event);
  if (result.success) {
    return { success: true, data: result.data };
  }
  return {
    success: false,
    errors: result.error.errors.map(e => `${e.path.join('.')}: ${e.message}`),
  };
}

/**
 * Assert decision event is a signal, not a conclusion
 *
 * Decision events must:
 * - Emit signals (observations, classifications, measurements)
 * - NOT emit summaries, recommendations, or synthesis
 */
export function assertSignalNotConclusion(event: HardenedDecisionEventBase): void {
  const prohibitedPatterns = [
    /recommend/i,
    /suggest/i,
    /should/i,
    /summary/i,
    /conclusion/i,
    /synthesis/i,
    /action required/i,
    /take action/i,
  ];

  const eventStr = JSON.stringify(event.outputs);

  for (const pattern of prohibitedPatterns) {
    if (pattern.test(eventStr)) {
      throw new Error(
        `DecisionEvent contains prohibited conclusion language: ${pattern}. ` +
        'Agents MUST emit signals, NOT conclusions.'
      );
    }
  }
}
