/**
 * Edge Function Handler for Usage Pattern Agent.
 *
 * CONSTITUTION:
 * - Google Cloud Edge Function deployment
 * - Stateless execution
 * - Deterministic behavior
 * - No orchestration logic
 * - No remediation logic
 * - No policy enforcement
 * - No alert triggering
 * - No direct SQL access
 * - Async, non-blocking writes to ruvector-service only
 *
 * This handler MAY:
 * - Ingest telemetry
 * - Aggregate telemetry
 * - Classify telemetry
 * - Analyze telemetry
 * - Generate summaries or specifications
 *
 * This handler MUST NOT:
 * - Modify system state
 * - Influence live execution
 */

import { v4 as uuidv4 } from 'uuid';
import { loadConfig, validateConfig, AgentConfig } from './config.js';
import { RuvectorClient } from './ruvector-client.js';
import { UsagePatternAnalyzer } from './analyzer.js';
import { DecisionEventEmitter } from './decision-emitter.js';
import {
  AnalysisRequest,
  AnalysisRequestSchema,
  UsagePatternAnalysis,
  ErrorResponse,
  ErrorCode,
} from '../contracts/schemas.js';

/**
 * HTTP Request interface for Edge Function.
 */
export interface EdgeRequest {
  method: string;
  url: string;
  headers: Record<string, string>;
  body?: unknown;
}

/**
 * HTTP Response interface for Edge Function.
 */
export interface EdgeResponse {
  status: number;
  headers: Record<string, string>;
  body: unknown;
}

/**
 * Context provided by the Edge Function runtime.
 */
export interface EdgeContext {
  requestId: string;
  timestamp: Date;
  region?: string;
}

/**
 * Main Edge Function handler.
 *
 * CONSTITUTION: This is the entry point for the Usage Pattern Agent.
 * - Stateless execution
 * - Deterministic behavior
 * - READ-ONLY and ADVISORY only
 */
export async function handleRequest(
  request: EdgeRequest,
  context?: EdgeContext
): Promise<EdgeResponse> {
  const executionRef = context?.requestId || uuidv4();
  const startTime = Date.now();

  // Load and validate configuration
  const config = loadConfig();
  const configErrors = validateConfig(config);

  if (configErrors.length > 0) {
    return createErrorResponse(
      'INTERNAL_ERROR',
      `Configuration validation failed: ${configErrors.join(', ')}`,
      executionRef
    );
  }

  // Initialize services
  const ruvectorClient = new RuvectorClient(config.ruvector);
  const analyzer = new UsagePatternAnalyzer(config, ruvectorClient);
  const emitter = new DecisionEventEmitter(config, ruvectorClient);

  try {
    // Route based on HTTP method and path
    const path = new URL(request.url, 'http://localhost').pathname;

    if (request.method === 'POST' && path === '/analyze') {
      return await handleAnalyze(request, config, analyzer, emitter, executionRef, startTime);
    }

    if (request.method === 'GET' && path === '/health') {
      return await handleHealth(ruvectorClient, config);
    }

    if (request.method === 'GET' && path === '/status') {
      return handleStatus(config);
    }

    if (request.method === 'GET' && path.startsWith('/analysis/')) {
      const analysisId = path.split('/')[2];
      return await handleGetAnalysis(analysisId, ruvectorClient, executionRef);
    }

    return createErrorResponse(
      'INVALID_INPUT',
      `Unknown endpoint: ${request.method} ${path}`,
      executionRef
    );
  } catch (error) {
    return createErrorResponse(
      'INTERNAL_ERROR',
      error instanceof Error ? error.message : String(error),
      executionRef
    );
  }
}

/**
 * Handle POST /analyze - Main analysis endpoint.
 *
 * CONSTITUTION: This is the core analytical operation.
 * - Consumes normalized telemetry events
 * - Performs statistical aggregation
 * - Produces analytical summaries
 * - Persists DecisionEvent to ruvector-service
 */
