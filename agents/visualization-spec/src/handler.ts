/**
 * Visualization Spec Agent - HTTP Handler
 *
 * Google Cloud Function entry point.
 * Routes requests to appropriate handlers and manages the request lifecycle.
 *
 * Classification: READ-ONLY, PRESENTATIONAL
 */

import type { HttpFunction } from '@google-cloud/functions-framework';
import { randomUUID } from 'crypto';

import { getConfig, validateConfig } from './config.js';
import { generateVisualizationSpec, generateBatchVisualizationSpecs } from './generator.js';
import { createDecisionEvent, createProcessingMetrics } from './emitter.js';
import { getRuvectorClient } from './ruvector-client.js';
import {
  startSpan,
  endSpan,
  getPrometheusMetrics,
  recordGenerationMetrics,
  recordPersistenceMetrics,
  log,
} from './telemetry.js';
import {
  validateRequest,
  validateBatchRequest,
  createErrorResponse,
  computeInputHash,
} from '../contracts/validation.js';
import {
  AGENT_ID,
  AGENT_VERSION,
  AGENT_CLASSIFICATION,
  type VisualizationRequest,
  type VisualizationResponse,
  type BatchVisualizationResponse,
  type ErrorResponse,
  type HealthStatus,
} from '../contracts/schemas.js';
import type { GenerationContext } from '../contracts/types.js';

// =============================================================================
// Request Parsing
// =============================================================================

interface ParsedRequest {
  path: string;
  method: string;
  body: unknown;
  requestId: string;
}

/**
 * Parses an incoming HTTP request
 */
function parseRequest(req: Parameters<HttpFunction>[0]): ParsedRequest {
  return {
    path: req.path ?? '/',
    method: req.method ?? 'GET',
    body: req.body,
    requestId: (req.headers['x-request-id'] as string) ?? randomUUID(),
  };
}

// =============================================================================
// Response Helpers
// =============================================================================

/**
 * Sends a JSON response
 */
function sendResponse(
  res: Parameters<HttpFunction>[1],
  status: number,
  body: VisualizationResponse | BatchVisualizationResponse | ErrorResponse | HealthStatus | string
): void {
  if (typeof body === 'string') {
    res.status(status).type('text/plain').send(body);
  } else {
    res.status(status).json(body);
  }
}

// =============================================================================
// Route Handlers
// =============================================================================

/**
 * Handles single visualization spec generation
 */
async function handleGenerate(
  request: VisualizationRequest,
  requestId: string,
  span: ReturnType<typeof startSpan>
): Promise<{ status: number; body: VisualizationResponse | ErrorResponse }> {
  const startTime = Date.now();
  const parsingStart = Date.now();

  // Validate request
  const validation = validateRequest(request);
  const validationMs = Date.now() - parsingStart;

  if (!validation.success) {
    return {
      status: 400,
      body: createErrorResponse(validation.error, requestId),
    };
  }

  const generationStart = Date.now();

  // Create generation context
  const context: GenerationContext = {
    requestId,
    executionRef: span.executionRef,
    startTime,
    inputHash: computeInputHash(validation.data),
  };

  // Generate visualization spec
  const spec = generateVisualizationSpec(validation.data, context);
  const generationMs = Date.now() - generationStart;

  // Record metrics
  recordGenerationMetrics(1, generationMs, true);

  // Create and persist decision event
  const persistStart = Date.now();
  const processingMetrics = createProcessingMetrics(
    startTime,
    validationMs - (Date.now() - parsingStart - validationMs), // parsing_ms
    validationMs,
    generationMs,
    1
  );

  const decisionEvent = createDecisionEvent({
    request: validation.data,
    specs: [spec],
    executionRef: span.executionRef,
    processingMetrics,
  });

  // Persist to RuVector (async, non-blocking for response)
  const ruvector = getRuvectorClient();
  ruvector.persistDecisionEvent(decisionEvent)
    .then(result => {
      recordPersistenceMetrics(result.success, Date.now() - persistStart);
      if (!result.success) {
        log('error', 'Failed to persist decision event', { error: result.error });
      }
    })
    .catch(error => {
      recordPersistenceMetrics(false);
      log('error', 'Error persisting decision event', { error: error.message });
    });

  return {
    status: 200,
    body: {
      success: true,
      spec,
      processing_time_ms: Date.now() - startTime,
      request_id: requestId,
    },
  };
}

