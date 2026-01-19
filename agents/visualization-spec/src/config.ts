/**
 * Visualization Spec Agent - Configuration Loader
 *
 * Loads configuration from environment variables with sensible defaults.
 * Follows the configuration hierarchy: env vars > config file > defaults.
 */

import {
  AGENT_ID,
  AGENT_VERSION,
  AGENT_CLASSIFICATION,
} from '../contracts/schemas.js';
import type { GeneratorConfig, RuvectorConfig, ColorPalette } from '../contracts/types.js';
import { DEFAULT_COLOR_PALETTES } from '../contracts/types.js';

// =============================================================================
// Configuration Types
// =============================================================================

export interface Config {
  agent: AgentConfig;
  generator: GeneratorConfig;
  ruvector: RuvectorConfig;
  telemetry: TelemetryConfig;
  thresholds: ThresholdConfig;
}

export interface AgentConfig {
  id: string;
  version: string;
  classification: string;
}

export interface TelemetryConfig {
  metricsPrefix: string;
  spanPrefix: string;
  prometheusEnabled: boolean;
  openTelemetryEnabled: boolean;
  logLevel: 'debug' | 'info' | 'warn' | 'error';
}

export interface ThresholdConfig {
  maxSpecsPerBatch: number;
  maxSeriesPerChart: number;
  maxDimensionsPerChart: number;
  maxTimeRangeDays: number;
  requestTimeoutMs: number;
}

// =============================================================================
// Default Values
// =============================================================================

const DEFAULT_GENERATOR_CONFIG: GeneratorConfig = {
  defaultTheme: 'observatory',
  defaultColorScheme: 'default',
  maxSeriesPerChart: 10,
  maxDimensions: 5,
  defaultAnimations: true,
  defaultResponsive: true,
};

const DEFAULT_RUVECTOR_CONFIG: RuvectorConfig = {
  endpoint: 'http://ruvector-service:8080/api/v1',
  apiKey: '',
  timeoutMs: 30000,
  maxRetries: 3,
  retryBaseDelayMs: 100,
  poolSize: 5,
};

const DEFAULT_TELEMETRY_CONFIG: TelemetryConfig = {
  metricsPrefix: 'visualization_spec_agent',
  spanPrefix: 'viz_spec',
  prometheusEnabled: true,
  openTelemetryEnabled: true,
  logLevel: 'info',
};

const DEFAULT_THRESHOLD_CONFIG: ThresholdConfig = {
  maxSpecsPerBatch: 20,
  maxSeriesPerChart: 10,
  maxDimensionsPerChart: 5,
  maxTimeRangeDays: 365,
  requestTimeoutMs: 30000,
};

// =============================================================================
// Configuration Loading
// =============================================================================

/**
 * Loads the complete agent configuration
 */
export function loadConfig(): Config {
  return {
    agent: loadAgentConfig(),
    generator: loadGeneratorConfig(),
    ruvector: loadRuvectorConfig(),
    telemetry: loadTelemetryConfig(),
    thresholds: loadThresholdConfig(),
  };
}

/**
 * Loads agent identification configuration
 */
function loadAgentConfig(): AgentConfig {
  return {
    id: AGENT_ID,
    version: AGENT_VERSION,
    classification: AGENT_CLASSIFICATION,
  };
}

/**
 * Loads generator configuration from environment
 */
function loadGeneratorConfig(): GeneratorConfig {
  return {
    defaultTheme: parseEnum(
      process.env.VIZ_DEFAULT_THEME,
      ['light', 'dark', 'system', 'observatory', 'minimal'],
      DEFAULT_GENERATOR_CONFIG.defaultTheme
    ) as GeneratorConfig['defaultTheme'],
    defaultColorScheme: parseEnum(
      process.env.VIZ_DEFAULT_COLOR_SCHEME,
      ['default', 'categorical', 'sequential', 'diverging', 'status', 'heatmap'],
      DEFAULT_GENERATOR_CONFIG.defaultColorScheme
    ) as GeneratorConfig['defaultColorScheme'],
    maxSeriesPerChart: parseInt(process.env.VIZ_MAX_SERIES ?? '', 10) || DEFAULT_GENERATOR_CONFIG.maxSeriesPerChart,
    maxDimensions: parseInt(process.env.VIZ_MAX_DIMENSIONS ?? '', 10) || DEFAULT_GENERATOR_CONFIG.maxDimensions,
    defaultAnimations: parseBool(process.env.VIZ_DEFAULT_ANIMATIONS, DEFAULT_GENERATOR_CONFIG.defaultAnimations),
    defaultResponsive: parseBool(process.env.VIZ_DEFAULT_RESPONSIVE, DEFAULT_GENERATOR_CONFIG.defaultResponsive),
  };
}

/**
 * Loads RuVector client configuration from environment
 */
function loadRuvectorConfig(): RuvectorConfig {
  return {
    endpoint: process.env.RUVECTOR_ENDPOINT ?? DEFAULT_RUVECTOR_CONFIG.endpoint,
    apiKey: process.env.RUVECTOR_API_KEY ?? DEFAULT_RUVECTOR_CONFIG.apiKey,
    timeoutMs: parseInt(process.env.RUVECTOR_TIMEOUT_MS ?? '', 10) || DEFAULT_RUVECTOR_CONFIG.timeoutMs,
    maxRetries: parseInt(process.env.RUVECTOR_MAX_RETRIES ?? '', 10) || DEFAULT_RUVECTOR_CONFIG.maxRetries,
    retryBaseDelayMs: parseInt(process.env.RUVECTOR_RETRY_BASE_DELAY_MS ?? '', 10) || DEFAULT_RUVECTOR_CONFIG.retryBaseDelayMs,
    poolSize: parseInt(process.env.RUVECTOR_POOL_SIZE ?? '', 10) || DEFAULT_RUVECTOR_CONFIG.poolSize,
  };
}

