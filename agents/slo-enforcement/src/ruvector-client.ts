/**
 * SLO/SLA Enforcement Agent - RuVector Service Client
 *
 * Client for persisting DecisionEvents to ruvector-service.
 * This is the ONLY persistence mechanism for this agent.
 *
 * Constitutional Compliance:
 * - All persistence MUST go through ruvector-service
 * - NO direct SQL access
 * - NO local persistence
 */

import { loadConfig } from './config';
import type { DecisionEvent, ViolationQuery, AnalysisResult, SloViolation } from '../contracts';
import type { RuvectorHealthStatus } from '../types';

/**
 * RuVector Client for SLO Enforcement Agent
 */
export class RuvectorClient {
  private readonly endpoint: string;
  private readonly apiKey?: string;
  private readonly timeout: number;
  private readonly retryAttempts: number;
  private readonly retryDelayMs: number;
  private readonly maxRetryDelayMs: number;
  private readonly connectionPoolSize: number;

  // Connection pool tracking
  private activeConnections = 0;
  private readonly pendingRequests: Array<() => void> = [];

  constructor() {
    const config = loadConfig();
    this.endpoint = config.ruvector.endpoint;
    this.apiKey = config.ruvector.apiKey;
    this.timeout = config.ruvector.timeout;
    this.retryAttempts = config.ruvector.retryAttempts;
    this.retryDelayMs = config.ruvector.retryDelayMs;
    this.maxRetryDelayMs = config.ruvector.maxRetryDelayMs;
    this.connectionPoolSize = config.ruvector.connectionPoolSize;
  }

  /**
   * Persist a DecisionEvent to ruvector-service
   */
  async persistDecisionEvent(event: DecisionEvent): Promise<void> {
    await this.withRetry(async () => {
      await this.acquireConnection();
      try {
        const response = await fetch(`${this.endpoint}/api/v1/decisions`, {
          method: 'POST',
          headers: this.buildHeaders(),
          body: JSON.stringify(event),
          signal: AbortSignal.timeout(this.timeout),
        });

        if (!response.ok) {
          const errorBody = await response.text();
          throw new Error(`Failed to persist DecisionEvent: ${response.status} - ${errorBody}`);
        }
      } finally {
        this.releaseConnection();
      }
    });
  }

  /**
   * Persist multiple DecisionEvents in batch
   */
  async persistDecisionEventsBatch(events: DecisionEvent[]): Promise<void> {
    if (events.length === 0) return;

    await this.withRetry(async () => {
      await this.acquireConnection();
      try {
        const response = await fetch(`${this.endpoint}/api/v1/decisions/batch`, {
          method: 'POST',
          headers: this.buildHeaders(),
          body: JSON.stringify({ events }),
          signal: AbortSignal.timeout(this.timeout),
        });

        if (!response.ok) {
          const errorBody = await response.text();
          throw new Error(`Failed to persist DecisionEvents batch: ${response.status} - ${errorBody}`);
        }
      } finally {
        this.releaseConnection();
      }
    });
  }

  /**
   * Query violations from ruvector-service
   */
  async getViolations(query: ViolationQuery): Promise<SloViolation[]> {
    await this.acquireConnection();
    try {
      const params = new URLSearchParams();
      if (query.slo_id) params.set('slo_id', query.slo_id);
      if (query.breach_type) params.set('breach_type', query.breach_type);
      if (query.severity) params.set('severity', query.severity);
      if (query.provider) params.set('provider', query.provider);
      if (query.model) params.set('model', query.model);
      if (query.environment) params.set('environment', query.environment);
      if (query.start_time) params.set('start_time', query.start_time);
      if (query.end_time) params.set('end_time', query.end_time);
      if (query.is_sla !== undefined) params.set('is_sla', String(query.is_sla));
      params.set('limit', String(query.limit));
      params.set('offset', String(query.offset));
      params.set('sort_by', query.sort_by);
      params.set('sort_order', query.sort_order);

      const response = await fetch(
        `${this.endpoint}/api/v1/slo-violations?${params.toString()}`,
        {
          method: 'GET',
          headers: this.buildHeaders(),
          signal: AbortSignal.timeout(this.timeout),
        }
      );

      if (!response.ok) {
        const errorBody = await response.text();
        throw new Error(`Failed to query violations: ${response.status} - ${errorBody}`);
      }

      const data = await response.json() as { violations: SloViolation[] };
      return data.violations;
    } finally {
      this.releaseConnection();
    }
  }

