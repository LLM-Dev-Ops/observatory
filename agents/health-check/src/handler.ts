/**
 * Health Check Agent - HTTP Handler
 *
 * Google Cloud Function entry point for health evaluation.
 *
 * ENDPOINTS:
 * - POST /evaluate - Evaluate health for specified targets
 * - POST /evaluate/batch - Batch health evaluation
 * - GET /health - Agent health check
 * - GET /metrics - Prometheus metrics
 */

import type { HttpFunction } from '@google-cloud/functions-framework';
import {
  HealthEvaluationRequestSchema,
  HealthEvaluationResponseSchema,
  ErrorResponseSchema,
  type HealthEvaluationRequest,
  type HealthEvaluationResponse,
  type ErrorResponse,
  type ErrorCode,
} from '../contracts/schemas.js';
import { loadConfig } from './config.js';
import { evaluateHealth, summarizeEvaluations } from './evaluator.js';
import { initializeClient, getClient } from './ruvector-client.js';
import { createDecisionEvent, validateDecisionEventCompliance } from './emitter.js';
import {
  startSpan,
  endSpan,
  getPrometheusMetrics,
  recordEvaluationMetrics,
  recordPersistenceMetrics,
  info,
  error as logError,
} from './telemetry.js';

// ============================================================================
// REQUEST PARSING
// ============================================================================

interface ParsedRequest {
  path: string;
  method: string;
  body: unknown;
}

async function parseRequest(req: { url?: string; method?: string; body?: unknown }): Promise<ParsedRequest> {
  const url = new URL(req.url ?? '/', 'http://localhost');
  return {
    path: url.pathname,
    method: (req.method ?? 'GET').toUpperCase(),
    body: req.body,
  };
}

// ============================================================================
// ERROR RESPONSE HELPERS
// ============================================================================

function createErrorResponse(
  code: ErrorCode,
  message: string,
  executionRef?: string,
  details?: unknown
): ErrorResponse {
  return {
    success: false,
    error: {
      code,
      message,
      details,
      timestamp: new Date().toISOString(),
      execution_ref: executionRef,
    },
  };
}

function sendResponse(
  res: { status: (code: number) => { send: (body: unknown) => void; end: () => void } },
  statusCode: number,
  body: unknown,
  contentType: string = 'application/json'
): void {
  if (contentType === 'application/json') {
    res.status(statusCode).send(body);
  } else {
    res.status(statusCode).send(body);
  }
}

// ============================================================================
// HEALTH EVALUATION HANDLER
// ============================================================================

async function handleEvaluate(
  request: HealthEvaluationRequest,
  executionRef: string
): Promise<{ status: number; body: HealthEvaluationResponse | ErrorResponse }> {
  const config = loadConfig();
  const processingStartTime = Date.now();

  // Initialize RuVector client if needed
  try {
    getClient();
  } catch {
    initializeClient(config.ruvector);
  }

  const client = getClient();

  // Calculate time window
  const windowEnd = new Date();
  const windowSeconds = parseWindowToSeconds(request.options.evaluation_window);
  const windowStart = new Date(windowEnd.getTime() - windowSeconds * 1000);

  // Fetch telemetry aggregates from ruvector-service
  const telemetryMap = await client.fetchTelemetryAggregates(
    request.targets,
    windowStart.toISOString(),
    windowEnd.toISOString()
  );

  // Evaluate each target
  const evaluations = [];
  for (const target of request.targets) {
    const targetKey = `${target.type}:${target.id}`;
    const telemetry = telemetryMap.get(targetKey);

    if (!telemetry) {
      // Create minimal telemetry for targets without data
      const minimalTelemetry = {
        target_id: target.id,
        target_type: target.type,
        window_start: windowStart.toISOString(),
        window_end: windowEnd.toISOString(),
        request_count: 0,
        error_count: 0,
        latency_avg_ms: 0,
      };

      const evaluation = evaluateHealth({
        target,
        telemetry: minimalTelemetry,
        options: request.options,
        config,
      });

      evaluations.push(evaluation);
    } else {
      const evaluation = evaluateHealth({
        target,
        telemetry,
        options: request.options,
        config,
      });

      evaluations.push(evaluation);
    }
  }

  const processingTimeMs = Date.now() - processingStartTime;

  // Create and validate decision event
  const decisionEvent = createDecisionEvent({
    request,
    evaluations,
    processingTimeMs,
    executionRef,
  });

  // Validate constitutional compliance
  const compliance = validateDecisionEventCompliance(decisionEvent);
  if (!compliance.valid) {
    logError('Decision event compliance violation', {
      violations: compliance.violations,
      execution_ref: executionRef,
    });
    return {
      status: 500,
      body: createErrorResponse(
        'INTERNAL_ERROR',
        'Constitutional compliance violation',
        executionRef,
        compliance.violations
      ),
    };
  }

  // Persist decision event
  const persistResult = await client.persistDecisionEvent(decisionEvent);
  recordPersistenceMetrics(persistResult.success);

  if (!persistResult.success) {
    logError('Failed to persist decision event', {
      error: persistResult.error,
      execution_ref: executionRef,
    });
    // Continue - don't fail the request due to persistence issues
  }

  // Record metrics
  recordEvaluationMetrics(evaluations.length, processingTimeMs, true);

  const response: HealthEvaluationResponse = {
    success: true,
    evaluations,
    execution_ref: executionRef,
    processing_time_ms: processingTimeMs,
  };

  return { status: 200, body: response };
}

