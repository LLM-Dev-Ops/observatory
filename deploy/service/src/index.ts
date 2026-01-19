/**
 * LLM Observatory - Unified Cloud Run Service
 *
 * Single entry point routing to all Observatory agents.
 * OBSERVATION-ONLY: Does NOT execute workflows, trigger remediation, or own a database.
 * All persistence via ruvector-service client calls only.
 *
 * Copyright 2025 LLM Observatory Contributors
 * SPDX-License-Identifier: Apache-2.0
 */

import express, { Request, Response, NextFunction } from 'express';
import { createRequire } from 'module';
const require = createRequire(import.meta.url);
const pino = require('pino');
const pinoHttp = require('pino-http');
import { loadConfig, validateEnvironment } from './config.js';
import { createAgentRouter } from './router.js';
import { healthCheck, readinessCheck } from './health.js';

// ============================================================================
// CONFIGURATION
// ============================================================================

const config = loadConfig();
const logger = pino({
  level: config.logLevel,
  name: config.serviceName,
});

// Validate environment on startup
const envErrors = validateEnvironment();
if (envErrors.length > 0) {
  logger.error({ errors: envErrors }, 'Environment validation failed');
  process.exit(1);
}

// ============================================================================
// EXPRESS APPLICATION
// ============================================================================

const app = express();

// Middleware
app.use(express.json({ limit: '10mb' }));
app.use(pinoHttp({ logger }));

// CORS headers
app.use((req: Request, res: Response, next: NextFunction) => {
  res.header('Access-Control-Allow-Origin', '*');
  res.header('Access-Control-Allow-Methods', 'GET, POST, OPTIONS');
  res.header('Access-Control-Allow-Headers', 'Content-Type, Authorization, X-Correlation-ID');
  if (req.method === 'OPTIONS') {
    return res.status(204).end();
  }
  next();
});

// ============================================================================
// HEALTH ENDPOINTS (Cloud Run requirements)
// ============================================================================

app.get('/health', async (req: Request, res: Response) => {
  const health = await healthCheck();
  res.status(health.status === 'healthy' ? 200 : 503).json(health);
});

app.get('/ready', async (req: Request, res: Response) => {
  const ready = await readinessCheck();
  res.status(ready.ready ? 200 : 503).json(ready);
});

app.get('/', (req: Request, res: Response) => {
  res.json({
    service: config.serviceName,
    version: config.serviceVersion,
    environment: config.environment,
    classification: 'OBSERVATION-ONLY',
    endpoints: {
      health: '/health',
      ready: '/ready',
      agents: {
        telemetry: '/api/v1/telemetry/*',
        usage: '/api/v1/usage/*',
        failure: '/api/v1/failure/*',
        health: '/api/v1/health-check/*',
        slo: '/api/v1/slo/*',
        postmortem: '/api/v1/postmortem/*',
        visualization: '/api/v1/visualization/*',
      },
    },
    timestamp: new Date().toISOString(),
  });
});

// ============================================================================
// AGENT ROUTES
// ============================================================================

// Mount agent router at /api/v1
app.use('/api/v1', createAgentRouter(logger));

// ============================================================================
// ERROR HANDLING
// ============================================================================

app.use((err: Error, req: Request, res: Response, next: NextFunction) => {
  logger.error({ err, path: req.path, method: req.method }, 'Unhandled error');
  res.status(500).json({
    success: false,
    error: {
      code: 'INTERNAL_ERROR',
      message: 'Internal server error',
      timestamp: new Date().toISOString(),
    },
  });
});

// 404 handler
app.use((req: Request, res: Response) => {
  res.status(404).json({
    success: false,
    error: {
      code: 'NOT_FOUND',
      message: `Endpoint ${req.method} ${req.path} not found`,
      timestamp: new Date().toISOString(),
    },
  });
});

// ============================================================================
// SERVER STARTUP
// ============================================================================

const PORT = parseInt(process.env.PORT || '8080', 10);

app.listen(PORT, '0.0.0.0', () => {
  logger.info({
    port: PORT,
    service: config.serviceName,
    version: config.serviceVersion,
    environment: config.environment,
    ruvectorEndpoint: config.ruvectorServiceUrl,
  }, 'LLM Observatory unified service started');
});

// Graceful shutdown
process.on('SIGTERM', () => {
  logger.info('SIGTERM received, shutting down gracefully');
  process.exit(0);
});

process.on('SIGINT', () => {
  logger.info('SIGINT received, shutting down gracefully');
  process.exit(0);
});

export { app };
