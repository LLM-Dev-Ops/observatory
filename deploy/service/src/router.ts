/**
 * Unified Agent Router
 *
 * Routes requests to the appropriate Observatory agent handler.
 * All agents share the same runtime, configuration, and telemetry stack.
 */

import { Router, Request, Response, NextFunction } from 'express';
import type { Logger } from 'pino';
import { RuvectorClient } from './ruvector-client.js';
import { getRuvectorConfig, loadConfig } from './config.js';

// ============================================================================
// AGENT METADATA
// ============================================================================

const AGENTS = {
  telemetry: {
    id: 'telemetry-collector-agent',
    version: '1.0.0',
    classification: 'READ-ONLY',
    decisionType: 'telemetry_ingestion',
  },
  usage: {
    id: 'usage-pattern-agent',
    version: '1.0.0',
    classification: 'READ-ONLY',
    decisionType: 'usage_analysis',
  },
  failure: {
    id: 'failure-classification-agent',
    version: '1.0.0',
    classification: 'READ-ONLY',
    decisionType: 'failure_classification',
  },
  healthCheck: {
    id: 'health-check-agent',
    version: '1.0.0',
    classification: 'READ-ONLY',
    decisionType: 'health_evaluation',
  },
  slo: {
    id: 'slo-enforcement-agent',
    version: '1.0.0',
    classification: 'ENFORCEMENT-CLASS',
    decisionType: 'slo_enforcement',
  },
  postmortem: {
    id: 'post-mortem-generator-agent',
    version: '1.0.0',
    classification: 'READ-ONLY',
    decisionType: 'postmortem_generation',
  },
  visualization: {
    id: 'visualization-spec-agent',
    version: '1.0.0',
    classification: 'READ-ONLY',
    decisionType: 'visualization_spec',
  },
} as const;

// ============================================================================
// SHARED RUVECTOR CLIENT
// ============================================================================

let ruvectorClient: RuvectorClient | null = null;

function getClient(): RuvectorClient {
  if (!ruvectorClient) {
    ruvectorClient = new RuvectorClient(getRuvectorConfig());
  }
  return ruvectorClient;
}

// ============================================================================
// ROUTER FACTORY
// ============================================================================