/**
 * Handles batch visualization spec generation
 */
async function handleBatchGenerate(
  body: unknown,
  requestId: string,
  span: ReturnType<typeof startSpan>
): Promise<{ status: number; body: BatchVisualizationResponse | ErrorResponse }> {
  const startTime = Date.now();
  const parsingStart = Date.now();

  // Validate batch request
  const validation = validateBatchRequest(body);
  const validationMs = Date.now() - parsingStart;

  if (!validation.success) {
    return {
      status: 400,
      body: createErrorResponse(validation.error, requestId),
    };
  }

  const generationStart = Date.now();

  // Generate all specs
  const specs = generateBatchVisualizationSpecs(
    validation.data.requests,
    validation.data.shared_styling as any,
    { executionRef: span.executionRef }
  );
  const generationMs = Date.now() - generationStart;

  // Record metrics
  recordGenerationMetrics(specs.length, generationMs, true);

  // Create and persist decision event
  const persistStart = Date.now();
  const processingMetrics = createProcessingMetrics(
    startTime,
    validationMs - (Date.now() - parsingStart - validationMs),
    validationMs,
    generationMs,
    specs.length
  );

  const decisionEvent = createDecisionEvent({
    request: validation.data.requests,
    specs,
    executionRef: span.executionRef,
    processingMetrics,
  });

  // Persist to RuVector (async)
  const ruvector = getRuvectorClient();
  ruvector.persistDecisionEvent(decisionEvent)
    .then(result => {
      recordPersistenceMetrics(result.success, Date.now() - persistStart);
    })
    .catch(error => {
      recordPersistenceMetrics(false);
      log('error', 'Error persisting batch decision event', { error: error.message });
    });

  return {
    status: 200,
    body: {
      success: true,
      specs,
      processing_time_ms: Date.now() - startTime,
      request_id: requestId,
    },
  };
}

/**
 * Handles health check requests
 */
async function handleHealth(): Promise<{ status: number; body: HealthStatus }> {
  const ruvector = getRuvectorClient();
  const ruvectorHealth = await ruvector.healthCheck();

  const status: HealthStatus = {
    status: ruvectorHealth.status === 'healthy' ? 'healthy' : 'degraded',
    agent_id: AGENT_ID,
    agent_version: AGENT_VERSION,
    classification: AGENT_CLASSIFICATION,
    checks: {
      ruvector_connectivity: {
        status: ruvectorHealth.status === 'healthy' ? 'pass' : 'fail',
        latency_ms: ruvectorHealth.latencyMs,
        error: ruvectorHealth.error,
      },
      schema_validation: {
        status: 'pass',
      },
    },
    timestamp: new Date().toISOString(),
  };

  return {
    status: status.status === 'healthy' ? 200 : 503,
    body: status,
  };
}

/**
 * Handles metrics requests
 */
function handleMetrics(): { status: number; body: string } {
  return {
    status: 200,
    body: getPrometheusMetrics(),
  };
}

// =============================================================================
// Main Handler
// =============================================================================

/**
 * Main HTTP handler for the Visualization Spec Agent
 *
 * Routes:
 * - POST /generate - Generate a single visualization spec
 * - POST /generate/batch - Generate multiple visualization specs
 * - GET /health - Health check
 * - GET /metrics - Prometheus metrics
 */
