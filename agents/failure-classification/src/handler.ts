/**
 * Failure Classification Agent - HTTP Handler
 *
 * CONSTITUTIONAL CLASSIFICATION: READ-ONLY, DIAGNOSTIC
 *
 * This handler processes failure events and classifies them into
 * deterministic categories. It emits DecisionEvents to ruvector-service
 * but does NOT trigger any actions.
 *
 * HARDENED: Phase 1 Layer 1 deployment
 * - Mandatory startup assertions
 * - Performance boundary guards
 * - Contract assertions (≥1 DecisionEvent per run)
 * - Standardized logging
 */

import type { Request, Response } from '@google-cloud/functions-framework';
import { randomUUID } from 'crypto';
import {
  validateFailureEvent,
  validateBatchRequest,
  validateDecisionEvent,
  hashInput,
  hashInputs,
  ConstitutionalViolationError,
  ValidationError,
  type FailureEvent,
  type FailureClassification,
  type BatchClassificationResult,
  type DecisionEvent,
  type HandlerContext,
  type HandlerResponse,
  AGENT_METADATA,
} from '../contracts';
import { classifyFailure, ClassificationEngine } from './classifier';
import { RuvectorClient } from './ruvector-client';
import { loadConfig, loadConfigWithHardenedEnv } from './config';
import { emitTelemetry, startSpan, endSpan } from './telemetry';
import {
  initializeHardenedAgent,
  PerformanceGuard,
  ContractAssertions,
  PerformanceBoundaryError,
  ContractViolationError,
  logAgentAbort,
  logDecisionEventEmitted,
  type HardenedAgentContext,
} from '../../shared/hardening/index';

// =============================================================================
// HANDLER INITIALIZATION (HARDENED)
// =============================================================================

let hardenedContext: HardenedAgentContext | null = null;
let config = loadConfig();
let ruvectorClient = new RuvectorClient(config.ruvector);
const classificationEngine = new ClassificationEngine();

/**
 * Initialize hardened agent context.
 * CRASHES the container if initialization fails.
 */
async function ensureHardenedInitialization(): Promise<HardenedAgentContext> {
  if (hardenedContext) {
    return hardenedContext;
  }

  hardenedContext = await initializeHardenedAgent();
  config = loadConfigWithHardenedEnv(hardenedContext.environment);
  ruvectorClient = new RuvectorClient(config.ruvector);

  return hardenedContext;
}

// =============================================================================
// MAIN HANDLER (HARDENED)
// =============================================================================

/**
 * Main HTTP handler for Cloud Function
 *
 * HARDENED:
 * - Ensures hardened initialization on first call
 * - Creates new PerformanceGuard and ContractAssertions per request
 * - Asserts contracts are met at the end of each run
 */
export async function handleFailureClassification(
  req: Request,
  res: Response
): Promise<void> {
  const executionRef = randomUUID();
  const startTime = Date.now();

  // HARDENED: Ensure initialization (crashes container if fails)
  const hardened = await ensureHardenedInitialization();

  // HARDENED: Create per-request guards
  const performanceGuard = new PerformanceGuard();
  const contractAssertions = new ContractAssertions();

  const context: HandlerContext = {
    execution_ref: executionRef,
    correlation_id: req.headers['x-correlation-id'] as string,
    received_at: new Date().toISOString(),
    source_ip: req.ip,
  };

  // Start telemetry span
  const span = startSpan('failure-classification', executionRef);

  try {
    // HARDENED: Check latency boundary before processing
    performanceGuard.assertLatencyLimit();

    // Route based on path
    switch (req.path) {
      case '/classify':
        await handleSingleClassification(req, res, context, performanceGuard, contractAssertions);
        break;
      case '/classify/batch':
        await handleBatchClassification(req, res, context, performanceGuard, contractAssertions);
        break;
      case '/health':
        await handleHealthCheck(req, res);
        // Health check doesn't require DecisionEvent
        return;
      default:
        res.status(404).json({
          success: false,
          error: {
            code: 'NOT_FOUND',
            message: `Unknown endpoint: ${req.path}`,
          },
          metadata: buildMetadata(executionRef, startTime),
        });
        // 404 doesn't require DecisionEvent
        return;
    }

    // HARDENED: Assert contracts are met (≥1 DecisionEvent emitted)
    contractAssertions.assertContractsMet();

  } catch (error) {
    // HARDENED: Handle performance boundary and contract violations
    if (error instanceof PerformanceBoundaryError) {
      logAgentAbort('performance_boundary_exceeded', [error.message]);
      res.status(503).json({
        success: false,
        error: {
          code: 'PERFORMANCE_BOUNDARY_EXCEEDED',
          message: error.message,
        },
        metadata: buildMetadata(executionRef, startTime),
      });
      return;
    }

    if (error instanceof ContractViolationError) {
      logAgentAbort('contract_violation', [error.message]);
      res.status(500).json({
        success: false,
        error: {
          code: 'CONTRACT_VIOLATION',
          message: error.message,
        },
        metadata: buildMetadata(executionRef, startTime),
      });
      return;
    }

    await handleError(error, res, executionRef, startTime);
  } finally {
    endSpan(span, 'OK');
    emitTelemetry('classification_completed', {
      execution_ref: executionRef,
      duration_ms: Date.now() - startTime,
    });
  }
}

