/**
 * Configuration management for usage-pattern agent.
 * Loads configuration from environment variables with sensible defaults.
 *
 * CONSTITUTION:
 * - This agent is READ-ONLY and ADVISORY
 * - All persistence goes through ruvector-service
 * - Stateless execution at runtime
 */

import { RuvectorConfig } from './types/ruvector.js';

/**
 * Agent configuration interface.
 */
export interface AgentConfig {
  // Agent identity
  agentId: string;
  agentVersion: string;

  // Service endpoints
  ruvectorEndpoint: string;

  // Feature flags
  selfObservationEnabled: boolean;

  // Analysis limits
  maxEventsPerAnalysis: number;
  maxTimeWindowDays: number;
  defaultGranularity: 'minute' | 'hour' | 'day' | 'week' | 'month';

  // Timeouts
  analysisTimeoutMs: number;
  requestTimeoutMs: number;

  // Statistical thresholds
  minSampleSizeForTrends: number;
  minSampleSizeForSeasonality: number;
  confidenceThreshold: number;

  // Ruvector client config
  ruvector: RuvectorConfig;
}

/**
 * Load configuration from environment variables.
 */
export function loadConfig(): AgentConfig {
  const agentId = process.env.AGENT_ID || 'usage-pattern-agent';
  const agentVersion = process.env.AGENT_VERSION || '1.0.0';
  const ruvectorEndpoint = process.env.RUVECTOR_ENDPOINT || 'http://localhost:3001';

  return {
    agentId,
    agentVersion,
    ruvectorEndpoint,
    selfObservationEnabled: process.env.SELF_OBSERVATION_ENABLED === 'true',
    maxEventsPerAnalysis: parseInt(process.env.MAX_EVENTS_PER_ANALYSIS || '100000', 10),
    maxTimeWindowDays: parseInt(process.env.MAX_TIME_WINDOW_DAYS || '90', 10),
    defaultGranularity: (process.env.DEFAULT_GRANULARITY as AgentConfig['defaultGranularity']) || 'hour',
    analysisTimeoutMs: parseInt(process.env.ANALYSIS_TIMEOUT_MS || '60000', 10),
    requestTimeoutMs: parseInt(process.env.REQUEST_TIMEOUT_MS || '30000', 10),
    minSampleSizeForTrends: parseInt(process.env.MIN_SAMPLE_SIZE_FOR_TRENDS || '30', 10),
    minSampleSizeForSeasonality: parseInt(process.env.MIN_SAMPLE_SIZE_FOR_SEASONALITY || '168', 10), // 1 week hourly
    confidenceThreshold: parseFloat(process.env.CONFIDENCE_THRESHOLD || '0.8'),
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
 * Validate configuration.
 */
export function validateConfig(config: AgentConfig): string[] {
  const errors: string[] = [];

  if (!config.agentId) {
    errors.push('agentId is required');
  }

  if (!config.agentVersion) {
    errors.push('agentVersion is required');
  }

  if (!/^\d+\.\d+\.\d+$/.test(config.agentVersion)) {
    errors.push('agentVersion must be semantic version (x.y.z)');
  }

  if (!config.ruvectorEndpoint) {
    errors.push('ruvectorEndpoint is required');
  }

  if (config.maxEventsPerAnalysis <= 0) {
    errors.push('maxEventsPerAnalysis must be positive');
  }

  if (config.maxTimeWindowDays <= 0) {
    errors.push('maxTimeWindowDays must be positive');
  }

  if (config.analysisTimeoutMs <= 0) {
    errors.push('analysisTimeoutMs must be positive');
  }

  if (config.requestTimeoutMs <= 0) {
    errors.push('requestTimeoutMs must be positive');
  }

  if (config.minSampleSizeForTrends <= 0) {
    errors.push('minSampleSizeForTrends must be positive');
  }

  if (config.minSampleSizeForSeasonality <= 0) {
    errors.push('minSampleSizeForSeasonality must be positive');
  }

  if (config.confidenceThreshold < 0 || config.confidenceThreshold > 1) {
    errors.push('confidenceThreshold must be between 0 and 1');
  }

  // Ruvector config validation
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
 * Get default configuration.
 */
export function getDefaultConfig(): AgentConfig {
  return {
    agentId: 'usage-pattern-agent',
    agentVersion: '1.0.0',
    ruvectorEndpoint: 'http://localhost:3001',
    selfObservationEnabled: false,
    maxEventsPerAnalysis: 100000,
    maxTimeWindowDays: 90,
    defaultGranularity: 'hour',
    analysisTimeoutMs: 60000,
    requestTimeoutMs: 30000,
    minSampleSizeForTrends: 30,
    minSampleSizeForSeasonality: 168,
    confidenceThreshold: 0.8,
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
 * Merge partial configuration with defaults.
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
