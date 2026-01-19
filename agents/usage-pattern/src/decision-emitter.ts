/**
 * DecisionEvent Emitter for Usage Pattern Agent.
 *
 * CONSTITUTION:
 * - Every invocation MUST emit exactly ONE DecisionEvent to ruvector-service
 * - DecisionEvent schema MUST include all required fields
 * - Persistence is async and non-blocking
 *
 * DecisionEvent schema enforces:
 * - agent_id: 'usage-pattern-agent' (literal)
 * - decision_type: 'usage_pattern_analysis' (literal)
 * - confidence: Statistical (0.0-1.0 based on sample size and variance)
 * - constraints_applied: [] (always empty for advisory agent)
 * - classification: 'advisory' (literal)
 */

import { v4 as uuidv4 } from 'uuid';
import { AgentConfig } from './config.js';
import { RuvectorClient } from './ruvector-client.js';
import { PersistResult } from './types/ruvector.js';
import {
  UsagePatternAnalysis,
  UsagePatternDecisionEvent,
  UsagePatternDecisionEventSchema,
} from '../contracts/schemas.js';

/**
 * Parameters for emitting a DecisionEvent.
 */
export interface EmitParams {
  analysis: UsagePatternAnalysis;
  inputsHash: string;
  executionRef: string;
  processingTimeMs: number;
  eventsAnalyzed: number;
  memoryUsedBytes?: number;
}

/**
 * Result of emitting a DecisionEvent.
 */
export interface EmitResult {
  success: boolean;
  eventId?: string;
  error?: string;
  timestamp: Date;
}

/**
 * DecisionEvent Emitter class.
 *
 * Responsible for creating and persisting DecisionEvents
 * that conform to the constitutional constraints.
 */
export class DecisionEventEmitter {
  private config: AgentConfig;
  private client: RuvectorClient;

  constructor(config: AgentConfig, client: RuvectorClient) {
    this.config = config;
    this.client = client;
  }

  /**
   * Emit a DecisionEvent to ruvector-service.
   *
   * CONSTITUTION: This is the ONLY way to persist agent decisions.
   * - Exactly ONE DecisionEvent per invocation
   * - All constitutional constraints enforced via schema
   */
  async emit(params: EmitParams): Promise<EmitResult> {
    try {
      // Build DecisionEvent with constitutional constraints
      const decisionEvent = this.buildDecisionEvent(params);

      // Validate against schema (enforces constitutional constraints)
      const validationResult = UsagePatternDecisionEventSchema.safeParse(decisionEvent);
      if (!validationResult.success) {
        return {
          success: false,
          error: `Schema validation failed: ${validationResult.error.message}`,
          timestamp: new Date(),
        };
      }

      // Persist to ruvector-service
      const result = await this.client.persistDecisionEvent(validationResult.data);

      return {
        success: result.success,
        eventId: result.eventId,
        error: result.error,
        timestamp: result.timestamp,
      };
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : String(error),
        timestamp: new Date(),
      };
    }
  }

  /**
   * Build a DecisionEvent with all constitutional constraints.
   *
   * CONSTITUTION:
   * - agent_id: MUST be 'usage-pattern-agent'
   * - decision_type: MUST be 'usage_pattern_analysis'
   * - confidence: STATISTICAL (derived from analysis)
   * - constraints_applied: MUST be [] (empty)
   * - classification: MUST be 'advisory'
   */
  private buildDecisionEvent(params: EmitParams): UsagePatternDecisionEvent {
    return {
      // Agent metadata (constitutional)
      agent_id: 'usage-pattern-agent' as const, // Literal type
      agent_version: this.config.agentVersion,

      // Decision metadata (constitutional constraints)
      decision_type: 'usage_pattern_analysis' as const, // Literal type
      confidence: params.analysis.overall_confidence, // Statistical confidence
      constraints_applied: [] as never[], // ALWAYS empty for advisory

      // Classification (constitutional)
      classification: 'advisory' as const, // Literal type

      // Input/Output tracking
      inputs_hash: params.inputsHash,
      outputs: [params.analysis],

      // Execution tracking
      execution_ref: params.executionRef,
      timestamp: new Date().toISOString(),

      // Processing metrics
      processing_metrics: {
        events_analyzed: params.eventsAnalyzed,
        processing_time_ms: params.processingTimeMs,
        memory_used_bytes: params.memoryUsedBytes,
      },

      // Optional metadata
      metadata: {
        time_window: params.analysis.time_window,
        sample_size: params.analysis.sample_size,
        schema_version: params.analysis.schema_version,
      },
    };
  }

  /**
   * Emit telemetry about the agent itself (self-observation).
   *
   * CONSTITUTION: Self-observation is allowed but optional.
   */
  async emitSelfObservation(metrics: {
    analysisCount: number;
    avgProcessingTimeMs: number;
    errorCount: number;
    ruvectorLatencyMs: number;
  }): Promise<EmitResult> {
    if (!this.config.selfObservationEnabled) {
      return {
        success: true,
        timestamp: new Date(),
      };
    }

    try {
      const selfObservation = {
        agent_id: 'usage-pattern-agent',
        agent_version: this.config.agentVersion,
        event_type: 'self_observation',
        timestamp: new Date().toISOString(),
        metrics,
      };

      const result = await this.client.persistDecisionEvent({
        ...selfObservation,
        decision_type: 'agent_self_observation',
        confidence: 1.0, // Self-observation has perfect confidence
        constraints_applied: [],
      });

      return {
        success: result.success,
        eventId: result.eventId,
        error: result.error,
        timestamp: result.timestamp,
      };
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : String(error),
        timestamp: new Date(),
      };
    }
  }
}

/**
 * Validate that a DecisionEvent conforms to constitutional requirements.
 *
 * This is an additional safety check beyond schema validation.
 */
export function validateConstitutionalConstraints(
  event: UsagePatternDecisionEvent
): { valid: boolean; violations: string[] } {
  const violations: string[] = [];

  // Check agent_id literal
  if (event.agent_id !== 'usage-pattern-agent') {
    violations.push(`agent_id must be 'usage-pattern-agent', got '${event.agent_id}'`);
  }

  // Check decision_type literal
  if (event.decision_type !== 'usage_pattern_analysis') {
    violations.push(`decision_type must be 'usage_pattern_analysis', got '${event.decision_type}'`);
  }

  // Check classification literal
  if (event.classification !== 'advisory') {
    violations.push(`classification must be 'advisory', got '${event.classification}'`);
  }

  // Check constraints_applied is empty
  if (event.constraints_applied.length > 0) {
    violations.push(`constraints_applied must be empty for advisory agent, got ${event.constraints_applied.length} items`);
  }

  // Check confidence is in valid range
  if (event.confidence < 0 || event.confidence > 1) {
    violations.push(`confidence must be between 0 and 1, got ${event.confidence}`);
  }

  // Check inputs_hash is valid SHA256
  if (!/^[a-f0-9]{64}$/i.test(event.inputs_hash)) {
    violations.push(`inputs_hash must be a 64-character hex string (SHA256)`);
  }

  // Check outputs is non-empty
  if (event.outputs.length === 0) {
    violations.push(`outputs must contain at least one analysis result`);
  }

  // Check timestamp is valid ISO 8601
  if (isNaN(Date.parse(event.timestamp))) {
    violations.push(`timestamp must be a valid ISO 8601 datetime`);
  }

  // Check agent_version is semantic version
  if (!/^\d+\.\d+\.\d+$/.test(event.agent_version)) {
    violations.push(`agent_version must be semantic version (x.y.z)`);
  }

  return {
    valid: violations.length === 0,
    violations,
  };
}
