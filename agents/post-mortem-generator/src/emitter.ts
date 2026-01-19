/**
 * Post-Mortem Generator Agent - Decision Event Emitter
 *
 * CONSTITUTIONAL CLASSIFICATION: READ-ONLY
 *
 * Emits DecisionEvents for constitutional compliance tracking.
 * CRITICAL: constraints_applied MUST always be empty for this read-only agent.
 */

import { createHash } from 'crypto';
import type {
  PostMortemRequest,
  PostMortemReport,
  DecisionEvent,
} from '../contracts/schemas.js';
import { DecisionEventSchema, AGENT_METADATA } from '../contracts/schemas.js';

export interface EmitterInput {
  request: PostMortemRequest;
  reports: PostMortemReport[];
  confidence: number;
  executionRef: string;
}

/**
 * Create a DecisionEvent for the post-mortem generation.
 */
export function createDecisionEvent(input: EmitterInput): DecisionEvent {
  const { request, reports, confidence, executionRef } = input;

  // Compute inputs hash for reproducibility
  const inputsHash = computeInputsHash(request);

  const event: DecisionEvent = {
    agent_id: 'post-mortem-generator-agent',
    agent_version: AGENT_METADATA.version,
    decision_type: 'postmortem_generation',
    inputs_hash: inputsHash,
    outputs: reports,
    confidence,
    constraints_applied: [], // CONSTITUTIONAL: Always empty for read-only agent
    execution_ref: executionRef,
    timestamp: new Date().toISOString(),
  };

  return event;
}

/**
 * Compute SHA256 hash of inputs for reproducibility.
 */
function computeInputsHash(request: PostMortemRequest): string {
  const canonicalInput = JSON.stringify({
    time_range: request.time_range,
    scope: request.scope,
    options: request.options,
    incident_id: request.incident_id,
  });

  return createHash('sha256').update(canonicalInput).digest('hex');
}

/**
 * Validate that a DecisionEvent complies with constitutional constraints.
 */
export function validateDecisionEventCompliance(event: DecisionEvent): {
  valid: boolean;
  violations: string[];
} {
  const violations: string[] = [];

  // Validate against schema
  const parseResult = DecisionEventSchema.safeParse(event);
  if (!parseResult.success) {
    for (const issue of parseResult.error.issues) {
      violations.push(`Schema violation: ${issue.path.join('.')} - ${issue.message}`);
    }
  }

  // Constitutional constraint: agent_id must match
  if (event.agent_id !== 'post-mortem-generator-agent') {
    violations.push(`Invalid agent_id: expected 'post-mortem-generator-agent', got '${event.agent_id}'`);
  }

  // Constitutional constraint: decision_type must be correct
  if (event.decision_type !== 'postmortem_generation') {
    violations.push(`Invalid decision_type: expected 'postmortem_generation', got '${event.decision_type}'`);
  }

  // CRITICAL: Constitutional constraint - constraints_applied MUST be empty
  if (event.constraints_applied.length > 0) {
    violations.push(
      `CONSTITUTIONAL VIOLATION: constraints_applied must be empty for read-only agent, got ${event.constraints_applied.length} constraints`
    );
  }

  // Validate inputs_hash format
  if (!/^[a-f0-9]{64}$/.test(event.inputs_hash)) {
    violations.push(`Invalid inputs_hash: must be 64-character hex string (SHA256)`);
  }

  // Validate confidence range
  if (event.confidence < 0 || event.confidence > 1) {
    violations.push(`Invalid confidence: must be between 0 and 1, got ${event.confidence}`);
  }

  // Validate timestamp format
  try {
    new Date(event.timestamp).toISOString();
  } catch {
    violations.push(`Invalid timestamp format: ${event.timestamp}`);
  }

  return {
    valid: violations.length === 0,
    violations,
  };
}

/**
 * Verify that the agent has not violated any constitutional constraints.
 * This is called before any external actions.
 */
export function verifyConstitutionalCompliance(
  action: string,
  context: Record<string, unknown>
): void {
  // List of prohibited actions for read-only agents
  const prohibitedActions = [
    'trigger_alert',
    'initiate_remediation',
    'modify_state',
    'execute_sql',
    'invoke_agent',
    'trigger_orchestration',
    'trigger_retry',
    'modify_routing',
    'modify_policy',
    'modify_threshold',
    'write_constraint',
    'recommend_remediation',
  ];

  if (prohibitedActions.includes(action)) {
    throw new ConstitutionalViolationError(
      `Attempted prohibited action: ${action}`,
      action,
      context
    );
  }
}

/**
 * Error class for constitutional violations.
 */
export class ConstitutionalViolationError extends Error {
  public readonly action: string;
  public readonly context: Record<string, unknown>;

  constructor(message: string, action: string, context: Record<string, unknown>) {
    super(message);
    this.name = 'ConstitutionalViolationError';
    this.action = action;
    this.context = context;
  }
}
