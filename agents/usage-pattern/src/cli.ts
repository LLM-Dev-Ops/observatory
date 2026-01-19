#!/usr/bin/env node
/**
 * CLI for Usage Pattern Agent - Inspection, Replay, and Analysis.
 *
 * CONSTITUTION:
 * - This CLI provides inspection/replay/analysis capabilities only
 * - No orchestration hooks
 * - No execution triggers
 * - No auto-remediation paths
 *
 * Commands:
 * - analyze: Run usage pattern analysis
 * - inspect: Inspect a historical analysis
 * - replay: Replay analysis (dry-run by default)
 * - status: Show agent status
 * - health: Check ruvector-service health
 */

import { loadConfig, validateConfig } from './config.js';
import { RuvectorClient } from './ruvector-client.js';
import { UsagePatternAnalyzer } from './analyzer.js';
import { DecisionEventEmitter } from './decision-emitter.js';
import { handleRequest, EdgeRequest } from './handler.js';
import {
  CLIInvocation,
  CLIOutput,
  AnalysisRequest,
  UsagePatternAnalysis,
} from '../contracts/schemas.js';

/**
 * CLI argument parser (simple implementation).
 */
interface ParsedArgs {
  command: string;
  options: Record<string, string | boolean>;
  positional: string[];
}

function parseArgs(args: string[]): ParsedArgs {
  const result: ParsedArgs = {
    command: args[0] || 'help',
    options: {},
    positional: [],
  };

  for (let i = 1; i < args.length; i++) {
    const arg = args[i];

    if (arg.startsWith('--')) {
      const [key, value] = arg.slice(2).split('=');
      if (value !== undefined) {
        result.options[key] = value;
      } else if (args[i + 1] && !args[i + 1].startsWith('-')) {
        result.options[key] = args[++i];
      } else {
        result.options[key] = true;
      }
    } else if (arg.startsWith('-')) {
      const key = arg.slice(1);
      if (args[i + 1] && !args[i + 1].startsWith('-')) {
        result.options[key] = args[++i];
      } else {
        result.options[key] = true;
      }
    } else {
      result.positional.push(arg);
    }
  }

  return result;
}

/**
 * Format output based on requested format.
 */
function formatOutput(data: unknown, format: string): string {
  switch (format) {
    case 'json':
      return JSON.stringify(data, null, 2);
    case 'table':
      return formatTable(data);
    case 'csv':
      return formatCsv(data);
    default:
      return JSON.stringify(data, null, 2);
  }
}

/**
 * Format data as ASCII table.
 */
function formatTable(data: unknown): string {
  if (Array.isArray(data)) {
    if (data.length === 0) return '(empty)';
    const headers = Object.keys(data[0] as object);
    const rows = data.map((row) =>
      headers.map((h) => String((row as Record<string, unknown>)[h] ?? ''))
    );

    const widths = headers.map((h, i) =>
      Math.max(h.length, ...rows.map((r) => r[i].length))
    );

    const separator = widths.map((w) => '-'.repeat(w)).join('-+-');
    const headerRow = headers.map((h, i) => h.padEnd(widths[i])).join(' | ');
    const dataRows = rows.map((r) =>
      r.map((c, i) => c.padEnd(widths[i])).join(' | ')
    );

    return [headerRow, separator, ...dataRows].join('\n');
  }

  if (typeof data === 'object' && data !== null) {
    const entries = Object.entries(data as Record<string, unknown>);
    const maxKeyLen = Math.max(...entries.map(([k]) => k.length));
    return entries
      .map(([k, v]) => `${k.padEnd(maxKeyLen)}: ${JSON.stringify(v)}`)
      .join('\n');
  }

  return String(data);
}

/**
 * Format data as CSV.
 */
function formatCsv(data: unknown): string {
  if (!Array.isArray(data) || data.length === 0) {
    return '';
  }

  const headers = Object.keys(data[0] as object);
  const rows = data.map((row) =>
    headers.map((h) => {
      const val = (row as Record<string, unknown>)[h];
      const str = String(val ?? '');
      return str.includes(',') || str.includes('"')
        ? `"${str.replace(/"/g, '""')}"`
        : str;
    })
  );

  return [headers.join(','), ...rows.map((r) => r.join(','))].join('\n');
}

/**
 * Print help message.
 */
