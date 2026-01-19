/**
 * Health Check Endpoints
 *
 * Required for Cloud Run deployment.
 */

import { RuvectorClient } from './ruvector-client.js';
import { getRuvectorConfig, loadConfig } from './config.js';

let ruvectorClient: RuvectorClient | null = null;

function getClient(): RuvectorClient {
  if (!ruvectorClient) {
    ruvectorClient = new RuvectorClient(getRuvectorConfig());
  }
  return ruvectorClient;
}

export interface HealthCheckResult {
  status: 'healthy' | 'degraded' | 'unhealthy';
  service: string;
  version: string;
  environment: string;
  classification: string;
  components: {
    ruvector: {
      healthy: boolean;
      latencyMs: number;
      endpoint: string;
      error?: string;
    };
  };
  agents: {
    id: string;
    status: 'ready';
  }[];
  timestamp: string;
}

export interface ReadinessResult {
  ready: boolean;
  checks: {
    name: string;
    passed: boolean;
    error?: string;
  }[];
  timestamp: string;
}

/**
 * Full health check including all dependencies
 */
export async function healthCheck(): Promise<HealthCheckResult> {
  const config = loadConfig();
  const client = getClient();

  const ruvectorHealth = await client.healthCheck();

  const agents = [
    { id: 'telemetry-collector-agent', status: 'ready' as const },
    { id: 'usage-pattern-agent', status: 'ready' as const },
    { id: 'failure-classification-agent', status: 'ready' as const },
    { id: 'health-check-agent', status: 'ready' as const },
    { id: 'slo-enforcement-agent', status: 'ready' as const },
    { id: 'post-mortem-generator-agent', status: 'ready' as const },
    { id: 'visualization-spec-agent', status: 'ready' as const },
  ];

  const status = ruvectorHealth.healthy ? 'healthy' : 'degraded';

  return {
    status,
    service: config.serviceName,
    version: config.serviceVersion,
    environment: config.environment,
    classification: 'OBSERVATION-ONLY',
    components: {
      ruvector: {
        healthy: ruvectorHealth.healthy,
        latencyMs: ruvectorHealth.latencyMs,
        endpoint: ruvectorHealth.endpoint,
        error: ruvectorHealth.error,
      },
    },
    agents,
    timestamp: new Date().toISOString(),
  };
}

/**
 * Kubernetes/Cloud Run readiness probe
 */
export async function readinessCheck(): Promise<ReadinessResult> {
  const checks: ReadinessResult['checks'] = [];

  // Check ruvector connectivity
  try {
    const client = getClient();
    const health = await client.healthCheck();
    checks.push({
      name: 'ruvector-service',
      passed: health.healthy,
      error: health.error,
    });
  } catch (error) {
    checks.push({
      name: 'ruvector-service',
      passed: false,
      error: error instanceof Error ? error.message : String(error),
    });
  }

  // Check environment variables
  const requiredEnvVars = ['RUVECTOR_SERVICE_URL'];
  for (const envVar of requiredEnvVars) {
    checks.push({
      name: `env:${envVar}`,
      passed: !!process.env[envVar],
      error: !process.env[envVar] ? 'Missing required environment variable' : undefined,
    });
  }

  const allPassed = checks.every((c) => c.passed);

  return {
    ready: allPassed,
    checks,
    timestamp: new Date().toISOString(),
  };
}
