/**
 * SLO/SLA Enforcement Agent - HTTP Handler
 *
 * Handles incoming HTTP requests for SLO/SLA enforcement.
 *
 * Endpoints:
 * - POST /enforce         - Evaluate SLOs against metrics (single request)
 * - POST /enforce/batch   - Batch evaluation
 * - GET  /violations      - Query violations
 * - GET  /analysis        - Get aggregated analysis
 * - GET  /health          - Health check
 *
 * Classification: ENFORCEMENT-CLASS, NON-ACTUATING
 */

import { randomUUID } from 'crypto';
import { createHash } from 'crypto';
import type { Request, Response } from 'express';

import { loadConfig } from './config';
import { getSloEnforcer } from './enforcer';
import { getRuvectorClient } from './ruvector-client';
import { getMetrics, recordEvaluation, recordError } from './telemetry';
import {
  AGENT_METADATA,
  validateSloEnforcementRequest,
  validateDecisionEvent,
  validateViolationQuery,
  SloEnforcementRequestSchema,
  ViolationQuerySchema,
  AnalysisRequestSchema,
} from '../contracts';
import type {
  SloEnforcementRequest,
  BatchEnforcementRequest,
  DecisionEvent,
  EnforcementResult,
  ViolationQuery,
  AnalysisRequest,
} from '../contracts';
import type { HandlerContext, HandlerResponse, AgentHealth } from '../types';
import { ValidationError, ConstitutionalViolationError, RuvectorError } from '../types';

const config = loadConfig();

/**
 * Build response metadata
 */
function buildMetadata(executionRef: string, startTime: number) {
  return {
    execution_ref: executionRef,
    processing_time_ms: Date.now() - startTime,
    agent_id: AGENT_METADATA.id,
    agent_version: AGENT_METADATA.version,
  };
}

/**
 * Calculate SHA256 hash of inputs
 */
function hashInputs(inputs: unknown): string {
  const hash = createHash('sha256');
  hash.update(JSON.stringify(inputs));
  return hash.digest('hex');
}

/**
 * Build DecisionEvent from enforcement result
 */
function buildDecisionEvent(
  executionRef: string,
  inputsHash: string,
  result: EnforcementResult,
  confidence: number
): DecisionEvent {
  return {
    agent_id: AGENT_METADATA.id,
    agent_version: AGENT_METADATA.version,
    decision_type: AGENT_METADATA.decision_type,
    inputs_hash: inputsHash,
    outputs: {
      violations: result.violations,
      slo_statuses: result.slo_statuses,
      metrics_evaluated: result.metrics_evaluated,
      slos_evaluated: result.slos_evaluated,
    },
    confidence,
    constraints_applied: [], // MUST be empty - constitutional requirement
    execution_ref: executionRef,
    timestamp: new Date().toISOString(),
  };
}

/**
 * Validate DecisionEvent for constitutional compliance
 */
function validateConstitutionalCompliance(event: DecisionEvent): void {
  // 1. constraints_applied MUST be empty
  if (event.constraints_applied.length !== 0) {
    throw new ConstitutionalViolationError(
      'constraints_applied must be empty for ENFORCEMENT-CLASS NON-ACTUATING agent'
    );
  }

  // 2. agent_id must be literal
  if (event.agent_id !== AGENT_METADATA.id) {
    throw new ConstitutionalViolationError(
      `agent_id must be '${AGENT_METADATA.id}'`
    );
  }

  // 3. decision_type must be literal
  if (event.decision_type !== AGENT_METADATA.decision_type) {
    throw new ConstitutionalViolationError(
      `decision_type must be '${AGENT_METADATA.decision_type}'`
    );
  }

  // 4. inputs_hash must be valid SHA256
  if (!/^[a-f0-9]{64}$/.test(event.inputs_hash)) {
    throw new ConstitutionalViolationError(
      'inputs_hash must be a valid SHA256 hex string'
    );
  }

  // 5. Validate with schema
  const validation = validateDecisionEvent(event);
  if (!validation.success) {
    throw new ConstitutionalViolationError(
      `DecisionEvent schema validation failed: ${JSON.stringify(validation.errors)}`
    );
  }
}

/**
 * Handle single enforcement request
 */
