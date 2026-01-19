/**
 * Main Edge Function handler for telemetry ingestion
 * Deployed as Google Cloud Function
 * Copyright 2025 LLM Observatory Contributors
 * SPDX-License-Identifier: Apache-2.0
 */

import { normalizeEvent, normalizeEvents, ValidationEventError } from './normalizer.js';
import { createDecisionEvent, addSelfObservation } from './emitter.js';
import {
  startSpan,
  endSpan,
  emitAgentTelemetry,
  getAgentMetrics,
  logger,
} from './telemetry.js';
import { RuvectorClient } from './ruvector-client.js';
import { loadConfig, validateConfig } from './config.js';
import {
  TelemetryEvent,
  TelemetryIngestionResponse,
  NormalizedTelemetry,
} from './types/schemas.js';

/**
 * Global configuration and client (initialized on cold start)
 */
let ruvectorClient: RuvectorClient | null = null;

/**
 * Initialize ruvector client
 */
function initializeClient(): RuvectorClient {
  if (ruvectorClient) {
    return ruvectorClient;
  }

  const config = loadConfig();
  const errors = validateConfig(config);

  if (errors.length > 0) {
    logger.error('Configuration validation failed', { errors });
    throw new Error(`Invalid configuration: ${errors.join(', ')}`);
  }

  ruvectorClient = new RuvectorClient(config.ruvector);
  logger.info('Ruvector client initialized', {
    endpoint: config.ruvector.endpoint,
  });

  return ruvectorClient;
}

/**
 * Main HTTP handler for Cloud Functions
 * Handles POST /ingest with telemetry events
 */
export async function handleTelemetryIngestion(req: Request): Promise<Response> {
  const span = startSpan('telemetry-ingestion');
  const processingStartTime = Date.now();

  try {
    // Validate HTTP method
    if (req.method !== 'POST') {
      return createErrorResponse(405, 'Method not allowed. Use POST.', span.executionRef);
    }

    // Parse request body
    let body: any;
    try {
      body = await req.json();
    } catch (error) {
      return createErrorResponse(
        400,
        'Invalid JSON body',
        span.executionRef,
        error instanceof Error ? error.message : String(error)
      );
    }

    // Support both single event and batch
    const events: TelemetryEvent[] = Array.isArray(body) ? body : [body];

    if (events.length === 0) {
      return createErrorResponse(400, 'No events provided', span.executionRef);
    }

    // Validate batch size
    const maxBatchSize = 100;
    if (events.length > maxBatchSize) {
      return createErrorResponse(
        413,
        `Batch size exceeds maximum of ${maxBatchSize}`,
        span.executionRef
      );
    }

    // Normalize events
    const { normalized, errors: normalizationErrors } = normalizeEvents(events);

    // Initialize ruvector client
    const client = initializeClient();

    // Create decision event
    const processingTimeMs = Date.now() - processingStartTime;
    let decisionEvent = createDecisionEvent(events, normalized, processingTimeMs);

    // Add self-observation if enabled
    const config = loadConfig();
    if (config.selfObservationEnabled) {
      const agentMetrics = getAgentMetrics();
      decisionEvent = addSelfObservation(decisionEvent, agentMetrics);
    }

    // Persist decision event to ruvector-service (async, non-blocking)
    const persistResult = await client.persistDecisionEvent(decisionEvent);

    if (!persistResult.success) {
      logger.warn('Failed to persist decision event', {
        error: persistResult.error,
        retries: persistResult.retries,
        executionRef: span.executionRef,
      });
    }

    // Emit self-observation telemetry
    if (config.selfObservationEnabled) {
      const agentTelemetry = emitAgentTelemetry(span.executionRef);
      // Fire and forget - don't await
      client.persistDecisionEvent(agentTelemetry).catch((error) => {
        logger.warn('Failed to persist agent telemetry', {
          error: error instanceof Error ? error.message : String(error),
        });
      });
    }

    // Record success
    endSpan(span, true);

    // Create response
    const response: TelemetryIngestionResponse = {
      success: true,
      processed: normalized.length,
      failed: normalizationErrors.length,
      eventIds: normalized.map((n) => n.eventId),
      errors: normalizationErrors.length > 0 ? normalizationErrors : undefined,
      executionRef: span.executionRef,
      processingTimeMs: Date.now() - processingStartTime,
    };

    return new Response(JSON.stringify(response), {
      status: 200,
      headers: {
        'Content-Type': 'application/json',
        'X-Execution-Ref': span.executionRef,
        'X-Processing-Time-Ms': String(response.processingTimeMs),
      },
    });
  } catch (error) {
    // Record error
    endSpan(span, false);

    logger.error('Telemetry ingestion failed', {
      error: error instanceof Error ? error.message : String(error),
      stack: error instanceof Error ? error.stack : undefined,
      executionRef: span.executionRef,
    });

    if (error instanceof ValidationEventError) {
      return createErrorResponse(
        400,
        'Validation failed',
        span.executionRef,
        JSON.stringify(error.errors)
      );
    }

    return createErrorResponse(
      500,
      'Internal server error',
      span.executionRef,
      error instanceof Error ? error.message : String(error)
    );
  }
}

/**
 * Health check endpoint handler
 */
export async function handleHealthCheck(req: Request): Promise<Response> {
  try {
    const client = initializeClient();
    const health = await client.healthCheck();

    const agentMetrics = getAgentMetrics();

    return new Response(
      JSON.stringify({
        status: health.healthy ? 'healthy' : 'unhealthy',
        agent: {
          id: 'telemetry-collector-agent',
          version: '1.0.0',
          uptime: agentMetrics.uptimeSeconds,
        },
        ruvector: {
          healthy: health.healthy,
          endpoint: health.endpoint,
          latencyMs: health.latencyMs,
          error: health.error,
        },
        metrics: agentMetrics,
        timestamp: new Date().toISOString(),
      }),
      {
        status: health.healthy ? 200 : 503,
        headers: {
          'Content-Type': 'application/json',
        },
      }
    );
  } catch (error) {
    return new Response(
      JSON.stringify({
        status: 'unhealthy',
        error: error instanceof Error ? error.message : String(error),
        timestamp: new Date().toISOString(),
      }),
      {
        status: 503,
        headers: {
          'Content-Type': 'application/json',
        },
      }
    );
  }
}

/**
 * Create error response
 */
function createErrorResponse(
  status: number,
  message: string,
  executionRef: string,
  details?: string
): Response {
  return new Response(
    JSON.stringify({
      success: false,
      error: message,
      details,
      executionRef,
      timestamp: new Date().toISOString(),
    }),
    {
      status,
      headers: {
        'Content-Type': 'application/json',
        'X-Execution-Ref': executionRef,
      },
    }
  );
}

/**
 * CORS preflight handler
 */
export function handleCORS(req: Request): Response {
  return new Response(null, {
    status: 204,
    headers: {
      'Access-Control-Allow-Origin': '*',
      'Access-Control-Allow-Methods': 'GET, POST, OPTIONS',
      'Access-Control-Allow-Headers': 'Content-Type, Authorization',
      'Access-Control-Max-Age': '86400',
    },
  });
}
