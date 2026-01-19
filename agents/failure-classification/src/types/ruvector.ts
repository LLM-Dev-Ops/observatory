/**
 * RuVector-specific types
 *
 * These types are specific to the ruvector-service integration.
 */

// Re-export from contracts for convenience
export type {
  RuvectorConfig,
  RuvectorHealthStatus,
  DecisionQuery,
} from '../../contracts';

// =============================================================================
// RUVECTOR API TYPES
// =============================================================================

/**
 * RuVector API response wrapper
 */
export interface RuvectorApiResponse<T> {
  success: boolean;
  data?: T;
  error?: {
    code: string;
    message: string;
    details?: Record<string, unknown>;
  };
  metadata?: {
    request_id: string;
    duration_ms: number;
  };
}

/**
 * Decision event persistence request
 */
export interface PersistDecisionEventRequest {
  event: import('../../contracts').DecisionEvent;
  options?: {
    idempotency_key?: string;
    priority?: 'normal' | 'high';
  };
}

/**
 * Decision event persistence response
 */
export interface PersistDecisionEventResponse {
  id: string;
  execution_ref: string;
  persisted_at: string;
}

/**
 * Batch persistence request
 */
export interface BatchPersistRequest {
  events: import('../../contracts').DecisionEvent[];
  options?: {
    idempotency_key?: string;
    fail_on_first_error?: boolean;
  };
}

/**
 * Batch persistence response
 */
export interface BatchPersistResponse {
  total: number;
  succeeded: number;
  failed: number;
  results: Array<{
    execution_ref: string;
    success: boolean;
    error?: string;
  }>;
}

/**
 * Query decision events request
 */
export interface QueryDecisionEventsRequest {
  filters?: {
    agent_id?: string;
    agent_version?: string;
    decision_type?: string;
    start_time?: string;
    end_time?: string;
  };
  pagination?: {
    limit?: number;
    offset?: number;
    cursor?: string;
  };
  sort?: {
    field: string;
    order: 'asc' | 'desc';
  };
}

/**
 * Query decision events response
 */
export interface QueryDecisionEventsResponse {
  events: import('../../contracts').DecisionEvent[];
  total: number;
  has_more: boolean;
  next_cursor?: string;
}

/**
 * Aggregation request
 */
export interface AggregationRequest {
  agent_id: string;
  start_time: string;
  end_time: string;
  group_by: string;
  metrics?: string[];
}

/**
 * Aggregation response
 */
export interface AggregationResponse {
  aggregations: Array<{
    key: string;
    count: number;
    percentage: number;
    metrics?: Record<string, number>;
  }>;
  total: number;
  time_range: {
    start: string;
    end: string;
  };
}

// =============================================================================
// ERROR TYPES
// =============================================================================

/**
 * RuVector error codes
 */
export enum RuvectorErrorCode {
  VALIDATION_ERROR = 'VALIDATION_ERROR',
  NOT_FOUND = 'NOT_FOUND',
  CONFLICT = 'CONFLICT',
  RATE_LIMITED = 'RATE_LIMITED',
  INTERNAL_ERROR = 'INTERNAL_ERROR',
  SERVICE_UNAVAILABLE = 'SERVICE_UNAVAILABLE',
  TIMEOUT = 'TIMEOUT',
  AUTHENTICATION_ERROR = 'AUTHENTICATION_ERROR',
  AUTHORIZATION_ERROR = 'AUTHORIZATION_ERROR',
}

/**
 * Map HTTP status to error code
 */
export function httpStatusToErrorCode(status: number): RuvectorErrorCode {
  switch (status) {
    case 400:
      return RuvectorErrorCode.VALIDATION_ERROR;
    case 401:
      return RuvectorErrorCode.AUTHENTICATION_ERROR;
    case 403:
      return RuvectorErrorCode.AUTHORIZATION_ERROR;
    case 404:
      return RuvectorErrorCode.NOT_FOUND;
    case 409:
      return RuvectorErrorCode.CONFLICT;
    case 429:
      return RuvectorErrorCode.RATE_LIMITED;
    case 500:
      return RuvectorErrorCode.INTERNAL_ERROR;
    case 503:
      return RuvectorErrorCode.SERVICE_UNAVAILABLE;
    case 504:
      return RuvectorErrorCode.TIMEOUT;
    default:
      return RuvectorErrorCode.INTERNAL_ERROR;
  }
}
