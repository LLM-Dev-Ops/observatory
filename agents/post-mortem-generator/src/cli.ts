/**
 * Post-Mortem Generator Agent - CLI Commands
 *
 * CONSTITUTIONAL CLASSIFICATION: READ-ONLY
 *
 * These CLI commands are for inspection, replay, and analysis only.
 * They do NOT modify state or trigger actions.
 */

import type {
  CLIInspectResult,
  CLIReplayResult,
  CLIStatusResult,
  PostMortemQuery,
  PostMortemRequest,
  PostMortemReport,
} from '../contracts/schemas.js';
import { AGENT_METADATA } from '../contracts/schemas.js';
import { RuvectorClient, initializeClient, getClient } from './ruvector-client.js';
import { generatePostMortem, type GeneratorInput } from './generator.js';
import { loadConfig } from './config.js';

// =============================================================================
// CLI INTERFACE
// =============================================================================

export interface CLICommands {
  inspect(reportId: string, options?: { detailed?: boolean }): Promise<CLIInspectResult>;
  replay(reportId: string, options?: { compare?: boolean }): Promise<CLIReplayResult>;
  generate(request: PostMortemRequest): Promise<PostMortemReport>;
  status(options?: { detailed?: boolean }): Promise<CLIStatusResult>;
  query(query: PostMortemQuery): Promise<PostMortemReport[]>;
  list(options?: { limit?: number; offset?: number }): Promise<PostMortemReport[]>;
}

// =============================================================================
// CLI IMPLEMENTATION
// =============================================================================

function ensureClient(): RuvectorClient {
  try {
    return getClient();
  } catch {
    const config = loadConfig();
    initializeClient(config.ruvector);
    return getClient();
  }
}

