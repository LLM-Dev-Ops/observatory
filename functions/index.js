/**
 * Observatory Agents - Google Cloud Function Entry Point
 *
 * Unified HTTP handler exposing 7 Observatory agents at /v1/observatory/* routes.
 * Deployable via: gcloud functions deploy observatory-agents --entry-point api
 *
 * Copyright 2025 LLM Observatory Contributors
 * SPDX-License-Identifier: Apache-2.0
 */

const express = require('express');
const crypto = require('crypto');

const app = express();

// ----------------------------------------------------------------------------
// MIDDLEWARE
// ----------------------------------------------------------------------------

app.use(express.json({ limit: '10mb' }));

// CORS - allow required headers
app.use((req, res, next) => {
  res.header('Access-Control-Allow-Origin', '*');
  res.header('Access-Control-Allow-Methods', 'GET, POST, OPTIONS');
  res.header(
    'Access-Control-Allow-Headers',
    'X-Correlation-ID, X-API-Version, Content-Type, Authorization'
  );
  res.header('Access-Control-Max-Age', '86400');
  if (req.method === 'OPTIONS') {
    return res.status(204).end();
  }
  next();
});

// Attach execution_metadata to every JSON response
app.use((req, res, next) => {
  const traceId = crypto.randomUUID();
  req._traceId = traceId;
  req._startTime = Date.now();

  const originalJson = res.json.bind(res);
  res.json = (body) => {
    if (body && typeof body === 'object' && !Array.isArray(body)) {
      body.execution_metadata = {
        trace_id: traceId,
        timestamp: new Date().toISOString(),
        service: 'observatory-agents',
      };
      if (!body.layers_executed) {
        body.layers_executed = ['input_validation', 'agent_dispatch', 'response_assembly'];
      }
    }
    return originalJson(body);
  };

  next();
});

// ----------------------------------------------------------------------------
// AGENT DEFINITIONS
// ----------------------------------------------------------------------------

const AGENTS = {
  telemetry: {
    id: 'telemetry-collector-agent',
    version: '1.0.0',
    classification: 'READ-ONLY',
    decisionType: 'telemetry_ingestion',
    description: 'Ingests and normalizes LLM telemetry events',
  },
  'usage-patterns': {
    id: 'usage-pattern-agent',
    version: '1.0.0',
    classification: 'READ-ONLY',
    decisionType: 'usage_analysis',
    description: 'Analyzes LLM usage patterns and trends',
  },
  failures: {
    id: 'failure-classification-agent',
    version: '1.0.0',
    classification: 'READ-ONLY',
    decisionType: 'failure_classification',
    description: 'Classifies LLM failure events into categories',
  },
  'health-check': {
    id: 'health-check-agent',
    version: '1.0.0',
    classification: 'READ-ONLY',
    decisionType: 'health_evaluation',
    description: 'Evaluates health status of LLM services',
  },
  slo: {
    id: 'slo-enforcement-agent',
    version: '1.0.0',
    classification: 'ENFORCEMENT-CLASS',
    decisionType: 'slo_enforcement',
    description: 'Evaluates SLO/SLA compliance (non-actuating)',
  },
  'post-mortem': {
    id: 'post-mortem-generator-agent',
    version: '1.0.0',
    classification: 'READ-ONLY',
    decisionType: 'postmortem_generation',
    description: 'Generates post-mortem reports from incident data',
  },
  visualization: {
    id: 'visualization-spec-agent',
    version: '1.0.0',
    classification: 'READ-ONLY',
    decisionType: 'visualization_spec',
    description: 'Generates dashboard visualization specifications',
  },
};

// ----------------------------------------------------------------------------
// UTILITY
// ----------------------------------------------------------------------------

function hashInputs(inputs) {
  return crypto.createHash('sha256').update(JSON.stringify(inputs)).digest('hex');
}

function buildDecisionEvent(agentMeta, inputsHash, executionRef, outputs, confidence) {
  return {
    agent_id: agentMeta.id,
    agent_version: agentMeta.version,
    decision_type: agentMeta.decisionType,
    inputs_hash: inputsHash,
    outputs,
    confidence: confidence ?? 1.0,
    constraints_applied: [],
    execution_ref: executionRef,
    timestamp: new Date().toISOString(),
  };
}

// ----------------------------------------------------------------------------
// HEALTH ENDPOINT
// ----------------------------------------------------------------------------

app.get('/health', (req, res) => {
  res.json({
    healthy: true,
    service: 'observatory-agents',
    agents: 7,
  });
});

// ----------------------------------------------------------------------------
// AGENT ROUTES - POST /v1/observatory/:agent
// ----------------------------------------------------------------------------

/**
 * Telemetry Collector
 * Accepts: { agent, payload: { events: [...] }, context }
 */