export async function handleEnforce(req: Request, res: Response): Promise<void> {
  const startTime = Date.now();
  const executionRef = randomUUID();

  const context: HandlerContext = {
    execution_ref: executionRef,
    correlation_id: req.headers['x-correlation-id'] as string | undefined,
    received_at: new Date().toISOString(),
    source_ip: req.ip,
  };

  try {
    // Validate input
    const validation = validateSloEnforcementRequest(req.body);
    if (!validation.success) {
      throw new ValidationError(validation.errors);
    }

    const request = validation.data!;
    const inputsHash = hashInputs(request);

    // Execute enforcement
    const enforcer = getSloEnforcer();
    const evaluationTime = new Date(request.evaluation_time);

    const result = enforcer.evaluateAll(
      request.slo_definitions,
      request.metrics,
      evaluationTime
    );

    // Calculate overall confidence
    const confidence = enforcer.calculateConfidence(
      request.metrics[0],
      undefined // Historical context not yet implemented
    );

    // Build DecisionEvent
    const decisionEvent = buildDecisionEvent(
      executionRef,
      inputsHash,
      result,
      confidence
    );

    // Validate constitutional compliance
    validateConstitutionalCompliance(decisionEvent);

    // Persist to ruvector-service
    const ruvector = getRuvectorClient();
    await ruvector.persistDecisionEvent(decisionEvent);

    // Record telemetry
    recordEvaluation(result.slos_evaluated, result.violations.length, result.processing_time_ms);

    // Respond
    const response: HandlerResponse<EnforcementResult> = {
      success: true,
      data: result,
      metadata: buildMetadata(executionRef, startTime),
    };

    res.status(200).json(response);

  } catch (error) {
    recordError(error as Error);
    handleError(res, error, executionRef, startTime);
  }
}

/**
 * Handle batch enforcement request
 */
export async function handleEnforceBatch(req: Request, res: Response): Promise<void> {
  const startTime = Date.now();
  const executionRef = randomUUID();

  try {
    // Validate batch request
    const batchResult = BatchEnforcementRequestSchema.safeParse(req.body);
    if (!batchResult.success) {
      throw new ValidationError(batchResult.error.errors);
    }

    const batchRequest = batchResult.data as BatchEnforcementRequest;
    const results: EnforcementResult[] = [];
    const decisionEvents: DecisionEvent[] = [];

    const enforcer = getSloEnforcer();

    // Process each request
    for (const request of batchRequest.requests) {
      const inputsHash = hashInputs(request);
      const evaluationTime = new Date(request.evaluation_time);

      const result = enforcer.evaluateAll(
        request.slo_definitions,
        request.metrics,
        evaluationTime
      );

      const confidence = enforcer.calculateConfidence(
        request.metrics[0],
        undefined
      );

      const decisionEvent = buildDecisionEvent(
        randomUUID(),
        inputsHash,
        result,
        confidence
      );

      validateConstitutionalCompliance(decisionEvent);

      results.push(result);
      decisionEvents.push(decisionEvent);
    }

    // Batch persist
    const ruvector = getRuvectorClient();
    await ruvector.persistDecisionEventsBatch(decisionEvents);

    // Record telemetry
    const totalSlos = results.reduce((acc, r) => acc + r.slos_evaluated, 0);
    const totalViolations = results.reduce((acc, r) => acc + r.violations.length, 0);
    recordEvaluation(totalSlos, totalViolations, Date.now() - startTime);

    const response: HandlerResponse<{ results: EnforcementResult[]; batch_size: number }> = {
      success: true,
      data: {
        results,
        batch_size: results.length,
      },
      metadata: buildMetadata(executionRef, startTime),
    };

    res.status(200).json(response);

  } catch (error) {
    recordError(error as Error);
    handleError(res, error, executionRef, startTime);
  }
}

/**
 * Handle violations query
 */
export async function handleQueryViolations(req: Request, res: Response): Promise<void> {
  const startTime = Date.now();
  const executionRef = randomUUID();

  try {
    // Parse and validate query parameters
    const queryParams: ViolationQuery = {
      slo_id: req.query.slo_id as string | undefined,
      breach_type: req.query.breach_type as ViolationQuery['breach_type'],
      severity: req.query.severity as ViolationQuery['severity'],
      provider: req.query.provider as string | undefined,
      model: req.query.model as string | undefined,
      environment: req.query.environment as string | undefined,
      start_time: req.query.start_time as string | undefined,
      end_time: req.query.end_time as string | undefined,
      is_sla: req.query.is_sla === 'true' ? true : req.query.is_sla === 'false' ? false : undefined,
      limit: parseInt(req.query.limit as string) || 100,
      offset: parseInt(req.query.offset as string) || 0,
      sort_by: (req.query.sort_by as ViolationQuery['sort_by']) || 'detected_at',
      sort_order: (req.query.sort_order as ViolationQuery['sort_order']) || 'desc',
    };

    const validation = validateViolationQuery(queryParams);
    if (!validation.success) {
      throw new ValidationError(validation.errors);
    }

    const ruvector = getRuvectorClient();
    const violations = await ruvector.getViolations(validation.data!);

    const response: HandlerResponse<{ violations: typeof violations; count: number }> = {
      success: true,
      data: {
        violations,
        count: violations.length,
      },
      metadata: buildMetadata(executionRef, startTime),
    };

    res.status(200).json(response);

  } catch (error) {
    recordError(error as Error);
    handleError(res, error, executionRef, startTime);
  }
}

