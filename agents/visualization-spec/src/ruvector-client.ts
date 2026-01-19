/**
 * Visualization Spec Agent - RuVector Client
 *
 * HTTP client for persisting decision events to ruvector-service.
 * This is the ONLY persistence mechanism - no direct database access allowed.
 *
 * Classification: READ-ONLY (agent does not modify system state)
 */

import { getConfig } from './config.js';
import type { RuvectorConfig } from '../contracts/types.js';
import {
  AGENT_ID,
  type VisualizationDecisionEvent,
  type VisualizationSpec,
} from '../contracts/schemas.js';

// =============================================================================
// Types
// =============================================================================

export interface PersistResult {
  success: boolean;
  eventId?: string;
  error?: string;
}

export interface QueryResult {
  success: boolean;
  events: VisualizationDecisionEvent[];
  total: number;
  error?: string;
}

export interface HealthCheckResult {
  status: 'healthy' | 'unhealthy';
  latencyMs: number;
  error?: string;
}

interface RuvectorPersistRequest {
  event_type: 'decision_event';
  agent_id: typeof AGENT_ID;
  payload: VisualizationDecisionEvent;
}

interface RuvectorQueryRequest {
  agent_id: typeof AGENT_ID;
  filters?: {
    spec_id?: string;
    execution_ref?: string;
    time_range?: {
      start: string;
      end: string;
    };
  };
  limit?: number;
}

// =============================================================================
// Connection Pool
// =============================================================================

interface ConnectionPool {
  active: number;
  max: number;
}

const pool: ConnectionPool = {
  active: 0,
  max: 5,
};

/**
 * Acquires a connection slot from the pool
 */
async function acquireConnection(): Promise<void> {
  const config = getConfig();
  pool.max = config.ruvector.poolSize;

  while (pool.active >= pool.max) {
    await new Promise(resolve => setTimeout(resolve, 10));
  }
  pool.active++;
}

/**
 * Releases a connection slot back to the pool
 */
function releaseConnection(): void {
  pool.active = Math.max(0, pool.active - 1);
}

// =============================================================================
// Retry Logic
// =============================================================================

interface RetryConfig {
  maxRetries: number;
  baseDelayMs: number;
}

/**
 * Executes an operation with exponential backoff retry
 */
async function withRetry<T>(
  operation: () => Promise<T>,
  config: RetryConfig
): Promise<T> {
  let lastError: Error | null = null;

  for (let attempt = 0; attempt <= config.maxRetries; attempt++) {
    try {
      return await operation();
    } catch (error) {
      lastError = error as Error;

      if (attempt < config.maxRetries) {
        // Exponential backoff with jitter
        const delay = config.baseDelayMs * Math.pow(2, attempt);
        const jitter = Math.random() * delay * 0.1;
        await new Promise(resolve => setTimeout(resolve, delay + jitter));
      }
    }
  }

  throw lastError;
}

// =============================================================================
// RuVector Client
// =============================================================================

export class RuvectorClient {
  private readonly config: RuvectorConfig;

  constructor(config?: RuvectorConfig) {
    this.config = config ?? getConfig().ruvector;
  }

  /**
   * Persists a decision event to ruvector-service
   *
   * This is the ONLY write operation performed by this agent.
   * All decision events are immutable records for audit purposes.
   */
  async persistDecisionEvent(event: VisualizationDecisionEvent): Promise<PersistResult> {
    await acquireConnection();

    try {
      const request: RuvectorPersistRequest = {
        event_type: 'decision_event',
        agent_id: AGENT_ID,
        payload: event,
      };

      const result = await withRetry(
        () => this.sendRequest('POST', '/decision-events', request),
        {
          maxRetries: this.config.maxRetries,
          baseDelayMs: this.config.retryBaseDelayMs,
        }
      );

      return {
        success: true,
        eventId: result.event_id,
      };
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Unknown error';
      return {
        success: false,
        error: `Failed to persist decision event: ${errorMessage}`,
      };
    } finally {
      releaseConnection();
    }
  }

  /**
   * Queries past decision events for inspection/replay
   */
  async queryDecisionEvents(
    filters?: {
      specId?: string;
      executionRef?: string;
      startTime?: string;
      endTime?: string;
    },
    limit: number = 10
  ): Promise<QueryResult> {
    await acquireConnection();

    try {
      const request: RuvectorQueryRequest = {
        agent_id: AGENT_ID,
        filters: filters ? {
          spec_id: filters.specId,
          execution_ref: filters.executionRef,
          time_range: filters.startTime && filters.endTime ? {
            start: filters.startTime,
            end: filters.endTime,
          } : undefined,
        } : undefined,
        limit,
      };

      const result = await withRetry(
        () => this.sendRequest('POST', '/decision-events/query', request),
        {
          maxRetries: this.config.maxRetries,
          baseDelayMs: this.config.retryBaseDelayMs,
        }
      );

      return {
        success: true,
        events: result.events ?? [],
        total: result.total ?? 0,
      };
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Unknown error';
      return {
        success: false,
        events: [],
        total: 0,
        error: `Failed to query decision events: ${errorMessage}`,
      };
    } finally {
      releaseConnection();
    }
  }

  /**
   * Retrieves a specific decision event by spec ID
   */
  async getDecisionEventBySpecId(specId: string): Promise<{
    success: boolean;
    event?: VisualizationDecisionEvent;
    error?: string;
  }> {
    const result = await this.queryDecisionEvents({ specId }, 1);

    if (!result.success) {
      return { success: false, error: result.error };
    }

    if (result.events.length === 0) {
      return { success: false, error: `No decision event found for spec_id: ${specId}` };
    }

    return { success: true, event: result.events[0] };
  }

  /**
   * Checks RuVector service health
   */
  async healthCheck(): Promise<HealthCheckResult> {
    const startTime = Date.now();

    try {
      await this.sendRequest('GET', '/health', undefined, 5000);
      return {
        status: 'healthy',
        latencyMs: Date.now() - startTime,
      };
    } catch (error) {
      return {
        status: 'unhealthy',
        latencyMs: Date.now() - startTime,
        error: error instanceof Error ? error.message : 'Unknown error',
      };
    }
  }

  /**
   * Sends an HTTP request to ruvector-service
   */
  private async sendRequest(
    method: string,
    path: string,
    body?: unknown,
    timeoutOverride?: number
  ): Promise<Record<string, unknown>> {
    const url = `${this.config.endpoint}${path}`;
    const timeout = timeoutOverride ?? this.config.timeoutMs;

    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), timeout);

    try {
      const response = await fetch(url, {
        method,
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${this.config.apiKey}`,
          'X-Agent-ID': AGENT_ID,
        },
        body: body ? JSON.stringify(body) : undefined,
        signal: controller.signal,
      });

      clearTimeout(timeoutId);

      if (!response.ok) {
        const errorBody = await response.text();
        throw new Error(`HTTP ${response.status}: ${errorBody}`);
      }

      return await response.json();
    } catch (error) {
      clearTimeout(timeoutId);

      if (error instanceof Error && error.name === 'AbortError') {
        throw new Error(`Request timeout after ${timeout}ms`);
      }

      throw error;
    }
  }
}

// =============================================================================
// Singleton Instance
// =============================================================================

let clientInstance: RuvectorClient | null = null;

/**
 * Gets the RuVector client singleton
 */
export function getRuvectorClient(): RuvectorClient {
  if (!clientInstance) {
    clientInstance = new RuvectorClient();
  }
  return clientInstance;
}

/**
 * Resets the client (for testing)
 */
export function resetRuvectorClient(): void {
  clientInstance = null;
}
