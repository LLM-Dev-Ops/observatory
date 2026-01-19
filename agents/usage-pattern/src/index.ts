/**
 * Usage Pattern Agent - Main Entry Point
 *
 * CONSTITUTION:
 * - Classification: READ-ONLY, ADVISORY
 * - decision_type: "usage_pattern_analysis"
 * - confidence: STATISTICAL (0.0-1.0 based on sample size and variance)
 * - constraints_applied: ALWAYS [] (advisory agent applies no constraints)
 *
 * Purpose:
 * Analyze aggregated telemetry to identify system usage trends, hotspots,
 * growth patterns, and behavioral characteristics over time.
 *
 * Scope:
 * - Consume normalized telemetry events
 * - Perform statistical aggregation across time windows
 * - Identify trends, seasonality, and usage distributions
 * - Produce analytical summaries suitable for dashboards and forecasting
 *
 * This agent MUST NOT:
 * - Classify failures
 * - Evaluate health
 * - Enforce thresholds
 * - Generate alerts
 * - Modify system behavior
 * - Trigger orchestration
 *
 * Primary consumers:
 * - LLM-Analytics-Hub
 * - Governance dashboards
 * - Platform usage reporting
 */

// Configuration
export { loadConfig, validateConfig, getDefaultConfig, mergeConfig } from './config.js';
export type { AgentConfig } from './config.js';

// Ruvector client
export { RuvectorClient } from './ruvector-client.js';
export type {
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

// Core analyzer
export { UsagePatternAnalyzer } from './analyzer.js';

// Decision event emitter
export {
  DecisionEventEmitter,
  validateConstitutionalConstraints,
} from './decision-emitter.js';
export type { EmitParams, EmitResult } from './decision-emitter.js';

// Edge Function handler
export { handleRequest, usagePatternAnalyzer } from './handler.js';
export type { EdgeRequest, EdgeResponse, EdgeContext } from './handler.js';

// Contract schemas
export {
  // Input schemas
  NormalizedTelemetryInputSchema,
  AnalysisRequestSchema,

  // Output schemas
  TimeBucketSchema,
  DistributionStatsSchema,
  ProviderUsageSchema,
  TrendAnalysisSchema,
  SeasonalityPatternSchema,
  UsageHotspotSchema,
  GrowthPatternSchema,
  UsagePatternAnalysisSchema,

  // Decision event schema
  UsagePatternDecisionEventSchema,

  // CLI schemas
  CLIInvocationSchema,
  CLIOutputSchema,

  // Error schemas
  ErrorCodeSchema,
  ErrorResponseSchema,
} from '../contracts/schemas.js';

export type {
  NormalizedTelemetryInput,
  AnalysisRequest,
  TimeBucket,
  DistributionStats,
  ProviderUsage,
  TrendAnalysis,
  SeasonalityPattern,
  UsageHotspot,
  GrowthPattern,
  UsagePatternAnalysis,
  UsagePatternDecisionEvent,
  CLIInvocation,
  CLIOutput,
  ErrorCode,
  ErrorResponse,
} from '../contracts/schemas.js';

/**
 * Agent metadata for registration in agentics-contracts.
 */
export const AGENT_METADATA = {
  // Identity
  agent_id: 'usage-pattern-agent',
  agent_version: '1.0.0',
  agent_type: 'observatory',

  // Classification (CONSTITUTIONAL)
  classification: 'advisory' as const,
  read_only: true,
  advisory: true,

  // Decision configuration
  decision_type: 'usage_pattern_analysis' as const,
  confidence_type: 'statistical' as const,
  constraints_applied: [] as const,

  // Capabilities
  capabilities: [
    'telemetry_aggregation',
    'trend_analysis',
    'seasonality_detection',
    'distribution_statistics',
    'provider_usage_breakdown',
    'hotspot_identification',
    'growth_pattern_analysis',
  ],

  // Explicit non-responsibilities (CONSTITUTIONAL)
  non_responsibilities: [
    'failure_classification',
    'health_evaluation',
    'threshold_enforcement',
    'alert_generation',
    'orchestration_trigger',
    'remediation_trigger',
    'policy_modification',
    'routing_modification',
    'system_state_modification',
  ],

  // Integration
  primary_consumers: [
    'llm-analytics-hub',
    'governance-dashboards',
    'platform-usage-reporting',
  ],

  // Endpoints
  endpoints: {
    analyze: {
      method: 'POST',
      path: '/analyze',
      description: 'Run usage pattern analysis',
    },
    health: {
      method: 'GET',
      path: '/health',
      description: 'Health check',
    },
    status: {
      method: 'GET',
      path: '/status',
      description: 'Agent status and capabilities',
    },
    getAnalysis: {
      method: 'GET',
      path: '/analysis/:id',
      description: 'Retrieve historical analysis',
    },
  },

  // CLI commands
  cli_commands: ['analyze', 'inspect', 'replay', 'status', 'health'],

  // Deployment
  deployment: {
    platform: 'google-cloud',
    type: 'edge-function',
    runtime: 'nodejs20',
    memory: '512MB',
    timeout: '60s',
    region: 'us-central1',
  },

  // Schema references
  schemas: {
    input: 'AnalysisRequestSchema',
    output: 'UsagePatternAnalysisSchema',
    decision_event: 'UsagePatternDecisionEventSchema',
  },
} as const;

/**
 * Agent registration for agentics-contracts.
 */
export function getAgentRegistration() {
  return {
    ...AGENT_METADATA,
    registered_at: new Date().toISOString(),
    schema_version: '1.0.0',
  };
}
