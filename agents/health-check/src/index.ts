/**
 * Health Check Agent - Main Exports
 *
 * CLASSIFICATION: ADVISORY, NON-ACTUATING
 * DECISION_TYPE: health_evaluation
 *
 * This agent evaluates the health state of services and agents
 * based on telemetry-derived indicators.
 */

// ============================================================================
// CONTRACTS
// ============================================================================

export {
  // Constants
  AGENT_ID,
  AGENT_VERSION,
  AGENT_CLASSIFICATION,
  DECISION_TYPE,

  // Enums
  HealthStateSchema,
  HealthTrendSchema,
  IndicatorTypeSchema,
  TargetTypeSchema,
  EvaluationWindowSchema,
  TrendWindowSchema,

  // Schemas
  HealthIndicatorSchema,
  HealthTrendAnalysisSchema,
  HealthEvaluationSchema,
  HealthCheckDecisionEventSchema,
  TelemetryAggregatesInputSchema,
  HealthEvaluationRequestSchema,
  HealthEvaluationResponseSchema,
  ErrorResponseSchema,
  CLIInvocationSchema,

  // Types
  type HealthState,
  type HealthTrend,
  type IndicatorType,
  type TargetType,
  type EvaluationWindow,
  type TrendWindow,
  type HealthIndicator,
  type HealthTrendAnalysis,
  type HealthEvaluation,
  type HealthCheckDecisionEvent,
  type TelemetryAggregatesInput,
  type HealthEvaluationRequest,
  type HealthEvaluationResponse,
  type ErrorCode,
  type ErrorResponse,
  type CLIInvocation,
} from '../contracts/schemas.js';

// ============================================================================
// CONFIGURATION
// ============================================================================

export {
  loadConfig,
  resetConfig,
  getDefaultThresholds,
  getDefaultHysteresis,
  getDefaultIndicatorWeights,
  type Config,
  type ThresholdConfig,
  type HysteresisConfig,
  type IndicatorWeights,
  type RuvectorConfig,
  type AgentConfig,
  type TelemetryConfig,
} from './config.js';

// ============================================================================
// INDICATORS
// ============================================================================

export {
  evaluateIndicator,
  evaluateLatencyIndicator,
  evaluateErrorRateIndicator,
  evaluateThroughputIndicator,
  evaluateSaturationIndicator,
  evaluateAvailabilityIndicator,
  calculateIndicatorConfidence,
  buildHealthIndicator,
  extractIndicatorsFromTelemetry,
  isWorse,
  isBetter,
  getWorstState,
  type IndicatorEvaluationResult,
} from './indicators.js';

// ============================================================================
// HYSTERESIS
// ============================================================================

export {
  createInitialHysteresisState,
  applyHysteresis,
  updateHysteresisState,
  buildStateTransition,
  loadFromStateTransition,
  evaluateWithHysteresis,
  type HysteresisState,
  type HysteresisResult,
  type HysteresisEvaluationInput,
  type HysteresisEvaluationResult,
} from './hysteresis.js';

// ============================================================================
// CONFIDENCE
// ============================================================================

export {
  calculateSampleFactor,
  calculateCoverageFactor,
  calculateIndicatorFactor,
  calculateFreshnessFactor,
  calculateVarianceFactor,
  calculateConfidence,
  classifyConfidence,
  isSufficientConfidence,
  type ConfidenceFactors,
  type ConfidenceInput,
  type ConfidenceResult,
  type ConfidenceLevel,
} from './confidence.js';

// ============================================================================
// TRENDS
// ============================================================================

export {
  linearRegression,
  determineTrend,
  isHigherBetter,
  analyzeTrend,
  aggregateTrends,
  type TrendDataInput,
  type TrendAnalysisInput,
} from './trends.js';

// ============================================================================
// EVALUATOR
// ============================================================================

export {
  computeCompositeState,
  buildAggregateStatistics,
  parseWindowToSeconds,
  buildEvaluationWindowSpec,
  evaluateHealth,
  evaluateHealthBatch,
  summarizeEvaluations,
  type EvaluationContext,
  type BatchEvaluationInput,
  type EvaluationSummary,
} from './evaluator.js';

// ============================================================================
// RUVECTOR CLIENT
// ============================================================================

export {
  RuvectorClient,
  initializeClient,
  getClient,
  type PersistResult,
  type BatchPersistResult,
  type HealthStatus,
  type DecisionQuery,
} from './ruvector-client.js';

// ============================================================================
// EMITTER
// ============================================================================

export {
  generateExecutionRef,
  calculateInputsHash,
  calculateOverallConfidence,
  buildProcessingMetrics,
  createDecisionEvent,
  validateDecisionEventCompliance,
  type DecisionEventInput,
} from './emitter.js';

// ============================================================================
// TELEMETRY
// ============================================================================

export {
  startSpan,
  endSpan,
  recordEvaluationMetrics,
  recordPersistenceMetrics,
  getMetrics,
  getPrometheusMetrics,
  resetMetrics,
  log,
  debug,
  info,
  warn,
  error,
  type Span,
} from './telemetry.js';

// ============================================================================
// HANDLER
// ============================================================================

export {
  handleHealthEvaluation,
  handleEvaluate,
  handleHealthCheck,
  handleMetrics,
  createErrorResponse,
} from './handler.js';