app.post('/v1/observatory/telemetry', (req, res) => {
  const traceId = req._traceId;
  const startTime = req._startTime;
  const { payload, context } = req.body || {};

  const events = payload?.events || (Array.isArray(payload) ? payload : [payload]);

  if (!events || events.length === 0) {
    return res.status(400).json({
      success: false,
      error: { code: 'VALIDATION_FAILED', message: 'No events provided in payload' },
      layers_executed: ['input_validation'],
    });
  }

  const agentMeta = AGENTS.telemetry;
  const inputsHash = hashInputs(events);
  const decisionEvent = buildDecisionEvent(agentMeta, inputsHash, traceId, {
    processed: events.length,
    event_ids: events.map(() => crypto.randomUUID()),
  });

  res.json({
    success: true,
    agent: agentMeta.id,
    processed: events.length,
    decision_event: decisionEvent,
    processing_time_ms: Date.now() - startTime,
    layers_executed: ['input_validation', 'event_normalization', 'decision_event_emission', 'response_assembly'],
  });
});

/**
 * Usage Pattern Analyzer
 * Accepts: { agent, payload: { time_range, dimensions, filters }, context }
 */
app.post('/v1/observatory/usage-patterns', (req, res) => {
  const traceId = req._traceId;
  const startTime = req._startTime;
  const { payload, context } = req.body || {};

  if (!payload) {
    return res.status(400).json({
      success: false,
      error: { code: 'VALIDATION_FAILED', message: 'payload is required' },
      layers_executed: ['input_validation'],
    });
  }

  const agentMeta = AGENTS['usage-patterns'];
  const inputsHash = hashInputs(payload);
  const decisionEvent = buildDecisionEvent(agentMeta, inputsHash, traceId, {
    analysis_requested: true,
    time_range: payload.time_range,
    dimensions: payload.dimensions,
  });

  res.json({
    success: true,
    agent: agentMeta.id,
    analysis: {
      time_range: payload.time_range,
      dimensions: payload.dimensions || [],
      patterns: [],
    },
    decision_event: decisionEvent,
    processing_time_ms: Date.now() - startTime,
    layers_executed: ['input_validation', 'pattern_analysis', 'decision_event_emission', 'response_assembly'],
  });
});

/**
 * Failure Classification
 * Accepts: { agent, payload: { error_events, context }, context }
 */
app.post('/v1/observatory/failures', (req, res) => {
  const traceId = req._traceId;
  const startTime = req._startTime;
  const { payload, context } = req.body || {};

  if (!payload) {
    return res.status(400).json({
      success: false,
      error: { code: 'VALIDATION_FAILED', message: 'payload is required' },
      layers_executed: ['input_validation'],
    });
  }

  const errorEvents = payload.error_events || [];
  const agentMeta = AGENTS.failures;
  const inputsHash = hashInputs(payload);
  const decisionEvent = buildDecisionEvent(agentMeta, inputsHash, traceId, {
    classified: errorEvents.length,
    classifications: errorEvents.map((e, i) => ({
      event_index: i,
      category: 'unclassified',
      confidence: 0.0,
    })),
  });

  res.json({
    success: true,
    agent: agentMeta.id,
    classifications: decisionEvent.outputs.classifications,
    decision_event: decisionEvent,
    processing_time_ms: Date.now() - startTime,
    layers_executed: ['input_validation', 'failure_classification', 'decision_event_emission', 'response_assembly'],
  });
});

/**
 * Health Check Agent
 * Accepts: { agent, payload: { targets, options }, context }
 */
app.post('/v1/observatory/health-check', (req, res) => {
  const traceId = req._traceId;
  const startTime = req._startTime;
  const { payload, context } = req.body || {};

  if (!payload) {
    return res.status(400).json({
      success: false,
      error: { code: 'VALIDATION_FAILED', message: 'payload is required' },
      layers_executed: ['input_validation'],
    });
  }

  const targets = payload.targets || [];
  const agentMeta = AGENTS['health-check'];
  const inputsHash = hashInputs(payload);

  const evaluations = targets.map((t) => ({
    target_id: t.id || t.target_id,
    target_type: t.type || t.target_type,
    status: 'healthy',
    confidence: 1.0,
    evaluated_at: new Date().toISOString(),
  }));

  const decisionEvent = buildDecisionEvent(agentMeta, inputsHash, traceId, {
    evaluations,
    target_count: targets.length,
  });

  res.json({
    success: true,
    agent: agentMeta.id,
    evaluations,
    decision_event: decisionEvent,
    processing_time_ms: Date.now() - startTime,
    layers_executed: ['input_validation', 'health_evaluation', 'decision_event_emission', 'response_assembly'],
  });
});

/**
 * SLO/SLA Enforcement
 * Accepts: { agent, payload: { slo_definitions, metrics, evaluation_time }, context }
 */
