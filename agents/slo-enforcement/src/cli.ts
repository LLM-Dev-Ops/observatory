#!/usr/bin/env node
/**
 * SLO/SLA Enforcement Agent - CLI
 *
 * Command-line interface for inspection, replay, and analysis.
 *
 * Commands:
 * - slo-enforce evaluate   - Evaluate SLOs from file
 * - slo-enforce query      - Query violations
 * - slo-enforce replay     - Replay a decision event
 * - slo-enforce analyze    - Get violation analysis
 * - slo-enforce health     - Check agent health
 */

import { readFileSync, writeFileSync, existsSync } from 'fs';
import { getRuvectorClient } from './ruvector-client';
import { getSloEnforcer } from './enforcer';
import { getMetrics } from './telemetry';
import { loadConfig } from './config';
import {
  AGENT_METADATA,
  validateSloEnforcementRequest,
  validateViolationQuery,
} from '../contracts';
import type {
  SloEnforcementRequest,
  ViolationQuery,
  SloViolation,
  AnalysisResult,
} from '../contracts';

// Parse command line arguments
interface CliArgs {
  command: string;
  subcommand?: string;
  options: Record<string, string | boolean>;
  positional: string[];
}

function parseArgs(argv: string[]): CliArgs {
  const args = argv.slice(2);
  const options: Record<string, string | boolean> = {};
  const positional: string[] = [];
  let command = '';
  let subcommand: string | undefined;

  for (let i = 0; i < args.length; i++) {
    const arg = args[i];

    if (arg.startsWith('--')) {
      const key = arg.slice(2);
      const nextArg = args[i + 1];

      if (nextArg && !nextArg.startsWith('-')) {
        options[key] = nextArg;
        i++;
      } else {
        options[key] = true;
      }
    } else if (arg.startsWith('-')) {
      const key = arg.slice(1);
      const nextArg = args[i + 1];

      if (nextArg && !nextArg.startsWith('-')) {
        options[key] = nextArg;
        i++;
      } else {
        options[key] = true;
      }
    } else if (!command) {
      command = arg;
    } else if (!subcommand) {
      subcommand = arg;
    } else {
      positional.push(arg);
    }
  }

  return { command, subcommand, options, positional };
}

// Help text
function printHelp(): void {
  console.log(`
SLO/SLA Enforcement Agent CLI v${AGENT_METADATA.version}

USAGE:
  slo-enforce <command> [options]

COMMANDS:
  evaluate    Evaluate SLOs against metrics from files
  query       Query violations from ruvector-service
  replay      Replay a decision event by execution reference
  analyze     Get aggregated violation analysis
  health      Check agent health status
  help        Show this help message

EXAMPLES:
  # Evaluate SLOs from files
  slo-enforce evaluate --slos slos.json --metrics metrics.json

  # Query violations
  slo-enforce query --severity critical --limit 10

  # Replay a decision
  slo-enforce replay --execution-ref abc123-def456

  # Get analysis
  slo-enforce analyze --start 2024-01-01T00:00:00Z --end 2024-01-02T00:00:00Z

  # Health check
  slo-enforce health

GLOBAL OPTIONS:
  --format <json|table>  Output format (default: table)
  --output <file>        Write output to file
  --help, -h             Show help for a command
  --version, -v          Show version

For more information, visit: https://github.com/org/observatory/agents/slo-enforcement
`);
}

// Format output
function formatOutput(
  data: unknown,
  format: 'json' | 'table' = 'table'
): string {
  if (format === 'json') {
    return JSON.stringify(data, null, 2);
  }

  // Table format - handle specific types
  if (Array.isArray(data)) {
    return formatTable(data);
  }

  return JSON.stringify(data, null, 2);
}

function formatTable(rows: unknown[]): string {
  if (rows.length === 0) return 'No results';

  const first = rows[0] as Record<string, unknown>;
  const columns = Object.keys(first);

  // Calculate column widths
  const widths = columns.map((col) => {
    const values = rows.map((row) => String((row as Record<string, unknown>)[col] ?? ''));
    return Math.max(col.length, ...values.map((v) => v.length));
  });

  // Build table
  const header = columns.map((col, i) => col.padEnd(widths[i])).join(' | ');
  const separator = widths.map((w) => '-'.repeat(w)).join('-+-');
  const dataRows = rows.map((row) =>
    columns.map((col, i) => String((row as Record<string, unknown>)[col] ?? '').padEnd(widths[i])).join(' | ')
  );

  return [header, separator, ...dataRows].join('\n');
}

