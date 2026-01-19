/**
 * Visualization Spec Agent - Zod Validation Schemas
 *
 * Classification: READ-ONLY, PRESENTATIONAL
 * Decision Type: visualization_specification
 *
 * Generates declarative visualization specifications for dashboards
 * and analytics interfaces without rendering UI or querying raw databases.
 */

import { z } from 'zod';

// =============================================================================
// Agent Constants
// =============================================================================

export const AGENT_ID = 'visualization-spec-agent' as const;
export const AGENT_VERSION = '1.0.0' as const;
export const AGENT_CLASSIFICATION = 'READ-ONLY' as const;
export const DECISION_TYPE = 'visualization_specification' as const;

// =============================================================================
// Enum Schemas
// =============================================================================

export const VisualizationTypeSchema = z.enum([
  'line_chart',
  'area_chart',
  'bar_chart',
  'stacked_bar_chart',
  'pie_chart',
  'donut_chart',
  'scatter_plot',
  'heatmap',
  'table',
  'metric_card',
  'gauge',
  'histogram',
  'box_plot',
  'candlestick',
  'treemap',
  'sankey',
  'funnel',
  'radar',
  'sparkline',
]);

export const DataSourceTypeSchema = z.enum([
  'telemetry_aggregates',
  'metric_series',
  'event_stream',
  'decision_events',
  'health_evaluations',
  'failure_classifications',
  'custom_query_result',
]);

export const AggregationTypeSchema = z.enum([
  'sum',
  'avg',
  'min',
  'max',
  'count',
  'p50',
  'p75',
  'p90',
  'p95',
  'p99',
  'rate',
  'delta',
]);

export const TimeGranularitySchema = z.enum([
  '1m',
  '5m',
  '15m',
  '30m',
  '1h',
  '6h',
  '12h',
  '1d',
  '7d',
  '30d',
]);

export const ChartThemeSchema = z.enum([
  'light',
  'dark',
  'system',
  'observatory',
  'minimal',
]);

export const LegendPositionSchema = z.enum([
  'top',
  'bottom',
  'left',
  'right',
  'none',
]);

export const AxisScaleSchema = z.enum([
  'linear',
  'logarithmic',
  'time',
  'category',
]);

export const ColorSchemeSchema = z.enum([
  'default',
  'categorical',
  'sequential',
  'diverging',
  'status',  // green/yellow/red for health
  'heatmap',
]);

// =============================================================================
// Data Source Schemas
// =============================================================================

export const DataSourceSpecSchema = z.object({
  type: DataSourceTypeSchema,
  source_id: z.string().min(1).max(256),
  filters: z.record(z.string(), z.union([z.string(), z.number(), z.boolean(), z.array(z.string())])).optional(),
  time_field: z.string().optional().default('timestamp'),
  value_fields: z.array(z.string()).min(1).max(20).optional(),
}).strict();

export const TimeRangeSpecSchema = z.object({
  start: z.string().datetime(),
  end: z.string().datetime(),
  granularity: TimeGranularitySchema.optional(),
  timezone: z.string().optional().default('UTC'),
}).strict().refine(
  (data) => new Date(data.start) < new Date(data.end),
  { message: 'Start time must be before end time' }
);

// =============================================================================
// Metric & Dimension Schemas
// =============================================================================

