/**
 * RuVector Service Client
 *
 * CONSTITUTIONAL CLASSIFICATION: READ-ONLY persistence client
 *
 * This client handles persistence of DecisionEvents to ruvector-service.
 * It is the ONLY allowed persistence mechanism for this agent.
 * Direct SQL access is PROHIBITED.
 */

import type {
  DecisionEvent,
  RuvectorConfig,
  RuvectorHealthStatus,
  DecisionQuery,
  FailureClassification,
  AnalysisResult,
} from '../contracts';

// =============================================================================
// DEFAULT CONFIGURATION
// =============================================================================

const DEFAULT_CONFIG: RuvectorConfig = {
  endpoint: 'http://localhost:3001',
  apiKey: undefined,
  timeout: 30000,
  retryAttempts: 3,
  retryDelayMs: 1000,
  maxRetryDelayMs: 10000,
  connectionPoolSize: 5,
};

// =============================================================================
// RUVECTOR CLIENT
// =============================================================================

export class RuvectorClient {
  private config: RuvectorConfig;
  private activeConnections: number = 0;
  private requestQueue: Array<() => Promise<void>> = [];

  constructor(config: Partial<RuvectorConfig> = {}) {
    this.config = { ...DEFAULT_CONFIG, ...config };
  }

  // ===========================================================================
  // PERSISTENCE METHODS
  // ===========================================================================

  /**
   * Persist a single decision event
   *
   * This is the primary persistence method. Each invocation of the agent
   * MUST call this exactly once.
   */
  async persistDecisionEvent(event: DecisionEvent): Promise<void> {
    await this.withRetry(async () => {
      await this.acquireConnection();
      try {
        const response = await fetch(
          `${this.config.endpoint}/api/v1/decision-events`,
          {
            method: 'POST',
            headers: this.buildHeaders(),
            body: JSON.stringify(event),
            signal: AbortSignal.timeout(this.config.timeout),
          }
        );

        if (!response.ok) {
          const error = await response.text();
          throw new RuvectorError(
            `Failed to persist decision event: ${response.status} ${error}`,
            response.status
          );
        }
      } finally {
        this.releaseConnection();
      }
    });
  }

  /**
   * Persist multiple decision events in batch
   */
  async persistDecisionEventsBatch(events: DecisionEvent[]): Promise<void> {
    await this.withRetry(async () => {
      await this.acquireConnection();
      try {
        const response = await fetch(
          `${this.config.endpoint}/api/v1/decision-events/batch`,
          {
            method: 'POST',
            headers: this.buildHeaders(),
            body: JSON.stringify({ events }),
            signal: AbortSignal.timeout(this.config.timeout),
          }
        );

        if (!response.ok) {
          const error = await response.text();
          throw new RuvectorError(
            `Failed to persist decision events batch: ${response.status} ${error}`,
            response.status
          );
        }
      } finally {
        this.releaseConnection();
      }
    });
  }

  // ===========================================================================
  // QUERY METHODS (for CLI inspection/analysis)
  // ===========================================================================

  /**
   * Query decision events
   */
  async getDecisionEvents(query: DecisionQuery): Promise<DecisionEvent[]> {
    await this.acquireConnection();
    try {
      const params = new URLSearchParams();
      if (query.agentId) params.set('agent_id', query.agentId);
      if (query.agentVersion) params.set('agent_version', query.agentVersion);
      if (query.startTime) params.set('start_time', query.startTime);
      if (query.endTime) params.set('end_time', query.endTime);
      if (query.eventTypes) params.set('event_types', query.eventTypes.join(','));
      if (query.limit) params.set('limit', String(query.limit));
      if (query.offset) params.set('offset', String(query.offset));
      if (query.sortBy) params.set('sort_by', query.sortBy);
      if (query.sortOrder) params.set('sort_order', query.sortOrder);

      const response = await fetch(
        `${this.config.endpoint}/api/v1/decision-events?${params}`,
        {
          method: 'GET',
          headers: this.buildHeaders(),
          signal: AbortSignal.timeout(this.config.timeout),
        }
      );

      if (!response.ok) {
        const error = await response.text();
        throw new RuvectorError(
          `Failed to query decision events: ${response.status} ${error}`,
          response.status
        );
      }

      const data = await response.json();
      return data.events || [];
    } finally {
      this.releaseConnection();
    }
  }

  /**
   * Get a specific decision event by execution ref
   */
  async getDecisionEventByRef(executionRef: string): Promise<DecisionEvent | null> {
    await this.acquireConnection();
    try {
      const response = await fetch(
        `${this.config.endpoint}/api/v1/decision-events/${executionRef}`,
        {
          method: 'GET',
          headers: this.buildHeaders(),
          signal: AbortSignal.timeout(this.config.timeout),
        }
      );

      if (response.status === 404) {
        return null;
      }

      if (!response.ok) {
        const error = await response.text();
        throw new RuvectorError(
          `Failed to get decision event: ${response.status} ${error}`,
          response.status
        );
      }

      return await response.json();
    } finally {
      this.releaseConnection();
    }
  }

