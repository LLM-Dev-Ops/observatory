/**
 * Failure Classification Agent - HTTP Handler
 *
 * CONSTITUTIONAL CLASSIFICATION: READ-ONLY, DIAGNOSTIC
 *
 * This handler processes failure events and classifies them into
 * deterministic categories. It emits DecisionEvents to ruvector-service
 * but does NOT trigger any actions.
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
import { loadConfig } from './config';
import { emitTelemetry, startSpan, endSpan } from './telemetry';

// =============================================================================
// HANDLER INITIALIZATION
// =============================================================================

const config = loadConfig();
const ruvectorClient = new RuvectorClient(config.ruvector);
const classificationEngine = new ClassificationEngine();

// =============================================================================
// MAIN HANDLER
// =============================================================================

/**
 * Main HTTP handler for Cloud Function
 */
export async function handleFailureClassification(
  req: Request,
  res: Response
): Promise<void> {
  const executionRef = randomUUID();
  const startTime = Date.now();

  const context: HandlerContext = {
    execution_ref: executionRef,
    correlation_id: req.headers['x-correlation-id'] as string,
    received_at: new Date().toISOString(),
    source_ip: req.ip,
  };

  // Start telemetry span
  const span = startSpan('failure-classification', executionRef);

  try {
    // Route based on path
    switch (req.path) {
      case '/classify':
        await handleSingleClassification(req, res, context);
        break;
      case '/classify/batch':
        await handleBatchClassification(req, res, context);
        break;
      case '/health':
        await handleHealthCheck(req, res);
        break;
      default:
        res.status(404).json({
          success: false,
          error: {
            code: 'NOT_FOUND',
            message: `Unknown endpoint: ${req.path}`,
          },
          metadata: buildMetadata(executionRef, startTime),
        });
    }
  } catch (error) {
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
// SINGLE CLASSIFICATION HANDLER
// =============================================================================

async function handleSingleClassification(
  req: Request,
  res: Response,
  context: HandlerContext
): Promise<void> {
  const startTime = Date.now();

  // Validate input
  const validation = validateFailureEvent(req.body);
  if (!validation.success) {
    throw new ValidationError(validation.errors!);
  }

  const event = validation.data!;

  // Classify the failure
  const classification = await classifyFailure(event, classificationEngine);

  // Build decision event
  const decisionEvent = buildDecisionEvent(
    [classification],
    hashInput(event),
    context.execution_ref,
    classification.confidence
  );

  // Validate decision event (constitutional compliance)
  const decisionValidation = validateDecisionEvent(decisionEvent);
  if (!decisionValidation.success) {
    throw new ConstitutionalViolationError(
      decisionValidation.errors![0].message,
      'decision_event_validation'
    );
  }

  // Persist to ruvector-service
  await ruvectorClient.persistDecisionEvent(decisionEvent);

  // Return response
  const response: HandlerResponse<FailureClassification> = {
    success: true,
    data: classification,
    metadata: buildMetadata(context.execution_ref, startTime),
  };

  res.status(200).json(response);
}

// =============================================================================
// BATCH CLASSIFICATION HANDLER
// =============================================================================

async function handleBatchClassification(
  req: Request,
  res: Response,
  context: HandlerContext
): Promise<void> {
  const startTime = Date.now();

  // Validate batch request
  const validation = validateBatchRequest(req.body);
  if (!validation.success) {
    throw new ValidationError(validation.errors!);
  }

  const { events, correlation_id } = validation.data!;
  const batchId = correlation_id || randomUUID();

  // Classify all events
  const classifications: FailureClassification[] = [];
  const failures: Array<{ span_id: string; error: string }> = [];

  for (const event of events) {
    try {
      const classification = await classifyFailure(event, classificationEngine);
      classifications.push(classification);
    } catch (error) {
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

  // Build decision event
  const decisionEvent = buildDecisionEvent(
    classifications,
    hashInputs(events),
    context.execution_ref,
    avgConfidence
  );

  // Validate and persist
  const decisionValidation = validateDecisionEvent(decisionEvent);
  if (!decisionValidation.success) {
    throw new ConstitutionalViolationError(
      decisionValidation.errors![0].message,
      'decision_event_validation'
    );
  }

  await ruvectorClient.persistDecisionEvent(decisionEvent);

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
// HELPER FUNCTIONS
// =============================================================================

function buildDecisionEvent(
  classifications: FailureClassification[],
  inputsHash: string,
  executionRef: string,
  confidence: number
): DecisionEvent {
  return {
    agent_id: 'failure-classification-agent',
    agent_version: AGENT_METADATA.version,
    decision_type: 'failure_classification',
    inputs_hash: inputsHash,
    outputs: classifications,
    confidence,
    constraints_applied: [], // ALWAYS empty for read-only agent
    execution_ref: executionRef,
    timestamp: new Date().toISOString(),
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
