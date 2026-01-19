/**
 * Health Check Agent - RuVector Client
 *
 * HTTP client for persisting decision events and fetching telemetry
 * from ruvector-service.
 *
 * CONSTITUTIONAL CONSTRAINT:
 * - All persistence MUST go through this client
 * - NO direct database access is allowed
 */

import type { RuvectorConfig } from './config.js';
import type {
  HealthCheckDecisionEvent,
  TelemetryAggregatesInput,
  TargetSpec,
} from '../contracts/schemas.js';

// ============================================================================
// CONNECTION POOL
// ============================================================================

interface ConnectionPool {
  active: number;
  max: number;
}

// Simple connection pool tracking (for concurrency limiting)
const pool: ConnectionPool = {
  active: 0,
  max: 5,
};

async function acquireConnection(maxWaitMs: number = 30000): Promise<void> {
  const startTime = Date.now();

  while (pool.active >= pool.max) {
    if (Date.now() - startTime > maxWaitMs) {
      throw new Error('Connection pool exhausted - timeout waiting for available connection');
    }
    await new Promise(resolve => setTimeout(resolve, 100));
  }

  pool.active++;
}

function releaseConnection(): void {
  pool.active = Math.max(0, pool.active - 1);
}

// ============================================================================
// RETRY LOGIC
// ============================================================================

interface RetryConfig {
  maxRetries: number;
  baseDelayMs: number;
  maxDelayMs: number;
}

async function withRetry<T>(
  operation: () => Promise<T>,
  config: RetryConfig
): Promise<T> {
  let lastError: Error | null = null;

  for (let attempt = 0; attempt <= config.maxRetries; attempt++) {
    try {
      return await operation();
    } catch (error) {
      lastError = error instanceof Error ? error : new Error(String(error));

      if (attempt < config.maxRetries) {
        // Exponential backoff with jitter
        const delay = Math.min(
          config.baseDelayMs * Math.pow(2, attempt) + Math.random() * 100,
          config.maxDelayMs
        );
        await new Promise(resolve => setTimeout(resolve, delay));
      }
    }
  }

  throw lastError ?? new Error('Retry failed with unknown error');
}

// ============================================================================
// PERSIST RESULT
// ============================================================================

export interface PersistResult {
  success: boolean;
  event_id?: string;
  error?: string;
  retries?: number;
}

export interface BatchPersistResult {
  success: boolean;
  persisted_count: number;
  failed_count: number;
  errors?: string[];
}

// ============================================================================
// HEALTH STATUS
// ============================================================================

export interface HealthStatus {
  status: 'healthy' | 'degraded' | 'unhealthy';
  latency_ms: number;
  error?: string;
}

// ============================================================================
// QUERY INTERFACE
// ============================================================================

export interface DecisionQuery {
  agent_id?: string;
  target_id?: string;
  target_type?: string;
  from_timestamp?: string;
  to_timestamp?: string;
  limit?: number;
}

// ============================================================================
// RUVECTOR CLIENT
// ============================================================================

export class RuvectorClient {
  private readonly endpoint: string;
  private readonly apiKey: string;
  private readonly timeoutMs: number;
  private readonly retryConfig: RetryConfig;

  constructor(config: RuvectorConfig) {
    this.endpoint = config.endpoint.replace(/\/$/, ''); // Remove trailing slash
    this.apiKey = config.apiKey;
    this.timeoutMs = config.timeout_ms;
    this.retryConfig = {
      maxRetries: config.max_retries,
      baseDelayMs: config.retry_base_delay_ms,
      maxDelayMs: 10000,
    };

    // Update pool size from config
    pool.max = config.pool_size;
  }

