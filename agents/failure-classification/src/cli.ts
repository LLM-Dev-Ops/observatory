/**
 * Failure Classification Agent - CLI Commands
 *
 * CONSTITUTIONAL CLASSIFICATION: READ-ONLY, DIAGNOSTIC
 *
 * These CLI commands are for inspection, replay, and analysis only.
 * They do NOT modify state or trigger actions.
 */

import type {
  CLIInspectResult,
  CLIReplayResult,
  CLIStatusResult,
  AnalysisQuery,
  AnalysisResult,
  FailureEvent,
  ClassificationQuery,
} from '../contracts';
import { AGENT_METADATA } from '../contracts';
import { RuvectorClient } from './ruvector-client';
import { classifyFailure, ClassificationEngine } from './classifier';
import { loadConfig } from './config';

// =============================================================================
// CLI INTERFACE
// =============================================================================

export interface CLICommands {
  inspect(spanId: string, options?: { includeSignals?: boolean }): Promise<CLIInspectResult>;
  replay(spanId: string, options?: { compare?: boolean }): Promise<CLIReplayResult>;
  analyze(query: AnalysisQuery): Promise<AnalysisResult>;
  status(options?: { detailed?: boolean }): Promise<CLIStatusResult>;
  query(query: ClassificationQuery): Promise<any[]>;
}

// =============================================================================
// CLI IMPLEMENTATION
// =============================================================================

const config = loadConfig();
const ruvectorClient = new RuvectorClient(config.ruvector);
const classificationEngine = new ClassificationEngine();

