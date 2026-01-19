/**
 * Ruvector-specific types for usage-pattern agent.
 * All types related to ruvector-service integration.
 *
 * CONSTITUTION: All persistence goes through ruvector-service HTTP client.
 * - NO direct database access
 * - NO direct SQL execution
 * - All operations are async and non-blocking
 */

/**
 * Configuration for ruvector-service client.
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

/**
 * Result of persisting a decision event.
 */
export interface PersistResult {
  success: boolean;
  eventId: string;
  timestamp: Date;
  error?: string;
  retries?: number;
}

/**
 * Query parameters for retrieving decision events.
 */
export interface DecisionQuery {
  agentId?: string;
  agentVersion?: string;
  decisionType?: string;
  startTime?: Date;
  endTime?: Date;
  limit?: number;
  offset?: number;
  sortBy?: 'timestamp' | 'decision_type' | 'agent_id';
  sortOrder?: 'asc' | 'desc';
}

/**
 * Query parameters for retrieving telemetry events for analysis.
 */
export interface TelemetryQuery {
  startTime: Date;
  endTime: Date;
  providers?: string[];
  models?: string[];
  environments?: string[];
  userIds?: string[];
  limit?: number;
  offset?: number;
  aggregations?: AggregationSpec[];
}

/**
 * Aggregation specification for telemetry queries.
 */
export interface AggregationSpec {
  field: string;
  type: 'sum' | 'avg' | 'min' | 'max' | 'count' | 'distinct_count' | 'percentile';
  alias?: string;
  percentile?: number;
}

/**
 * Health status of ruvector-service connection.
 */
export interface HealthStatus {
  healthy: boolean;
  latencyMs: number;
  lastCheck: Date;
  endpoint: string;
  version?: string;
  error?: string;
}

/**
 * Batch persist request.
 */
export interface BatchPersistRequest {
  events: unknown[];
  options?: {
    failFast?: boolean;
    continueOnError?: boolean;
  };
}

/**
 * Batch persist result.
 */
export interface BatchPersistResult {
  total: number;
  successful: number;
  failed: number;
  results: PersistResult[];
  errors: Array<{ index: number; error: string }>;
}

/**
 * Telemetry event from ruvector-service (raw format).
 */
export interface StoredTelemetryEvent {
  id: string;
  span_id: string;
  trace_id: string;
  parent_span_id?: string;
  name: string;
  provider: string;
  model: string;
  token_usage?: {
    prompt_tokens: number;
    completion_tokens: number;
    total_tokens: number;
  };
  cost?: {
    amount_usd: number;
    currency: string;
    prompt_cost?: number;
    completion_cost?: number;
  };
  latency: {
    total_ms: number;
    ttft_ms?: number;
    start_time: string;
    end_time: string;
  };
  status: 'OK' | 'ERROR' | 'UNSET';
  metadata: {
    user_id?: string;
    session_id?: string;
    environment?: string;
    tags: string[];
    attributes: Record<string, string>;
  };
  normalized_at: string;
  created_at: string;
}

/**
 * Aggregated telemetry response from ruvector-service.
 */
export interface AggregatedTelemetryResponse {
  query: TelemetryQuery;
  total_count: number;
  events?: StoredTelemetryEvent[];
  aggregations?: Record<string, number | Record<string, number>>;
  metadata: {
    query_time_ms: number;
    from_cache: boolean;
  };
}

/**
 * Agent status information.
 */
export interface AgentStatus {
  agentId: string;
  agentVersion: string;
  uptime: number;
  analysesPerformed: number;
  lastAnalysisTimestamp?: Date;
  ruvectorConnected: boolean;
  errors: number;
  health: 'healthy' | 'degraded' | 'unhealthy';
  metrics: {
    avgAnalysisTimeMs: number;
    eventsAnalyzedTotal: number;
    analysesPerHour: number;
  };
}