  /**
   * Persist a single decision event to ruvector-service.
   */
  async persistDecisionEvent(event: HealthCheckDecisionEvent): Promise<PersistResult> {
    try {
      await acquireConnection(this.timeoutMs);

      let retries = 0;
      const result = await withRetry(async () => {
        retries++;
        const response = await fetch(`${this.endpoint}/api/events`, {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
            'Authorization': `Bearer ${this.apiKey}`,
            'X-Agent-ID': event.agent_id,
            'X-Agent-Version': event.agent_version,
          },
          body: JSON.stringify(event),
          signal: AbortSignal.timeout(this.timeoutMs),
        });

        if (!response.ok) {
          const errorText = await response.text();
          throw new Error(`HTTP ${response.status}: ${errorText}`);
        }

        return response.json() as Promise<{ event_id: string }>;
      }, this.retryConfig);

      return {
        success: true,
        event_id: result.event_id,
        retries: retries - 1,
      };

    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : String(error),
      };
    } finally {
      releaseConnection();
    }
  }

  /**
   * Persist multiple decision events in a batch.
   */
  async persistDecisionEventsBatch(
    events: HealthCheckDecisionEvent[]
  ): Promise<BatchPersistResult> {
    const results = await Promise.allSettled(
      events.map(event => this.persistDecisionEvent(event))
    );

    const errors: string[] = [];
    let persistedCount = 0;
    let failedCount = 0;

    for (const result of results) {
      if (result.status === 'fulfilled' && result.value.success) {
        persistedCount++;
      } else {
        failedCount++;
        if (result.status === 'rejected') {
          errors.push(result.reason instanceof Error ? result.reason.message : String(result.reason));
        } else if (result.value.error) {
          errors.push(result.value.error);
        }
      }
    }

    return {
      success: failedCount === 0,
      persisted_count: persistedCount,
      failed_count: failedCount,
      errors: errors.length > 0 ? errors : undefined,
    };
  }

  /**
   * Fetch telemetry aggregates for targets.
   */
  async fetchTelemetryAggregates(
    targets: TargetSpec[],
    windowStart: string,
    windowEnd: string
  ): Promise<Map<string, TelemetryAggregatesInput>> {
    const results = new Map<string, TelemetryAggregatesInput>();

    try {
      await acquireConnection(this.timeoutMs);

      const response = await withRetry(async () => {
        const res = await fetch(`${this.endpoint}/api/telemetry/aggregates`, {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
            'Authorization': `Bearer ${this.apiKey}`,
          },
          body: JSON.stringify({
            targets: targets.map(t => ({ type: t.type, id: t.id })),
            window_start: windowStart,
            window_end: windowEnd,
          }),
          signal: AbortSignal.timeout(this.timeoutMs),
        });

        if (!res.ok) {
          const errorText = await res.text();
          throw new Error(`HTTP ${res.status}: ${errorText}`);
        }

        return res.json() as Promise<{ aggregates: TelemetryAggregatesInput[] }>;
      }, this.retryConfig);

      for (const aggregate of response.aggregates) {
        const key = `${aggregate.target_type}:${aggregate.target_id}`;
        results.set(key, aggregate);
      }

    } catch (error) {
      // Log error but return empty map (don't fail entire operation)
      console.error('Failed to fetch telemetry aggregates:', error);
    } finally {
      releaseConnection();
    }

    return results;
  }

  /**
   * Query decision events.
   */
  async getDecisionEvents(query: DecisionQuery): Promise<HealthCheckDecisionEvent[]> {
    try {
      await acquireConnection(this.timeoutMs);

      const params = new URLSearchParams();
      if (query.agent_id) params.set('agent_id', query.agent_id);
      if (query.target_id) params.set('target_id', query.target_id);
      if (query.target_type) params.set('target_type', query.target_type);
      if (query.from_timestamp) params.set('from', query.from_timestamp);
      if (query.to_timestamp) params.set('to', query.to_timestamp);
      if (query.limit) params.set('limit', String(query.limit));

      const response = await withRetry(async () => {
        const res = await fetch(`${this.endpoint}/api/events?${params}`, {
          method: 'GET',
          headers: {
            'Authorization': `Bearer ${this.apiKey}`,
          },
          signal: AbortSignal.timeout(this.timeoutMs),
        });

        if (!res.ok) {
          const errorText = await res.text();
          throw new Error(`HTTP ${res.status}: ${errorText}`);
        }

        return res.json() as Promise<{ events: HealthCheckDecisionEvent[] }>;
      }, this.retryConfig);

      return response.events;

    } catch (error) {
      console.error('Failed to query decision events:', error);
      return [];
    } finally {
      releaseConnection();
    }
  }

  /**
   * Health check for ruvector-service connectivity.
   */
  async healthCheck(): Promise<HealthStatus> {
    const startTime = Date.now();

    try {
      const response = await fetch(`${this.endpoint}/health`, {
        method: 'GET',
        headers: {
          'Authorization': `Bearer ${this.apiKey}`,
        },
        signal: AbortSignal.timeout(5000), // 5 second timeout for health check
      });

      const latencyMs = Date.now() - startTime;

      if (response.ok) {
        return {
          status: latencyMs < 100 ? 'healthy' : (latencyMs < 1000 ? 'degraded' : 'unhealthy'),
          latency_ms: latencyMs,
        };
      }

      return {
        status: 'unhealthy',
        latency_ms: latencyMs,
        error: `HTTP ${response.status}`,
      };

    } catch (error) {
      return {
        status: 'unhealthy',
        latency_ms: Date.now() - startTime,
        error: error instanceof Error ? error.message : String(error),
      };
    }
  }

  /**
   * Get pool statistics.
   */
  getPoolStats(): { active: number; max: number; utilization: number } {
    return {
      active: pool.active,
      max: pool.max,
      utilization: pool.max > 0 ? pool.active / pool.max : 0,
    };
  }
}

// ============================================================================
// SINGLETON INSTANCE
// ============================================================================

let clientInstance: RuvectorClient | null = null;

/**
 * Initialize the RuVector client singleton.
 */
export function initializeClient(config: RuvectorConfig): RuvectorClient {
  clientInstance = new RuvectorClient(config);
  return clientInstance;
}

/**
 * Get the RuVector client singleton.
 */
export function getClient(): RuvectorClient {
  if (!clientInstance) {
    throw new Error('RuVector client not initialized. Call initializeClient first.');
  }
  return clientInstance;
}