async function handleAnalyze(
  request: EdgeRequest,
  config: AgentConfig,
  analyzer: UsagePatternAnalyzer,
  emitter: DecisionEventEmitter,
  executionRef: string,
  startTime: number
): Promise<EdgeResponse> {
  // Parse and validate request
  if (!request.body) {
    return createErrorResponse('INVALID_INPUT', 'Request body is required', executionRef);
  }

  const parseResult = AnalysisRequestSchema.safeParse(request.body);
  if (!parseResult.success) {
    return createErrorResponse(
      'VALIDATION_FAILED',
      `Invalid request: ${parseResult.error.message}`,
      executionRef
    );
  }

  const analysisRequest = parseResult.data;

  // Validate time window
  const start = new Date(analysisRequest.time_window.start);
  const end = new Date(analysisRequest.time_window.end);
  const daysDiff = (end.getTime() - start.getTime()) / (1000 * 60 * 60 * 24);

  if (daysDiff > config.maxTimeWindowDays) {
    return createErrorResponse(
      'TIME_WINDOW_TOO_LARGE',
      `Time window exceeds maximum of ${config.maxTimeWindowDays} days`,
      executionRef
    );
  }

  try {
    // Perform analysis
    const analysis = await analyzer.analyze(analysisRequest);

    // Check for minimum data
    if (analysis.sample_size === 0) {
      return createErrorResponse(
        'INSUFFICIENT_DATA',
        'No telemetry events found in the specified time window',
        executionRef
      );
    }

    const processingTimeMs = Date.now() - startTime;

    // Emit DecisionEvent to ruvector-service
    const inputsHash = analyzer.computeInputsHash(analysisRequest);
    const emitResult = await emitter.emit({
      analysis,
      inputsHash,
      executionRef,
      processingTimeMs,
      eventsAnalyzed: analysis.sample_size,
    });

    if (!emitResult.success) {
      // Log warning but don't fail the request
      console.warn(`Failed to emit DecisionEvent: ${emitResult.error}`);
    }

    // Return analysis result
    return {
      status: 200,
      headers: {
        'Content-Type': 'application/json',
        'X-Execution-Ref': executionRef,
        'X-Processing-Time-Ms': processingTimeMs.toString(),
        'X-Decision-Event-Id': emitResult.eventId || '',
      },
      body: {
        success: true,
        data: analysis,
        metadata: {
          execution_ref: executionRef,
          processing_time_ms: processingTimeMs,
          decision_event_persisted: emitResult.success,
        },
      },
    };
  } catch (error) {
    if ((error as Error).message?.includes('timeout')) {
      return createErrorResponse('ANALYSIS_TIMEOUT', 'Analysis timed out', executionRef);
    }

    if ((error as Error).message?.includes('connection')) {
      return createErrorResponse('RUVECTOR_CONNECTION_ERROR', 'Failed to connect to ruvector-service', executionRef);
    }

    throw error;
  }
}

/**
 * Handle GET /health - Health check endpoint.
 */
async function handleHealth(
  client: RuvectorClient,
  config: AgentConfig
): Promise<EdgeResponse> {
  const healthStatus = await client.healthCheck();

  return {
    status: healthStatus.healthy ? 200 : 503,
    headers: { 'Content-Type': 'application/json' },
    body: {
      status: healthStatus.healthy ? 'healthy' : 'unhealthy',
      agent: {
        id: config.agentId,
        version: config.agentVersion,
        classification: 'advisory',
      },
      dependencies: {
        ruvector: {
          healthy: healthStatus.healthy,
          latency_ms: healthStatus.latencyMs,
          endpoint: healthStatus.endpoint,
          error: healthStatus.error,
        },
      },
      timestamp: new Date().toISOString(),
    },
  };
}

/**
 * Handle GET /status - Agent status endpoint.
 */
