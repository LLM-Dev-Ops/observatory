/**
 * Environment Configuration
 *
 * All configuration resolved via environment variables or Secret Manager.
 * NO hardcoded service names, URLs, or credentials.
 *
 * HARDENED: Phase 1 Layer 1 deployment
 * - Mandatory environment variables enforced
 * - Ruvector REQUIRED (service crashes if unavailable)
 * - Agent identity standardization
 */

export interface ServiceConfig {
  // Service identity
  serviceName: string;
  serviceVersion: string;
  environment: 'dev' | 'staging' | 'prod';

  // HARDENED: Agent identity (Phase 1 Layer 1)
  agentName: string;
  agentDomain: string;
  agentPhase: 'phase1';
  agentLayer: 'layer1';

  // RuVector service (memory/persistence layer) - REQUIRED
  ruvectorServiceUrl: string;
  ruvectorApiKey: string;

  // Telemetry
  telemetryEndpoint?: string;

  // Logging
  logLevel: string;

  // Agent-specific
  selfObservationEnabled: boolean;
  maxBatchSize: number;
  requestTimeoutMs: number;

  // HARDENED: Performance boundaries
  maxTokens: number;
  maxLatencyMs: number;
  maxCallsPerRun: number;
}

// HARDENED: Performance boundary defaults
const PERFORMANCE_BOUNDARIES = {
  MAX_TOKENS: 800,
  MAX_LATENCY_MS: 1500,
  MAX_CALLS_PER_RUN: 2,
} as const;

/**
 * Load configuration from environment variables
 *
 * HARDENED: Enforces Phase 1 Layer 1 requirements
 */
export function loadConfig(): ServiceConfig {
  return {
    serviceName: process.env.SERVICE_NAME || 'llm-observatory',
    serviceVersion: process.env.SERVICE_VERSION || '1.0.0',
    environment: (process.env.PLATFORM_ENV || 'dev') as ServiceConfig['environment'],

    // HARDENED: Agent identity (Phase 1 Layer 1)
    agentName: process.env.AGENT_NAME || 'llm-observatory-unified',
    agentDomain: process.env.AGENT_DOMAIN || 'observability',
    agentPhase: 'phase1',
    agentLayer: 'layer1',

    // HARDENED: Ruvector is REQUIRED
    ruvectorServiceUrl: process.env.RUVECTOR_SERVICE_URL || '',
    ruvectorApiKey: process.env.RUVECTOR_API_KEY || '',

    telemetryEndpoint: process.env.TELEMETRY_ENDPOINT,

    logLevel: process.env.LOG_LEVEL || 'info',

    selfObservationEnabled: process.env.SELF_OBSERVATION_ENABLED !== 'false',
    maxBatchSize: parseInt(process.env.MAX_BATCH_SIZE || '100', 10),
    requestTimeoutMs: parseInt(process.env.REQUEST_TIMEOUT_MS || '30000', 10),

    // HARDENED: Performance boundaries
    maxTokens: parseInt(process.env.MAX_TOKENS || String(PERFORMANCE_BOUNDARIES.MAX_TOKENS), 10),
    maxLatencyMs: parseInt(process.env.MAX_LATENCY_MS || String(PERFORMANCE_BOUNDARIES.MAX_LATENCY_MS), 10),
    maxCallsPerRun: parseInt(process.env.MAX_CALLS_PER_RUN || String(PERFORMANCE_BOUNDARIES.MAX_CALLS_PER_RUN), 10),
  };
}

/**
 * Validate required environment variables
 *
 * HARDENED: Phase 1 Layer 1 mandatory requirements
 * - RUVECTOR_SERVICE_URL (required)
 * - RUVECTOR_API_KEY (required, from Google Secret Manager)
 * - AGENT_NAME (required)
 * - AGENT_DOMAIN (required)
 * - AGENT_PHASE=phase1 (required)
 * - AGENT_LAYER=layer1 (required)
 */
export function validateEnvironment(): string[] {
  const errors: string[] = [];

  // HARDENED: Ruvector requirements (mandatory)
  if (!process.env.RUVECTOR_SERVICE_URL) {
    errors.push('RUVECTOR_SERVICE_URL is required');
  }

  if (!process.env.RUVECTOR_API_KEY) {
    errors.push('RUVECTOR_API_KEY is required (must be from Google Secret Manager)');
  }

  // HARDENED: Agent identity requirements (mandatory)
  if (!process.env.AGENT_NAME) {
    errors.push('AGENT_NAME is required');
  }

  if (!process.env.AGENT_DOMAIN) {
    errors.push('AGENT_DOMAIN is required');
  }

  if (process.env.AGENT_PHASE !== 'phase1') {
    errors.push('AGENT_PHASE must be "phase1"');
  }

  if (process.env.AGENT_LAYER !== 'layer1') {
    errors.push('AGENT_LAYER must be "layer1"');
  }

  // Validate PLATFORM_ENV if set
  const env = process.env.PLATFORM_ENV;
  if (env && !['dev', 'staging', 'prod'].includes(env)) {
    errors.push(`PLATFORM_ENV must be one of: dev, staging, prod (got: ${env})`);
  }

  return errors;
}

/**
 * HARDENED: Structured logging for agent lifecycle events
 */
export function logAgentStarted(data: Record<string, unknown> = {}): void {
  console.log(JSON.stringify({
    event: 'agent_started',
    timestamp: new Date().toISOString(),
    agent_name: process.env.AGENT_NAME,
    agent_domain: process.env.AGENT_DOMAIN,
    agent_phase: process.env.AGENT_PHASE,
    agent_layer: process.env.AGENT_LAYER,
    ...data,
  }));
}

export function logAgentAbort(reason: string, details: string[]): void {
  console.error(JSON.stringify({
    event: 'agent_abort',
    timestamp: new Date().toISOString(),
    reason,
    details,
    agent_name: process.env.AGENT_NAME || 'unknown',
    agent_domain: process.env.AGENT_DOMAIN || 'unknown',
    agent_phase: process.env.AGENT_PHASE || 'unknown',
    agent_layer: process.env.AGENT_LAYER || 'unknown',
  }));
}

export function logDecisionEventEmitted(executionRef: string, agentName: string): void {
  console.log(JSON.stringify({
    event: 'decision_event_emitted',
    timestamp: new Date().toISOString(),
    execution_ref: executionRef,
    agent_name: agentName,
  }));
}

/**
 * Get RuVector client configuration
 */
export function getRuvectorConfig() {
  const config = loadConfig();
  return {
    endpoint: config.ruvectorServiceUrl,
    apiKey: config.ruvectorApiKey,
    timeout: config.requestTimeoutMs,
    retryAttempts: 3,
    retryDelayMs: 1000,
    maxRetryDelayMs: 10000,
    connectionPoolSize: 10,
  };
}
