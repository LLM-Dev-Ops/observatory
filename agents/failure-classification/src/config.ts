/**
 * Failure Classification Agent - Configuration
 *
 * CONSTITUTIONAL CLASSIFICATION: READ-ONLY, DIAGNOSTIC
 *
 * Configuration is loaded from environment variables.
 *
 * HARDENED: Phase 1 Layer 1 deployment
 * - Mandatory environment variables enforced
 * - Performance boundaries applied
 * - Contract assertions enabled
 */

import type { RuvectorConfig } from '../contracts';
import {
  PERFORMANCE_BOUNDARIES,
  type AgentIdentity,
  type HardenedEnvironment,
} from '../../shared/hardening/index';

// =============================================================================
// AGENT CONFIGURATION INTERFACE
// =============================================================================

export interface AgentConfig {
  // Agent identification (original)
  agentId: string;
  agentVersion: string;
  classification: 'READ-ONLY';

  // HARDENED: Agent identity (Phase 1 Layer 1)
  identity: AgentIdentity;

  // RuVector configuration
  ruvector: RuvectorConfig;

  // Processing configuration
  batchSize: number;
  timeoutMs: number;
  maxPayloadSizeBytes: number;

  // HARDENED: Performance boundaries
  maxTokens: number;
  maxLatencyMs: number;
  maxCallsPerRun: number;

  // Feature flags
  schemaValidationEnabled: boolean;
  selfObservationEnabled: boolean;

  // Retry configuration
  maxRetries: number;
  retryBackoffMs: number;
}

// =============================================================================
// DEFAULT CONFIGURATION
// =============================================================================

const DEFAULT_CONFIG: Omit<AgentConfig, 'identity' | 'ruvector'> & {
  ruvector: Omit<RuvectorConfig, 'endpoint' | 'apiKey'> & {
    endpoint: string;
    apiKey: string | undefined;
  };
} = {
  agentId: 'failure-classification-agent',
  agentVersion: '1.0.0',
  classification: 'READ-ONLY',

  ruvector: {
    endpoint: 'http://localhost:3001',
    apiKey: undefined,
    timeout: 30000,
    retryAttempts: 3,
    retryDelayMs: 1000,
    maxRetryDelayMs: 10000,
    connectionPoolSize: 5,
  },

  batchSize: 100,
  timeoutMs: 30000,
  maxPayloadSizeBytes: 10485760, // 10MB

  // HARDENED: Performance boundaries (conservative defaults)
  maxTokens: PERFORMANCE_BOUNDARIES.MAX_TOKENS,
  maxLatencyMs: PERFORMANCE_BOUNDARIES.MAX_LATENCY_MS,
  maxCallsPerRun: PERFORMANCE_BOUNDARIES.MAX_CALLS_PER_RUN,

  schemaValidationEnabled: true,
  selfObservationEnabled: true,

  maxRetries: 3,
  retryBackoffMs: 100,
};

// =============================================================================
// CONFIGURATION LOADER
// =============================================================================

/**
 * Load configuration from environment variables.
 *
 * HARDENED: Uses hardened environment for mandatory variables.
 * Call loadConfigWithHardenedEnv() for full hardening with startup assertions.
 */
export function loadConfig(): AgentConfig {
  // HARDENED: Build identity from environment (may be undefined for legacy mode)
  const identity: AgentIdentity = {
    agentName: process.env.AGENT_NAME || 'failure-classification-agent',
    agentDomain: process.env.AGENT_DOMAIN || 'diagnostics',
    agentPhase: 'phase1',
    agentLayer: 'layer1',
  };

  return {
    agentId: process.env.AGENT_ID || DEFAULT_CONFIG.agentId,
    agentVersion: process.env.AGENT_VERSION || DEFAULT_CONFIG.agentVersion,
    classification: 'READ-ONLY', // CONSTITUTIONAL: Cannot be overridden

    // HARDENED: Agent identity
    identity,

    ruvector: {
      // HARDENED: Prefer RUVECTOR_SERVICE_URL over legacy RUVECTOR_ENDPOINT
      endpoint: process.env.RUVECTOR_SERVICE_URL ||
                process.env.RUVECTOR_ENDPOINT ||
                DEFAULT_CONFIG.ruvector.endpoint,
      apiKey: process.env.RUVECTOR_API_KEY || DEFAULT_CONFIG.ruvector.apiKey,
      timeout: parseInt(
        process.env.RUVECTOR_TIMEOUT || String(DEFAULT_CONFIG.ruvector.timeout),
        10
      ),
      retryAttempts: parseInt(
        process.env.RUVECTOR_RETRY_ATTEMPTS ||
          String(DEFAULT_CONFIG.ruvector.retryAttempts),
        10
      ),
      retryDelayMs: parseInt(
        process.env.RUVECTOR_RETRY_DELAY_MS ||
          String(DEFAULT_CONFIG.ruvector.retryDelayMs),
        10
      ),
      maxRetryDelayMs: parseInt(
        process.env.RUVECTOR_MAX_RETRY_DELAY_MS ||
          String(DEFAULT_CONFIG.ruvector.maxRetryDelayMs),
        10
      ),
      connectionPoolSize: parseInt(
        process.env.RUVECTOR_CONNECTION_POOL_SIZE ||
          String(DEFAULT_CONFIG.ruvector.connectionPoolSize),
        10
      ),
    },

    batchSize: parseInt(
      process.env.BATCH_SIZE || String(DEFAULT_CONFIG.batchSize),
      10
    ),
    timeoutMs: parseInt(
      process.env.TIMEOUT_MS || String(DEFAULT_CONFIG.timeoutMs),
      10
    ),
    maxPayloadSizeBytes: parseInt(
      process.env.MAX_PAYLOAD_SIZE_BYTES ||
        String(DEFAULT_CONFIG.maxPayloadSizeBytes),
      10
    ),

    // HARDENED: Performance boundaries
    maxTokens: parseInt(
      process.env.MAX_TOKENS || String(DEFAULT_CONFIG.maxTokens),
      10
    ),
    maxLatencyMs: parseInt(
      process.env.MAX_LATENCY_MS || String(DEFAULT_CONFIG.maxLatencyMs),
      10
    ),
    maxCallsPerRun: parseInt(
      process.env.MAX_CALLS_PER_RUN || String(DEFAULT_CONFIG.maxCallsPerRun),
      10
    ),

    schemaValidationEnabled:
      process.env.SCHEMA_VALIDATION_ENABLED !== 'false',
    selfObservationEnabled:
      process.env.SELF_OBSERVATION_ENABLED === 'true',

    maxRetries: parseInt(
      process.env.MAX_RETRIES || String(DEFAULT_CONFIG.maxRetries),
      10
    ),
    retryBackoffMs: parseInt(
      process.env.RETRY_BACKOFF_MS || String(DEFAULT_CONFIG.retryBackoffMs),
      10
    ),
  };
}