  /**
   * Get classification by span ID
   */
  async getClassificationBySpanId(spanId: string): Promise<FailureClassification | null> {
    const events = await this.getDecisionEvents({
      agentId: 'failure-classification-agent',
      eventTypes: ['failure_classification'],
      limit: 100,
    });

    for (const event of events) {
      const classification = event.outputs.find(
        (o: FailureClassification) => o.span_id === spanId
      );
      if (classification) {
        return classification;
      }
    }

    return null;
  }

  /**
   * Get classification statistics for analysis
   */
  async getClassificationStats(
    startTime: string,
    endTime: string,
    groupBy: string
  ): Promise<AnalysisResult> {
    await this.acquireConnection();
    try {
      const params = new URLSearchParams({
        agent_id: 'failure-classification-agent',
        start_time: startTime,
        end_time: endTime,
        group_by: groupBy,
      });

      const response = await fetch(
        `${this.config.endpoint}/api/v1/decision-events/aggregate?${params}`,
        {
          method: 'GET',
          headers: this.buildHeaders(),
          signal: AbortSignal.timeout(this.config.timeout),
        }
      );

      if (!response.ok) {
        const error = await response.text();
        throw new RuvectorError(
          `Failed to get classification stats: ${response.status} ${error}`,
          response.status
        );
      }

      return await response.json();
    } finally {
      this.releaseConnection();
    }
  }

  // ===========================================================================
  // HEALTH CHECK
  // ===========================================================================

  /**
   * Check ruvector-service health
   */
  async healthCheck(): Promise<RuvectorHealthStatus> {
    const startTime = Date.now();

    try {
      const response = await fetch(`${this.config.endpoint}/health`, {
        method: 'GET',
        headers: this.buildHeaders(),
        signal: AbortSignal.timeout(5000),
      });

      const latencyMs = Date.now() - startTime;

      if (!response.ok) {
        return {
          healthy: false,
          latencyMs,
          lastCheck: new Date().toISOString(),
          endpoint: this.config.endpoint,
          error: `Health check failed: ${response.status}`,
        };
      }

      const data = await response.json();

      return {
        healthy: true,
        latencyMs,
        lastCheck: new Date().toISOString(),
        endpoint: this.config.endpoint,
        version: data.version,
      };
    } catch (error) {
      return {
        healthy: false,
        latencyMs: Date.now() - startTime,
        lastCheck: new Date().toISOString(),
        endpoint: this.config.endpoint,
        error: error instanceof Error ? error.message : 'Unknown error',
      };
    }
  }

  // ===========================================================================
  // CONNECTION POOL
  // ===========================================================================

  /**
   * Acquire a connection from the pool
   */
  private async acquireConnection(): Promise<void> {
    if (this.activeConnections < this.config.connectionPoolSize) {
      this.activeConnections++;
      return;
    }

    // Wait for a connection to become available
    return new Promise((resolve) => {
      this.requestQueue.push(async () => {
        this.activeConnections++;
        resolve();
      });
    });
  }

  /**
   * Release a connection back to the pool
   */
  private releaseConnection(): void {
    this.activeConnections--;

    if (this.requestQueue.length > 0) {
      const next = this.requestQueue.shift();
      if (next) next();
    }
  }

  /**
   * Get connection pool status
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

  // ===========================================================================
  // RETRY LOGIC
  // ===========================================================================

  /**
   * Execute with retry logic
   */
  private async withRetry<T>(fn: () => Promise<T>): Promise<T> {
    let lastError: Error | undefined;
    let delay = this.config.retryDelayMs;

    for (let attempt = 0; attempt < this.config.retryAttempts; attempt++) {
      try {
        return await fn();
      } catch (error) {
        lastError = error instanceof Error ? error : new Error(String(error));

        // Don't retry on client errors (4xx)
        if (error instanceof RuvectorError && error.statusCode >= 400 && error.statusCode < 500) {
          throw error;
        }

        if (attempt < this.config.retryAttempts - 1) {
          await this.sleep(delay);
          delay = Math.min(delay * 2, this.config.maxRetryDelayMs);
        }
      }
    }

    throw lastError || new Error('Retry failed');
  }

  /**
   * Sleep for specified milliseconds
   */
  private sleep(ms: number): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, ms));
  }

  // ===========================================================================
  // HELPERS
  // ===========================================================================

  /**
   * Build request headers
   */
  private buildHeaders(): HeadersInit {
    const headers: HeadersInit = {
      'Content-Type': 'application/json',
      'User-Agent': 'failure-classification-agent/1.0.0',
    };

    if (this.config.apiKey) {
      headers['Authorization'] = `Bearer ${this.config.apiKey}`;
    }

    return headers;
  }
}

// =============================================================================
// ERROR CLASS
// =============================================================================

export class RuvectorError extends Error {
  public readonly statusCode: number;

  constructor(message: string, statusCode: number) {
    super(message);
    this.name = 'RuvectorError';
    this.statusCode = statusCode;
  }
}
