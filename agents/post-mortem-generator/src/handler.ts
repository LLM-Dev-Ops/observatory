/**
 * Post-Mortem Generator Agent - HTTP Handler
 *
 * Google Cloud Function entry point for post-mortem generation.
 *
 * CONSTITUTIONAL CLASSIFICATION: READ-ONLY
 *
 * ENDPOINTS:
 * - POST /generate - Generate a post-mortem report
 * - POST /generate/batch - Batch post-mortem generation
 * - GET /health - Agent health check
 * - GET /metrics - Prometheus metrics
 *
 * This agent MUST NOT:
 * - Influence live systems
 * - Write advisory constraints
 * - Recommend remediation actions
 * - Trigger alerts
 * - Modify system state
 */

import type { HttpFunction } from '@google-cloud/functions-framework';
import {
  PostMortemRequestSchema,
  BatchPostMortemRequestSchema,
  ErrorResponseSchema,
  type PostMortemRequest,
  type PostMortemReport,
  type ErrorResponse,
  type ErrorCode,
  type SuccessResponse,
  type BatchPostMortemResult,
} from '../contracts/schemas.js';
import { loadConfig } from './config.js';
import { generatePostMortem, type GeneratorInput } from './generator.js';
import { initializeClient, getClient } from './ruvector-client.js';
import { createDecisionEvent, validateDecisionEventCompliance } from './emitter.js';
import {
  startSpan,
  endSpan,
  getPrometheusMetrics,
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
// POST-MORTEM GENERATION HANDLER
// ============================================================================

async function handleGenerate(
  request: PostMortemRequest,
  executionRef: string
): Promise<{ status: number; body: SuccessResponse | ErrorResponse }> {
  const config = loadConfig();
  const processingStartTime = Date.now();

  // Initialize RuVector client if needed
  try {
    getClient();
  } catch {
    initializeClient(config.ruvector);
  }

  const client = getClient();

  // Parse time range
  const startTime = new Date(request.time_range.start_time);
  const endTime = new Date(request.time_range.end_time);

  // Fetch data from ruvector-service
  let failureData, healthData, telemetryData, failureClassifications, healthEvaluations;

  try {
    // Fetch aggregated failure data
    failureData = await client.getAggregatedFailures({
      startTime,
      endTime,
      providers: request.scope?.providers,
      models: request.scope?.models,
      categories: request.scope?.include_categories,
      severities: request.scope?.min_severity ? [request.scope.min_severity] : undefined,
    });

    // Fetch aggregated health data
    healthData = await client.getAggregatedHealth({
      startTime,
      endTime,
      targetIds: request.scope?.services,
    });

    // Fetch aggregated telemetry data
    telemetryData = await client.getAggregatedTelemetry({
      startTime,
      endTime,
      providers: request.scope?.providers,
      models: request.scope?.models,
    });

    // Fetch detailed failure classifications for timeline
    const classificationResponse = await client.getFailureClassifications({
      startTime,
      endTime,
      providers: request.scope?.providers,
      models: request.scope?.models,
      categories: request.scope?.include_categories,
      limit: request.options?.max_timeline_events || 1000,
    });
    failureClassifications = classificationResponse.classifications;

    // Fetch health evaluations for timeline
    const healthResponse = await client.getHealthEvaluations({
      startTime,
      endTime,
      targetIds: request.scope?.services,
      limit: request.options?.max_timeline_events || 1000,
    });
    healthEvaluations = healthResponse.evaluations;
  } catch (err) {
    const errorMessage = err instanceof Error ? err.message : String(err);
    logError('Failed to fetch data from ruvector-service', {
      error: errorMessage,
      execution_ref: executionRef,
    });
    return {
      status: 502,
      body: createErrorResponse(
        'PERSISTENCE_ERROR',
        'Failed to fetch historical data',
        executionRef,
        { error: errorMessage }
      ),
    };
  }

  // Check for insufficient data
  if (
    failureData.total_failures === 0 &&
    healthData.health_transitions.length === 0 &&
    telemetryData.total_requests === 0
  ) {
    return {
      status: 422,
      body: createErrorResponse(
        'INSUFFICIENT_DATA',
        'No data available for the specified time range and scope',
        executionRef
      ),
    };
  }

  // Generate post-mortem report
  const generatorInput: GeneratorInput = {
    request,
    failureData,
    healthData,
    telemetryData,
    failureClassifications,
    healthEvaluations,
  };

  const result = generatePostMortem(generatorInput);
  const processingTimeMs = Date.now() - processingStartTime;

  // Create and validate decision event
  const decisionEvent = createDecisionEvent({
    request,
    reports: [result.report],
    confidence: result.confidence,
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
        'CONSTITUTIONAL_VIOLATION',
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

  info('Post-mortem generated successfully', {
    report_id: result.report.report_id,
    confidence: result.confidence,
    processing_time_ms: processingTimeMs,
    execution_ref: executionRef,
  });

  const response: SuccessResponse = {
    success: true,
    report: result.report,
    execution_ref: executionRef,
    processing_time_ms: processingTimeMs,
  };

  return { status: 200, body: response };
}

// ============================================================================
// BATCH GENERATION HANDLER
// ============================================================================

async function handleBatchGenerate(
  requests: PostMortemRequest[],
  correlationId: string | undefined,
  executionRef: string
): Promise<{ status: number; body: BatchPostMortemResult | ErrorResponse }> {
  const processingStartTime = Date.now();
  const reports: PostMortemReport[] = [];
  let successfulCount = 0;
  let failedCount = 0;

  for (const request of requests) {
    try {
      const result = await handleGenerate(request, `${executionRef}-${reports.length}`);
      if (result.status === 200 && 'report' in result.body) {
        reports.push((result.body as SuccessResponse).report);
        successfulCount++;
      } else {
        failedCount++;
      }
    } catch {
      failedCount++;
    }
  }

  const processingTimeMs = Date.now() - processingStartTime;

  const response: BatchPostMortemResult = {
    reports,
    batch_id: correlationId || executionRef,
    total_requested: requests.length,
    successful_count: successfulCount,
    failed_count: failedCount,
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
  const poolStats = client.getConnectionPoolStatus();

  const overallStatus = ruvectorHealth.healthy ? 'healthy' : 'degraded';

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
// MAIN HANDLER
// ============================================================================

/**
 * Main HTTP handler for Google Cloud Functions.
 */
export const handlePostMortemGeneration: HttpFunction = async (req, res) => {
  const span = startSpan('postmortem-generation');

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

    if (parsed.path === '/generate' && parsed.method === 'POST') {
      // Validate request body
      const parseResult = PostMortemRequestSchema.safeParse(parsed.body);
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

      const result = await handleGenerate(parseResult.data, span.executionRef);
      endSpan(span, result.status < 400);
      sendResponse(res, result.status, result.body);
      return;
    }

    if (parsed.path === '/generate/batch' && parsed.method === 'POST') {
      // Validate request body
      const parseResult = BatchPostMortemRequestSchema.safeParse(parsed.body);
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

      const result = await handleBatchGenerate(
        parseResult.data.requests,
        parseResult.data.correlation_id,
        span.executionRef
      );
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
  handleGenerate,
  handleBatchGenerate,
  handleHealthCheck,
  handleMetrics,
  createErrorResponse,
};