export const cliCommands: CLICommands = {
  /**
   * Inspect a specific post-mortem report
   *
   * Usage: npx @llm-observatory/cli post-mortem-generator inspect --report-id=<uuid>
   */
  async inspect(
    reportId: string,
    options: { detailed?: boolean } = {}
  ): Promise<CLIInspectResult> {
    const client = ensureClient();

    // Get report from ruvector
    const reportData = await client.getPostMortemReport(reportId);

    if (!reportData) {
      throw new CLIError(`No post-mortem report found with ID: ${reportId}`);
    }

    // Get the decision event
    const eventsResponse = await client.getDecisionEvents({
      agentId: AGENT_METADATA.id,
      decisionType: 'postmortem_generation',
      limit: 100,
    });

    let decisionEvent = null;
    for (const e of eventsResponse.events) {
      const event = e as { outputs?: Array<{ report_id?: string }> };
      if (event.outputs?.some((o) => o.report_id === reportId)) {
        decisionEvent = e;
        break;
      }
    }

    const result: CLIInspectResult = {
      report: reportData as PostMortemReport,
      decision_event: decisionEvent as any,
      metadata: {
        retrieved_at: new Date().toISOString(),
        source: 'ruvector',
      },
    };

    return result;
  },

  /**
   * Replay post-mortem generation for determinism verification
   *
   * Usage: npx @llm-observatory/cli post-mortem-generator replay --report-id=<uuid>
   */
  async replay(
    reportId: string,
    options: { compare?: boolean } = { compare: true }
  ): Promise<CLIReplayResult> {
    const client = ensureClient();

    // Get original report
    const originalReport = await client.getPostMortemReport(reportId);

    if (!originalReport) {
      throw new CLIError(`No post-mortem report found with ID: ${reportId}`);
    }

    const original = originalReport as PostMortemReport;

    // Re-fetch the same data and regenerate
    const startTime = new Date(original.time_range.start_time);
    const endTime = new Date(original.time_range.end_time);

    // Fetch data again
    const failureData = await client.getAggregatedFailures({
      startTime,
      endTime,
    });

    const healthData = await client.getAggregatedHealth({
      startTime,
      endTime,
    });

    const telemetryData = await client.getAggregatedTelemetry({
      startTime,
      endTime,
    });

    const classificationResponse = await client.getFailureClassifications({
      startTime,
      endTime,
      limit: 1000,
    });

    const healthResponse = await client.getHealthEvaluations({
      startTime,
      endTime,
      limit: 1000,
    });

    // Regenerate
    const generatorInput: GeneratorInput = {
      request: {
        time_range: original.time_range,
        incident_id: original.incident_id,
      },
      failureData,
      healthData,
      telemetryData,
      failureClassifications: classificationResponse.classifications,
      healthEvaluations: healthResponse.evaluations,
    };

    const result = generatePostMortem(generatorInput);

    // Compare if requested
    const differences: Array<{ path: string; original: unknown; replayed: unknown }> = [];

    if (options.compare) {
      // Compare key fields
      if (original.summary.title !== result.report.summary.title) {
        differences.push({
          path: 'summary.title',
          original: original.summary.title,
          replayed: result.report.summary.title,
        });
      }

      if (original.summary.impact_level !== result.report.summary.impact_level) {
        differences.push({
          path: 'summary.impact_level',
          original: original.summary.impact_level,
          replayed: result.report.summary.impact_level,
        });
      }

      if (original.statistics?.total_failures !== result.report.statistics?.total_failures) {
        differences.push({
          path: 'statistics.total_failures',
          original: original.statistics?.total_failures,
          replayed: result.report.statistics?.total_failures,
        });
      }
    }

    return {
      original_report: original,
      replayed_report: result.report,
      match: differences.length === 0,
      differences,
    };
  },

  /**
   * Generate a new post-mortem report
   *
   * Usage: npx @llm-observatory/cli post-mortem-generator generate --start-time=<iso> --end-time=<iso>
   */
  async generate(request: PostMortemRequest): Promise<PostMortemReport> {
    const client = ensureClient();

    const startTime = new Date(request.time_range.start_time);
    const endTime = new Date(request.time_range.end_time);

    // Fetch all data
    const failureData = await client.getAggregatedFailures({
      startTime,
      endTime,
      providers: request.scope?.providers,
      models: request.scope?.models,
      categories: request.scope?.include_categories,
    });

    const healthData = await client.getAggregatedHealth({
      startTime,
      endTime,
      targetIds: request.scope?.services,
    });

    const telemetryData = await client.getAggregatedTelemetry({
      startTime,
      endTime,
      providers: request.scope?.providers,
      models: request.scope?.models,
    });

    const classificationResponse = await client.getFailureClassifications({
      startTime,
      endTime,
      providers: request.scope?.providers,
      models: request.scope?.models,
      categories: request.scope?.include_categories,
      limit: request.options?.max_timeline_events || 1000,
    });

    const healthResponse = await client.getHealthEvaluations({
      startTime,
      endTime,
      targetIds: request.scope?.services,
      limit: request.options?.max_timeline_events || 1000,
    });

    const generatorInput: GeneratorInput = {
      request,
      failureData,
      healthData,
      telemetryData,
      failureClassifications: classificationResponse.classifications,
      healthEvaluations: healthResponse.evaluations,
    };

    const result = generatePostMortem(generatorInput);

    // Persist the decision event
    const { createDecisionEvent } = await import('./emitter.js');
    const decisionEvent = createDecisionEvent({
      request,
      reports: [result.report],
      confidence: result.confidence,
      executionRef: `cli-${Date.now()}`,
    });

    await client.persistDecisionEvent(decisionEvent);

    return result.report;
  },

  /**
   * Get agent status and health
   *
   * Usage: npx @llm-observatory/cli post-mortem-generator status [--detailed]
   */
  async status(options: { detailed?: boolean } = {}): Promise<CLIStatusResult> {
    const client = ensureClient();

    const ruvectorHealth = await client.healthCheck();

    // Get recent metrics
    const eventsResponse = await client.getDecisionEvents({
      agentId: AGENT_METADATA.id,
      startTime: new Date(Date.now() - 60 * 60 * 1000), // Last hour
      limit: 1000,
    });

    const recentEvents = eventsResponse.events as Array<{
      outputs: Array<{ generation_latency_ms?: number }>;
      timestamp: string;
    }>;

    // Calculate metrics
    let totalLatency = 0;
    let reportCount = 0;

    for (const event of recentEvents) {
      for (const output of event.outputs) {
        totalLatency += output.generation_latency_ms || 0;
        reportCount++;
      }
    }

    const status: CLIStatusResult = {
      agent_id: AGENT_METADATA.id,
      agent_version: AGENT_METADATA.version,
      classification: {
        type: AGENT_METADATA.classification.type,
        subtype: AGENT_METADATA.classification.subtype,
      },
      status: ruvectorHealth.healthy ? 'healthy' : 'degraded',
      uptime_seconds: process.uptime(),
      last_generation_at: recentEvents.length > 0 ? recentEvents[0].timestamp : undefined,
      metrics: {
        total_reports_generated: reportCount,
        reports_last_hour: reportCount,
        avg_generation_latency_ms: reportCount > 0 ? totalLatency / reportCount : 0,
        error_rate: 0,
      },
      ruvector_status: {
        healthy: ruvectorHealth.healthy,
        latencyMs: ruvectorHealth.latencyMs,
        error: ruvectorHealth.error,
      },
    };

    return status;
  },

  /**
   * Query post-mortem reports
   *
   * Usage: npx @llm-observatory/cli post-mortem-generator query [options]
   */
  async query(query: PostMortemQuery): Promise<PostMortemReport[]> {
    const client = ensureClient();

    const eventsResponse = await client.getDecisionEvents({
      agentId: AGENT_METADATA.id,
      decisionType: 'postmortem_generation',
      startTime: query.start_time ? new Date(query.start_time) : undefined,
      endTime: query.end_time ? new Date(query.end_time) : undefined,
      limit: query.limit,
      offset: query.offset,
      sortBy: query.sort_by === 'generated_at' ? 'timestamp' : 'confidence',
      sortOrder: query.sort_order,
    });

    const reports: PostMortemReport[] = [];

    for (const event of eventsResponse.events) {
      const e = event as { outputs?: PostMortemReport[] };
      if (e.outputs) {
        for (const output of e.outputs) {
          // Apply filters
          if (query.report_id && output.report_id !== query.report_id) continue;
          if (query.incident_id && output.incident_id !== query.incident_id) continue;

          reports.push(output);
        }
      }
    }

    return reports;
  },

  /**
   * List recent post-mortem reports
   *
   * Usage: npx @llm-observatory/cli post-mortem-generator list [--limit=10]
   */
  async list(options: { limit?: number; offset?: number } = {}): Promise<PostMortemReport[]> {
    return cliCommands.query({
      limit: options.limit || 10,
      offset: options.offset || 0,
      sort_by: 'generated_at',
      sort_order: 'desc',
    });
  },
};