// ============================================================================
// HEALTH CHECK HANDLER
// ============================================================================

async function handleHealthCheck(): Promise<{ status: number; body: unknown }> {
  const config = loadConfig();

  // Initialize client if needed
  try {
    getClient();
  } catch {
    initializeClient(config.ruvector);
  }

  const client = getClient();
  const ruvectorHealth = await client.healthCheck();
  const poolStats = client.getPoolStats();

  const overallStatus = ruvectorHealth.status === 'healthy' ? 'healthy' : 'degraded';

  return {
    status: overallStatus === 'healthy' ? 200 : 503,
    body: {
      status: overallStatus,
      agent_id: config.agent.id,
      agent_version: config.agent.version,
      classification: config.agent.classification,
      components: {
        ruvector: ruvectorHealth,
      },
      pool_stats: poolStats,
      timestamp: new Date().toISOString(),
    },
  };
}

// ============================================================================
// METRICS HANDLER
// ============================================================================

function handleMetrics(): { status: number; body: string; contentType: string } {
  return {
    status: 200,
    body: getPrometheusMetrics(),
    contentType: 'text/plain; charset=utf-8',
  };
}

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

function parseWindowToSeconds(window: string): number {
  const match = window.match(/^(\d+)(m|h|d)$/);
  if (!match) return 300; // Default 5 minutes

  const value = parseInt(match[1]!, 10);
  const unit = match[2];

  switch (unit) {
    case 'm': return value * 60;
    case 'h': return value * 60 * 60;
    case 'd': return value * 60 * 60 * 24;
    default: return 300;
  }
}

// ============================================================================
// MAIN HANDLER
// ============================================================================

/**
 * Main HTTP handler for Google Cloud Functions.
 */
export const handleHealthEvaluation: HttpFunction = async (req, res) => {
  const span = startSpan('health-evaluation');

  try {
    const parsed = await parseRequest(req);

    info('Request received', {
      path: parsed.path,
      method: parsed.method,
      execution_ref: span.executionRef,
    });

    // Route request
    if (parsed.path === '/health' && parsed.method === 'GET') {
      const result = await handleHealthCheck();
      endSpan(span, result.status < 400);
      sendResponse(res, result.status, result.body);
      return;
    }

    if (parsed.path === '/metrics' && parsed.method === 'GET') {
      const result = handleMetrics();
      endSpan(span, true);
      res.status(result.status).send(result.body);
      return;
    }

    if (parsed.path === '/evaluate' && parsed.method === 'POST') {
      // Validate request body
      const parseResult = HealthEvaluationRequestSchema.safeParse(parsed.body);
      if (!parseResult.success) {
        endSpan(span, false, { error: 'Validation failed' });
        sendResponse(res, 400, createErrorResponse(
          'VALIDATION_FAILED',
          'Invalid request body',
          span.executionRef,
          parseResult.error.issues
        ));
        return;
      }

      const result = await handleEvaluate(parseResult.data, span.executionRef);
      endSpan(span, result.status < 400);
      sendResponse(res, result.status, result.body);
      return;
    }

    // Method not allowed
    endSpan(span, false, { error: 'Method not allowed' });
    sendResponse(res, 405, createErrorResponse(
      'INVALID_INPUT',
      `Method ${parsed.method} not allowed for ${parsed.path}`,
      span.executionRef
    ));

  } catch (err) {
    const errorMessage = err instanceof Error ? err.message : String(err);
    logError('Unhandled error', {
      error: errorMessage,
      execution_ref: span.executionRef,
    });
    endSpan(span, false, { error: errorMessage });
    sendResponse(res, 500, createErrorResponse(
      'INTERNAL_ERROR',
      'Internal server error',
      span.executionRef
    ));
  }
};

// ============================================================================
// EXPORTS FOR TESTING
// ============================================================================

export {
  handleEvaluate,
  handleHealthCheck,
  handleMetrics,
  parseWindowToSeconds,
  createErrorResponse,
};
