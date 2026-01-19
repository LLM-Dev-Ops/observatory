/**
 * Configuration management for telemetry-collector agent
 * Loads configuration from environment variables with sensible defaults
 */

import { RuvectorConfig } from './types/ruvector.js';

/**
 * Agent configuration interface
 */
export interface AgentConfig {
  agentId: string;
  agentVersion: string;
  ruvectorEndpoint: string;
  selfObservationEnabled: boolean;
  batchSize: number;
  timeoutMs: number;
  ruvector: RuvectorConfig;
}

/**
 * Load configuration from environment variables
 */
export function loadConfig(): AgentConfig {
  const agentId = process.env.AGENT_ID || 'telemetry-collector';
  const agentVersion = process.env.AGENT_VERSION || '1.0.0';
  const ruvectorEndpoint = process.env.RUVECTOR_ENDPOINT || 'http://localhost:3001';

  return {
    agentId,
    agentVersion,
    ruvectorEndpoint,
    selfObservationEnabled: process.env.SELF_OBSERVATION_ENABLED === 'true',
    batchSize: parseInt(process.env.BATCH_SIZE || '10', 10),
    timeoutMs: parseInt(process.env.TIMEOUT_MS || '30000', 10),
    ruvector: {
      endpoint: ruvectorEndpoint,
      apiKey: process.env.RUVECTOR_API_KEY,
      timeout: parseInt(process.env.RUVECTOR_TIMEOUT || '30000', 10),
      retryAttempts: parseInt(process.env.RUVECTOR_RETRY_ATTEMPTS || '3', 10),
      retryDelayMs: parseInt(process.env.RUVECTOR_RETRY_DELAY_MS || '1000', 10),
      maxRetryDelayMs: parseInt(process.env.RUVECTOR_MAX_RETRY_DELAY_MS || '10000', 10),
      connectionPoolSize: parseInt(process.env.RUVECTOR_CONNECTION_POOL_SIZE || '5', 10),
    },
  };
}

/**
 * Validate configuration
 */
export function validateConfig(config: AgentConfig): string[] {
  const errors: string[] = [];

  if (!config.agentId) {
    errors.push('agentId is required');
  }

  if (!config.agentVersion) {
    errors.push('agentVersion is required');
  }

  if (!config.ruvectorEndpoint) {
    errors.push('ruvectorEndpoint is required');
  }

  if (config.batchSize <= 0) {
    errors.push('batchSize must be positive');
  }

  if (config.timeoutMs <= 0) {
    errors.push('timeoutMs must be positive');
  }

  if (config.ruvector.timeout <= 0) {
    errors.push('ruvector.timeout must be positive');
  }

  if (config.ruvector.retryAttempts < 0) {
    errors.push('ruvector.retryAttempts must be non-negative');
  }

  if (config.ruvector.retryDelayMs <= 0) {
    errors.push('ruvector.retryDelayMs must be positive');
  }

  if (config.ruvector.maxRetryDelayMs < config.ruvector.retryDelayMs) {
    errors.push('ruvector.maxRetryDelayMs must be >= retryDelayMs');
  }

  if (config.ruvector.connectionPoolSize <= 0) {
    errors.push('ruvector.connectionPoolSize must be positive');
  }

  return errors;
}

/**
 * Get default configuration
 */
export function getDefaultConfig(): AgentConfig {
  return {
    agentId: 'telemetry-collector',
    agentVersion: '1.0.0',
    ruvectorEndpoint: 'http://localhost:3001',
    selfObservationEnabled: false,
    batchSize: 10,
    timeoutMs: 30000,
    ruvector: {
      endpoint: 'http://localhost:3001',
      timeout: 30000,
      retryAttempts: 3,
      retryDelayMs: 1000,
      maxRetryDelayMs: 10000,
      connectionPoolSize: 5,
    },
  };
}

/**
 * Merge partial configuration with defaults
 */
export function mergeConfig(
  partial: Partial<AgentConfig>,
  defaults: AgentConfig = getDefaultConfig()
): AgentConfig {
  return {
    ...defaults,
    ...partial,
    ruvector: {
      ...defaults.ruvector,
      ...(partial.ruvector || {}),
    },
  };
}
