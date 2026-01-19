/**
 * Visualization Spec Agent - Main Exports
 *
 * Classification: READ-ONLY, PRESENTATIONAL
 * Decision Type: visualization_specification
 *
 * Generates declarative visualization specifications for dashboards
 * and analytics interfaces without rendering UI or querying databases.
 */

// =============================================================================
// Contract Exports
// =============================================================================

// Schemas
export {
  // Constants
  AGENT_ID,
  AGENT_VERSION,
  AGENT_CLASSIFICATION,
  DECISION_TYPE,

  // Enum Schemas
  VisualizationTypeSchema,
  DataSourceTypeSchema,
  AggregationTypeSchema,
  TimeGranularitySchema,
  ChartThemeSchema,
  LegendPositionSchema,
  AxisScaleSchema,
  ColorSchemeSchema,

  // Data Schemas
  DataSourceSpecSchema,
  TimeRangeSpecSchema,
  MetricSpecSchema,
  DimensionSpecSchema,
  StylingSpecSchema,
  ThresholdLineSchema,
  AlertZoneSchema,

  // Request/Response Schemas
  VisualizationRequestSchema,
  BatchVisualizationRequestSchema,
  VisualizationSpecSchema,
  VisualizationResponseSchema,
  BatchVisualizationResponseSchema,
  ErrorResponseSchema,

  // Decision Event Schemas
  VisualizationDecisionEventSchema,
  ProcessingMetricsSchema,

  // Health & CLI Schemas
  HealthStatusSchema,
  CLIInvocationSchema,

  // RuVector Schemas
  RuvectorPersistRequestSchema,
  RuvectorPersistResponseSchema,
  RuvectorQueryRequestSchema,
} from '../contracts/schemas.js';

// Types
export type {
  VisualizationType,
  DataSourceType,
  AggregationType,
  TimeGranularity,
  ChartTheme,
  LegendPosition,
  AxisScale,
  ColorScheme,
  DataSourceSpec,
  TimeRangeSpec,
  MetricSpec,
  DimensionSpec,
  StylingSpec,
  ThresholdLine,
  AlertZone,
  VisualizationRequest,
  BatchVisualizationRequest,
  VisualizationSpec,
  VisualizationResponse,
  BatchVisualizationResponse,
  ErrorResponse,
  VisualizationDecisionEvent,
  ProcessingMetrics,
  HealthStatus,
  CLIInvocation,
} from '../contracts/schemas.js';

// Additional Types
export type {
  GeneratorConfig,
  ColorPalette,
  AxisConfig,
  SeriesConfig,
  GenerationContext,
  RuvectorConfig,
  Span,
  ErrorCode,
  AgentError,
} from '../contracts/types.js';

// Type Constants
export {
  VISUALIZATION_CATEGORIES,
  DEFAULT_COLOR_PALETTES,
  FORMAT_PATTERNS,
} from '../contracts/types.js';

// =============================================================================
// Validation Exports
// =============================================================================

export {
  validateRequest,
  validateBatchRequest,
  safeParse,
  checkMetricCompatibility,
  checkTimeRangeValidity,
  validateVisualizationType,
  getVisualizationTypeInfo,
  listVisualizationTypes,
  formatZodError,
  createErrorResponse,
  createAgentError,
  computeInputHash,
  verifyInputHash,
} from '../contracts/validation.js';

// =============================================================================
// Configuration Exports
// =============================================================================

export {
  loadConfig,
  getConfig,
  resetConfig,
  validateConfig,
  getColorPalette,
  getSeriesColor,
} from './config.js';

export type {
  Config,
  AgentConfig,
  TelemetryConfig,
  ThresholdConfig,
} from './config.js';

// =============================================================================
// Generator Exports
// =============================================================================

export {
  generateVisualizationSpec,
  generateBatchVisualizationSpecs,
  getRecommendedColorScheme,
  getRecommendedAggregation,
  supportsStacking,
  supportsMultipleYAxes,
} from './generator.js';

// =============================================================================
// Decision Event Exports
// =============================================================================

export {
  createDecisionEvent,
  createProcessingMetrics,
  verifyReplayDeterminism,
} from './emitter.js';

export type {
  CreateDecisionEventInput,
} from './emitter.js';

// =============================================================================
// RuVector Client Exports
// =============================================================================

export {
  RuvectorClient,
  getRuvectorClient,
  resetRuvectorClient,
} from './ruvector-client.js';

export type {
  PersistResult,
  QueryResult,
  HealthCheckResult,
} from './ruvector-client.js';

// =============================================================================
// Telemetry Exports
// =============================================================================

export {
  startSpan,
  endSpan,
  incrementCounter,
  recordHistogram,
  setGauge,
  recordGenerationMetrics,
  recordPersistenceMetrics,
  getPrometheusMetrics,
  resetMetrics,
  log,
} from './telemetry.js';

// =============================================================================
// Handler Exports
// =============================================================================

export {
  handleVisualizationSpec,
  handleCLI,
} from './handler.js';