function printHelp(): void {
  console.log(`
Usage Pattern Agent CLI

USAGE:
  usage-pattern <command> [options]

COMMANDS:
  analyze     Run usage pattern analysis
  inspect     Inspect a historical analysis by ID
  replay      Replay a historical analysis
  status      Show agent status and capabilities
  health      Check ruvector-service health
  help        Show this help message

ANALYZE OPTIONS:
  --start <datetime>        Start of time window (ISO 8601)
  --end <datetime>          End of time window (ISO 8601)
  --granularity <value>     Time bucket granularity (minute|hour|day|week|month)
  --providers <list>        Comma-separated provider filter
  --models <list>           Comma-separated model filter
  --format <value>          Output format (json|table|csv)
  --include-trends          Include trend analysis
  --include-seasonality     Include seasonality detection
  --include-forecasts       Include forecasting

INSPECT OPTIONS:
  --id <analysis-id>        Analysis ID to inspect

REPLAY OPTIONS:
  --id <analysis-id>        Analysis ID to replay
  --dry-run                 Don't persist new DecisionEvent (default: true)

EXAMPLES:
  # Analyze last 24 hours
  usage-pattern analyze --start "2025-01-18T00:00:00Z" --end "2025-01-19T00:00:00Z"

  # Analyze with hourly granularity, JSON output
  usage-pattern analyze --start "2025-01-18T00:00:00Z" --end "2025-01-19T00:00:00Z" \\
    --granularity hour --format json

  # Filter by provider
  usage-pattern analyze --start "2025-01-18T00:00:00Z" --end "2025-01-19T00:00:00Z" \\
    --providers "openai,anthropic"

  # Inspect historical analysis
  usage-pattern inspect --id "550e8400-e29b-41d4-a716-446655440000"

  # Check health
  usage-pattern health

CONSTITUTION:
  This agent is READ-ONLY and ADVISORY.
  It does NOT classify failures, evaluate health, enforce thresholds, or generate alerts.
  Primary consumers: LLM-Analytics-Hub, Governance dashboards, Platform usage reporting.
`);
}

/**
 * Run the analyze command.
 */
async function runAnalyze(args: ParsedArgs): Promise<CLIOutput> {
  const startTime = args.options.start as string;
  const endTime = args.options.end as string;

  if (!startTime || !endTime) {
    return {
      success: false,
      command: 'analyze',
      timestamp: new Date().toISOString(),
      error: {
        code: 'INVALID_INPUT',
        message: '--start and --end are required',
      },
    };
  }

  const granularity = (args.options.granularity as string) || 'hour';
  const providers = args.options.providers
    ? (args.options.providers as string).split(',')
    : undefined;
  const models = args.options.models
    ? (args.options.models as string).split(',')
    : undefined;

  const request: AnalysisRequest = {
    time_window: {
      start: startTime,
      end: endTime,
      granularity: granularity as 'minute' | 'hour' | 'day' | 'week' | 'month',
    },
    filters: {
      providers,
      models,
    },
    options: {
      include_trends: args.options['include-trends'] === true,
      include_seasonality: args.options['include-seasonality'] === true,
      include_forecasts: args.options['include-forecasts'] === true,
      include_distributions: true,
      percentiles: [50, 90, 95, 99],
    },
  };

  const edgeRequest: EdgeRequest = {
    method: 'POST',
    url: 'http://localhost/analyze',
    headers: { 'Content-Type': 'application/json' },
    body: request,
  };

  const response = await handleRequest(edgeRequest);

  if (response.status !== 200) {
    const body = response.body as { error?: { code: string; message: string } };
    return {
      success: false,
      command: 'analyze',
      timestamp: new Date().toISOString(),
      error: {
        code: body.error?.code || 'INTERNAL_ERROR',
        message: body.error?.message || 'Analysis failed',
      },
    };
  }

  const body = response.body as { data: UsagePatternAnalysis };
  return {
    success: true,
    command: 'analyze',
    timestamp: new Date().toISOString(),
    result: body.data,
  };
}

/**
 * Run the inspect command.
 */