/**
 * Loads telemetry configuration from environment
 */
function loadTelemetryConfig(): TelemetryConfig {
  return {
    metricsPrefix: process.env.TELEMETRY_METRICS_PREFIX ?? DEFAULT_TELEMETRY_CONFIG.metricsPrefix,
    spanPrefix: process.env.TELEMETRY_SPAN_PREFIX ?? DEFAULT_TELEMETRY_CONFIG.spanPrefix,
    prometheusEnabled: parseBool(process.env.TELEMETRY_PROMETHEUS_ENABLED, DEFAULT_TELEMETRY_CONFIG.prometheusEnabled),
    openTelemetryEnabled: parseBool(process.env.TELEMETRY_OTEL_ENABLED, DEFAULT_TELEMETRY_CONFIG.openTelemetryEnabled),
    logLevel: parseEnum(
      process.env.LOG_LEVEL,
      ['debug', 'info', 'warn', 'error'],
      DEFAULT_TELEMETRY_CONFIG.logLevel
    ) as TelemetryConfig['logLevel'],
  };
}

/**
 * Loads threshold configuration from environment
 */
function loadThresholdConfig(): ThresholdConfig {
  return {
    maxSpecsPerBatch: parseInt(process.env.VIZ_MAX_SPECS_PER_BATCH ?? '', 10) || DEFAULT_THRESHOLD_CONFIG.maxSpecsPerBatch,
    maxSeriesPerChart: parseInt(process.env.VIZ_MAX_SERIES_PER_CHART ?? '', 10) || DEFAULT_THRESHOLD_CONFIG.maxSeriesPerChart,
    maxDimensionsPerChart: parseInt(process.env.VIZ_MAX_DIMENSIONS_PER_CHART ?? '', 10) || DEFAULT_THRESHOLD_CONFIG.maxDimensionsPerChart,
    maxTimeRangeDays: parseInt(process.env.VIZ_MAX_TIME_RANGE_DAYS ?? '', 10) || DEFAULT_THRESHOLD_CONFIG.maxTimeRangeDays,
    requestTimeoutMs: parseInt(process.env.REQUEST_TIMEOUT_MS ?? '', 10) || DEFAULT_THRESHOLD_CONFIG.requestTimeoutMs,
  };
}

// =============================================================================
// Color Palette Helpers
// =============================================================================

/**
 * Gets the color palette for a given theme
 */
export function getColorPalette(theme: string): ColorPalette {
  return DEFAULT_COLOR_PALETTES[theme] ?? DEFAULT_COLOR_PALETTES['observatory'];
}

/**
 * Gets a color from the palette by index (wraps around)
 */
export function getSeriesColor(palette: ColorPalette, index: number): string {
  const colors = palette.primary;
  return colors[index % colors.length];
}

// =============================================================================
// Utility Functions
// =============================================================================

/**
 * Parses an environment variable as an enum value
 */
function parseEnum<T extends string>(
  value: string | undefined,
  allowed: readonly T[],
  defaultValue: T
): T {
  if (!value) return defaultValue;
  return allowed.includes(value as T) ? (value as T) : defaultValue;
}

/**
 * Parses an environment variable as a boolean
 */
function parseBool(value: string | undefined, defaultValue: boolean): boolean {
  if (!value) return defaultValue;
  return value.toLowerCase() === 'true' || value === '1';
}

// =============================================================================
// Configuration Validation
// =============================================================================

/**
 * Validates the loaded configuration
 */
export function validateConfig(config: Config): { valid: true } | { valid: false; errors: string[] } {
  const errors: string[] = [];

  // Validate RuVector config
  if (!config.ruvector.endpoint) {
    errors.push('RUVECTOR_ENDPOINT is required');
  }

  // Validate thresholds
  if (config.thresholds.maxSpecsPerBatch < 1 || config.thresholds.maxSpecsPerBatch > 100) {
    errors.push('maxSpecsPerBatch must be between 1 and 100');
  }

  if (config.thresholds.maxSeriesPerChart < 1 || config.thresholds.maxSeriesPerChart > 50) {
    errors.push('maxSeriesPerChart must be between 1 and 50');
  }

  if (config.thresholds.requestTimeoutMs < 1000 || config.thresholds.requestTimeoutMs > 300000) {
    errors.push('requestTimeoutMs must be between 1000 and 300000');
  }

  if (errors.length > 0) {
    return { valid: false, errors };
  }

  return { valid: true };
}

// =============================================================================
// Singleton Instance
// =============================================================================

let configInstance: Config | null = null;

/**
 * Gets the configuration singleton (lazy-loaded)
 */
export function getConfig(): Config {
  if (!configInstance) {
    configInstance = loadConfig();
  }
  return configInstance;
}

/**
 * Resets the configuration (for testing)
 */
export function resetConfig(): void {
  configInstance = null;
}
