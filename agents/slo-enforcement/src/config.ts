/**
 * SLO/SLA Enforcement Agent - Configuration
 *
 * Environment-based configuration with sensible defaults.
 * All configuration is loaded at startup and cached.
 */

import { AGENT_METADATA } from '../contracts';

/**
 * Environment variable helper with type conversion
 */
function getEnv(key: string, defaultValue: string): string {
  return process.env[key] ?? defaultValue;
}

function getEnvNumber(key: string, defaultValue: number): number {
  const value = process.env[key];
  if (value === undefined) return defaultValue;
  const parsed = parseFloat(value);
  return isNaN(parsed) ? defaultValue : parsed;
}

function getEnvBoolean(key: string, defaultValue: boolean): boolean {
  const value = process.env[key];
  if (value === undefined) return defaultValue;
  return value.toLowerCase() === 'true' || value === '1';
}

/**
 * Agent Configuration
 */
export interface AgentConfig {
  id: string;
  version: string;
  classification: 'enforcement-class';
  decision_type: string;
  actuating: false;
}

/**
 * RuVector Service Configuration
 */
export interface RuvectorConfig {
  endpoint: string;
  apiKey?: string;
  timeout: number;
  retryAttempts: number;
  retryDelayMs: number;
  maxRetryDelayMs: number;
  connectionPoolSize: number;
}

/**
 * Evaluation Configuration
 */
export interface EvaluationConfig {
  maxSlosPerRequest: number;
  maxMetricsPerRequest: number;
  maxBatchSize: number;
  defaultWarningThresholdPct: number;
  enableHistoricalContext: boolean;
  historicalWindowHours: number;
}

/**
 * Confidence Calculation Configuration
 */
export interface ConfidenceConfig {
  minSampleSize: number;          // Minimum samples for high confidence
  maxDataAgeMs: number;           // Maximum data age for high confidence
  volatilityThreshold: number;    // Threshold for considering data volatile
  minConfidence: number;          // Minimum confidence to return
}

/**
 * Telemetry Configuration
 */
export interface TelemetryConfig {
  enabled: boolean;
  sampleRate: number;
  metricsInterval: number;
}

/**
 * Complete Configuration
 */
export interface Config {
  agent: AgentConfig;
  ruvector: RuvectorConfig;
  evaluation: EvaluationConfig;
  confidence: ConfidenceConfig;
  telemetry: TelemetryConfig;
}

/**
 * Build configuration from environment
 */
function buildConfig(): Config {
  return {
    agent: {
      id: AGENT_METADATA.id,
      version: AGENT_METADATA.version,
      classification: 'enforcement-class',
      decision_type: AGENT_METADATA.decision_type,
      actuating: false,
    },

    ruvector: {
      endpoint: getEnv('RUVECTOR_ENDPOINT', 'http://localhost:3001'),
      apiKey: process.env.RUVECTOR_API_KEY,
      timeout: getEnvNumber('RUVECTOR_TIMEOUT_MS', 5000),
      retryAttempts: getEnvNumber('RUVECTOR_RETRY_ATTEMPTS', 3),
      retryDelayMs: getEnvNumber('RUVECTOR_RETRY_DELAY_MS', 1000),
      maxRetryDelayMs: getEnvNumber('RUVECTOR_MAX_RETRY_DELAY_MS', 10000),
      connectionPoolSize: getEnvNumber('RUVECTOR_POOL_SIZE', 5),
    },

    evaluation: {
      maxSlosPerRequest: getEnvNumber('MAX_SLOS_PER_REQUEST', 100),
      maxMetricsPerRequest: getEnvNumber('MAX_METRICS_PER_REQUEST', 1000),
      maxBatchSize: getEnvNumber('MAX_BATCH_SIZE', 50),
      defaultWarningThresholdPct: getEnvNumber('DEFAULT_WARNING_THRESHOLD_PCT', 80),
      enableHistoricalContext: getEnvBoolean('ENABLE_HISTORICAL_CONTEXT', false),
      historicalWindowHours: getEnvNumber('HISTORICAL_WINDOW_HOURS', 24),
    },

    confidence: {
      minSampleSize: getEnvNumber('MIN_SAMPLE_SIZE', 10),
      maxDataAgeMs: getEnvNumber('MAX_DATA_AGE_MS', 300000), // 5 minutes
      volatilityThreshold: getEnvNumber('VOLATILITY_THRESHOLD', 0.3),
      minConfidence: getEnvNumber('MIN_CONFIDENCE', 0.5),
    },

    telemetry: {
      enabled: getEnvBoolean('TELEMETRY_ENABLED', true),
      sampleRate: getEnvNumber('TELEMETRY_SAMPLE_RATE', 1.0),
      metricsInterval: getEnvNumber('METRICS_INTERVAL_MS', 60000),
    },
  };
}

// Cache configuration
let cachedConfig: Config | null = null;

/**
 * Load and cache configuration
 */
export function loadConfig(): Config {
  if (cachedConfig !== null) {
    return cachedConfig;
  }
  cachedConfig = buildConfig();
  return cachedConfig;
}

/**
 * Reset cached configuration (for testing)
 */
export function resetConfig(): void {
  cachedConfig = null;
}

/**
 * Validate configuration
 */
export function validateConfig(config: Config): string[] {
  const errors: string[] = [];

  // Validate RuVector endpoint
  if (!config.ruvector.endpoint.startsWith('http')) {
    errors.push('RUVECTOR_ENDPOINT must be a valid HTTP(S) URL');
  }

  // Validate timeouts
  if (config.ruvector.timeout < 100) {
    errors.push('RUVECTOR_TIMEOUT_MS must be at least 100ms');
  }

  // Validate pool size
  if (config.ruvector.connectionPoolSize < 1) {
    errors.push('RUVECTOR_POOL_SIZE must be at least 1');
  }

  // Validate confidence settings
  if (config.confidence.minConfidence < 0 || config.confidence.minConfidence > 1) {
    errors.push('MIN_CONFIDENCE must be between 0 and 1');
  }

  // Validate sample rate
  if (config.telemetry.sampleRate < 0 || config.telemetry.sampleRate > 1) {
    errors.push('TELEMETRY_SAMPLE_RATE must be between 0 and 1');
  }

  return errors;
}

export default loadConfig;