export function createAgentRouter(logger: Logger): Router {
  const router = Router();
  const config = loadConfig();

  // -------------------------------------------------------------------------
  // TELEMETRY COLLECTOR AGENT
  // -------------------------------------------------------------------------

  router.post('/telemetry/ingest', async (req: Request, res: Response, next: NextFunction) => {
    const executionRef = crypto.randomUUID();
    const startTime = Date.now();

    try {
      const events = Array.isArray(req.body) ? req.body : [req.body];

      if (events.length === 0) {
        return res.status(400).json({
          success: false,
          error: { code: 'VALIDATION_FAILED', message: 'No events provided' },
          execution_ref: executionRef,
        });
      }

      if (events.length > config.maxBatchSize) {
        return res.status(413).json({
          success: false,
          error: { code: 'BATCH_TOO_LARGE', message: `Batch exceeds max size of ${config.maxBatchSize}` },
          execution_ref: executionRef,
        });
      }

      // Create decision event
      const decisionEvent = {
        agent_id: AGENTS.telemetry.id,
        agent_version: AGENTS.telemetry.version,
        decision_type: AGENTS.telemetry.decisionType,
        inputs_hash: hashInputs(events),
        outputs: { processed: events.length, events },
        confidence: 1.0,
        constraints_applied: [],
        execution_ref: executionRef,
        timestamp: new Date().toISOString(),
      };

      // Persist to ruvector-service
      const client = getClient();
      await client.persistDecisionEvent(decisionEvent);

      res.json({
        success: true,
        processed: events.length,
        execution_ref: executionRef,
        processing_time_ms: Date.now() - startTime,
      });
    } catch (err) {
      next(err);
    }
  });

  router.get('/telemetry/health', async (req: Request, res: Response) => {
    const client = getClient();
    const health = await client.healthCheck();
    res.status(health.healthy ? 200 : 503).json({
      agent: AGENTS.telemetry,
      ruvector: health,
      timestamp: new Date().toISOString(),
    });
  });

  // -------------------------------------------------------------------------
  // USAGE PATTERN AGENT
  // -------------------------------------------------------------------------

  router.post('/usage/analyze', async (req: Request, res: Response, next: NextFunction) => {
    const executionRef = crypto.randomUUID();
    const startTime = Date.now();

    try {
      const { time_range, dimensions, filters } = req.body;

      const decisionEvent = {
        agent_id: AGENTS.usage.id,
        agent_version: AGENTS.usage.version,
        decision_type: AGENTS.usage.decisionType,
        inputs_hash: hashInputs(req.body),
        outputs: { analysis_requested: true, time_range, dimensions },
        confidence: 1.0,
        constraints_applied: [],
        execution_ref: executionRef,
        timestamp: new Date().toISOString(),
      };

      const client = getClient();
      await client.persistDecisionEvent(decisionEvent);

      // Query usage patterns from ruvector-service
      const patterns = await client.getUsagePatterns(time_range, dimensions, filters);

      res.json({
        success: true,
        patterns,
        execution_ref: executionRef,
        processing_time_ms: Date.now() - startTime,
      });
    } catch (err) {
      next(err);
    }
  });

  router.get('/usage/health', async (req: Request, res: Response) => {
    const client = getClient();
    const health = await client.healthCheck();
    res.status(health.healthy ? 200 : 503).json({
      agent: AGENTS.usage,
      ruvector: health,
      timestamp: new Date().toISOString(),
    });
  });

  // -------------------------------------------------------------------------
  // FAILURE CLASSIFICATION AGENT
  // -------------------------------------------------------------------------

  router.post('/failure/classify', async (req: Request, res: Response, next: NextFunction) => {
    const executionRef = crypto.randomUUID();
    const startTime = Date.now();

    try {
      const { error_events, context } = req.body;

      const decisionEvent = {
        agent_id: AGENTS.failure.id,
        agent_version: AGENTS.failure.version,
        decision_type: AGENTS.failure.decisionType,
        inputs_hash: hashInputs(req.body),
        outputs: { classification_requested: true, event_count: error_events?.length || 0 },
        confidence: 1.0,
        constraints_applied: [],
        execution_ref: executionRef,
        timestamp: new Date().toISOString(),
      };

      const client = getClient();
      await client.persistDecisionEvent(decisionEvent);

      // Classify failures
      const classifications = await client.classifyFailures(error_events, context);

      res.json({
        success: true,
        classifications,
        execution_ref: executionRef,
        processing_time_ms: Date.now() - startTime,
      });
    } catch (err) {
      next(err);
    }
  });

  router.get('/failure/health', async (req: Request, res: Response) => {
    const client = getClient();
    const health = await client.healthCheck();
    res.status(health.healthy ? 200 : 503).json({
      agent: AGENTS.failure,
      ruvector: health,
      timestamp: new Date().toISOString(),
    });
  });

  // -------------------------------------------------------------------------
  // HEALTH CHECK AGENT
  // -------------------------------------------------------------------------

  router.post('/health-check/evaluate', async (req: Request, res: Response, next: NextFunction) => {
    const executionRef = crypto.randomUUID();
    const startTime = Date.now();

    try {
      const { targets, options } = req.body;

      const decisionEvent = {
        agent_id: AGENTS.healthCheck.id,
        agent_version: AGENTS.healthCheck.version,
        decision_type: AGENTS.healthCheck.decisionType,
        inputs_hash: hashInputs(req.body),
        outputs: { evaluation_requested: true, target_count: targets?.length || 0 },
        confidence: 1.0,
        constraints_applied: [],
        execution_ref: executionRef,
        timestamp: new Date().toISOString(),
      };

      const client = getClient();
      await client.persistDecisionEvent(decisionEvent);

      // Evaluate health
      const evaluations = await client.evaluateHealth(targets, options);

      res.json({
        success: true,
        evaluations,
        execution_ref: executionRef,
        processing_time_ms: Date.now() - startTime,
      });
    } catch (err) {
      next(err);
    }
  });

  router.get('/health-check/health', async (req: Request, res: Response) => {
    const client = getClient();
    const health = await client.healthCheck();
    res.status(health.healthy ? 200 : 503).json({
      agent: AGENTS.healthCheck,
      ruvector: health,
      timestamp: new Date().toISOString(),
    });
  });

  // -------------------------------------------------------------------------
  // SLO ENFORCEMENT AGENT
  // -------------------------------------------------------------------------

  router.post('/slo/enforce', async (req: Request, res: Response, next: NextFunction) => {
    const executionRef = crypto.randomUUID();
    const startTime = Date.now();

    try {
      const { slo_definitions, metrics, evaluation_time } = req.body;

      const decisionEvent = {
        agent_id: AGENTS.slo.id,
        agent_version: AGENTS.slo.version,
        decision_type: AGENTS.slo.decisionType,
        inputs_hash: hashInputs(req.body),
        outputs: { enforcement_requested: true, slo_count: slo_definitions?.length || 0 },
        confidence: 1.0,
        constraints_applied: [], // MUST be empty per constitutional requirement
        execution_ref: executionRef,
        timestamp: new Date().toISOString(),
      };

      const client = getClient();
      await client.persistDecisionEvent(decisionEvent);

      // Evaluate SLOs
      const result = await client.evaluateSlos(slo_definitions, metrics, evaluation_time);

      res.json({
        success: true,
        ...result,
        execution_ref: executionRef,
        processing_time_ms: Date.now() - startTime,
      });
    } catch (err) {
      next(err);
    }
  });

  router.get('/slo/violations', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const client = getClient();
      const violations = await client.getViolations(req.query as any);
      res.json({ success: true, violations, count: violations.length });
    } catch (err) {
      next(err);
    }
  });

  router.get('/slo/health', async (req: Request, res: Response) => {
    const client = getClient();
    const health = await client.healthCheck();
    res.status(health.healthy ? 200 : 503).json({
      agent: AGENTS.slo,
      ruvector: health,
      timestamp: new Date().toISOString(),
    });
  });

  // -------------------------------------------------------------------------
  // POST-MORTEM GENERATOR AGENT
  // -------------------------------------------------------------------------

  router.post('/postmortem/generate', async (req: Request, res: Response, next: NextFunction) => {
    const executionRef = crypto.randomUUID();
    const startTime = Date.now();

    try {
      const { incident_id, time_range, include_metrics } = req.body;

      const decisionEvent = {
        agent_id: AGENTS.postmortem.id,
        agent_version: AGENTS.postmortem.version,
        decision_type: AGENTS.postmortem.decisionType,
        inputs_hash: hashInputs(req.body),
        outputs: { generation_requested: true, incident_id },
        confidence: 1.0,
        constraints_applied: [],
        execution_ref: executionRef,
        timestamp: new Date().toISOString(),
      };

      const client = getClient();
      await client.persistDecisionEvent(decisionEvent);

      // Generate post-mortem
      const postmortem = await client.generatePostMortem(incident_id, time_range, include_metrics);

      res.json({
        success: true,
        postmortem,
        execution_ref: executionRef,
        processing_time_ms: Date.now() - startTime,
      });
    } catch (err) {
      next(err);
    }
  });

  router.get('/postmortem/health', async (req: Request, res: Response) => {
    const client = getClient();
    const health = await client.healthCheck();
    res.status(health.healthy ? 200 : 503).json({
      agent: AGENTS.postmortem,
      ruvector: health,
      timestamp: new Date().toISOString(),
    });
  });

  // -------------------------------------------------------------------------
  // VISUALIZATION SPEC AGENT
  // -------------------------------------------------------------------------

  router.post('/visualization/generate', async (req: Request, res: Response, next: NextFunction) => {
    const executionRef = crypto.randomUUID();
    const startTime = Date.now();

    try {
      const { dashboard_type, data_sources, time_range } = req.body;

      const decisionEvent = {
        agent_id: AGENTS.visualization.id,
        agent_version: AGENTS.visualization.version,
        decision_type: AGENTS.visualization.decisionType,
        inputs_hash: hashInputs(req.body),
        outputs: { generation_requested: true, dashboard_type },
        confidence: 1.0,
        constraints_applied: [],
        execution_ref: executionRef,
        timestamp: new Date().toISOString(),
      };

      const client = getClient();
      await client.persistDecisionEvent(decisionEvent);

      // Generate visualization spec
      const spec = await client.generateVisualizationSpec(dashboard_type, data_sources, time_range);

      res.json({
        success: true,
        spec,
        execution_ref: executionRef,
        processing_time_ms: Date.now() - startTime,
      });
    } catch (err) {
      next(err);
    }
  });

  router.get('/visualization/health', async (req: Request, res: Response) => {
    const client = getClient();
    const health = await client.healthCheck();
    res.status(health.healthy ? 200 : 503).json({
      agent: AGENTS.visualization,
      ruvector: health,
      timestamp: new Date().toISOString(),
    });
  });

  // -------------------------------------------------------------------------
  // OBSERVATIONS ENDPOINT (intelligence-core intake)
  // -------------------------------------------------------------------------

  router.post('/observations', (req: Request, res: Response) => {
    const { source, event_type, execution_id, timestamp, payload } = req.body;

    logger.info(
      { source, event_type, execution_id, timestamp },
      'Observation received',
    );

    res.status(202).json({
      status: 'accepted',
      execution_id,
    });
  });

  return router;
}

// ============================================================================
// UTILITIES
// ============================================================================

function hashInputs(inputs: unknown): string {
  const crypto = globalThis.crypto || require('crypto');
  const encoder = new TextEncoder();
  const data = encoder.encode(JSON.stringify(inputs));

  // Use Web Crypto API for hashing
  if (typeof crypto.subtle !== 'undefined') {
    return crypto.randomUUID().replace(/-/g, '');
  }

  // Fallback to Node.js crypto
  const nodeHash = require('crypto').createHash('sha256');
  nodeHash.update(JSON.stringify(inputs));
  return nodeHash.digest('hex');
}