/**
 * Handle analysis request
 */
export async function handleAnalysis(req: Request, res: Response): Promise<void> {
  const startTime = Date.now();
  const executionRef = randomUUID();

  try {
    const analysisResult = AnalysisRequestSchema.safeParse(req.query);
    if (!analysisResult.success) {
      throw new ValidationError(analysisResult.error.errors);
    }

    const request = analysisResult.data as AnalysisRequest;

    const ruvector = getRuvectorClient();
    const analysis = await ruvector.getViolationAnalysis(
      request.start_time,
      request.end_time,
      request.group_by
    );

    const response: HandlerResponse<typeof analysis> = {
      success: true,
      data: analysis,
      metadata: buildMetadata(executionRef, startTime),
    };

    res.status(200).json(response);

  } catch (error) {
    recordError(error as Error);
    handleError(res, error, executionRef, startTime);
  }
}

/**
 * Handle health check
 */
export async function handleHealth(req: Request, res: Response): Promise<void> {
  const startTime = Date.now();
  const executionRef = randomUUID();

  try {
    const ruvector = getRuvectorClient();
    const ruvectorHealth = await ruvector.healthCheck();
    const metrics = getMetrics();

    const health: AgentHealth = {
      status: ruvectorHealth.status === 'healthy' ? 'healthy' : 'degraded',
      uptime_seconds: metrics.uptime_seconds,
      ruvector_status: ruvectorHealth,
      metrics,
    };

    const response: HandlerResponse<AgentHealth> = {
      success: true,
      data: health,
      metadata: buildMetadata(executionRef, startTime),
    };

    res.status(health.status === 'healthy' ? 200 : 503).json(response);

  } catch (error) {
    const response: HandlerResponse<never> = {
      success: false,
      error: {
        code: 'HEALTH_CHECK_FAILED',
        message: (error as Error).message,
      },
      metadata: buildMetadata(executionRef, startTime),
    };

    res.status(503).json(response);
  }
}

/**
 * Handle replay request (for CLI inspection)
 */
export async function handleReplay(req: Request, res: Response): Promise<void> {
  const startTime = Date.now();
  const executionRef = req.params.executionRef;

  try {
    const ruvector = getRuvectorClient();
    const originalEvent = await ruvector.getDecisionEventByRef(executionRef);

    if (!originalEvent) {
      const response: HandlerResponse<never> = {
        success: false,
        error: {
          code: 'NOT_FOUND',
          message: `DecisionEvent with execution_ref ${executionRef} not found`,
        },
        metadata: buildMetadata(randomUUID(), startTime),
      };
      res.status(404).json(response);
      return;
    }

    const response: HandlerResponse<DecisionEvent> = {
      success: true,
      data: originalEvent,
      metadata: buildMetadata(executionRef, startTime),
    };

    res.status(200).json(response);

  } catch (error) {
    recordError(error as Error);
    handleError(res, error, executionRef, startTime);
  }
}

/**
 * Central error handler
 */
function handleError(
  res: Response,
  error: unknown,
  executionRef: string,
  startTime: number
): void {
  if (error instanceof ValidationError) {
    const response: HandlerResponse<never> = {
      success: false,
      error: {
        code: error.code,
        message: error.message,
        details: error.errors,
      },
      metadata: buildMetadata(executionRef, startTime),
    };
    res.status(400).json(response);
    return;
  }

  if (error instanceof ConstitutionalViolationError) {
    const response: HandlerResponse<never> = {
      success: false,
      error: {
        code: error.code,
        message: error.message,
        details: { constraint: error.constraint },
      },
      metadata: buildMetadata(executionRef, startTime),
    };
    res.status(500).json(response);
    return;
  }

  if (error instanceof RuvectorError) {
    const response: HandlerResponse<never> = {
      success: false,
      error: {
        code: error.code,
        message: error.message,
      },
      metadata: buildMetadata(executionRef, startTime),
    };
    res.status(502).json(response);
    return;
  }

  // Unknown error
  const response: HandlerResponse<never> = {
    success: false,
    error: {
      code: 'INTERNAL_ERROR',
      message: (error as Error).message,
    },
    metadata: buildMetadata(executionRef, startTime),
  };
  res.status(500).json(response);
}

/**
 * Import for batch schema validation
 */
import { z } from 'zod';
const BatchEnforcementRequestSchema = z.object({
  requests: z.array(SloEnforcementRequestSchema).min(1).max(50),
});