// =============================================================================
// SINGLE CLASSIFICATION HANDLER (HARDENED)
// =============================================================================

async function handleSingleClassification(
  req: Request,
  res: Response,
  context: HandlerContext,
  performanceGuard: PerformanceGuard,
  contractAssertions: ContractAssertions
): Promise<void> {
  const startTime = Date.now();

  // HARDENED: Assert call limit
  performanceGuard.assertCallLimit();

  // Validate input
  const validation = validateFailureEvent(req.body);
  if (!validation.success) {
    throw new ValidationError(validation.errors!);
  }

  const event = validation.data!;

  // HARDENED: Check latency before classification
  performanceGuard.assertLatencyLimit();

  // Classify the failure
  const classification = await classifyFailure(event, classificationEngine);

  // HARDENED: Build decision event with identity fields
  const decisionEvent = buildDecisionEvent(
    [classification],
    hashInput(event),
    context.execution_ref,
    classification.confidence,
    'failure_signal' // event_type: signal, NOT conclusion
  );

  // Validate decision event (constitutional compliance)
  const decisionValidation = validateDecisionEvent(decisionEvent);
  if (!decisionValidation.success) {
    throw new ConstitutionalViolationError(
      decisionValidation.errors![0].message,
      'decision_event_validation'
    );
  }

  // HARDENED: Assert call limit before persistence
  performanceGuard.assertCallLimit();

  // Persist to ruvector-service
  await ruvectorClient.persistDecisionEvent(decisionEvent);

  // HARDENED: Record DecisionEvent emission for contract assertion
  contractAssertions.recordDecisionEventEmitted(
    context.execution_ref,
    config.identity.agentName
  );

  // Return response
  const response: HandlerResponse<FailureClassification> = {
    success: true,
    data: classification,
    metadata: buildMetadata(context.execution_ref, startTime),
  };

  res.status(200).json(response);
}

// =============================================================================
// BATCH CLASSIFICATION HANDLER (HARDENED)
// =============================================================================

async function handleBatchClassification(
  req: Request,
  res: Response,
  context: HandlerContext,
  performanceGuard: PerformanceGuard,
  contractAssertions: ContractAssertions
): Promise<void> {
  const startTime = Date.now();

  // HARDENED: Assert call limit
  performanceGuard.assertCallLimit();

  // Validate batch request
  const validation = validateBatchRequest(req.body);
  if (!validation.success) {
    throw new ValidationError(validation.errors!);
  }

  const { events, correlation_id } = validation.data!;
  const batchId = correlation_id || randomUUID();

  // HARDENED: Check latency before batch processing
  performanceGuard.assertLatencyLimit();

  // Classify all events
  const classifications: FailureClassification[] = [];
  const failures: Array<{ span_id: string; error: string }> = [];

  for (const event of events) {
    try {
      // HARDENED: Check latency during batch processing
      performanceGuard.assertLatencyLimit();

      const classification = await classifyFailure(event, classificationEngine);
      classifications.push(classification);
    } catch (error) {
      if (error instanceof PerformanceBoundaryError) {
        throw error; // Re-throw performance boundary errors
      }
      failures.push({
        span_id: event.span_id,
        error: error instanceof Error ? error.message : 'Unknown error',
      });
    }
  }

  // Calculate aggregate confidence
  const avgConfidence = classifications.length > 0
    ? classifications.reduce((sum, c) => sum + c.confidence, 0) / classifications.length
    : 0;

  // HARDENED: Build decision event with identity fields
  const decisionEvent = buildDecisionEvent(
    classifications,
    hashInputs(events),
    context.execution_ref,
    avgConfidence,
    'failure_batch_signal' // event_type: signal, NOT conclusion
  );

  // Validate and persist
  const decisionValidation = validateDecisionEvent(decisionEvent);
  if (!decisionValidation.success) {
    throw new ConstitutionalViolationError(
      decisionValidation.errors![0].message,
      'decision_event_validation'
    );
  }

  // HARDENED: Assert call limit before persistence
  performanceGuard.assertCallLimit();

  await ruvectorClient.persistDecisionEvent(decisionEvent);

  // HARDENED: Record DecisionEvent emission for contract assertion
  contractAssertions.recordDecisionEventEmitted(
    context.execution_ref,
    config.identity.agentName
  );

  // Build response
  const result: BatchClassificationResult = {
    classifications,
    batch_id: batchId,
    total_events: events.length,
    classified_count: classifications.length,
    failed_count: failures.length,
    processing_time_ms: Date.now() - startTime,
  };

  const response: HandlerResponse<BatchClassificationResult> = {
    success: true,
    data: result,
    metadata: buildMetadata(context.execution_ref, startTime),
  };

  res.status(200).json(response);
}

