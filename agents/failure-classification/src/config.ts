/**
 * Failure Classification Agent - Configuration
 *
 * CONSTITUTIONAL CLASSIFICATION: READ-ONLY, DIAGNOSTIC
 *
 * Configuration is loaded from environment variables.
 */

import type { RuvectorConfig } from '../contracts';

// =============================================================================
// AGENT CONFIGURATION INTERFACE
// =============================================================================

export interface AgentConfig {
  // Agent identification
  agentId: string;
  agentVersion: string;
  classification: 'READ-ONLY';

  // RuVector configuration
  ruvector: RuvectorConfig;

  // Processing configuration
  batchSize: number;
  timeoutMs: number;
  maxPayloadSizeBytes: number;

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

const DEFAULT_CONFIG: AgentConfig = {
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

  schemaValidationEnabled: true,
  selfObservationEnabled: true,

  maxRetries: 3,
  retryBackoffMs: 100,
};

// =============================================================================
// CONFIGURATION LOADER
// =============================================================================

/**
 * Load configuration from environment variables
 */
export function loadConfig(): AgentConfig {
  return {
    agentId: process.env.AGENT_ID || DEFAULT_CONFIG.agentId,
    agentVersion: process.env.AGENT_VERSION || DEFAULT_CONFIG.agentVersion,
    classification: 'READ-ONLY', // CONSTITUTIONAL: Cannot be overridden

    ruvector: {
      endpoint: process.env.RUVECTOR_ENDPOINT || DEFAULT_CONFIG.ruvector.endpoint,
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

  return errors;
}

// =============================================================================
// EXPORTS
// =============================================================================

export { DEFAULT_CONFIG };
