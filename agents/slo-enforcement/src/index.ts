/**
 * SLO/SLA Enforcement Agent - Entry Point
 *
 * Google Cloud Edge Function entry point for the SLO/SLA Enforcement Agent.
 *
 * Classification: ENFORCEMENT-CLASS, NON-ACTUATING
 *
 * This agent:
 * - Detects violations of defined SLOs and SLAs
 * - Evaluates telemetry metrics against policy-defined thresholds
 * - Detects breaches and near-breaches
 * - Emits structured violation events
 * - Persists violation history for governance and audit
 *
 * This agent MUST NOT:
 * - Trigger alerts directly
 * - Initiate remediation
 * - Change policies or thresholds at runtime
 * - Modify system state in any way
 *
 * Primary consumers:
 * - LLM-Governance-Dashboard
 * - LLM-Policy-Engine
 * - Incident review workflows
 */

import type { Request, Response } from 'express';
import {
  handleEnforce,
  handleEnforceBatch,
  handleQueryViolations,
  handleAnalysis,
  handleHealth,
  handleReplay,
} from './handler';
import { startMetricsLogger } from './telemetry';
import { loadConfig, validateConfig } from './config';
import { AGENT_METADATA } from '../contracts';

// Validate configuration on startup
const config = loadConfig();
const configErrors = validateConfig(config);
if (configErrors.length > 0) {
  console.error('Configuration errors:', configErrors);
  process.exit(1);
}

// Start metrics logger
startMetricsLogger();

/**
 * Main Cloud Function handler
 *
 * Routes requests to appropriate handlers based on path and method.
 */
export async function handleSloEnforcement(req: Request, res: Response): Promise<void> {
  // CORS headers
  res.set('Access-Control-Allow-Origin', '*');
  res.set('Access-Control-Allow-Methods', 'GET, POST, OPTIONS');
  res.set('Access-Control-Allow-Headers', 'Content-Type, X-Correlation-ID, Authorization');

  // Handle preflight
  if (req.method === 'OPTIONS') {
    res.status(204).send('');
    return;
  }

  // Add agent identification header
  res.set('X-Agent-ID', AGENT_METADATA.id);
  res.set('X-Agent-Version', AGENT_METADATA.version);

  // Route based on path and method
  const path = req.path.replace(/\/$/, ''); // Remove trailing slash

  try {
    switch (true) {
      // POST /enforce - Single enforcement
      case path === '/enforce' && req.method === 'POST':
        await handleEnforce(req, res);
        break;

      // POST /enforce/batch - Batch enforcement
      case path === '/enforce/batch' && req.method === 'POST':
        await handleEnforceBatch(req, res);
        break;

      // GET /violations - Query violations
      case path === '/violations' && req.method === 'GET':
        await handleQueryViolations(req, res);
        break;

      // GET /analysis - Get analysis
      case path === '/analysis' && req.method === 'GET':
        await handleAnalysis(req, res);
        break;

      // GET /health - Health check
      case path === '/health' && req.method === 'GET':
        await handleHealth(req, res);
        break;

      // GET /replay/:executionRef - Replay/inspect decision
      case path.startsWith('/replay/') && req.method === 'GET':
        req.params = { executionRef: path.split('/')[2] };
        await handleReplay(req, res);
        break;

      // Root path - Agent info
      case path === '' || path === '/':
        res.status(200).json({
          agent_id: AGENT_METADATA.id,
          agent_version: AGENT_METADATA.version,
          classification: AGENT_METADATA.classification,
          decision_type: AGENT_METADATA.decision_type,
          actuating: AGENT_METADATA.actuating,
          endpoints: [
            { method: 'POST', path: '/enforce', description: 'Evaluate SLOs against metrics' },
            { method: 'POST', path: '/enforce/batch', description: 'Batch evaluation' },
            { method: 'GET', path: '/violations', description: 'Query violations' },
            { method: 'GET', path: '/analysis', description: 'Get aggregated analysis' },
            { method: 'GET', path: '/health', description: 'Health check' },
            { method: 'GET', path: '/replay/:id', description: 'Replay/inspect decision' },
          ],
          non_responsibilities: [
            'Triggering alerts directly',
            'Initiating remediation',
            'Changing policies or thresholds at runtime',
            'Modifying system state',
          ],
          primary_consumers: [
            'LLM-Governance-Dashboard',
            'LLM-Policy-Engine',
            'Incident review workflows',
          ],
        });
        break;

      // Unknown route
      default:
        res.status(404).json({
          success: false,
          error: {
            code: 'NOT_FOUND',
            message: `Unknown endpoint: ${req.method} ${path}`,
          },
        });
    }
  } catch (error) {
    console.error('Unhandled error:', error);
    res.status(500).json({
      success: false,
      error: {
        code: 'INTERNAL_ERROR',
        message: (error as Error).message,
      },
    });
  }
}

// Export for Google Cloud Functions
export { handleSloEnforcement as sloEnforcement };

// Export handlers for direct testing
export {
  handleEnforce,
  handleEnforceBatch,
  handleQueryViolations,
  handleAnalysis,
  handleHealth,
  handleReplay,
};
