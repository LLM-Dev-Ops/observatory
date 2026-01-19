/**
 * Visualization Spec Agent - TypeScript Type Definitions
 *
 * Re-exports types from schemas and provides additional utility types.
 */

// Re-export all types from schemas
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
} from './schemas.js';

// Re-export constants
export {
  AGENT_ID,
  AGENT_VERSION,
  AGENT_CLASSIFICATION,
  DECISION_TYPE,
} from './schemas.js';

// =============================================================================
// Additional Utility Types
// =============================================================================

/**
 * Configuration for the visualization generator
 */
export interface GeneratorConfig {
  defaultTheme: 'light' | 'dark' | 'system' | 'observatory' | 'minimal';
  defaultColorScheme: 'default' | 'categorical' | 'sequential' | 'diverging' | 'status' | 'heatmap';
  maxSeriesPerChart: number;
  maxDimensions: number;
  defaultAnimations: boolean;
  defaultResponsive: boolean;
}

/**
 * Color palette for different themes
 */
export interface ColorPalette {
  primary: string[];
  secondary: string[];
  status: {
    healthy: string;
    degraded: string;
    unhealthy: string;
  };
  background: string;
  text: string;
  grid: string;
}

/**
 * Axis configuration derived during generation
 */
export interface AxisConfig {
  type: 'x' | 'y' | 'y2';
  label?: string;
  scale: 'linear' | 'logarithmic' | 'time' | 'category';
  domain?: [number | string, number | string];
  format?: string;
  gridLines: boolean;
}

/**
 * Series configuration derived during generation
 */
export interface SeriesConfig {
  id: string;
  name: string;
  field: string;
  aggregation: string;
  color: string;
  format?: string;
  unit?: string;
}

/**
 * Internal generation context
 */
export interface GenerationContext {
  requestId: string;
  executionRef: string;
  startTime: number;
  inputHash: string;
}

/**
 * Visualization type metadata
 */
export interface VisualizationTypeInfo {
  type: string;
  category: 'time_series' | 'categorical' | 'comparative' | 'hierarchical';
  supportsTimeSeries: boolean;
  supportsDimensions: boolean;
  minMetrics: number;
  maxMetrics: number;
  description: string;
}

/**
 * RuVector client configuration
 */
export interface RuvectorConfig {
  endpoint: string;
  apiKey: string;
  timeoutMs: number;
  maxRetries: number;
  retryBaseDelayMs: number;
  poolSize: number;
}

/**
 * Telemetry span for tracing
 */
export interface Span {
  executionRef: string;
  spanId: string;
  startTime: number;
  operationName: string;
  attributes: Record<string, unknown>;
}

/**
 * Agent error codes
 */
export type ErrorCode =
  | 'INVALID_DATA_SOURCE'
  | 'UNSUPPORTED_VISUALIZATION_TYPE'
  | 'INCOMPATIBLE_METRICS'
  | 'INVALID_TIME_RANGE'
  | 'RUVECTOR_UNAVAILABLE'
  | 'SCHEMA_VALIDATION_FAILED'
  | 'INTERNAL_ERROR';

/**
 * Structured agent error
 */
export interface AgentError {
  code: ErrorCode;
  message: string;
  details?: Record<string, unknown>;
  recoverable: boolean;
}

/**
 * Visualization type category enumeration
 */
export const VISUALIZATION_CATEGORIES = {
  time_series: ['line_chart', 'area_chart', 'sparkline', 'candlestick'],
  categorical: ['bar_chart', 'stacked_bar_chart', 'pie_chart', 'donut_chart', 'funnel'],
  comparative: ['scatter_plot', 'heatmap', 'box_plot', 'histogram', 'radar'],
  hierarchical: ['treemap', 'sankey'],
  single_value: ['metric_card', 'gauge'],
  tabular: ['table'],
} as const;

/**
 * Default color palettes
 */
export const DEFAULT_COLOR_PALETTES: Record<string, ColorPalette> = {
  observatory: {
    primary: ['#3B82F6', '#10B981', '#F59E0B', '#EF4444', '#8B5CF6', '#EC4899', '#06B6D4', '#84CC16'],
    secondary: ['#93C5FD', '#6EE7B7', '#FCD34D', '#FCA5A5', '#C4B5FD', '#F9A8D4', '#67E8F9', '#BEF264'],
    status: {
      healthy: '#10B981',
      degraded: '#F59E0B',
      unhealthy: '#EF4444',
    },
    background: '#0F172A',
    text: '#F8FAFC',
    grid: '#334155',
  },
  light: {
    primary: ['#2563EB', '#059669', '#D97706', '#DC2626', '#7C3AED', '#DB2777', '#0891B2', '#65A30D'],
    secondary: ['#BFDBFE', '#A7F3D0', '#FDE68A', '#FECACA', '#DDD6FE', '#FBCFE8', '#A5F3FC', '#D9F99D'],
    status: {
      healthy: '#059669',
      degraded: '#D97706',
      unhealthy: '#DC2626',
    },
    background: '#FFFFFF',
    text: '#1E293B',
    grid: '#E2E8F0',
  },
  dark: {
    primary: ['#60A5FA', '#34D399', '#FBBF24', '#F87171', '#A78BFA', '#F472B6', '#22D3EE', '#A3E635'],
    secondary: ['#1E40AF', '#065F46', '#92400E', '#991B1B', '#5B21B6', '#9D174D', '#155E75', '#3F6212'],
    status: {
      healthy: '#34D399',
      degraded: '#FBBF24',
      unhealthy: '#F87171',
    },
    background: '#1E293B',
    text: '#F1F5F9',
    grid: '#475569',
  },
};

/**
 * Format patterns for different data types
 */
export const FORMAT_PATTERNS = {
  percentage: '0.00%',
  decimal: '0.00',
  integer: '0,0',
  currency: '$0,0.00',
  duration_ms: '0,0ms',
  duration_s: '0,0.00s',
  bytes: '0.00b',
  scientific: '0.00e+0',
} as const;