export const cliCommands: CLICommands = {
  /**
   * Inspect a specific classification result
   *
   * Usage: npx @llm-observatory/cli failure-classification inspect --span-id=<span_id>
   */
  async inspect(
    spanId: string,
    options: { includeSignals?: boolean } = {}
  ): Promise<CLIInspectResult> {
    // Get classification from ruvector
    const classification = await ruvectorClient.getClassificationBySpanId(spanId);

    if (!classification) {
      throw new CLIError(`No classification found for span_id: ${spanId}`);
    }

    // Get the original decision event
    const events = await ruvectorClient.getDecisionEvents({
      agentId: 'failure-classification-agent',
      limit: 100,
    });

    let decisionEvent = null;
    let event = null;

    for (const e of events) {
      if (e.outputs.some((o: any) => o.span_id === spanId)) {
        decisionEvent = e;
        break;
      }
    }

    const result: CLIInspectResult = {
      event: event as any, // Would need to fetch from telemetry source
      classification: options.includeSignals
        ? classification
        : { ...classification, classification_signals: [] },
      decision_event: decisionEvent as any,
    };

    return result;
  },

  /**
   * Replay classification for determinism verification
   *
   * Usage: npx @llm-observatory/cli failure-classification replay --span-id=<span_id>
   */
  async replay(
    spanId: string,
    options: { compare?: boolean } = { compare: true }
  ): Promise<CLIReplayResult> {
    // Get original classification
    const originalClassification = await ruvectorClient.getClassificationBySpanId(spanId);

    if (!originalClassification) {
      throw new CLIError(`No classification found for span_id: ${spanId}`);
    }

    // We need the original event to replay
    // In a real implementation, this would fetch from telemetry storage
    // For now, we'll throw an error indicating the limitation
    throw new CLIError(
      'Replay requires access to original failure event. ' +
      'This would be fetched from telemetry storage in production.'
    );

    // The implementation would look like:
    // const originalEvent = await getOriginalEvent(spanId);
    // const replayedClassification = await classifyFailure(originalEvent, classificationEngine);
    //
    // return {
    //   original_event: originalEvent,
    //   original_classification: originalClassification,
    //   replayed_classification: replayedClassification,
    //   match: deepEqual(originalClassification, replayedClassification),
    //   differences: findDifferences(originalClassification, replayedClassification),
    // };
  },

  /**
   * Analyze classification statistics
   *
   * Usage: npx @llm-observatory/cli failure-classification analyze [options]
   */
  async analyze(query: AnalysisQuery): Promise<AnalysisResult> {
    const endTime = query.end_time || new Date().toISOString();
    const startTime =
      query.start_time ||
      new Date(Date.now() - query.time_window_hours * 60 * 60 * 1000).toISOString();

    const stats = await ruvectorClient.getClassificationStats(
      startTime,
      endTime,
      query.group_by
    );

    return stats;
  },

  /**
   * Get agent status and health
   *
   * Usage: npx @llm-observatory/cli failure-classification status [--detailed]
   */
  async status(options: { detailed?: boolean } = {}): Promise<CLIStatusResult> {
    const ruvectorHealth = await ruvectorClient.healthCheck();

    // Get recent metrics
    const recentEvents = await ruvectorClient.getDecisionEvents({
      agentId: 'failure-classification-agent',
      startTime: new Date(Date.now() - 60 * 60 * 1000).toISOString(), // Last hour
      limit: 1000,
    });

    // Calculate metrics
    let totalLatency = 0;
    let totalConfidence = 0;
    let categoryCount: Record<string, number> = {};

    for (const event of recentEvents) {
      for (const output of event.outputs) {
        totalLatency += output.classification_latency_ms || 0;
        totalConfidence += output.confidence || 0;
        categoryCount[output.category] = (categoryCount[output.category] || 0) + 1;
      }
    }

    const totalClassifications = recentEvents.reduce(
      (sum, e) => sum + e.outputs.length,
      0
    );

    const status: CLIStatusResult = {
      agent_id: AGENT_METADATA.id,
      agent_version: AGENT_METADATA.version,
      classification: AGENT_METADATA.classification,
      status: ruvectorHealth.healthy ? 'healthy' : 'degraded',
      uptime_seconds: process.uptime(),
      last_classification_at:
        recentEvents.length > 0 ? recentEvents[0].timestamp : undefined,
      metrics: {
        total_classifications: totalClassifications,
        classifications_last_hour: totalClassifications,
        avg_latency_ms:
          totalClassifications > 0 ? totalLatency / totalClassifications : 0,
        error_rate: 0, // Would need error tracking
      },
      ruvector_status: ruvectorHealth,
    };

    return status;
  },

  /**
   * Query classifications
   *
   * Usage: npx @llm-observatory/cli failure-classification query [options]
   */
  async query(query: ClassificationQuery): Promise<any[]> {
    const events = await ruvectorClient.getDecisionEvents({
      agentId: 'failure-classification-agent',
      startTime: query.start_time,
      endTime: query.end_time,
      limit: query.limit,
      offset: query.offset,
      sortBy: query.sort_by,
      sortOrder: query.sort_order,
    });

    // Filter and flatten outputs
    const results = [];

    for (const event of events) {
      for (const output of event.outputs) {
        // Apply filters
        if (query.span_id && output.span_id !== query.span_id) continue;
        if (query.trace_id && output.trace_id !== query.trace_id) continue;
        if (query.category && output.category !== query.category) continue;
        if (query.severity && output.severity !== query.severity) continue;
        if (query.cause && output.cause !== query.cause) continue;

        results.push({
          ...output,
          execution_ref: event.execution_ref,
          agent_version: event.agent_version,
        });
      }
    }

    return results;
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
    let result: any;

    switch (command) {
      case 'inspect':
        if (!options['span-id']) {
          throw new CLIError('--span-id is required');
        }
        result = await cliCommands.inspect(options['span-id'], {
          includeSignals: options['include-signals'] === 'true',
        });
        break;

      case 'replay':
        if (!options['span-id']) {
          throw new CLIError('--span-id is required');
        }
        result = await cliCommands.replay(options['span-id'], {
          compare: options['compare'] !== 'false',
        });
        break;

      case 'analyze':
        result = await cliCommands.analyze({
          group_by: (options['group-by'] as any) || 'category',
          time_window_hours: parseInt(options['time-window'] || '24', 10),
          provider: options['provider'] as any,
        });
        break;

      case 'status':
        result = await cliCommands.status({
          detailed: options['detailed'] === 'true',
        });
        break;

      case 'query':
        result = await cliCommands.query({
          span_id: options['span-id'],
          trace_id: options['trace-id'],
          provider: options['provider'] as any,
          category: options['category'] as any,
          severity: options['severity'] as any,
          cause: options['cause'] as any,
          start_time: options['start-time'],
          end_time: options['end-time'],
          limit: parseInt(options['limit'] || '100', 10),
          offset: parseInt(options['offset'] || '0', 10),
          sort_by: (options['sort-by'] as any) || 'timestamp',
          sort_order: (options['sort-order'] as any) || 'desc',
        });
        break;

      default:
        throw new CLIError(
          `Unknown command: ${command}. ` +
          `Available commands: inspect, replay, analyze, status, query`
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