  /**
   * Get DecisionEvent by execution reference
   */
  async getDecisionEventByRef(executionRef: string): Promise<DecisionEvent | null> {
    await this.acquireConnection();
    try {
      const response = await fetch(
        `${this.endpoint}/api/v1/decisions/${executionRef}`,
        {
          method: 'GET',
          headers: this.buildHeaders(),
          signal: AbortSignal.timeout(this.timeout),
        }
      );

      if (response.status === 404) {
        return null;
      }

      if (!response.ok) {
        const errorBody = await response.text();
        throw new Error(`Failed to get DecisionEvent: ${response.status} - ${errorBody}`);
      }

      return await response.json() as DecisionEvent;
    } finally {
      this.releaseConnection();
    }
  }

  /**
   * Get aggregated violation analysis
   */
  async getViolationAnalysis(
    startTime: string,
    endTime: string,
    groupBy: string[]
  ): Promise<AnalysisResult> {
    await this.acquireConnection();
    try {
      const params = new URLSearchParams();
      params.set('start_time', startTime);
      params.set('end_time', endTime);
      params.set('group_by', groupBy.join(','));

      const response = await fetch(
        `${this.endpoint}/api/v1/slo-violations/analysis?${params.toString()}`,
        {
          method: 'GET',
          headers: this.buildHeaders(),
          signal: AbortSignal.timeout(this.timeout),
        }
      );

      if (!response.ok) {
        const errorBody = await response.text();
        throw new Error(`Failed to get analysis: ${response.status} - ${errorBody}`);
      }

      return await response.json() as AnalysisResult;
    } finally {
      this.releaseConnection();
    }
  }

  /**
   * Health check for ruvector-service
   */
  async healthCheck(): Promise<RuvectorHealthStatus> {
    const startTime = Date.now();
    try {
      const response = await fetch(`${this.endpoint}/health`, {
        method: 'GET',
        headers: this.buildHeaders(),
        signal: AbortSignal.timeout(2000), // Short timeout for health check
      });

      const latency = Date.now() - startTime;

      if (!response.ok) {
        return {
          status: 'unhealthy',
          latency_ms: latency,
          error_count_1h: -1,
        };
      }

      const data = await response.json() as { status: string; error_count_1h?: number };
      return {
        status: data.status === 'healthy' ? 'healthy' : 'degraded',
        latency_ms: latency,
        error_count_1h: data.error_count_1h ?? 0,
      };
    } catch (error) {
      return {
        status: 'unhealthy',
        latency_ms: Date.now() - startTime,
        error_count_1h: -1,
      };
    }
  }

  /**
   * Build request headers
   */
  private buildHeaders(): Record<string, string> {
    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
      'User-Agent': 'slo-enforcement-agent/1.0.0',
    };

    if (this.apiKey) {
      headers['Authorization'] = `Bearer ${this.apiKey}`;
    }

    return headers;
  }

  /**
   * Acquire a connection from the pool
   */
  private async acquireConnection(): Promise<void> {
    if (this.activeConnections < this.connectionPoolSize) {
      this.activeConnections++;
      return;
    }

    // Wait for a connection to become available
    return new Promise<void>((resolve) => {
      this.pendingRequests.push(resolve);
    });
  }

  /**
   * Release a connection back to the pool
   */
  private releaseConnection(): void {
    this.activeConnections--;

    // Resume a pending request if any
    const next = this.pendingRequests.shift();
    if (next) {
      this.activeConnections++;
      next();
    }
  }

  /**
   * Get connection pool status
   */
  getConnectionPoolStatus(): { active: number; max: number; available: number; queueSize: number } {
    return {
      active: this.activeConnections,
      max: this.connectionPoolSize,
      available: this.connectionPoolSize - this.activeConnections,
      queueSize: this.pendingRequests.length,
    };
  }

  /**
   * Execute with retry logic
   */
  private async withRetry<T>(fn: () => Promise<T>): Promise<T> {
    let lastError: Error | null = null;
    let delay = this.retryDelayMs;

    for (let attempt = 1; attempt <= this.retryAttempts; attempt++) {
      try {
        return await fn();
      } catch (error) {
        lastError = error as Error;

        // Don't retry on 4xx errors (client errors)
        if (error instanceof Error && error.message.includes('4')) {
          throw error;
        }

        // Last attempt, don't retry
        if (attempt === this.retryAttempts) {
          break;
        }

        // Exponential backoff
        await new Promise((resolve) => setTimeout(resolve, delay));
        delay = Math.min(delay * 2, this.maxRetryDelayMs);
      }
    }

    throw lastError;
  }
}

// Singleton instance
let clientInstance: RuvectorClient | null = null;

/**
 * Get the singleton RuVector client instance
 */
export function getRuvectorClient(): RuvectorClient {
  if (clientInstance === null) {
    clientInstance = new RuvectorClient();
  }
  return clientInstance;
}

/**
 * Reset the client instance (for testing)
 */
export function resetRuvectorClient(): void {
  clientInstance = null;
}

export default getRuvectorClient;