// Commands
async function cmdEvaluate(args: CliArgs): Promise<void> {
  const slosFile = args.options['slos'] as string;
  const metricsFile = args.options['metrics'] as string;
  const outputFile = args.options['output'] as string | undefined;
  const format = (args.options['format'] as 'json' | 'table') ?? 'table';

  if (!slosFile || !metricsFile) {
    console.error('Error: --slos and --metrics are required');
    process.exit(1);
  }

  if (!existsSync(slosFile)) {
    console.error(`Error: SLOs file not found: ${slosFile}`);
    process.exit(1);
  }

  if (!existsSync(metricsFile)) {
    console.error(`Error: Metrics file not found: ${metricsFile}`);
    process.exit(1);
  }

  try {
    const slos = JSON.parse(readFileSync(slosFile, 'utf-8'));
    const metrics = JSON.parse(readFileSync(metricsFile, 'utf-8'));

    const request: SloEnforcementRequest = {
      slo_definitions: slos,
      metrics: metrics,
      evaluation_time: new Date().toISOString(),
    };

    const validation = validateSloEnforcementRequest(request);
    if (!validation.success) {
      console.error('Validation error:', JSON.stringify(validation.errors, null, 2));
      process.exit(1);
    }

    const enforcer = getSloEnforcer();
    const result = enforcer.evaluateAll(
      request.slo_definitions,
      request.metrics,
      new Date(request.evaluation_time)
    );

    const output = formatOutput(result, format);

    if (outputFile) {
      writeFileSync(outputFile, output);
      console.log(`Results written to: ${outputFile}`);
    } else {
      console.log(output);
    }

    // Summary
    console.log(`\nSummary:`);
    console.log(`  SLOs evaluated: ${result.slos_evaluated}`);
    console.log(`  Metrics evaluated: ${result.metrics_evaluated}`);
    console.log(`  Violations found: ${result.violations.length}`);
    console.log(`  Processing time: ${result.processing_time_ms}ms`);

  } catch (error) {
    console.error('Error:', (error as Error).message);
    process.exit(1);
  }
}

async function cmdQuery(args: CliArgs): Promise<void> {
  const format = (args.options['format'] as 'json' | 'table') ?? 'table';

  const query: Partial<ViolationQuery> = {
    slo_id: args.options['slo-id'] as string | undefined,
    severity: args.options['severity'] as ViolationQuery['severity'],
    breach_type: args.options['breach-type'] as ViolationQuery['breach_type'],
    provider: args.options['provider'] as string | undefined,
    start_time: args.options['start'] as string | undefined,
    end_time: args.options['end'] as string | undefined,
    is_sla: args.options['sla-only'] ? true : undefined,
    limit: parseInt(args.options['limit'] as string) || 100,
    offset: parseInt(args.options['offset'] as string) || 0,
    sort_by: (args.options['sort-by'] as ViolationQuery['sort_by']) || 'detected_at',
    sort_order: (args.options['sort-order'] as ViolationQuery['sort_order']) || 'desc',
  };

  try {
    const ruvector = getRuvectorClient();
    const violations = await ruvector.getViolations(query as ViolationQuery);

    if (violations.length === 0) {
      console.log('No violations found matching the query');
      return;
    }

    // Simplify for table display
    const simplified = violations.map((v: SloViolation) => ({
      violation_id: v.violation_id.slice(0, 8) + '...',
      slo_name: v.slo_name.slice(0, 30),
      severity: v.severity,
      breach_type: v.breach_type,
      detected_at: v.detected_at,
      is_sla: v.is_sla ? 'YES' : 'no',
    }));

    console.log(formatOutput(format === 'json' ? violations : simplified, format));
    console.log(`\nTotal: ${violations.length} violations`);

  } catch (error) {
    console.error('Error:', (error as Error).message);
    process.exit(1);
  }
}

