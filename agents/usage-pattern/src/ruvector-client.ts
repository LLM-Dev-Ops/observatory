/**
 * Ruvector-service HTTP client for Usage Pattern Agent.
 *
 * CONSTITUTION: All persistence goes through this client - NO direct database access.
 * - Async, non-blocking operations
 * - Connection pooling with retry logic
 * - Timeout enforcement
 * - Graceful error handling
 */

import {
  RuvectorConfig,
  PersistResult,
  DecisionQuery,
  TelemetryQuery,
  HealthStatus,
  BatchPersistRequest,
  BatchPersistResult,
  StoredTelemetryEvent,
  AggregatedTelemetryResponse,
} from './types/ruvector.js';

/**
 * HTTP client for ruvector-service.
 * Handles async, non-blocking operations with retry logic and connection pooling.
 */
export class RuvectorClient {
  private config: RuvectorConfig;
  private activeConnections: number = 0;

  constructor(config: RuvectorConfig) {
    this.config = config;
  }

  /**
   * Persist a single decision event.
   */
  async persistDecisionEvent(event: unknown): Promise<PersistResult> {
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
          eventId: data.eventId || data.id || (event as { eventId?: string }).eventId || '',
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
            eventId: (event as { eventId?: string }).eventId || '',
            timestamp: new Date(),
            error: error instanceof Error ? error.message : String(error),
            retries,
          };
        }
      }
    }

    return {
      success: false,
      eventId: '',
      timestamp: new Date(),
      error: 'Max retries exceeded',
      retries,
    };
  }

  /**
   * Persist multiple decision events in batch.
   */
  async persistDecisionEventsBatch(request: BatchPersistRequest): Promise<BatchPersistResult> {
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
   * Get decision events matching query.
   */
  async getDecisionEvents(query: DecisionQuery): Promise<unknown[]> {
    try {
      await this.acquireConnection();

      const queryParams = new URLSearchParams();
      if (query.agentId) queryParams.set('agentId', query.agentId);
      if (query.agentVersion) queryParams.set('agentVersion', query.agentVersion);
      if (query.decisionType) queryParams.set('decisionType', query.decisionType);
      if (query.startTime) queryParams.set('startTime', query.startTime.toISOString());
      if (query.endTime) queryParams.set('endTime', query.endTime.toISOString());
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
   * Get telemetry events for analysis.
   * This retrieves normalized telemetry data from ruvector-service.
   */
  async getTelemetryEvents(query: TelemetryQuery): Promise<AggregatedTelemetryResponse> {
    try {
      await this.acquireConnection();

      const queryParams = new URLSearchParams();
      queryParams.set('startTime', query.startTime.toISOString());
      queryParams.set('endTime', query.endTime.toISOString());
      if (query.providers?.length) queryParams.set('providers', query.providers.join(','));
      if (query.models?.length) queryParams.set('models', query.models.join(','));
      if (query.environments?.length) queryParams.set('environments', query.environments.join(','));
      if (query.userIds?.length) queryParams.set('userIds', query.userIds.join(','));
      if (query.limit) queryParams.set('limit', query.limit.toString());
      if (query.offset) queryParams.set('offset', query.offset.toString());

      const url = `${this.config.endpoint}/api/telemetry?${queryParams.toString()}`;

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

      return {
        query,
        total_count: data.total_count || data.events?.length || 0,
        events: data.events || [],
        aggregations: data.aggregations,
        metadata: {
          query_time_ms: data.metadata?.query_time_ms || 0,
          from_cache: data.metadata?.from_cache || false,
        },
      };
    } catch (error) {
      this.releaseConnection();
      throw error;
    }
  }

  /**
   * Get aggregated telemetry metrics.
   * Retrieves pre-aggregated metrics for efficient analysis.
   */
  async getAggregatedMetrics(query: TelemetryQuery): Promise<AggregatedTelemetryResponse> {
    try {
      await this.acquireConnection();

      const response = await this.fetchWithTimeout(
        `${this.config.endpoint}/api/telemetry/aggregate`,
        {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
            ...(this.config.apiKey && { Authorization: `Bearer ${this.config.apiKey}` }),
          },
          body: JSON.stringify({
            startTime: query.startTime.toISOString(),
            endTime: query.endTime.toISOString(),
            providers: query.providers,
            models: query.models,
            environments: query.environments,
            userIds: query.userIds,
            aggregations: query.aggregations,
          }),
        },
        this.config.timeout
      );

      this.releaseConnection();

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      }

      const data = await response.json();

      return {
        query,
        total_count: data.total_count || 0,
        aggregations: data.aggregations || {},
        metadata: {
          query_time_ms: data.metadata?.query_time_ms || 0,
          from_cache: data.metadata?.from_cache || false,
        },
      };
    } catch (error) {
      this.releaseConnection();
      throw error;
    }
  }

  /**
   * Stream telemetry events for large datasets.
   * Uses pagination to avoid memory issues.
   */
  async *streamTelemetryEvents(
    query: TelemetryQuery,
    batchSize: number = 1000
  ): AsyncGenerator<StoredTelemetryEvent[], void, unknown> {
    let offset = 0;
    let hasMore = true;

    while (hasMore) {
      const response = await this.getTelemetryEvents({
        ...query,
        limit: batchSize,
        offset,
      });

      if (response.events && response.events.length > 0) {
        yield response.events;
        offset += response.events.length;
        hasMore = response.events.length === batchSize;
      } else {
        hasMore = false;
      }
    }
  }

  /**
   * Health check for ruvector-service.
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
   * Connection pool management - acquire connection.
   */
  private async acquireConnection(): Promise<void> {
    while (this.activeConnections >= this.config.connectionPoolSize) {
      await this.sleep(100);
    }
    this.activeConnections++;
  }

  /**
   * Connection pool management - release connection.
   */
  private releaseConnection(): void {
    this.activeConnections = Math.max(0, this.activeConnections - 1);
  }

  /**
   * Fetch with timeout.
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
   * Calculate exponential backoff delay.
   */
  private calculateBackoff(retryCount: number): number {
    const delay = this.config.retryDelayMs * Math.pow(2, retryCount);
    return Math.min(delay, this.config.maxRetryDelayMs);
  }

  /**
   * Sleep utility.
   */
  private sleep(ms: number): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, ms));
  }

  /**
   * Get current connection pool status.
   */
  getConnectionPoolStatus(): {
    active: number;
    max: number;
    available: number;
  } {
    return {
      active: this.activeConnections,
      max: this.config.connectionPoolSize,
      available: this.config.connectionPoolSize - this.activeConnections,
    };
  }
}