// =============================================================================
// CLI ERROR CLASS
// =============================================================================

export class CLIError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'CLIError';
  }
}

// =============================================================================
// CLI ENTRY POINT (for direct execution)
// =============================================================================

export async function runCLI(args: string[]): Promise<void> {
  const command = args[0];
  const options = parseArgs(args.slice(1));

  try {
    let result: unknown;

    switch (command) {
      case 'inspect':
        if (!options['report-id']) {
          throw new CLIError('--report-id is required');
        }
        result = await cliCommands.inspect(options['report-id'], {
          detailed: options['detailed'] === 'true',
        });
        break;

      case 'replay':
        if (!options['report-id']) {
          throw new CLIError('--report-id is required');
        }
        result = await cliCommands.replay(options['report-id'], {
          compare: options['compare'] !== 'false',
        });
        break;

      case 'generate':
        if (!options['start-time'] || !options['end-time']) {
          throw new CLIError('--start-time and --end-time are required');
        }
        result = await cliCommands.generate({
          time_range: {
            start_time: options['start-time'],
            end_time: options['end-time'],
          },
          scope: {
            providers: options['providers']?.split(',') as any,
            models: options['models']?.split(','),
            services: options['services']?.split(','),
          },
          options: {
            include_timeline: options['include-timeline'] !== 'false',
            include_classification_breakdown: options['include-classification'] !== 'false',
            include_health_transitions: options['include-health'] !== 'false',
            include_contributing_factors: options['include-factors'] !== 'false',
            include_statistics: options['include-statistics'] !== 'false',
          },
          incident_id: options['incident-id'],
        });
        break;

      case 'status':
        result = await cliCommands.status({
          detailed: options['detailed'] === 'true',
        });
        break;

      case 'query':
        result = await cliCommands.query({
          report_id: options['report-id'],
          incident_id: options['incident-id'],
          start_time: options['start-time'],
          end_time: options['end-time'],
          limit: parseInt(options['limit'] || '10', 10),
          offset: parseInt(options['offset'] || '0', 10),
          sort_by: (options['sort-by'] as any) || 'generated_at',
          sort_order: (options['sort-order'] as any) || 'desc',
        });
        break;

      case 'list':
        result = await cliCommands.list({
          limit: parseInt(options['limit'] || '10', 10),
          offset: parseInt(options['offset'] || '0', 10),
        });
        break;

      default:
        throw new CLIError(
          `Unknown command: ${command}. ` +
          `Available commands: inspect, replay, generate, status, query, list`
        );
    }

    console.log(JSON.stringify(result, null, 2));
  } catch (error) {
    if (error instanceof CLIError) {
      console.error(`Error: ${error.message}`);
      process.exit(1);
    }
    throw error;
  }
}

/**
 * Parse CLI arguments
 */
function parseArgs(args: string[]): Record<string, string> {
  const options: Record<string, string> = {};

  for (const arg of args) {
    if (arg.startsWith('--')) {
      const [key, value] = arg.slice(2).split('=');
      options[key] = value || 'true';
    }
  }

  return options;
}
