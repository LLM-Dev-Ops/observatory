/**
 * DecisionEvent emitter module
 * Creates decision events for ruvector-service persistence
 * Copyright 2025 LLM Observatory Contributors
 * SPDX-License-Identifier: Apache-2.0
 */

import {
  DecisionEvent,
  TelemetryEvent,
  NormalizedTelemetry,
  AgentTelemetryEvent,
} from './types/schemas.js';

/**
 * Agent constants
 */
const AGENT_ID = 'telemetry-collector-agent';
const AGENT_VERSION = '1.0.0';

/**
 * Create a decision event from telemetry inputs and outputs
 * Per constitution: read-only, non-enforcing, non-analytical
 */
export function createDecisionEvent(
  inputs: TelemetryEvent[],
  outputs: NormalizedTelemetry[],
  processingTimeMs: number
): DecisionEvent {
  const eventId = generateDecisionEventId();
  const executionRef = generateExecutionRef();

  return {
    eventId,
    eventType: 'telemetry_ingestion',
    timestamp: new Date(),

    // Agent identity
    agentId: AGENT_ID,
    agentVersion: AGENT_VERSION,

    // Decision details
    decisionType: 'read_only_observation',
    inputs,
    outputs,

    // Decision metadata (per constitution)
    confidence: 1.0, // Always 1.0 for read-only agent
    constraintsApplied: [], // Empty per constitution (non-enforcing)
    executionRef,

    // Processing metrics
    processingTimeMs,
    batchSize: inputs.length,
  };
}

/**
 * Add self-observation telemetry to decision event
 */
export function addSelfObservation(
  decisionEvent: DecisionEvent,
  agentMetrics: {
    ingestionCount: number;
    errorCount: number;
    avgLatencyMs: number;
  }
): DecisionEvent {
  return {
    ...decisionEvent,
    selfObservation: {
      agentTelemetry: agentMetrics,
    },
  };
}

/**
 * Generate unique decision event ID
 */
function generateDecisionEventId(): string {
  const timestamp = Date.now().toString(36);
  const random = Math.random().toString(36).substring(2, 15);
  return `dec_${timestamp}_${random}`;
}

/**
 * Generate unique execution reference
 * Format: agent_id:timestamp:random
 */
export function generateExecutionRef(): string {
  const timestamp = Date.now();
  const random = Math.random().toString(36).substring(2, 10);
  return `${AGENT_ID}:${timestamp}:${random}`;
}

/**
 * Create agent telemetry event for self-observation
 * Per constitution: self-observation is allowed
 */
export function createAgentTelemetryEvent(
  metrics: {
    ingestionCount: number;
    errorCount: number;
    avgLatencyMs: number;
  },
  executionRef: string
): AgentTelemetryEvent {
  const successRate =
    metrics.ingestionCount > 0
      ? (metrics.ingestionCount - metrics.errorCount) / metrics.ingestionCount
      : 1.0;

  return {
    eventType: 'agent_telemetry',
    agentId: AGENT_ID,
    agentVersion: AGENT_VERSION,
    timestamp: new Date(),
    metrics: {
      ...metrics,
      successRate,
    },
    executionRef,
  };
}

/**
 * Batch create decision events from multiple telemetry batches
 */
export function createDecisionEvents(
  batches: Array<{
    inputs: TelemetryEvent[];
    outputs: NormalizedTelemetry[];
    processingTimeMs: number;
  }>
): DecisionEvent[] {
  return batches.map((batch) =>
    createDecisionEvent(batch.inputs, batch.outputs, batch.processingTimeMs)
  );
}

/**
 * Validate decision event before persistence
 */
export function validateDecisionEvent(event: DecisionEvent): {
  valid: boolean;
  errors: string[];
} {
  const errors: string[] = [];

  if (!event.eventId) {
    errors.push('Event ID is required');
  }

  if (!event.agentId) {
    errors.push('Agent ID is required');
  }

  if (!event.agentVersion) {
    errors.push('Agent version is required');
  }

  if (!event.executionRef) {
    errors.push('Execution reference is required');
  }

  if (!Array.isArray(event.inputs)) {
    errors.push('Inputs must be an array');
  }

  if (!Array.isArray(event.outputs)) {
    errors.push('Outputs must be an array');
  }

  if (event.confidence !== 1.0) {
    errors.push('Confidence must be 1.0 for read-only agent');
  }

  if (!Array.isArray(event.constraintsApplied) || event.constraintsApplied.length !== 0) {
    errors.push('Constraints applied must be empty array per constitution');
  }

  if (event.processingTimeMs < 0) {
    errors.push('Processing time must be non-negative');
  }

  return {
    valid: errors.length === 0,
    errors,
  };
}

/**
 * Get agent information
 */
export function getAgentInfo(): {
  agentId: string;
  agentVersion: string;
} {
  return {
    agentId: AGENT_ID,
    agentVersion: AGENT_VERSION,
  };
}