function handleStatus(config: AgentConfig): EdgeResponse {
  return {
    status: 200,
    headers: { 'Content-Type': 'application/json' },
    body: {
      agent_id: config.agentId,
      agent_version: config.agentVersion,
      classification: 'advisory',
      decision_type: 'usage_pattern_analysis',
      capabilities: [
        'telemetry_aggregation',
        'trend_analysis',
        'seasonality_detection',
        'distribution_statistics',
        'provider_usage_breakdown',
        'hotspot_identification',
        'growth_pattern_analysis',
      ],
      constraints: {
        max_events_per_analysis: config.maxEventsPerAnalysis,
        max_time_window_days: config.maxTimeWindowDays,
        min_sample_size_for_trends: config.minSampleSizeForTrends,
        min_sample_size_for_seasonality: config.minSampleSizeForSeasonality,
      },
      constitution: {
        read_only: true,
        advisory: true,
        constraints_applied: false,
        can_classify_failures: false,
        can_evaluate_health: false,
        can_enforce_thresholds: false,
        can_generate_alerts: false,
        can_trigger_orchestration: false,
        can_modify_system_state: false,
      },
      timestamp: new Date().toISOString(),
    },
  };
}

/**
 * Handle GET /analysis/:id - Get historical analysis.
 */
async function handleGetAnalysis(
  analysisId: string,
  client: RuvectorClient,
  executionRef: string
): Promise<EdgeResponse> {
  try {
    const events = await client.getDecisionEvents({
      agentId: 'usage-pattern-agent',
      decisionType: 'usage_pattern_analysis',
      limit: 1,
    });

    // Find the analysis by ID in the outputs
    for (const event of events) {
      const eventObj = event as { outputs?: Array<{ analysis_id?: string }> };
      if (eventObj.outputs) {
        for (const output of eventObj.outputs) {
          if (output.analysis_id === analysisId) {
            return {
              status: 200,
              headers: { 'Content-Type': 'application/json' },
              body: {
                success: true,
                data: output,
              },
            };
          }
        }
      }
    }

    return createErrorResponse(
      'INVALID_INPUT',
      `Analysis not found: ${analysisId}`,
      executionRef
    );
  } catch (error) {
    return createErrorResponse(
      'RUVECTOR_CONNECTION_ERROR',
      `Failed to retrieve analysis: ${error instanceof Error ? error.message : String(error)}`,
      executionRef
    );
  }
}

/**
 * Create standardized error response.
 */
function createErrorResponse(
  code: ErrorCode,
  message: string,
  executionRef: string
): EdgeResponse {
  const errorResponse: ErrorResponse = {
    error: {
      code,
      message,
      timestamp: new Date().toISOString(),
      execution_ref: executionRef,
    },
  };

  const statusMap: Record<ErrorCode, number> = {
    INVALID_INPUT: 400,
    VALIDATION_FAILED: 400,
    INSUFFICIENT_DATA: 422,
    TIME_WINDOW_TOO_LARGE: 400,
    RUVECTOR_CONNECTION_ERROR: 503,
    ANALYSIS_TIMEOUT: 504,
    INTERNAL_ERROR: 500,
  };

  return {
    status: statusMap[code] || 500,
    headers: {
      'Content-Type': 'application/json',
      'X-Execution-Ref': executionRef,
    },
    body: errorResponse,
  };
}

/**
 * Export for Google Cloud Functions.
 */
export const usagePatternAnalyzer = async (req: unknown, res: unknown): Promise<void> => {
  // Convert Cloud Functions request/response to Edge format
  const request = req as { method: string; url: string; headers: Record<string, string>; body: unknown };
  const response = res as {
    status: (code: number) => { set: (headers: Record<string, string>) => { json: (body: unknown) => void } };
  };

  const edgeRequest: EdgeRequest = {
    method: request.method,
    url: request.url,
    headers: request.headers,
    body: request.body,
  };

  const edgeResponse = await handleRequest(edgeRequest);

  response
    .status(edgeResponse.status)
    .set(edgeResponse.headers)
    .json(edgeResponse.body);
};
