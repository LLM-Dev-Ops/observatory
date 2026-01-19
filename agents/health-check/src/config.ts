/**
 * Health Check Agent - Configuration
 *
 * Loads configuration from environment variables with sensible defaults.
 * All thresholds are configurable per-environment.
 */

import type { IndicatorType } from '../contracts/schemas.js';

// ============================================================================
// THRESHOLD CONFIGURATION
// ============================================================================

export interface LatencyThresholds {
  healthy_max_p95_ms: number;
  degraded_max_p95_ms: number;
}

export interface ErrorRateThresholds {
  healthy_max_percentage: number;
  degraded_max_percentage: number;
}

export interface ThroughputThresholds {
  healthy_min_rps: number;
  degraded_min_rps: number;
}

export interface SaturationThresholds {
  healthy_max_percentage: number;
  degraded_max_percentage: number;
}

export interface AvailabilityThresholds {
  healthy_min_percentage: number;
  degraded_min_percentage: number;
}

export interface ThresholdConfig {
  latency: LatencyThresholds;
  error_rate: ErrorRateThresholds;
  throughput: ThroughputThresholds;
  saturation: SaturationThresholds;
  availability: AvailabilityThresholds;
}

// ============================================================================
// HYSTERESIS CONFIGURATION
// ============================================================================

export interface HysteresisConfig {
  threshold_to_improve: number;
  threshold_to_degrade: number;
}

// ============================================================================
// INDICATOR WEIGHTS
// ============================================================================

export type IndicatorWeights = Record<IndicatorType, number>;

// ============================================================================
// RUVECTOR CLIENT CONFIGURATION
// ============================================================================

export interface RuvectorConfig {
  endpoint: string;
  apiKey: string;
  timeout_ms: number;
  max_retries: number;
  retry_base_delay_ms: number;
  pool_size: number;
}

// ============================================================================
// AGENT CONFIGURATION
// ============================================================================

export interface AgentConfig {
  id: string;
  version: string;
  classification: 'advisory';
  decision_type: 'health_evaluation';
}

// ============================================================================
// TELEMETRY CONFIGURATION
// ============================================================================

export interface TelemetryConfig {
  self_observation_enabled: boolean;
  otel_endpoint: string | null;
  log_level: 'debug' | 'info' | 'warn' | 'error';
}

// ============================================================================
// COMPLETE CONFIGURATION
// ============================================================================

export interface Config {
  agent: AgentConfig;
  thresholds: ThresholdConfig;
  hysteresis: HysteresisConfig;
  indicator_weights: IndicatorWeights;
  ruvector: RuvectorConfig;
  telemetry: TelemetryConfig;
  default_evaluation_window: string;
  default_trend_window: string;
}

// ============================================================================
// DEFAULT VALUES
// ============================================================================

const DEFAULT_THRESHOLDS: ThresholdConfig = {
  latency: {
    healthy_max_p95_ms: 500,
    degraded_max_p95_ms: 2000,
  },
  error_rate: {
    healthy_max_percentage: 1,
    degraded_max_percentage: 5,
  },
  throughput: {
    healthy_min_rps: 10,
    degraded_min_rps: 1,
  },
  saturation: {
    healthy_max_percentage: 70,
    degraded_max_percentage: 90,
  },
  availability: {
    healthy_min_percentage: 99.9,
    degraded_min_percentage: 99.0,
  },
};

const DEFAULT_HYSTERESIS: HysteresisConfig = {
  threshold_to_improve: 3,  // Slow improvement
  threshold_to_degrade: 1,  // Quick degradation
};

const DEFAULT_INDICATOR_WEIGHTS: IndicatorWeights = {
  error_rate: 3.0,      // Highest weight - user impact
  availability: 2.5,    // High weight - uptime critical
  latency: 2.0,         // High weight - user experience
  throughput: 1.5,      // Medium weight
  saturation: 1.0,      // Lower weight - leading indicator
};

// ============================================================================
// ENVIRONMENT VARIABLE PARSING
// ============================================================================

function getEnv(key: string, defaultValue: string): string {
  return process.env[key] ?? defaultValue;
}

function getEnvNumber(key: string, defaultValue: number): number {
  const value = process.env[key];
  if (value === undefined) return defaultValue;
  const parsed = parseFloat(value);
  return isNaN(parsed) ? defaultValue : parsed;
}

function getEnvInt(key: string, defaultValue: number): number {
  const value = process.env[key];
  if (value === undefined) return defaultValue;
  const parsed = parseInt(value, 10);
  return isNaN(parsed) ? defaultValue : parsed;
}

function getEnvBool(key: string, defaultValue: boolean): boolean {
  const value = process.env[key];
  if (value === undefined) return defaultValue;
  return value.toLowerCase() === 'true' || value === '1';
}

// ============================================================================
// CONFIGURATION LOADER
// ============================================================================

let cachedConfig: Config | null = null;

/**
 * Load configuration from environment variables.
 * Configuration is cached after first load.
 */