/**
 * Load configuration with hardened environment assertions.
 * CRASHES the container if mandatory variables are missing.
 */
export function loadConfigWithHardenedEnv(hardenedEnv: HardenedEnvironment): AgentConfig {
  const baseConfig = loadConfig();

  return {
    ...baseConfig,
    identity: hardenedEnv.identity,
    ruvector: {
      ...baseConfig.ruvector,
      endpoint: hardenedEnv.ruvector.serviceUrl,
      apiKey: hardenedEnv.ruvector.apiKey,
    },
  };
}

// =============================================================================
// CONFIGURATION VALIDATION
// =============================================================================

/**
 * Validate configuration
 */
export function validateConfig(config: AgentConfig): string[] {
  const errors: string[] = [];

  // Validate agent identification
  if (!config.agentId) {
    errors.push('Agent ID is required');
  }

  if (!config.agentVersion.match(/^\d+\.\d+\.\d+$/)) {
    errors.push('Agent version must be semantic version (x.y.z)');
  }

  // CONSTITUTIONAL: Classification must be READ-ONLY
  if (config.classification !== 'READ-ONLY') {
    errors.push('CONSTITUTIONAL VIOLATION: Classification must be READ-ONLY');
  }

  // HARDENED: Validate agent identity
  if (!config.identity.agentName) {
    errors.push('HARDENED: Agent name is required');
  }

  if (!config.identity.agentDomain) {
    errors.push('HARDENED: Agent domain is required');
  }

  if (config.identity.agentPhase !== 'phase1') {
    errors.push('HARDENED: Agent phase must be "phase1"');
  }

  if (config.identity.agentLayer !== 'layer1') {
    errors.push('HARDENED: Agent layer must be "layer1"');
  }

  // Validate RuVector configuration
  if (!config.ruvector.endpoint) {
    errors.push('RuVector endpoint is required');
  }

  if (config.ruvector.timeout < 1000) {
    errors.push('RuVector timeout must be at least 1000ms');
  }

  if (config.ruvector.retryAttempts < 1) {
    errors.push('RuVector retry attempts must be at least 1');
  }

  // Validate processing configuration
  if (config.batchSize < 1 || config.batchSize > 1000) {
    errors.push('Batch size must be between 1 and 1000');
  }

  if (config.timeoutMs < 1000) {
    errors.push('Timeout must be at least 1000ms');
  }

  // HARDENED: Validate performance boundaries
  if (config.maxTokens > PERFORMANCE_BOUNDARIES.MAX_TOKENS) {
    errors.push(`HARDENED: MAX_TOKENS cannot exceed ${PERFORMANCE_BOUNDARIES.MAX_TOKENS}`);
  }

  if (config.maxLatencyMs > PERFORMANCE_BOUNDARIES.MAX_LATENCY_MS) {
    errors.push(`HARDENED: MAX_LATENCY_MS cannot exceed ${PERFORMANCE_BOUNDARIES.MAX_LATENCY_MS}`);
  }

  if (config.maxCallsPerRun > PERFORMANCE_BOUNDARIES.MAX_CALLS_PER_RUN) {
    errors.push(`HARDENED: MAX_CALLS_PER_RUN cannot exceed ${PERFORMANCE_BOUNDARIES.MAX_CALLS_PER_RUN}`);
  }

  return errors;
}

// =============================================================================
// EXPORTS
// =============================================================================

export { DEFAULT_CONFIG };
