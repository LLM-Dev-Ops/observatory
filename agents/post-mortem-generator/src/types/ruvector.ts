/**
 * RuVector Service Types for Post-Mortem Generator Agent
 *
 * Types for interacting with ruvector-service for persistence and querying.
 * CONSTITUTION: All persistence goes through ruvector-service - NO direct database access.
 */

import type {
  FailureCategory,
  FailureSeverity,
  FailureCause,
  HealthState,
  Provider,
} from '../../contracts/schemas.js';

// =============================================================================
// CONFIGURATION
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

// =============================================================================
// PERSISTENCE TYPES
// =============================================================================

export interface PersistResult {
  success: boolean;
  eventId: string;
  timestamp: Date;
  error?: string;
  retries: number;
}

export interface BatchPersistRequest {
  events: unknown[];
  correlationId?: string;
}

export interface BatchPersistResult {
  total: number;
  successful: number;
  failed: number;
  results: PersistResult[];
  errors: Array<{ index: number; error: string }>;
}

// =============================================================================
// QUERY TYPES
// =============================================================================

export interface DecisionQuery {
  agentId?: string;
  agentVersion?: string;
  decisionType?: string;
  startTime?: Date;
  endTime?: Date;
  limit?: number;
  offset?: number;
  sortBy?: 'timestamp' | 'confidence';
  sortOrder?: 'asc' | 'desc';
}

export interface TelemetryQuery {
  startTime: Date;
  endTime: Date;
  providers?: Provider[];
  models?: string[];
  environments?: string[];
  userIds?: string[];
  limit?: number;
  offset?: number;
  aggregations?: string[];
}

export interface FailureClassificationQuery {
  startTime: Date;
  endTime: Date;
  providers?: Provider[];
  models?: string[];
  categories?: FailureCategory[];
  severities?: FailureSeverity[];
  causes?: FailureCause[];
  limit?: number;
  offset?: number;
}

export interface HealthEvaluationQuery {
  startTime: Date;
  endTime: Date;
  targetIds?: string[];
  targetTypes?: string[];
  healthStates?: HealthState[];
  limit?: number;
  offset?: number;
}

// =============================================================================
// STORED DATA TYPES
// =============================================================================

export interface StoredFailureClassification {
  span_id: string;
  trace_id: string;
  provider: Provider;
  model: string;
  category: FailureCategory;
  severity: FailureSeverity;
  cause: FailureCause;
  confidence: number;
  classified_at: string;
  classification_signals: Array<{
    signal_type: string;
    signal_value: string;
    weight: number;
  }>;
  recommendations: string[];
  execution_ref: string;
}

export interface StoredHealthEvaluation {
  evaluation_id: string;
  target_id: string;
  target_type: string;
  health_state: HealthState;
  previous_state?: HealthState;
  confidence: number;
  evaluated_at: string;
  indicators: Array<{
    type: string;
    value: number;
    threshold: number;
    state: HealthState;
  }>;
  execution_ref: string;
}

export interface StoredTelemetryEvent {
  span_id: string;
  trace_id: string;
  provider: Provider;
  model: string;
  status: 'OK' | 'ERROR' | 'UNSET';
  timestamp: string;
  latency_ms: number;
  token_usage?: {
    prompt_tokens: number;
    completion_tokens: number;
    total_tokens: number;
  };
  cost_usd?: number;
  error?: {
    code: string;
    message: string;
    http_status?: number;
  };
  metadata?: Record<string, unknown>;
}

// =============================================================================
// AGGREGATION TYPES
// =============================================================================

export interface AggregatedFailureData {
  total_failures: number;
  by_category: Map<FailureCategory, number>;
  by_severity: Map<FailureSeverity, number>;
  by_cause: Map<FailureCause, number>;
  by_provider: Map<Provider, {
    count: number;
    models: Set<string>;
    first_occurrence: string;
    last_occurrence: string;
  }>;
  time_series: Array<{
    timestamp: string;
    count: number;
    error_rate: number;
  }>;
}

export interface AggregatedHealthData {
  health_transitions: Array<{
    timestamp: string;
    target_id: string;
    target_type: string;
    from_state: HealthState;
    to_state: HealthState;
    duration_in_previous_state_ms?: number;
  }>;
  state_durations: Map<HealthState, number>;
  current_states: Map<string, HealthState>;
}

export interface AggregatedTelemetryData {
  total_requests: number;
  total_errors: number;
  error_rate: number;
  latency_stats: {
    min_ms: number;
    max_ms: number;
    avg_ms: number;
    p50_ms: number;
    p95_ms: number;
    p99_ms: number;
  };
  by_provider: Map<Provider, {
    request_count: number;
    error_count: number;
    models: Set<string>;
  }>;
  peak_error_rate: {
    value: number;
    timestamp: string;
  };
}

// =============================================================================
// HEALTH STATUS
// =============================================================================

export interface HealthStatus {
  healthy: boolean;
  latencyMs: number;
  lastCheck: Date;
  endpoint: string;
  version?: string;
  error?: string;
}

// =============================================================================
// RESPONSE TYPES
// =============================================================================

export interface FailureClassificationResponse {
  query: FailureClassificationQuery;
  classifications: StoredFailureClassification[];
  total_count: number;
  metadata: {
    query_time_ms: number;
    from_cache: boolean;
  };
}

export interface HealthEvaluationResponse {
  query: HealthEvaluationQuery;
  evaluations: StoredHealthEvaluation[];
  total_count: number;
  metadata: {
    query_time_ms: number;
    from_cache: boolean;
  };
}

export interface TelemetryResponse {
  query: TelemetryQuery;
  events: StoredTelemetryEvent[];
  total_count: number;
  metadata: {
    query_time_ms: number;
    from_cache: boolean;
  };
}

export interface DecisionEventResponse {
  events: unknown[];
  total_count: number;
  metadata: {
    query_time_ms: number;
  };
}
