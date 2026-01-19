/**
 * RuVector Service Client
 *
 * CONSTITUTIONAL REQUIREMENT: ALL persistence goes through this client.
 * LLM-Observatory does NOT connect directly to Google SQL.
 * LLM-Observatory does NOT execute SQL directly.
 * All DecisionEvents are written via ruvector-service.
 */

export interface RuvectorConfig {
  endpoint: string;
  apiKey?: string;
  timeout: number;
  retryAttempts: number;
  retryDelayMs: number;
  maxRetryDelayMs: number;
  connectionPoolSize: number;
}

export interface PersistResult {
  success: boolean;
  eventId?: string;
  error?: string;
  retries: number;
}

export interface HealthStatus {
  healthy: boolean;
  latencyMs: number;
  endpoint: string;
  error?: string;
  version?: string;
}

export class RuvectorClient {
  private config: RuvectorConfig;
  private activeConnections = 0;

  constructor(config: RuvectorConfig) {
    this.config = config;
  }

  // ==========================================================================
  // DECISION EVENT PERSISTENCE (append-only, idempotent)
  // ==========================================================================

  async persistDecisionEvent(event: any): Promise<PersistResult> {
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
              'X-Idempotency-Key': event.execution_ref || event.eventId,
            },
            body: JSON.stringify(event),
          },
          this.config.timeout
        );

        this.releaseConnection();

        if (!response.ok) {
          throw new Error(`HTTP ${response.status}: ${response.statusText}`);
        }

        const data = await response.json() as Record<string, unknown>;

        return {
          success: true,
          eventId: (data.eventId as string) || (data.id as string) || event.execution_ref,
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
            error: error instanceof Error ? error.message : String(error),
            retries,
          };
        }
      }
    }

    return {
      success: false,
      error: 'Max retries exceeded',
      retries,
    };
  }

  async persistDecisionEventsBatch(events: any[]): Promise<{ successful: number; failed: number }> {
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
          body: JSON.stringify({ events }),
        },
        this.config.timeout
      );

      this.releaseConnection();

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}`);
      }

      const data = await response.json() as Record<string, unknown>;
      return { successful: (data.successful as number) || events.length, failed: (data.failed as number) || 0 };
    } catch (error) {
      this.releaseConnection();
      return { successful: 0, failed: events.length };
    }
  }

  // ==========================================================================
  // QUERY OPERATIONS (read from ruvector-service)
  // ==========================================================================

  async getUsagePatterns(timeRange: any, dimensions: any, filters: any): Promise<any[]> {
    const params = new URLSearchParams();
    if (timeRange?.start) params.set('start', timeRange.start);
    if (timeRange?.end) params.set('end', timeRange.end);
    if (dimensions) params.set('dimensions', JSON.stringify(dimensions));
    if (filters) params.set('filters', JSON.stringify(filters));

    const response = await this.fetchWithTimeout(
      `${this.config.endpoint}/api/usage/patterns?${params}`,
      {
        method: 'GET',
        headers: this.getHeaders(),
      },
      this.config.timeout
    );

    if (!response.ok) {
      throw new Error(`Failed to get usage patterns: ${response.status}`);
    }

    const data = await response.json() as Record<string, unknown>;
    return (data.patterns as unknown[]) || [];
  }

  async classifyFailures(errorEvents: any[], context: any): Promise<any[]> {
    const response = await this.fetchWithTimeout(
      `${this.config.endpoint}/api/failure/classify`,
      {
        method: 'POST',
        headers: this.getHeaders(),
        body: JSON.stringify({ error_events: errorEvents, context }),
      },
      this.config.timeout
    );

    if (!response.ok) {
      throw new Error(`Failed to classify failures: ${response.status}`);
    }

    const data = await response.json() as Record<string, unknown>;
    return (data.classifications as unknown[]) || [];
  }

  async evaluateHealth(targets: any[], options: any): Promise<any[]> {
    const response = await this.fetchWithTimeout(
      `${this.config.endpoint}/api/health/evaluate`,
      {
        method: 'POST',
        headers: this.getHeaders(),
        body: JSON.stringify({ targets, options }),
      },
      this.config.timeout
    );

    if (!response.ok) {
      throw new Error(`Failed to evaluate health: ${response.status}`);
    }

    const data = await response.json() as Record<string, unknown>;
    return (data.evaluations as unknown[]) || [];
  }

  async evaluateSlos(sloDefinitions: any[], metrics: any[], evaluationTime: string): Promise<any> {
    const response = await this.fetchWithTimeout(
      `${this.config.endpoint}/api/slo/evaluate`,
      {
        method: 'POST',
        headers: this.getHeaders(),
        body: JSON.stringify({ slo_definitions: sloDefinitions, metrics, evaluation_time: evaluationTime }),
      },
      this.config.timeout
    );

    if (!response.ok) {
      throw new Error(`Failed to evaluate SLOs: ${response.status}`);
    }

    return response.json();
  }

  async getViolations(query: any): Promise<any[]> {
    const params = new URLSearchParams();
    Object.entries(query).forEach(([key, value]) => {
      if (value !== undefined && value !== null) {
        params.set(key, String(value));
      }
    });

    const response = await this.fetchWithTimeout(
      `${this.config.endpoint}/api/slo/violations?${params}`,
      {
        method: 'GET',
        headers: this.getHeaders(),
      },
      this.config.timeout
    );

    if (!response.ok) {
      throw new Error(`Failed to get violations: ${response.status}`);
    }

    const data = await response.json() as Record<string, unknown>;
    return (data.violations as unknown[]) || [];
  }

  async generatePostMortem(incidentId: string, timeRange: any, includeMetrics: boolean): Promise<any> {
    const response = await this.fetchWithTimeout(
      `${this.config.endpoint}/api/postmortem/generate`,
      {
        method: 'POST',
        headers: this.getHeaders(),
        body: JSON.stringify({ incident_id: incidentId, time_range: timeRange, include_metrics: includeMetrics }),
      },
      this.config.timeout
    );

    if (!response.ok) {
      throw new Error(`Failed to generate post-mortem: ${response.status}`);
    }

    return response.json();
  }

  async generateVisualizationSpec(dashboardType: string, dataSources: any[], timeRange: any): Promise<any> {
    const response = await this.fetchWithTimeout(
      `${this.config.endpoint}/api/visualization/generate`,
      {
        method: 'POST',
        headers: this.getHeaders(),
        body: JSON.stringify({ dashboard_type: dashboardType, data_sources: dataSources, time_range: timeRange }),
      },
      this.config.timeout
    );

    if (!response.ok) {
      throw new Error(`Failed to generate visualization spec: ${response.status}`);
    }

    return response.json();
  }

  // ==========================================================================
  // HEALTH CHECK
  // ==========================================================================

  async healthCheck(): Promise<HealthStatus> {
    const startTime = Date.now();

    try {
      const response = await this.fetchWithTimeout(
        `${this.config.endpoint}/health`,
        {
          method: 'GET',
          headers: this.getHeaders(),
        },
        this.config.timeout
      );

      const latencyMs = Date.now() - startTime;

      if (!response.ok) {
        return {
          healthy: false,
          latencyMs,
          endpoint: this.config.endpoint,
          error: `HTTP ${response.status}`,
        };
      }

      const data = await response.json() as Record<string, unknown>;

      return {
        healthy: true,
        latencyMs,
        endpoint: this.config.endpoint,
        version: data.version as string | undefined,
      };
    } catch (error) {
      return {
        healthy: false,
        latencyMs: Date.now() - startTime,
        endpoint: this.config.endpoint,
        error: error instanceof Error ? error.message : String(error),
      };
    }
  }

  // ==========================================================================
  // CONNECTION POOL & UTILITIES
  // ==========================================================================

  private getHeaders(): Record<string, string> {
    return {
      'Content-Type': 'application/json',
      ...(this.config.apiKey && { Authorization: `Bearer ${this.config.apiKey}` }),
    };
  }

  private async acquireConnection(): Promise<void> {
    while (this.activeConnections >= this.config.connectionPoolSize) {
      await this.sleep(50);
    }
    this.activeConnections++;
  }

  private releaseConnection(): void {
    this.activeConnections = Math.max(0, this.activeConnections - 1);
  }

  private async fetchWithTimeout(url: string, options: RequestInit, timeoutMs: number): Promise<Response> {
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), timeoutMs);

    try {
      const response = await fetch(url, { ...options, signal: controller.signal });
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

  getPoolStats() {
    return {
      active: this.activeConnections,
      max: this.config.connectionPoolSize,
      available: this.config.connectionPoolSize - this.activeConnections,
    };
  }
}
