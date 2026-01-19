/**
 * RuVector Service HTTP Client for Post-Mortem Generator Agent
 *
 * CONSTITUTION: All persistence goes through this client - NO direct database access.
 * - Async, non-blocking operations
 * - Connection pooling with retry logic
 * - Timeout enforcement
 * - Graceful error handling
 */

import type {
  RuvectorConfig,
  PersistResult,
  DecisionQuery,
  FailureClassificationQuery,
  HealthEvaluationQuery,
  TelemetryQuery,
  HealthStatus,
  StoredFailureClassification,
  StoredHealthEvaluation,
  StoredTelemetryEvent,
  AggregatedFailureData,
  AggregatedHealthData,
  AggregatedTelemetryData,
  FailureClassificationResponse,
  HealthEvaluationResponse,
  TelemetryResponse,
  DecisionEventResponse,
} from './types/ruvector.js';
import type {
  FailureCategory,
  FailureSeverity,
  FailureCause,
  HealthState,
  Provider,
} from '../contracts/schemas.js';

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

  // ===========================================================================
  // PERSISTENCE METHODS
  // ===========================================================================

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
          eventId: data.eventId || data.id || (event as { execution_ref?: string }).execution_ref || '',
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
            eventId: (event as { execution_ref?: string }).execution_ref || '',
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

  // ===========================================================================
  // QUERY METHODS - FAILURE CLASSIFICATIONS
  // ===========================================================================

  /**
   * Get failure classifications from ruvector-service.
   */
  async getFailureClassifications(
    query: FailureClassificationQuery
  ): Promise<FailureClassificationResponse> {
    try {
      await this.acquireConnection();

      const queryParams = new URLSearchParams();
      queryParams.set('startTime', query.startTime.toISOString());
      queryParams.set('endTime', query.endTime.toISOString());
      if (query.providers?.length) queryParams.set('providers', query.providers.join(','));
      if (query.models?.length) queryParams.set('models', query.models.join(','));
      if (query.categories?.length) queryParams.set('categories', query.categories.join(','));
      if (query.severities?.length) queryParams.set('severities', query.severities.join(','));
      if (query.causes?.length) queryParams.set('causes', query.causes.join(','));
      if (query.limit) queryParams.set('limit', query.limit.toString());
      if (query.offset) queryParams.set('offset', query.offset.toString());

      const url = `${this.config.endpoint}/api/failure-classifications?${queryParams.toString()}`;

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
        classifications: data.classifications || data.events || [],
        total_count: data.total_count || data.classifications?.length || 0,
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
   * Get aggregated failure data for post-mortem analysis.
   */
  async getAggregatedFailures(
    query: FailureClassificationQuery
  ): Promise<AggregatedFailureData> {
    try {
      await this.acquireConnection();

      const response = await this.fetchWithTimeout(
        `${this.config.endpoint}/api/failure-classifications/aggregate`,
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
            categories: query.categories,
            severities: query.severities,
            causes: query.causes,
          }),
        },
        this.config.timeout
      );

      this.releaseConnection();

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      }

      const data = await response.json();

      // Convert arrays to Maps for easier processing
      const byCategory = new Map<FailureCategory, number>();
      const bySeverity = new Map<FailureSeverity, number>();
      const byCause = new Map<FailureCause, number>();
      const byProvider = new Map<Provider, {
        count: number;
        models: Set<string>;
        first_occurrence: string;
        last_occurrence: string;
      }>();

      if (data.by_category) {
        for (const item of data.by_category) {
          byCategory.set(item.category, item.count);
        }
      }

      if (data.by_severity) {
        for (const item of data.by_severity) {
          bySeverity.set(item.severity, item.count);
        }
      }

      if (data.by_cause) {
        for (const item of data.by_cause) {
          byCause.set(item.cause, item.count);
        }
      }

      if (data.by_provider) {
        for (const item of data.by_provider) {
          byProvider.set(item.provider, {
            count: item.count,
            models: new Set(item.models || []),
            first_occurrence: item.first_occurrence,
            last_occurrence: item.last_occurrence,
          });
        }
      }

      return {
        total_failures: data.total_failures || 0,
        by_category: byCategory,
        by_severity: bySeverity,
        by_cause: byCause,
        by_provider: byProvider,
        time_series: data.time_series || [],
      };
    } catch (error) {
      this.releaseConnection();
      throw error;
    }
  }

  // ===========================================================================
  // QUERY METHODS - HEALTH EVALUATIONS
  // ===========================================================================

  /**
   * Get health evaluations from ruvector-service.
   */
  async getHealthEvaluations(
    query: HealthEvaluationQuery
  ): Promise<HealthEvaluationResponse> {
    try {
      await this.acquireConnection();

      const queryParams = new URLSearchParams();
      queryParams.set('startTime', query.startTime.toISOString());
      queryParams.set('endTime', query.endTime.toISOString());
      if (query.targetIds?.length) queryParams.set('targetIds', query.targetIds.join(','));
      if (query.targetTypes?.length) queryParams.set('targetTypes', query.targetTypes.join(','));
      if (query.healthStates?.length) queryParams.set('healthStates', query.healthStates.join(','));
      if (query.limit) queryParams.set('limit', query.limit.toString());
      if (query.offset) queryParams.set('offset', query.offset.toString());

      const url = `${this.config.endpoint}/api/health-evaluations?${queryParams.toString()}`;

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
        evaluations: data.evaluations || data.events || [],
        total_count: data.total_count || data.evaluations?.length || 0,
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
   * Get aggregated health data for post-mortem analysis.
   */
  async getAggregatedHealth(
    query: HealthEvaluationQuery
  ): Promise<AggregatedHealthData> {
    try {
      await this.acquireConnection();

      const response = await this.fetchWithTimeout(
        `${this.config.endpoint}/api/health-evaluations/aggregate`,
        {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
            ...(this.config.apiKey && { Authorization: `Bearer ${this.config.apiKey}` }),
          },
          body: JSON.stringify({
            startTime: query.startTime.toISOString(),
            endTime: query.endTime.toISOString(),
            targetIds: query.targetIds,
            targetTypes: query.targetTypes,
            healthStates: query.healthStates,
          }),
        },
        this.config.timeout
      );

      this.releaseConnection();

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      }

      const data = await response.json();

      const stateDurations = new Map<HealthState, number>();
      const currentStates = new Map<string, HealthState>();

      if (data.state_durations) {
        for (const [state, duration] of Object.entries(data.state_durations)) {
          stateDurations.set(state as HealthState, duration as number);
        }
      }

      if (data.current_states) {
        for (const [targetId, state] of Object.entries(data.current_states)) {
          currentStates.set(targetId, state as HealthState);
        }
      }

      return {
        health_transitions: data.health_transitions || [],
        state_durations: stateDurations,
        current_states: currentStates,
      };
    } catch (error) {
      this.releaseConnection();
      throw error;
    }
  }

  // ===========================================================================
  // QUERY METHODS - TELEMETRY
  // ===========================================================================

  /**
   * Get telemetry data from ruvector-service.
   */
  async getTelemetryEvents(
    query: TelemetryQuery
  ): Promise<TelemetryResponse> {
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
        events: data.events || [],
        total_count: data.total_count || data.events?.length || 0,
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
   * Get aggregated telemetry data for post-mortem analysis.
   */
  async getAggregatedTelemetry(
    query: TelemetryQuery
  ): Promise<AggregatedTelemetryData> {
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
          }),
        },
        this.config.timeout
      );

      this.releaseConnection();

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      }

      const data = await response.json();

      const byProvider = new Map<Provider, {
        request_count: number;
        error_count: number;
        models: Set<string>;
      }>();

      if (data.by_provider) {
        for (const item of data.by_provider) {
          byProvider.set(item.provider, {
            request_count: item.request_count,
            error_count: item.error_count,
            models: new Set(item.models || []),
          });
        }
      }

      return {
        total_requests: data.total_requests || 0,
        total_errors: data.total_errors || 0,
        error_rate: data.error_rate || 0,
        latency_stats: data.latency_stats || {
          min_ms: 0,
          max_ms: 0,
          avg_ms: 0,
          p50_ms: 0,
          p95_ms: 0,
          p99_ms: 0,
        },
        by_provider: byProvider,
        peak_error_rate: data.peak_error_rate || {
          value: 0,
          timestamp: new Date().toISOString(),
        },
      };
    } catch (error) {
      this.releaseConnection();
      throw error;
    }
  }

  // ===========================================================================
  // QUERY METHODS - DECISION EVENTS
  // ===========================================================================

  /**
   * Get decision events from ruvector-service.
   */
  async getDecisionEvents(query: DecisionQuery): Promise<DecisionEventResponse> {
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

      return {
        events: data.events || data || [],
        total_count: data.total_count || data.events?.length || 0,
        metadata: {
          query_time_ms: data.metadata?.query_time_ms || 0,
        },
      };
    } catch (error) {
      this.releaseConnection();
      throw error;
    }
  }

  /**
   * Get a specific post-mortem report by ID.
   */
  async getPostMortemReport(reportId: string): Promise<unknown | null> {
    try {
      await this.acquireConnection();

      const response = await this.fetchWithTimeout(
        `${this.config.endpoint}/api/events/${reportId}`,
        {
          method: 'GET',
          headers: {
            ...(this.config.apiKey && { Authorization: `Bearer ${this.config.apiKey}` }),
          },
        },
        this.config.timeout
      );

      this.releaseConnection();

      if (response.status === 404) {
        return null;
      }

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      }

      return await response.json();
    } catch (error) {
      this.releaseConnection();
      throw error;
    }
  }

  // ===========================================================================
  // HEALTH CHECK
  // ===========================================================================

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

  // ===========================================================================
  // CONNECTION POOL MANAGEMENT
  // ===========================================================================

  private async acquireConnection(): Promise<void> {
    while (this.activeConnections >= this.config.connectionPoolSize) {
      await this.sleep(100);
    }
    this.activeConnections++;
  }

  private releaseConnection(): void {
    this.activeConnections = Math.max(0, this.activeConnections - 1);
  }

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

  // ===========================================================================
  // UTILITY METHODS
  // ===========================================================================

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

  private calculateBackoff(retryCount: number): number {
    const delay = this.config.retryDelayMs * Math.pow(2, retryCount);
    return Math.min(delay, this.config.maxRetryDelayMs);
  }

  private sleep(ms: number): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, ms));
  }
}

// =============================================================================
// SINGLETON CLIENT MANAGEMENT
// =============================================================================

let clientInstance: RuvectorClient | null = null;

export function initializeClient(config: RuvectorConfig): void {
  clientInstance = new RuvectorClient(config);
}

export function getClient(): RuvectorClient {
  if (!clientInstance) {
    throw new Error('RuvectorClient not initialized. Call initializeClient first.');
  }
  return clientInstance;
}