async function cmdReplay(args: CliArgs): Promise<void> {
  const executionRef = args.options['execution-ref'] as string;
  const format = (args.options['format'] as 'json' | 'table') ?? 'json';

  if (!executionRef) {
    console.error('Error: --execution-ref is required');
    process.exit(1);
  }

  try {
    const ruvector = getRuvectorClient();
    const event = await ruvector.getDecisionEventByRef(executionRef);

    if (!event) {
      console.error(`DecisionEvent not found: ${executionRef}`);
      process.exit(1);
    }

    console.log(formatOutput(event, format));

  } catch (error) {
    console.error('Error:', (error as Error).message);
    process.exit(1);
  }
}

async function cmdAnalyze(args: CliArgs): Promise<void> {
  const startTime = args.options['start'] as string;
  const endTime = args.options['end'] as string;
  const groupBy = ((args.options['group-by'] as string) ?? 'severity').split(',');
  const format = (args.options['format'] as 'json' | 'table') ?? 'table';

  if (!startTime || !endTime) {
    console.error('Error: --start and --end are required');
    process.exit(1);
  }

  try {
    const ruvector = getRuvectorClient();
    const analysis = await ruvector.getViolationAnalysis(startTime, endTime, groupBy);

    console.log(formatOutput(analysis, format));

    // Print summary
    console.log(`\nAnalysis Summary:`);
    console.log(`  Time Range: ${analysis.time_range.start} to ${analysis.time_range.end}`);
    console.log(`  Total Violations: ${analysis.total_violations}`);
    console.log(`  Total Evaluations: ${analysis.total_evaluations}`);
    console.log(`  Violation Rate: ${(analysis.violation_rate * 100).toFixed(2)}%`);
    console.log(`  SLA Breaches: ${analysis.sla_breaches}`);

  } catch (error) {
    console.error('Error:', (error as Error).message);
    process.exit(1);
  }
}

async function cmdHealth(_args: CliArgs): Promise<void> {
  try {
    const config = loadConfig();
    const ruvector = getRuvectorClient();
    const ruvectorHealth = await ruvector.healthCheck();
    const metrics = getMetrics();

    console.log(`SLO/SLA Enforcement Agent Health Check`);
    console.log(`======================================`);
    console.log(`Agent ID: ${AGENT_METADATA.id}`);
    console.log(`Version: ${AGENT_METADATA.version}`);
    console.log(`Classification: ${AGENT_METADATA.classification}`);
    console.log(`Decision Type: ${AGENT_METADATA.decision_type}`);
    console.log(`Actuating: ${AGENT_METADATA.actuating}`);
    console.log(``);
    console.log(`RuVector Service:`);
    console.log(`  Endpoint: ${config.ruvector.endpoint}`);
    console.log(`  Status: ${ruvectorHealth.status}`);
    console.log(`  Latency: ${ruvectorHealth.latency_ms}ms`);
    console.log(``);
    console.log(`Agent Metrics:`);
    console.log(`  Uptime: ${metrics.uptime_seconds}s`);
    console.log(`  Evaluations: ${metrics.evaluations_total}`);
    console.log(`  Violations Detected: ${metrics.violations_detected}`);
    console.log(`  SLA Breaches: ${metrics.sla_breaches_detected}`);
    console.log(`  Errors: ${metrics.errors_total}`);
    console.log(`  Avg Latency: ${metrics.avg_latency_ms}ms`);

    const overallStatus = ruvectorHealth.status === 'healthy' ? 'HEALTHY' : 'DEGRADED';
    console.log(``);
    console.log(`Overall Status: ${overallStatus}`);

  } catch (error) {
    console.error('Health check failed:', (error as Error).message);
    process.exit(1);
  }
}

// Main
async function main(): Promise<void> {
  const args = parseArgs(process.argv);

  if (args.options['version'] || args.options['v']) {
    console.log(`slo-enforce v${AGENT_METADATA.version}`);
    process.exit(0);
  }

  if (args.options['help'] || args.options['h'] || !args.command || args.command === 'help') {
    printHelp();
    process.exit(0);
  }

  switch (args.command) {
    case 'evaluate':
      await cmdEvaluate(args);
      break;
    case 'query':
      await cmdQuery(args);
      break;
    case 'replay':
      await cmdReplay(args);
      break;
    case 'analyze':
      await cmdAnalyze(args);
      break;
    case 'health':
      await cmdHealth(args);
      break;
    default:
      console.error(`Unknown command: ${args.command}`);
      printHelp();
      process.exit(1);
  }
}

main().catch((error) => {
  console.error('Fatal error:', error);
  process.exit(1);
});
