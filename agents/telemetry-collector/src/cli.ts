/**
 * CLI invocation endpoint for telemetry-collector agent
 * CONSTITUTION: CLI provides inspection/replay ONLY - no execution triggers
 *
 * Commands:
 * - inspect: View past DecisionEvents (read-only)
 * - replay: Re-process historical events (read-only analysis)
 * - analyze: Query patterns (does NOT modify anything)
 * - status: Agent health and metrics
 */

import { RuvectorClient } from './ruvector-client.js';
import { loadConfig } from './config.js';
import type {
  DecisionQuery,
  ReplayResult,
  AnalysisQuery,
  AnalysisResult,
  AgentStatus,
} from './types/ruvector.js';

/**
 * CLI Commands interface
 */
export interface CLICommands {
  inspect(eventId: string): Promise<any>;
  replay(eventId: string): Promise<ReplayResult>;
  analyze(query: AnalysisQuery): Promise<AnalysisResult>;
  status(): Promise<AgentStatus>;
}

/**
 * CLI implementation
 */
export class TelemetryCollectorCLI implements CLICommands {
  private ruvectorClient: RuvectorClient;
  private config: ReturnType<typeof loadConfig>;
  private startTime: Date;
  private eventsProcessed: number = 0;
  private errors: number = 0;

  constructor() {
    this.config = loadConfig();
    this.ruvectorClient = new RuvectorClient(this.config.ruvector);
    this.startTime = new Date();
  }

  /**
   * Inspect a specific decision event by ID
   * READ-ONLY operation
   */
  async inspect(eventId: string): Promise<any> {
    if (!eventId) {
      throw new Error('eventId is required');
    }

    try {
      const query: DecisionQuery = {
        limit: 1,
      };

      const events = await this.ruvectorClient.getDecisionEvents(query);
      const event = events.find((e) => e.eventId === eventId || e.id === eventId);

      if (!event) {
        throw new Error(`Event not found: ${eventId}`);
      }

      return {
        eventId: event.eventId || event.id,
        timestamp: event.timestamp,
        agentId: event.agentId,
        agentVersion: event.agentVersion,
        eventType: event.eventType,
        data: event.data || event,
        metadata: event.metadata,
      };
    } catch (error) {
      this.errors++;
      throw new Error(
        `Failed to inspect event ${eventId}: ${
          error instanceof Error ? error.message : String(error)
        }`
      );
    }
  }

  /**
   * Replay a historical event for read-only analysis
   * Does NOT modify any state or trigger execution
   */
  async replay(eventId: string): Promise<ReplayResult> {
    if (!eventId) {
      throw new Error('eventId is required');
    }

    try {
      const event = await this.inspect(eventId);

      const analysis = {
        modelUsed: event.data?.model || event.metadata?.model,
        toolsInvoked: event.data?.tools || event.metadata?.tools || [],
        errors: event.data?.errors || [],
        warnings: event.data?.warnings || [],
      };

      return {
        eventId: event.eventId,
        originalTimestamp: new Date(event.timestamp),
        replayTimestamp: new Date(),
        event: event.data,
        analysis,
        success: true,
      };
    } catch (error) {
      this.errors++;
      return {
        eventId,
        originalTimestamp: new Date(),
        replayTimestamp: new Date(),
        event: null,
        analysis: {
          errors: [error instanceof Error ? error.message : String(error)],
        },
        success: false,
        error: error instanceof Error ? error.message : String(error),
      };
    }
  }

  /**
   * Analyze patterns across decision events
   * READ-ONLY operation - queries existing data
   */
  async analyze(query: AnalysisQuery): Promise<AnalysisResult> {
    try {
      const decisionQuery: DecisionQuery = {
        agentId: query.agentId,
        startTime: query.startTime,
        endTime: query.endTime,
        limit: 1000,
      };

      const events = await this.ruvectorClient.getDecisionEvents(decisionQuery);

      const patterns = this.extractPatterns(events, query.pattern);
      const metrics = this.calculateMetrics(events, query.metrics);
      const insights = this.generateInsights(events, patterns, metrics);

      return {
        query,
        timestamp: new Date(),
        totalEvents: events.length,
        patterns,
        metrics,
        insights,
      };
    } catch (error) {
      this.errors++;
      throw new Error(
        `Failed to analyze events: ${error instanceof Error ? error.message : String(error)}`
      );
    }
  }