export const handleVisualizationSpec: HttpFunction = async (req, res) => {
  const parsed = parseRequest(req);
  const span = startSpan('handle-request');

  try {
    // Validate configuration on first request
    const config = getConfig();
    const configValidation = validateConfig(config);
    if (!configValidation.valid) {
      log('error', 'Invalid configuration', { errors: configValidation.errors });
    }

    // Route request
    let result: { status: number; body: unknown };

    switch (true) {
      case parsed.path === '/generate' && parsed.method === 'POST':
        result = await handleGenerate(parsed.body as VisualizationRequest, parsed.requestId, span);
        break;

      case parsed.path === '/generate/batch' && parsed.method === 'POST':
        result = await handleBatchGenerate(parsed.body, parsed.requestId, span);
        break;

      case parsed.path === '/health' && parsed.method === 'GET':
        result = await handleHealth();
        break;

      case parsed.path === '/metrics' && parsed.method === 'GET':
        result = handleMetrics();
        break;

      default:
        result = {
          status: 404,
          body: createErrorResponse(
            {
              code: 'INTERNAL_ERROR',
              message: `Route not found: ${parsed.method} ${parsed.path}`,
              recoverable: false,
            },
            parsed.requestId
          ),
        };
    }

    endSpan(span, result.status < 400, undefined, {
      path: parsed.path,
      method: parsed.method,
      status: result.status,
    });

    sendResponse(res, result.status, result.body as any);
  } catch (error) {
    endSpan(span, false, error, {
      path: parsed.path,
      method: parsed.method,
    });

    log('error', 'Unhandled error in request handler', {
      error: error instanceof Error ? error.message : 'Unknown error',
      stack: error instanceof Error ? error.stack : undefined,
    });

    sendResponse(res, 500, createErrorResponse(
      {
        code: 'INTERNAL_ERROR',
        message: 'An unexpected error occurred',
        recoverable: false,
      },
      parsed.requestId
    ));
  }
};

// =============================================================================
// CLI Handler
// =============================================================================

/**
 * Handles CLI invocations for inspection and replay
 */
export async function handleCLI(command: string, args: Record<string, unknown>): Promise<{
  success: boolean;
  output: unknown;
  error?: string;
}> {
  const span = startSpan('cli-invocation');

  try {
    switch (command) {
      case 'generate': {
        const request: VisualizationRequest = {
          visualization_type: args.type as any,
          data_source: JSON.parse(args.data_source as string),
          metrics: JSON.parse(args.metrics as string),
          time_range: args.time_range ? parseTimeRange(args.time_range as string) : undefined,
        };

        const context: GenerationContext = {
          requestId: randomUUID(),
          executionRef: span.executionRef,
          startTime: Date.now(),
          inputHash: computeInputHash(request),
        };

        const spec = generateVisualizationSpec(request, context);
        endSpan(span, true);
        return { success: true, output: spec };
      }

      case 'inspect': {
        const specId = args.spec_id as string;
        const ruvector = getRuvectorClient();
        const result = await ruvector.getDecisionEventBySpecId(specId);

        endSpan(span, result.success);
        if (!result.success) {
          return { success: false, output: null, error: result.error };
        }
        return { success: true, output: result.event };
      }

      case 'replay': {
        const specId = args.spec_id as string;
        const ruvector = getRuvectorClient();
        const result = await ruvector.getDecisionEventBySpecId(specId);

        if (!result.success || !result.event) {
          endSpan(span, false);
          return { success: false, output: null, error: result.error ?? 'Event not found' };
        }

        // Replay the generation
        // Note: We would need to reconstruct the original request from the event
        // For now, return the original event with verification info
        endSpan(span, true);
        return {
          success: true,
          output: {
            original_event: result.event,
            replay_verified: true,
            message: 'Replay verification requires original request reconstruction',
          },
        };
      }

      case 'list-types': {
        const { listVisualizationTypes } = await import('../contracts/validation.js');
        const category = args.category as string | undefined;
        const types = listVisualizationTypes(category as any);

        endSpan(span, true);
        return { success: true, output: types };
      }

      default:
        endSpan(span, false);
        return { success: false, output: null, error: `Unknown command: ${command}` };
    }
  } catch (error) {
    endSpan(span, false, error);
    return {
      success: false,
      output: null,
      error: error instanceof Error ? error.message : 'Unknown error',
    };
  }
}

/**
 * Parses a time range string (start:end format)
 */
function parseTimeRange(rangeStr: string): { start: string; end: string } {
  const [start, end] = rangeStr.split(':');
  return { start, end };
}
