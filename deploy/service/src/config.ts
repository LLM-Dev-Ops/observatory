/**
 * Environment Configuration
 *
 * All configuration resolved via environment variables or Secret Manager.
 * NO hardcoded service names, URLs, or credentials.
 */

export interface ServiceConfig {
  // Service identity
  serviceName: string;
  serviceVersion: string;
  environment: 'dev' | 'staging' | 'prod';

  // RuVector service (memory/persistence layer)
  ruvectorServiceUrl: string;
  ruvectorApiKey?: string;

  // Telemetry
  telemetryEndpoint?: string;

  // Logging
  logLevel: string;

  // Agent-specific
  selfObservationEnabled: boolean;
  maxBatchSize: number;
  requestTimeoutMs: number;
}

/**
 * Load configuration from environment variables
 */
export function loadConfig(): ServiceConfig {
  return {
    serviceName: process.env.SERVICE_NAME || 'llm-observatory',
    serviceVersion: process.env.SERVICE_VERSION || '1.0.0',
    environment: (process.env.PLATFORM_ENV || 'dev') as ServiceConfig['environment'],

    ruvectorServiceUrl: process.env.RUVECTOR_SERVICE_URL || 'http://localhost:8081',
    ruvectorApiKey: process.env.RUVECTOR_API_KEY,

    telemetryEndpoint: process.env.TELEMETRY_ENDPOINT,

    logLevel: process.env.LOG_LEVEL || 'info',

    selfObservationEnabled: process.env.SELF_OBSERVATION_ENABLED !== 'false',
    maxBatchSize: parseInt(process.env.MAX_BATCH_SIZE || '100', 10),
    requestTimeoutMs: parseInt(process.env.REQUEST_TIMEOUT_MS || '30000', 10),
  };
}

/**
 * Validate required environment variables
 */
export function validateEnvironment(): string[] {
  const errors: string[] = [];
  const required = [
    'RUVECTOR_SERVICE_URL',
  ];

  for (const envVar of required) {
    if (!process.env[envVar]) {
      errors.push(`Missing required environment variable: ${envVar}`);
    }
  }

  // Validate PLATFORM_ENV if set
  const env = process.env.PLATFORM_ENV;
  if (env && !['dev', 'staging', 'prod'].includes(env)) {
    errors.push(`PLATFORM_ENV must be one of: dev, staging, prod (got: ${env})`);
  }

  return errors;
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