  /**
   * Get agent status and health metrics
   * READ-ONLY operation
   */
  async status(): Promise<AgentStatus> {
    try {
      const health = await this.ruvectorClient.healthCheck();
      const uptime = Date.now() - this.startTime.getTime();
      const poolStatus = this.ruvectorClient.getConnectionPoolStatus();

      const errorRate = this.eventsProcessed > 0 ? this.errors / this.eventsProcessed : 0;
      const eventsPerSecond = this.eventsProcessed / (uptime / 1000);

      let healthStatus: 'healthy' | 'degraded' | 'unhealthy';
      if (!health.healthy) {
        healthStatus = 'unhealthy';
      } else if (errorRate > 0.1 || poolStatus.available === 0) {
        healthStatus = 'degraded';
      } else {
        healthStatus = 'healthy';
      }

      return {
        agentId: this.config.agentId,
        agentVersion: this.config.agentVersion,
        uptime,
        eventsProcessed: this.eventsProcessed,
        lastEventTimestamp: undefined,
        ruvectorConnected: health.healthy,
        errors: this.errors,
        health: healthStatus,
        metrics: {
          avgProcessingTimeMs: 0,
          eventsPerSecond,
          errorRate,
        },
      };
    } catch (error) {
      throw new Error(
        `Failed to get status: ${error instanceof Error ? error.message : String(error)}`
      );
    }
  }

  /**
   * Extract patterns from events
   */
  private extractPatterns(events: any[], patternFilter?: string): any[] {
    const patternMap = new Map<string, { count: number; examples: string[] }>();

    for (const event of events) {
      const eventType = event.eventType || event.type || 'unknown';

      if (patternFilter && !eventType.includes(patternFilter)) {
        continue;
      }

      const existing = patternMap.get(eventType) || { count: 0, examples: [] };
      existing.count++;
      if (existing.examples.length < 3) {
        existing.examples.push(event.eventId || event.id);
      }
      patternMap.set(eventType, existing);
    }

    return Array.from(patternMap.entries()).map(([pattern, data]) => ({
      pattern,
      count: data.count,
      examples: data.examples,
      confidence: data.count / events.length,
    }));
  }

  /**
   * Calculate metrics from events
   */
  private calculateMetrics(events: any[], requestedMetrics?: string[]): Record<string, any> {
    const metrics: Record<string, any> = {
      totalEvents: events.length,
      eventTypes: new Set(events.map((e) => e.eventType || e.type)).size,
    };

    if (!requestedMetrics || requestedMetrics.includes('avgDuration')) {
      const durations = events
        .map((e) => e.duration || e.metadata?.duration)
        .filter((d) => d !== undefined);
      metrics.avgDuration = durations.length > 0
        ? durations.reduce((a, b) => a + b, 0) / durations.length
        : 0;
    }

    if (!requestedMetrics || requestedMetrics.includes('errorRate')) {
      const errors = events.filter((e) => e.error || e.status === 'error').length;
      metrics.errorRate = events.length > 0 ? errors / events.length : 0;
    }

    return metrics;
  }

  /**
   * Generate insights from events and patterns
   */
  private generateInsights(events: any[], patterns: any[], metrics: Record<string, any>): string[] {
    const insights: string[] = [];

    if (events.length === 0) {
      insights.push('No events found for the given query');
      return insights;
    }

    const topPattern = patterns.sort((a, b) => b.count - a.count)[0];
    if (topPattern) {
      insights.push(
        `Most common event type: ${topPattern.pattern} (${topPattern.count} occurrences)`
      );
    }

    if (metrics.errorRate > 0.1) {
      insights.push(`High error rate detected: ${(metrics.errorRate * 100).toFixed(2)}%`);
    }

    if (metrics.avgDuration > 1000) {
      insights.push(
        `Average event duration is high: ${metrics.avgDuration.toFixed(2)}ms`
      );
    }

    return insights;
  }

  /**
   * Increment events processed counter
   */
  incrementEventsProcessed(): void {
    this.eventsProcessed++;
  }

  /**
   * Increment errors counter
   */
  incrementErrors(): void {
    this.errors++;
  }
}

/**
 * Exported CLI commands factory
 */
export function createCLI(): CLICommands {
  return new TelemetryCollectorCLI();
}

/**
 * CLI command implementations as individual functions
 */
export const cliCommands = {
  inspect: async (eventId: string): Promise<any> => {
    const cli = createCLI();
    return cli.inspect(eventId);
  },

  replay: async (eventId: string): Promise<ReplayResult> => {
    const cli = createCLI();
    return cli.replay(eventId);
  },

  analyze: async (query: AnalysisQuery): Promise<AnalysisResult> => {
    const cli = createCLI();
    return cli.analyze(query);
  },

  status: async (): Promise<AgentStatus> => {
    const cli = createCLI();
    return cli.status();
  },
};
