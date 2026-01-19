/**
 * SLO/SLA Enforcement Agent - Type Definitions
 *
 * Additional type definitions for internal use.
 */

import type { Request, Response } from 'express';
import type { SloDefinition, TelemetryMetric, SloViolation, SloStatus } from '../contracts';

/**
 * Handler Context - Passed through request processing
 */
export interface HandlerContext {
  execution_ref: string;
  correlation_id?: string;
  received_at: string;
  source_ip?: string;
}

/**
 * Handler Response - Standard response format
 */
export interface HandlerResponse<T> {
  success: boolean;
  data?: T;
  error?: {
    code: string;
    message: string;
    details?: unknown;
  };
  metadata: {
    execution_ref: string;
    processing_time_ms: number;
    agent_id: string;
    agent_version: string;
  };
}

/**
 * Express Request with typed body
 */
export interface TypedRequest<T> extends Request {
  body: T;
}

/**
 * Express Response with typed body
 */
export interface TypedResponse<T> extends Response {
  json(body: HandlerResponse<T>): this;
}

/**
 * Evaluation Context - Internal context for SLO evaluation
 */
export interface EvaluationContext {
  slo: SloDefinition;
  metric: TelemetryMetric;
  evaluation_time: Date;
  historical_context?: HistoricalContext;
}

/**
 * Historical Context - Previous evaluation data
 */
export interface HistoricalContext {
  previous_values: number[];
  previous_breaches: number;
  last_breach_at?: Date;
  average: number;
  p95: number;
  trend: 'improving' | 'stable' | 'degrading' | 'volatile';
}

/**
 * Evaluation Result - Result of a single SLO evaluation
 */
export interface EvaluationResult {
  slo: SloDefinition;
  metric: TelemetryMetric;
  is_violated: boolean;
  is_near_breach: boolean;
  violation?: SloViolation;
  status: SloStatus;
  confidence: number;
}

/**
 * Confidence Factors - Used to calculate overall confidence
 */
export interface ConfidenceFactors {
  sample_size_factor: number;      // Higher samples = higher confidence
  data_freshness_factor: number;   // More recent data = higher confidence
  consistency_factor: number;      // Less volatility = higher confidence
  coverage_factor: number;         // More SLOs evaluated = higher confidence
}

/**
 * RuVector Health Status
 */
export interface RuvectorHealthStatus {
  status: 'healthy' | 'degraded' | 'unhealthy';
  latency_ms: number;
  last_successful_write?: string;
  error_count_1h: number;
}

/**
 * Agent Metrics - Self-observation metrics
 */
export interface AgentMetrics {
  evaluations_total: number;
  violations_detected: number;
  errors_total: number;
  avg_latency_ms: number;
  uptime_seconds: number;
  last_evaluation_at?: string;
  slos_evaluated: number;
  sla_breaches_detected: number;
}

/**
 * Agent Health - Overall agent health status
 */
export interface AgentHealth {
  status: 'healthy' | 'degraded' | 'unhealthy';
  uptime_seconds: number;
  ruvector_status: RuvectorHealthStatus;
  metrics: AgentMetrics;
  last_error?: string;
}

/**
 * Error Codes
 */
export enum ErrorCode {
  VALIDATION_ERROR = 'VALIDATION_ERROR',
  CONSTITUTIONAL_VIOLATION = 'CONSTITUTIONAL_VIOLATION',
  RUVECTOR_ERROR = 'RUVECTOR_ERROR',
  INTERNAL_ERROR = 'INTERNAL_ERROR',
  EVALUATION_ERROR = 'EVALUATION_ERROR',
}

/**
 * Custom Error Classes
 */
export class ValidationError extends Error {
  readonly code = ErrorCode.VALIDATION_ERROR;
  readonly errors: unknown;

  constructor(errors: unknown) {
    super('Input validation failed');
    this.name = 'ValidationError';
    this.errors = errors;
  }
}

export class ConstitutionalViolationError extends Error {
  readonly code = ErrorCode.CONSTITUTIONAL_VIOLATION;
  readonly constraint: string;

  constructor(constraint: string) {
    super(`Constitutional constraint violated: ${constraint}`);
    this.name = 'ConstitutionalViolationError';
    this.constraint = constraint;
  }
}

export class RuvectorError extends Error {
  readonly code = ErrorCode.RUVECTOR_ERROR;

  constructor(message: string) {
    super(`RuVector service error: ${message}`);
    this.name = 'RuvectorError';
  }
}

export class EvaluationError extends Error {
  readonly code = ErrorCode.EVALUATION_ERROR;
  readonly slo_id: string;

  constructor(slo_id: string, message: string) {
    super(`Evaluation error for SLO ${slo_id}: ${message}`);
    this.name = 'EvaluationError';
    this.slo_id = slo_id;
  }
}
