/**
 * Ruvector-specific types for telemetry-collector agent
 * All types related to ruvector-service integration
 */

/**
 * Configuration for ruvector-service client
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
 * Result of persisting a decision event
 */
export interface PersistResult {
  success: boolean;
  eventId: string;
  timestamp: Date;
  error?: string;
  retries?: number;
}

/**
 * Query parameters for retrieving decision events
 */
export interface DecisionQuery {
  agentId?: string;
  agentVersion?: string;
  startTime?: Date;
  endTime?: Date;
  eventTypes?: string[];
  limit?: number;
  offset?: number;
  sortBy?: 'timestamp' | 'eventType' | 'agentId';
  sortOrder?: 'asc' | 'desc';
}

/**
 * Health status of ruvector-service connection
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
 * Result of replaying a historical event
 */
export interface ReplayResult {
  eventId: string;
  originalTimestamp: Date;
  replayTimestamp: Date;
  event: any;
  analysis: {
    modelUsed?: string;
    toolsInvoked?: string[];
    errors?: string[];
    warnings?: string[];
  };
  success: boolean;
  error?: string;
}

/**
 * Query parameters for analysis operations
 */
export interface AnalysisQuery {
  agentId?: string;
  startTime?: Date;
  endTime?: Date;
  pattern?: string;
  aggregateBy?: 'hour' | 'day' | 'week';
  metrics?: string[];
}

/**
 * Result of analysis query
 */
export interface AnalysisResult {
  query: AnalysisQuery;
  timestamp: Date;
  totalEvents: number;
  patterns: PatternMatch[];
  metrics: Record<string, any>;
  insights: string[];
}

/**
 * Pattern match found during analysis
 */
export interface PatternMatch {
  pattern: string;
  count: number;
  examples: string[];
  confidence: number;
}

/**
 * Agent status information
 */
export interface AgentStatus {
  agentId: string;
  agentVersion: string;
  uptime: number;
  eventsProcessed: number;
  lastEventTimestamp?: Date;
  ruvectorConnected: boolean;
  errors: number;
  health: 'healthy' | 'degraded' | 'unhealthy';
  metrics: {
    avgProcessingTimeMs: number;
    eventsPerSecond: number;
    errorRate: number;
  };
}

/**
 * Batch persist request
 */
export interface BatchPersistRequest {
  events: any[];
  options?: {
    failFast?: boolean;
    continueOnError?: boolean;
  };
}

/**
 * Batch persist result
 */
export interface BatchPersistResult {
  total: number;
  successful: number;
  failed: number;
  results: PersistResult[];
  errors: Array<{ index: number; error: string }>;
}