export const MetricSpecSchema = z.object({
  field: z.string().min(1).max(128),
  label: z.string().min(1).max(64).optional(),
  aggregation: AggregationTypeSchema.optional().default('avg'),
  format: z.string().optional(), // e.g., "0.00%", "0,0.00", "duration"
  color: z.string().regex(/^#[0-9A-Fa-f]{6}$/).optional(),
  unit: z.string().max(16).optional(),
}).strict();

export const DimensionSpecSchema = z.object({
  field: z.string().min(1).max(128),
  label: z.string().min(1).max(64).optional(),
  sort_order: z.enum(['asc', 'desc', 'none']).optional().default('none'),
  limit: z.number().int().min(1).max(100).optional(),
}).strict();

// =============================================================================
// Styling Schemas
// =============================================================================

export const AxisSpecSchema = z.object({
  label: z.string().max(64).optional(),
  scale: AxisScaleSchema.optional().default('linear'),
  min: z.number().optional(),
  max: z.number().optional(),
  format: z.string().optional(),
  grid_lines: z.boolean().optional().default(true),
}).strict();

export const LegendSpecSchema = z.object({
  position: LegendPositionSchema.optional().default('bottom'),
  show_values: z.boolean().optional().default(false),
  interactive: z.boolean().optional().default(true),
}).strict();

export const TooltipSpecSchema = z.object({
  enabled: z.boolean().optional().default(true),
  format: z.string().optional(),
  show_all_series: z.boolean().optional().default(true),
}).strict();

export const StylingSpecSchema = z.object({
  theme: ChartThemeSchema.optional().default('observatory'),
  color_scheme: ColorSchemeSchema.optional().default('default'),
  x_axis: AxisSpecSchema.optional(),
  y_axis: AxisSpecSchema.optional(),
  legend: LegendSpecSchema.optional(),
  tooltip: TooltipSpecSchema.optional(),
  title: z.string().max(128).optional(),
  subtitle: z.string().max(256).optional(),
  height: z.number().int().min(100).max(2000).optional(),
  width: z.number().int().min(200).max(4000).optional(),
  responsive: z.boolean().optional().default(true),
  animations: z.boolean().optional().default(true),
}).strict();

// =============================================================================
// Threshold & Alert Zone Schemas
// =============================================================================

export const ThresholdLineSchema = z.object({
  value: z.number(),
  label: z.string().max(32).optional(),
  color: z.string().regex(/^#[0-9A-Fa-f]{6}$/).optional(),
  style: z.enum(['solid', 'dashed', 'dotted']).optional().default('dashed'),
}).strict();

export const AlertZoneSchema = z.object({
  min: z.number().optional(),
  max: z.number().optional(),
  color: z.string().regex(/^#[0-9A-Fa-f]{6}$/).optional(),
  label: z.string().max(32).optional(),
}).strict();

// =============================================================================
// Request Schemas
// =============================================================================

export const VisualizationRequestSchema = z.object({
  data_source: DataSourceSpecSchema,
  visualization_type: VisualizationTypeSchema,
  metrics: z.array(MetricSpecSchema).min(1).max(10),
  dimensions: z.array(DimensionSpecSchema).max(5).optional(),
  time_range: TimeRangeSpecSchema.optional(),
  styling: StylingSpecSchema.optional(),
  thresholds: z.array(ThresholdLineSchema).max(5).optional(),
  alert_zones: z.array(AlertZoneSchema).max(3).optional(),
  request_id: z.string().uuid().optional(),
}).strict();

export const BatchVisualizationRequestSchema = z.object({
  requests: z.array(VisualizationRequestSchema).min(1).max(20),
  shared_styling: StylingSpecSchema.optional(),
  request_id: z.string().uuid().optional(),
}).strict();

// =============================================================================
// Output Visualization Spec Schemas
// =============================================================================

export const SeriesSpecSchema = z.object({
  id: z.string(),
  name: z.string(),
  field: z.string(),
  aggregation: AggregationTypeSchema,
  color: z.string().regex(/^#[0-9A-Fa-f]{6}$/),
  format: z.string().optional(),
  unit: z.string().optional(),
}).strict();

export const AxisOutputSpecSchema = z.object({
  type: z.enum(['x', 'y', 'y2']),
  label: z.string().optional(),
  scale: AxisScaleSchema,
  domain: z.tuple([z.number().or(z.string()), z.number().or(z.string())]).optional(),
  format: z.string().optional(),
  grid_lines: z.boolean(),
}).strict();

export const VisualizationSpecSchema = z.object({
  spec_id: z.string().uuid(),
  spec_version: z.literal('1.0'),
  visualization_type: VisualizationTypeSchema,
  data_source: DataSourceSpecSchema,
  time_range: TimeRangeSpecSchema.optional(),
  series: z.array(SeriesSpecSchema).min(1),
  dimensions: z.array(DimensionSpecSchema).optional(),
  axes: z.array(AxisOutputSpecSchema).max(3),
  thresholds: z.array(ThresholdLineSchema).optional(),
  alert_zones: z.array(AlertZoneSchema).optional(),
  styling: z.object({
    theme: ChartThemeSchema,
    color_scheme: ColorSchemeSchema,
    legend: LegendSpecSchema,
    tooltip: TooltipSpecSchema,
    title: z.string().optional(),
    subtitle: z.string().optional(),
    dimensions: z.object({
      height: z.number().optional(),
      width: z.number().optional(),
      responsive: z.boolean(),
    }),
    animations: z.boolean(),
  }),
  metadata: z.object({
    generated_at: z.string().datetime(),
    generator_version: z.string(),
    input_hash: z.string().length(64),
    deterministic: z.boolean(),
  }),
}).strict();

// =============================================================================
// Response Schemas
// =============================================================================

export const VisualizationResponseSchema = z.object({
  success: z.literal(true),
  spec: VisualizationSpecSchema,
  processing_time_ms: z.number().nonnegative(),
  request_id: z.string().uuid(),
}).strict();

export const BatchVisualizationResponseSchema = z.object({
  success: z.literal(true),
  specs: z.array(VisualizationSpecSchema),
  processing_time_ms: z.number().nonnegative(),
  request_id: z.string().uuid(),
}).strict();

export const ErrorResponseSchema = z.object({
  success: z.literal(false),
  error: z.object({
    code: z.string(),
    message: z.string(),
    details: z.record(z.string(), z.unknown()).optional(),
  }),
  request_id: z.string().uuid().optional(),
}).strict();

// =============================================================================
// Decision Event Schema
// =============================================================================

export const ProcessingMetricsSchema = z.object({
  parsing_ms: z.number().nonnegative(),
  validation_ms: z.number().nonnegative(),
  generation_ms: z.number().nonnegative(),
  total_ms: z.number().nonnegative(),
  specs_generated: z.number().int().nonnegative(),
}).strict();

export const VisualizationDecisionEventSchema = z.object({
  agent_id: z.literal(AGENT_ID),
  agent_version: z.string().regex(/^\d+\.\d+\.\d+$/),
  decision_type: z.literal(DECISION_TYPE),
  confidence: z.number().min(0).max(1),
  constraints_applied: z.array(z.never()).length(0), // Always empty for READ-ONLY
  classification: z.literal(AGENT_CLASSIFICATION),
  inputs_hash: z.string().length(64), // SHA256
  outputs: z.array(VisualizationSpecSchema).min(1),
  execution_ref: z.string().min(1),
  timestamp: z.string().datetime(),
  processing_metrics: ProcessingMetricsSchema,
}).strict();

// =============================================================================
// CLI Schemas
// =============================================================================

export const CLIGenerateCommandSchema = z.object({
  command: z.literal('generate'),
  type: VisualizationTypeSchema,
  data_source: z.string(), // JSON string
  metrics: z.string(), // JSON string
  time_range: z.string().optional(), // "start:end" format
  output: z.enum(['json', 'yaml', 'table']).optional().default('json'),
}).strict();

export const CLIInspectCommandSchema = z.object({
  command: z.literal('inspect'),
  spec_id: z.string().uuid(),
}).strict();

export const CLIReplayCommandSchema = z.object({
  command: z.literal('replay'),
  spec_id: z.string().uuid(),
  verify_determinism: z.boolean().optional().default(true),
}).strict();

export const CLIListTypesCommandSchema = z.object({
  command: z.literal('list-types'),
  category: z.enum(['time_series', 'categorical', 'comparative', 'hierarchical', 'all']).optional().default('all'),
}).strict();

export const CLIInvocationSchema = z.discriminatedUnion('command', [
  CLIGenerateCommandSchema,
  CLIInspectCommandSchema,
  CLIReplayCommandSchema,
  CLIListTypesCommandSchema,
]);

// =============================================================================
// Health & Status Schemas
// =============================================================================

export const HealthStatusSchema = z.object({
  status: z.enum(['healthy', 'degraded', 'unhealthy']),
  agent_id: z.literal(AGENT_ID),
  agent_version: z.literal(AGENT_VERSION),
  classification: z.literal(AGENT_CLASSIFICATION),
  checks: z.object({
    ruvector_connectivity: z.object({
      status: z.enum(['pass', 'fail']),
      latency_ms: z.number().optional(),
      error: z.string().optional(),
    }),
    schema_validation: z.object({
      status: z.enum(['pass', 'fail']),
    }),
  }),
  timestamp: z.string().datetime(),
}).strict();

// =============================================================================
// RuVector Persistence Schemas
// =============================================================================

export const RuvectorPersistRequestSchema = z.object({
  event_type: z.literal('decision_event'),
  agent_id: z.literal(AGENT_ID),
  payload: VisualizationDecisionEventSchema,
}).strict();

export const RuvectorPersistResponseSchema = z.object({
  success: z.boolean(),
  event_id: z.string().uuid().optional(),
  error: z.string().optional(),
}).strict();

export const RuvectorQueryRequestSchema = z.object({
  agent_id: z.literal(AGENT_ID),
  filters: z.object({
    spec_id: z.string().uuid().optional(),
    execution_ref: z.string().optional(),
    time_range: TimeRangeSpecSchema.optional(),
  }).optional(),
  limit: z.number().int().min(1).max(100).optional().default(10),
}).strict();

// =============================================================================
// Type Exports (inferred from schemas)
// =============================================================================

export type VisualizationType = z.infer<typeof VisualizationTypeSchema>;
export type DataSourceType = z.infer<typeof DataSourceTypeSchema>;
export type AggregationType = z.infer<typeof AggregationTypeSchema>;
export type TimeGranularity = z.infer<typeof TimeGranularitySchema>;
export type ChartTheme = z.infer<typeof ChartThemeSchema>;
export type LegendPosition = z.infer<typeof LegendPositionSchema>;
export type AxisScale = z.infer<typeof AxisScaleSchema>;
export type ColorScheme = z.infer<typeof ColorSchemeSchema>;

export type DataSourceSpec = z.infer<typeof DataSourceSpecSchema>;
export type TimeRangeSpec = z.infer<typeof TimeRangeSpecSchema>;
export type MetricSpec = z.infer<typeof MetricSpecSchema>;
export type DimensionSpec = z.infer<typeof DimensionSpecSchema>;
export type StylingSpec = z.infer<typeof StylingSpecSchema>;
export type ThresholdLine = z.infer<typeof ThresholdLineSchema>;
export type AlertZone = z.infer<typeof AlertZoneSchema>;

export type VisualizationRequest = z.infer<typeof VisualizationRequestSchema>;
export type BatchVisualizationRequest = z.infer<typeof BatchVisualizationRequestSchema>;
export type VisualizationSpec = z.infer<typeof VisualizationSpecSchema>;
export type VisualizationResponse = z.infer<typeof VisualizationResponseSchema>;
export type BatchVisualizationResponse = z.infer<typeof BatchVisualizationResponseSchema>;
export type ErrorResponse = z.infer<typeof ErrorResponseSchema>;

export type VisualizationDecisionEvent = z.infer<typeof VisualizationDecisionEventSchema>;
export type ProcessingMetrics = z.infer<typeof ProcessingMetricsSchema>;
export type HealthStatus = z.infer<typeof HealthStatusSchema>;

export type CLIInvocation = z.infer<typeof CLIInvocationSchema>;