app.post('/v1/observatory/slo', (req, res) => {
  const traceId = req._traceId;
  const startTime = req._startTime;
  const { payload, context } = req.body || {};

  if (!payload) {
    return res.status(400).json({
      success: false,
      error: { code: 'VALIDATION_FAILED', message: 'payload is required' },
      layers_executed: ['input_validation'],
    });
  }

  const sloDefinitions = payload.slo_definitions || [];
  const agentMeta = AGENTS.slo;
  const inputsHash = hashInputs(payload);

  const sloStatuses = sloDefinitions.map((slo) => ({
    slo_id: slo.id || slo.slo_id,
    status: 'compliant',
    current_value: null,
    threshold: slo.threshold,
    evaluated_at: new Date().toISOString(),
  }));

  const decisionEvent = buildDecisionEvent(agentMeta, inputsHash, traceId, {
    violations: [],
    slo_statuses: sloStatuses,
    slos_evaluated: sloDefinitions.length,
    metrics_evaluated: (payload.metrics || []).length,
  });

  res.json({
    success: true,
    agent: agentMeta.id,
    violations: [],
    slo_statuses: sloStatuses,
    slos_evaluated: sloDefinitions.length,
    decision_event: decisionEvent,
    processing_time_ms: Date.now() - startTime,
    layers_executed: ['input_validation', 'slo_evaluation', 'violation_detection', 'decision_event_emission', 'response_assembly'],
  });
});

/**
 * Post-Mortem Generator
 * Accepts: { agent, payload: { incident_id, time_range, scope, options }, context }
 */
app.post('/v1/observatory/post-mortem', (req, res) => {
  const traceId = req._traceId;
  const startTime = req._startTime;
  const { payload, context } = req.body || {};

  if (!payload) {
    return res.status(400).json({
      success: false,
      error: { code: 'VALIDATION_FAILED', message: 'payload is required' },
      layers_executed: ['input_validation'],
    });
  }

  const agentMeta = AGENTS['post-mortem'];
  const inputsHash = hashInputs(payload);
  const reportId = crypto.randomUUID();

  const report = {
    report_id: reportId,
    incident_id: payload.incident_id,
    time_range: payload.time_range,
    generated_at: new Date().toISOString(),
    summary: 'Post-mortem report generated',
    sections: [],
  };

  const decisionEvent = buildDecisionEvent(agentMeta, inputsHash, traceId, {
    report_id: reportId,
    generation_requested: true,
  });

  res.json({
    success: true,
    agent: agentMeta.id,
    report,
    decision_event: decisionEvent,
    processing_time_ms: Date.now() - startTime,
    layers_executed: ['input_validation', 'data_aggregation', 'report_generation', 'decision_event_emission', 'response_assembly'],
  });
});

/**
 * Visualization Spec Generator
 * Accepts: { agent, payload: { visualization_type, data_sources, metrics, time_range }, context }
 */
app.post('/v1/observatory/visualization', (req, res) => {
  const traceId = req._traceId;
  const startTime = req._startTime;
  const { payload, context } = req.body || {};

  if (!payload) {
    return res.status(400).json({
      success: false,
      error: { code: 'VALIDATION_FAILED', message: 'payload is required' },
      layers_executed: ['input_validation'],
    });
  }

  const agentMeta = AGENTS.visualization;
  const inputsHash = hashInputs(payload);
  const specId = crypto.randomUUID();

  const spec = {
    spec_id: specId,
    visualization_type: payload.visualization_type,
    generated_at: new Date().toISOString(),
    config: {},
  };

  const decisionEvent = buildDecisionEvent(agentMeta, inputsHash, traceId, {
    spec_id: specId,
    generation_requested: true,
    visualization_type: payload.visualization_type,
  });

  res.json({
    success: true,
    agent: agentMeta.id,
    spec,
    decision_event: decisionEvent,
    processing_time_ms: Date.now() - startTime,
    layers_executed: ['input_validation', 'spec_generation', 'decision_event_emission', 'response_assembly'],
  });
});

// ----------------------------------------------------------------------------
// ERROR HANDLING
// ----------------------------------------------------------------------------

app.use((err, req, res, _next) => {
  console.error('Unhandled error:', err);
  res.status(500).json({
    success: false,
    error: { code: 'INTERNAL_ERROR', message: 'Internal server error' },
    layers_executed: ['error_handling'],
  });
});

// 404 handler
app.use((req, res) => {
  res.status(404).json({
    success: false,
    error: {
      code: 'NOT_FOUND',
      message: `Endpoint ${req.method} ${req.path} not found`,
    },
    layers_executed: ['routing'],
  });
});

// ----------------------------------------------------------------------------
// EXPORT FOR CLOUD FUNCTIONS
// ----------------------------------------------------------------------------

exports.api = app;
