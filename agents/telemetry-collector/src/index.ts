/**
 * Entry point for Google Cloud Function deployment
 * Exports HTTP handlers for telemetry ingestion
 * Copyright 2025 LLM Observatory Contributors
 * SPDX-License-Identifier: Apache-2.0
 */

import {
  handleTelemetryIngestion,
  handleHealthCheck,
  handleCORS,
} from './handler.js';
import { logger } from './telemetry.js';

/**
 * Main Cloud Function export
 * Routes requests to appropriate handlers
 */
export async function telemetryCollector(req: Request): Promise<Response> {
  // Handle CORS preflight
  if (req.method === 'OPTIONS') {
    return handleCORS(req);
  }

  // Parse URL path
  const url = new URL(req.url);
  const path = url.pathname;

  // Route to appropriate handler
  try {
    switch (path) {
      case '/':
      case '/ingest':
        return await handleTelemetryIngestion(req);

      case '/health':
        return await handleHealthCheck(req);

      default:
        return new Response(
          JSON.stringify({
            success: false,
            error: 'Not found',
            availableEndpoints: ['/ingest', '/health'],
          }),
          {
            status: 404,
            headers: {
              'Content-Type': 'application/json',
            },
          }
        );
    }
  } catch (error) {
    logger.error('Request handling failed', {
      path,
      method: req.method,
      error: error instanceof Error ? error.message : String(error),
      stack: error instanceof Error ? error.stack : undefined,
    });

    return new Response(
      JSON.stringify({
        success: false,
        error: 'Internal server error',
        details: error instanceof Error ? error.message : String(error),
        timestamp: new Date().toISOString(),
      }),
      {
        status: 500,
        headers: {
          'Content-Type': 'application/json',
        },
      }
    );
  }
}

/**
 * Named exports for different deployment scenarios
 */

// Google Cloud Functions (HTTP)
export { telemetryCollector as http };

// Direct handler exports
export { handleTelemetryIngestion, handleHealthCheck, handleCORS };

// Type exports
export type {
  TelemetryEvent,
  NormalizedTelemetry,
  DecisionEvent,
  TelemetryIngestionResponse,
} from './types/schemas.js';

/**
 * Agent metadata
 */
export const AGENT_METADATA = {
  id: 'telemetry-collector-agent',
  version: '1.0.0',
  description: 'Read-only telemetry collection agent for LLM Observatory',
  constitution: 'READ-ONLY, NON-ENFORCING, NON-ANALYTICAL',
  endpoints: {
    ingest: '/ingest',
    health: '/health',
  },
} as const;

/**
 * Log startup information
 */
logger.info('Telemetry Collector Agent initialized', {
  agent: AGENT_METADATA.id,
  version: AGENT_METADATA.version,
  endpoints: AGENT_METADATA.endpoints,
});