// =============================================================================
// HEALTH CHECK HANDLER
// =============================================================================

async function handleHealthCheck(req: Request, res: Response): Promise<void> {
  const ruvectorHealth = await ruvectorClient.healthCheck();

  const status = ruvectorHealth.healthy ? 'healthy' : 'degraded';

  res.status(ruvectorHealth.healthy ? 200 : 503).json({
    status,
    agent_id: AGENT_METADATA.id,
    agent_version: AGENT_METADATA.version,
    classification: AGENT_METADATA.classification,
    checks: {
      ruvector: ruvectorHealth,
    },
    timestamp: new Date().toISOString(),
  });
}

// =============================================================================
// HELPER FUNCTIONS (HARDENED)
// =============================================================================

/**
 * Build a hardened DecisionEvent with identity fields.
 *
 * HARDENED: Includes source_agent, domain, phase, layer
 */
function buildDecisionEvent(
  classifications: FailureClassification[],
  inputsHash: string,
  executionRef: string,
  confidence: number,
  eventType: string = 'failure_signal'
): DecisionEvent {
  return {
    // HARDENED: Agent identity fields (Phase 1 Layer 1)
    source_agent: config.identity.agentName,
    domain: config.identity.agentDomain,
    phase: config.identity.agentPhase,
    layer: config.identity.agentLayer,

    // HARDENED: Event type (signal, NOT conclusion)
    event_type: eventType,

    // Original fields
    agent_id: 'failure-classification-agent',
    agent_version: AGENT_METADATA.version,
    decision_type: 'failure_classification',
    inputs_hash: inputsHash,
    outputs: classifications,
    confidence,
    constraints_applied: [], // ALWAYS empty for read-only agent
    execution_ref: executionRef,
    timestamp: new Date().toISOString(),

    // HARDENED: Evidence references (from classifications)
    evidence_refs: classifications.map(c => ({
      ref_type: 'span_id' as const,
      ref_value: c.span_id,
      timestamp: c.classified_at,
    })),
  };
}

function buildMetadata(executionRef: string, startTime: number) {
  return {
    execution_ref: executionRef,
    processing_time_ms: Date.now() - startTime,
    agent_id: AGENT_METADATA.id,
    agent_version: AGENT_METADATA.version,
  };
}

async function handleError(
  error: unknown,
  res: Response,
  executionRef: string,
  startTime: number
): Promise<void> {
  if (error instanceof ValidationError) {
    res.status(400).json({
      success: false,
      error: {
        code: 'VALIDATION_ERROR',
        message: error.message,
        details: { errors: error.errors },
      },
      metadata: buildMetadata(executionRef, startTime),
    });
  } else if (error instanceof ConstitutionalViolationError) {
    // Constitutional violations are serious - log and return 500
    console.error('CONSTITUTIONAL VIOLATION:', error.message, error.operation);
    res.status(500).json({
      success: false,
      error: {
        code: 'CONSTITUTIONAL_VIOLATION',
        message: 'Agent attempted prohibited operation',
        details: { operation: error.operation },
      },
      metadata: buildMetadata(executionRef, startTime),
    });
  } else {
    console.error('Unexpected error:', error);
    res.status(500).json({
      success: false,
      error: {
        code: 'INTERNAL_ERROR',
        message: 'An unexpected error occurred',
      },
      metadata: buildMetadata(executionRef, startTime),
    });
  }
}

// =============================================================================
// CLOUD FUNCTION ENTRY POINT
// =============================================================================

export { handleFailureClassification as handleClassification };