export function loadConfig(): Config {
  if (cachedConfig) {
    return cachedConfig;
  }

  const config: Config = {
    agent: {
      id: getEnv('AGENT_ID', 'health-check-agent'),
      version: getEnv('AGENT_VERSION', '1.0.0'),
      classification: 'advisory',
      decision_type: 'health_evaluation',
    },

    thresholds: {
      latency: {
        healthy_max_p95_ms: getEnvNumber('LATENCY_HEALTHY_MAX_P95_MS', DEFAULT_THRESHOLDS.latency.healthy_max_p95_ms),
        degraded_max_p95_ms: getEnvNumber('LATENCY_DEGRADED_MAX_P95_MS', DEFAULT_THRESHOLDS.latency.degraded_max_p95_ms),
      },
      error_rate: {
        healthy_max_percentage: getEnvNumber('ERROR_RATE_HEALTHY_MAX_PCT', DEFAULT_THRESHOLDS.error_rate.healthy_max_percentage),
        degraded_max_percentage: getEnvNumber('ERROR_RATE_DEGRADED_MAX_PCT', DEFAULT_THRESHOLDS.error_rate.degraded_max_percentage),
      },
      throughput: {
        healthy_min_rps: getEnvNumber('THROUGHPUT_HEALTHY_MIN_RPS', DEFAULT_THRESHOLDS.throughput.healthy_min_rps),
        degraded_min_rps: getEnvNumber('THROUGHPUT_DEGRADED_MIN_RPS', DEFAULT_THRESHOLDS.throughput.degraded_min_rps),
      },
      saturation: {
        healthy_max_percentage: getEnvNumber('SATURATION_HEALTHY_MAX_PCT', DEFAULT_THRESHOLDS.saturation.healthy_max_percentage),
        degraded_max_percentage: getEnvNumber('SATURATION_DEGRADED_MAX_PCT', DEFAULT_THRESHOLDS.saturation.degraded_max_percentage),
      },
      availability: {
        healthy_min_percentage: getEnvNumber('AVAILABILITY_HEALTHY_MIN_PCT', DEFAULT_THRESHOLDS.availability.healthy_min_percentage),
        degraded_min_percentage: getEnvNumber('AVAILABILITY_DEGRADED_MIN_PCT', DEFAULT_THRESHOLDS.availability.degraded_min_percentage),
      },
    },

    hysteresis: {
      threshold_to_improve: getEnvInt('HYSTERESIS_THRESHOLD_IMPROVE', DEFAULT_HYSTERESIS.threshold_to_improve),
      threshold_to_degrade: getEnvInt('HYSTERESIS_THRESHOLD_DEGRADE', DEFAULT_HYSTERESIS.threshold_to_degrade),
    },

    indicator_weights: {
      error_rate: getEnvNumber('WEIGHT_ERROR_RATE', DEFAULT_INDICATOR_WEIGHTS.error_rate),
      availability: getEnvNumber('WEIGHT_AVAILABILITY', DEFAULT_INDICATOR_WEIGHTS.availability),
      latency: getEnvNumber('WEIGHT_LATENCY', DEFAULT_INDICATOR_WEIGHTS.latency),
      throughput: getEnvNumber('WEIGHT_THROUGHPUT', DEFAULT_INDICATOR_WEIGHTS.throughput),
      saturation: getEnvNumber('WEIGHT_SATURATION', DEFAULT_INDICATOR_WEIGHTS.saturation),
    },

    ruvector: {
      endpoint: getEnv('RUVECTOR_ENDPOINT', 'https://ruvector-service.internal:443'),
      apiKey: getEnv('RUVECTOR_API_KEY', ''),
      timeout_ms: getEnvInt('RUVECTOR_TIMEOUT_MS', 5000),
      max_retries: getEnvInt('RUVECTOR_MAX_RETRIES', 3),
      retry_base_delay_ms: getEnvInt('RUVECTOR_RETRY_BASE_DELAY_MS', 1000),
      pool_size: getEnvInt('RUVECTOR_POOL_SIZE', 5),
    },

    telemetry: {
      self_observation_enabled: getEnvBool('SELF_OBSERVATION_ENABLED', true),
      otel_endpoint: process.env['OTEL_EXPORTER_OTLP_ENDPOINT'] ?? null,
      log_level: (getEnv('LOG_LEVEL', 'info') as 'debug' | 'info' | 'warn' | 'error'),
    },

    default_evaluation_window: getEnv('DEFAULT_EVALUATION_WINDOW', '5m'),
    default_trend_window: getEnv('DEFAULT_TREND_WINDOW', '1h'),
  };

  cachedConfig = config;
  return config;
}

/**
 * Reset cached configuration (for testing)
 */
export function resetConfig(): void {
  cachedConfig = null;
}

/**
 * Get the default thresholds (for testing and documentation)
 */
export function getDefaultThresholds(): ThresholdConfig {
  return { ...DEFAULT_THRESHOLDS };
}

/**
 * Get the default hysteresis config (for testing and documentation)
 */
export function getDefaultHysteresis(): HysteresisConfig {
  return { ...DEFAULT_HYSTERESIS };
}

/**
 * Get the default indicator weights (for testing and documentation)
 */
export function getDefaultIndicatorWeights(): IndicatorWeights {
  return { ...DEFAULT_INDICATOR_WEIGHTS };
}