async function runInspect(args: ParsedArgs): Promise<CLIOutput> {
  const analysisId = args.options.id as string;

  if (!analysisId) {
    return {
      success: false,
      command: 'inspect',
      timestamp: new Date().toISOString(),
      error: {
        code: 'INVALID_INPUT',
        message: '--id is required',
      },
    };
  }

  const edgeRequest: EdgeRequest = {
    method: 'GET',
    url: `http://localhost/analysis/${analysisId}`,
    headers: {},
  };

  const response = await handleRequest(edgeRequest);

  if (response.status !== 200) {
    const body = response.body as { error?: { code: string; message: string } };
    return {
      success: false,
      command: 'inspect',
      timestamp: new Date().toISOString(),
      error: {
        code: body.error?.code || 'INTERNAL_ERROR',
        message: body.error?.message || 'Inspection failed',
      },
    };
  }

  const body = response.body as { data: UsagePatternAnalysis };
  return {
    success: true,
    command: 'inspect',
    timestamp: new Date().toISOString(),
    result: body.data,
  };
}

/**
 * Run the replay command.
 */
async function runReplay(args: ParsedArgs): Promise<CLIOutput> {
  const analysisId = args.options.id as string;
  const dryRun = args.options['dry-run'] !== false; // Default to true

  if (!analysisId) {
    return {
      success: false,
      command: 'replay',
      timestamp: new Date().toISOString(),
      error: {
        code: 'INVALID_INPUT',
        message: '--id is required',
      },
    };
  }

  // First, inspect the original analysis
  const inspectResult = await runInspect({
    command: 'inspect',
    options: { id: analysisId },
    positional: [],
  });

  if (!inspectResult.success || !inspectResult.result) {
    return {
      success: false,
      command: 'replay',
      timestamp: new Date().toISOString(),
      error: {
        code: 'INVALID_INPUT',
        message: `Could not find analysis: ${analysisId}`,
      },
    };
  }

  const originalAnalysis = inspectResult.result as UsagePatternAnalysis;

  // Re-run the analysis with the same parameters
  const replayResult = await runAnalyze({
    command: 'analyze',
    options: {
      start: originalAnalysis.time_window.start,
      end: originalAnalysis.time_window.end,
      granularity: originalAnalysis.time_window.granularity,
      'include-trends': originalAnalysis.trends ? 'true' : '',
      'include-seasonality': originalAnalysis.seasonality ? 'true' : '',
    },
    positional: [],
  });

  return {
    success: replayResult.success,
    command: 'replay',
    timestamp: new Date().toISOString(),
    result: {
      status: dryRun ? 'dry_run' : 'executed',
      message: dryRun
        ? 'Replay completed (dry-run mode, no DecisionEvent persisted)'
        : 'Replay completed and DecisionEvent persisted',
      original_analysis_id: analysisId,
      new_analysis: replayResult.result,
    },
  };
}

/**
 * Run the status command.
 */
async function runStatus(): Promise<CLIOutput> {
  const edgeRequest: EdgeRequest = {
    method: 'GET',
    url: 'http://localhost/status',
    headers: {},
  };

  const response = await handleRequest(edgeRequest);

  return {
    success: true,
    command: 'status',
    timestamp: new Date().toISOString(),
    result: response.body as { status: string; message: string },
  };
}

/**
 * Run the health command.
 */
async function runHealth(): Promise<CLIOutput> {
  const edgeRequest: EdgeRequest = {
    method: 'GET',
    url: 'http://localhost/health',
    headers: {},
  };

  const response = await handleRequest(edgeRequest);

  return {
    success: response.status === 200,
    command: 'health',
    timestamp: new Date().toISOString(),
    result: response.body as { status: string; message: string },
  };
}

/**
 * Main CLI entry point.
 */
async function main(): Promise<void> {
  const args = parseArgs(process.argv.slice(2));
  const format = (args.options.format as string) || 'json';

  let result: CLIOutput;

  switch (args.command) {
    case 'analyze':
      result = await runAnalyze(args);
      break;
    case 'inspect':
      result = await runInspect(args);
      break;
    case 'replay':
      result = await runReplay(args);
      break;
    case 'status':
      result = await runStatus();
      break;
    case 'health':
      result = await runHealth();
      break;
    case 'help':
    case '--help':
    case '-h':
      printHelp();
      return;
    default:
      console.error(`Unknown command: ${args.command}`);
      printHelp();
      process.exit(1);
  }

  console.log(formatOutput(result, format));
  process.exit(result.success ? 0 : 1);
}

// Run CLI
main().catch((error) => {
  console.error('Fatal error:', error);
  process.exit(1);
});
