/**
 * Failure Classification Agent - TypeScript Types
 *
 * Re-exports Zod-inferred types and defines additional type utilities.
 */

// Re-export all types from schemas
export type {
  Provider,
  FailureCategory,
  FailureSeverity,
  FailureCause,
  SpanStatus,
  SpanEvent,
  Latency,
  ErrorDetails,
  FailureEvent,
  BatchClassificationRequest,
  FailureClassification,
  BatchClassificationResult,
  DecisionEvent,
  ClassificationQuery,
  AnalysisQuery,
  AnalysisResult,
} from './schemas';

// =============================================================================
// AGENT CLASSIFICATION TYPES
// =============================================================================

/**
 * Agent classification - defines agent's role and permissions
 */
export type AgentClassification = 'READ-ONLY' | 'ADVISORY' | 'ENFORCEMENT-CLASS';

/**
 * Agent capability - what the agent can do
 */
export type AgentCapability =
  | 'classify_failures'
  | 'aggregate_statistics'
  | 'generate_reports'
  | 'emit_decision_events';

/**
 * Prohibited operation - what the agent MUST NOT do
 */
export type ProhibitedOperation =
  | 'sql_execute'
  | 'sql_write'
  | 'orchestration_trigger'
  | 'state_modify'
  | 'constraint_apply'
  | 'retry_trigger'
  | 'alert_trigger'
  | 'remediation_trigger'
  | 'incident_correlation'
  | 'escalation_trigger';

// =============================================================================
// CLASSIFICATION SIGNAL TYPES
// =============================================================================

/**
 * Signal used for classification decision
 */
export interface ClassificationSignal {
  signal_type: string;
  signal_value: string;
  weight: number;
}

/**
 * Classification rule definition
 */
export interface ClassificationRule {
  id: string;
  name: string;
  description: string;
  conditions: RuleCondition[];
  output: {
    category: import('./schemas').FailureCategory;
    severity: import('./schemas').FailureSeverity;
    cause: import('./schemas').FailureCause;
  };
  priority: number;
  confidence_base: number;
}

export interface RuleCondition {
  field: string;
  operator: 'equals' | 'contains' | 'matches' | 'in' | 'gt' | 'lt' | 'gte' | 'lte';
  value: string | number | string[];
}

// =============================================================================
// HANDLER TYPES
// =============================================================================

/**
 * Handler request context
 */
export interface HandlerContext {
  execution_ref: string;
  correlation_id?: string;
  received_at: string;
  source_ip?: string;
}

/**
 * Handler response wrapper
 */
export interface HandlerResponse<T> {
  success: boolean;
  data?: T;
  error?: {
    code: string;
    message: string;
    details?: Record<string, unknown>;
  };
  metadata: {
    execution_ref: string;
    processing_time_ms: number;
    agent_id: string;
    agent_version: string;
  };
}

// =============================================================================
// RUVECTOR CLIENT TYPES
// =============================================================================

export interface RuvectorConfig {
  endpoint: string;
  apiKey?: string;
  timeout: number;
  retryAttempts: number;
  retryDelayMs: number;
  maxRetryDelayMs: number;
  connectionPoolSize: number;
}

export interface RuvectorHealthStatus {
  healthy: boolean;
  latencyMs: number;
  lastCheck: string;
  endpoint: string;
  version?: string;
  error?: string;
}

export interface DecisionQuery {
  agentId?: string;
  agentVersion?: string;
  startTime?: string;
  endTime?: string;
  eventTypes?: string[];
  limit?: number;
  offset?: number;
  sortBy?: string;
  sortOrder?: 'asc' | 'desc';
}

// =============================================================================
// CLI TYPES
// =============================================================================

export interface CLIInspectResult {
  event: import('./schemas').FailureEvent;
  classification: import('./schemas').FailureClassification;
  decision_event: import('./schemas').DecisionEvent;
}

export interface CLIReplayResult {
  original_event: import('./schemas').FailureEvent;
  original_classification: import('./schemas').FailureClassification;
  replayed_classification: import('./schemas').FailureClassification;
  match: boolean;
  differences?: Array<{
    field: string;
    original: unknown;
    replayed: unknown;
  }>;
}

export interface CLIStatusResult {
  agent_id: string;
  agent_version: string;
  classification: AgentClassification;
  status: 'healthy' | 'degraded' | 'unhealthy';
  uptime_seconds: number;
  last_classification_at?: string;
  metrics: {
    total_classifications: number;
    classifications_last_hour: number;
    avg_latency_ms: number;
    error_rate: number;
  };
  ruvector_status: RuvectorHealthStatus;
}

// =============================================================================
// METRICS TYPES
// =============================================================================

export interface ClassificationMetrics {
  total_events_processed: number;
  events_by_category: Record<string, number>;
  events_by_severity: Record<string, number>;
  events_by_cause: Record<string, number>;
  events_by_provider: Record<string, number>;
  avg_classification_latency_ms: number;
  avg_confidence: number;
  processing_errors: number;
}

// =============================================================================
// TELEMETRY TYPES (Self-Observation)
// =============================================================================

export interface TelemetrySpan {
  name: string;
  trace_id: string;
  span_id: string;
  parent_span_id?: string;
  start_time: string;
  end_time?: string;
  status: 'OK' | 'ERROR' | 'UNSET';
  attributes: Record<string, string | number | boolean>;
}
