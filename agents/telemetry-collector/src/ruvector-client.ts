/**
 * Ruvector-service HTTP client
 * CONSTITUTION: All persistence goes through this client - NO direct database access
 */

import {
  RuvectorConfig,
  PersistResult,
  DecisionQuery,
  HealthStatus,
  BatchPersistRequest,
  BatchPersistResult,
} from './types/ruvector.js';

/**
 * HTTP client for ruvector-service
 * Handles async, non-blocking writes with retry logic and connection pooling
 */
export class RuvectorClient {
  private config: RuvectorConfig;
  private activeConnections: number = 0;
  private requestQueue: Array<() => Promise<void>> = [];

  constructor(config: RuvectorConfig) {
    this.config = config;
  }

  /**
   * Persist a single decision event
   */
  async persistDecisionEvent(event: any): Promise<PersistResult> {
    const startTime = Date.now();
    let retries = 0;

    while (retries <= this.config.retryAttempts) {
      try {
        await this.acquireConnection();

        const response = await this.fetchWithTimeout(
          `${this.config.endpoint}/api/events`,
          {
            method: 'POST',
            headers: {
              'Content-Type': 'application/json',
              ...(this.config.apiKey && { Authorization: `Bearer ${this.config.apiKey}` }),
            },
            body: JSON.stringify(event),
          },
          this.config.timeout
        );

        this.releaseConnection();

        if (!response.ok) {
          throw new Error(`HTTP ${response.status}: ${response.statusText}`);
        }

        const data = await response.json();

        return {
          success: true,
          eventId: data.eventId || data.id || event.eventId,
          timestamp: new Date(),
          retries,
        };
      } catch (error) {
        this.releaseConnection();

        if (retries < this.config.retryAttempts) {
          const delay = this.calculateBackoff(retries);
          await this.sleep(delay);
          retries++;
        } else {
          return {
            success: false,
            eventId: event.eventId || '',
            timestamp: new Date(),
            error: error instanceof Error ? error.message : String(error),
            retries,
          };
        }
      }
    }

    return {
      success: false,
      eventId: event.eventId || '',
      timestamp: new Date(),
      error: 'Max retries exceeded',
      retries,
    };
  }

  /**
   * Persist multiple decision events in batch
   */
  async persistDecisionEventsBatch(request: BatchPersistRequest): Promise<BatchPersistResult> {
    const startTime = Date.now();

    try {
      await this.acquireConnection();

      const response = await this.fetchWithTimeout(
        `${this.config.endpoint}/api/events/batch`,
        {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
            ...(this.config.apiKey && { Authorization: `Bearer ${this.config.apiKey}` }),
          },
          body: JSON.stringify(request),
        },
        this.config.timeout
      );

      this.releaseConnection();

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      }

      const data = await response.json();

      return {
        total: request.events.length,
        successful: data.successful || 0,
        failed: data.failed || 0,
        results: data.results || [],
        errors: data.errors || [],
      };
    } catch (error) {
      this.releaseConnection();

      return {
        total: request.events.length,
        successful: 0,
        failed: request.events.length,
        results: [],
        errors: [
          {
            index: -1,
            error: error instanceof Error ? error.message : String(error),
          },
        ],
      };
    }
  }

  /**
   * Get decision events matching query
   */
  async getDecisionEvents(query: DecisionQuery): Promise<any[]> {
    try {
      await this.acquireConnection();

      const queryParams = new URLSearchParams();
      if (query.agentId) queryParams.set('agentId', query.agentId);
      if (query.agentVersion) queryParams.set('agentVersion', query.agentVersion);
      if (query.startTime) queryParams.set('startTime', query.startTime.toISOString());
      if (query.endTime) queryParams.set('endTime', query.endTime.toISOString());
      if (query.eventTypes) queryParams.set('eventTypes', query.eventTypes.join(','));
      if (query.limit) queryParams.set('limit', query.limit.toString());
      if (query.offset) queryParams.set('offset', query.offset.toString());
      if (query.sortBy) queryParams.set('sortBy', query.sortBy);
      if (query.sortOrder) queryParams.set('sortOrder', query.sortOrder);

      const url = `${this.config.endpoint}/api/events?${queryParams.toString()}`;

      const response = await this.fetchWithTimeout(
        url,
        {
          method: 'GET',
          headers: {
            ...(this.config.apiKey && { Authorization: `Bearer ${this.config.apiKey}` }),
          },
        },
        this.config.timeout
      );

      this.releaseConnection();

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      }

      const data = await response.json();
      return data.events || data || [];
    } catch (error) {
      this.releaseConnection();
      throw error;
    }
  }

  /**
   * Health check for ruvector-service
   */
  async healthCheck(): Promise<HealthStatus> {
    const startTime = Date.now();

    try {
      const response = await this.fetchWithTimeout(
        `${this.config.endpoint}/health`,
        {
          method: 'GET',
          headers: {
            ...(this.config.apiKey && { Authorization: `Bearer ${this.config.apiKey}` }),
          },
        },
        this.config.timeout
      );

      const latencyMs = Date.now() - startTime;

      if (!response.ok) {
        return {
          healthy: false,
          latencyMs,
          lastCheck: new Date(),
          endpoint: this.config.endpoint,
          error: `HTTP ${response.status}: ${response.statusText}`,
        };
      }

      const data = await response.json();

      return {
        healthy: true,
        latencyMs,
        lastCheck: new Date(),
        endpoint: this.config.endpoint,
        version: data.version,
      };
    } catch (error) {
      return {
        healthy: false,
        latencyMs: Date.now() - startTime,
        lastCheck: new Date(),
        endpoint: this.config.endpoint,
        error: error instanceof Error ? error.message : String(error),
      };
    }
  }

  /**
   * Connection pool management - acquire connection
   */
  private async acquireConnection(): Promise<void> {
    while (this.activeConnections >= this.config.connectionPoolSize) {
      await this.sleep(100);
    }
    this.activeConnections++;
  }

  /**
   * Connection pool management - release connection
   */
  private releaseConnection(): void {
    this.activeConnections = Math.max(0, this.activeConnections - 1);
  }

  /**
   * Fetch with timeout
   */
  private async fetchWithTimeout(
    url: string,
    options: RequestInit,
    timeoutMs: number
  ): Promise<Response> {
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), timeoutMs);

    try {
      const response = await fetch(url, {
        ...options,
        signal: controller.signal,
      });
      clearTimeout(timeoutId);
      return response;
    } catch (error) {
      clearTimeout(timeoutId);
      if ((error as Error).name === 'AbortError') {
        throw new Error(`Request timeout after ${timeoutMs}ms`);
      }
      throw error;
    }
  }

  /**
   * Calculate exponential backoff delay
   */
  private calculateBackoff(retryCount: number): number {
    const delay = this.config.retryDelayMs * Math.pow(2, retryCount);
    return Math.min(delay, this.config.maxRetryDelayMs);
  }

  /**
   * Sleep utility
   */
  private sleep(ms: number): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, ms));
  }

  /**
   * Get current connection pool status
   */
  getConnectionPoolStatus(): {
    active: number;
    max: number;
    available: number;
    queueSize: number;
  } {
    return {
      active: this.activeConnections,
      max: this.config.connectionPoolSize,
      available: this.config.connectionPoolSize - this.activeConnections,
      queueSize: this.requestQueue.length,
    };
  }
}
