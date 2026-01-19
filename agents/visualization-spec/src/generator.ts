/**
 * Visualization Spec Agent - Core Spec Generator
 *
 * Generates declarative visualization specifications.
 * This is the core business logic of the agent.
 *
 * Classification: READ-ONLY, PRESENTATIONAL
 * - Does NOT render UI
 * - Does NOT query databases
 * - Does NOT modify state
 */

import { randomUUID } from 'crypto';
import { getConfig, getColorPalette, getSeriesColor } from './config.js';
import { computeInputHash } from '../contracts/validation.js';
import {
  AGENT_VERSION,
  type VisualizationRequest,
  type VisualizationSpec,
  type VisualizationType,
  type MetricSpec,
  type DimensionSpec,
  type StylingSpec,
  type DataSourceSpec,
  type TimeRangeSpec,
  type ThresholdLine,
  type AlertZone,
} from '../contracts/schemas.js';
import type {
  SeriesConfig,
  AxisConfig,
  GenerationContext,
  ColorPalette,
} from '../contracts/types.js';

// =============================================================================
// Main Generator
// =============================================================================

/**
 * Generates a visualization specification from a request
 *
 * This is the primary entry point for spec generation.
 * The output is a declarative JSON specification that can be
 * consumed by dashboard renderers.
 */
export function generateVisualizationSpec(
  request: VisualizationRequest,
  context: GenerationContext
): VisualizationSpec {
  const config = getConfig();

  // Resolve styling with defaults
  const resolvedStyling = resolveStyling(request.styling);
  const palette = getColorPalette(resolvedStyling.theme);

  // Generate series configurations
  const series = generateSeries(request.metrics, palette);

  // Generate axes based on visualization type
  const axes = generateAxes(
    request.visualization_type,
    request.metrics,
    request.time_range,
    request.styling
  );

  // Build the complete specification
  const spec: VisualizationSpec = {
    spec_id: randomUUID(),
    spec_version: '1.0',
    visualization_type: request.visualization_type,
    data_source: request.data_source,
    time_range: request.time_range,
    series,
    dimensions: request.dimensions,
    axes,
    thresholds: request.thresholds,
    alert_zones: request.alert_zones,
    styling: {
      theme: resolvedStyling.theme,
      color_scheme: resolvedStyling.color_scheme,
      legend: resolvedStyling.legend ?? {
        position: 'bottom',
        show_values: false,
        interactive: true,
      },
      tooltip: resolvedStyling.tooltip ?? {
        enabled: true,
        show_all_series: true,
      },
      title: resolvedStyling.title,
      subtitle: resolvedStyling.subtitle,
      dimensions: {
        height: resolvedStyling.height,
        width: resolvedStyling.width,
        responsive: resolvedStyling.responsive ?? true,
      },
      animations: resolvedStyling.animations ?? true,
    },
    metadata: {
      generated_at: new Date().toISOString(),
      generator_version: AGENT_VERSION,
      input_hash: context.inputHash,
      deterministic: true,
    },
  };

  return spec;
}

/**
 * Generates multiple visualization specifications (batch)
 */
export function generateBatchVisualizationSpecs(
  requests: VisualizationRequest[],
  sharedStyling?: StylingSpec,
  context?: { executionRef: string }
): VisualizationSpec[] {
  return requests.map((request, index) => {
    // Merge shared styling with request-specific styling
    const mergedRequest = sharedStyling
      ? { ...request, styling: { ...sharedStyling, ...request.styling } }
      : request;

    const ctx: GenerationContext = {
      requestId: request.request_id ?? randomUUID(),
      executionRef: context?.executionRef ?? randomUUID(),
      startTime: Date.now(),
      inputHash: computeInputHash(mergedRequest),
    };

    return generateVisualizationSpec(mergedRequest, ctx);
  });
}

// =============================================================================
// Series Generation
// =============================================================================

/**
 * Generates series configurations from metric specifications
 */
function generateSeries(metrics: MetricSpec[], palette: ColorPalette): SeriesConfig[] {
  return metrics.map((metric, index) => ({
    id: `series-${index}`,
    name: metric.label ?? metric.field,
    field: metric.field,
    aggregation: metric.aggregation ?? 'avg',
    color: metric.color ?? getSeriesColor(palette, index),
    format: metric.format,
    unit: metric.unit,
  }));
}

// =============================================================================
// Axes Generation
// =============================================================================

/**
 * Generates axis configurations based on visualization type
 */
function generateAxes(
  vizType: VisualizationType,
  metrics: MetricSpec[],
  timeRange?: TimeRangeSpec,
  styling?: StylingSpec
): AxisConfig[] {
  const axes: AxisConfig[] = [];

  // X-axis configuration
  const xAxis = generateXAxis(vizType, timeRange, styling);
  if (xAxis) {
    axes.push(xAxis);
  }

  // Y-axis configuration
  const yAxis = generateYAxis(vizType, metrics, styling);
  if (yAxis) {
    axes.push(yAxis);
  }

  return axes;
}

/**
 * Generates X-axis configuration
 */
function generateXAxis(
  vizType: VisualizationType,
  timeRange?: TimeRangeSpec,
  styling?: StylingSpec
): AxisConfig | null {
  // Types that don't have a traditional X-axis
  const noXAxisTypes: VisualizationType[] = ['pie_chart', 'donut_chart', 'gauge', 'metric_card', 'treemap'];
  if (noXAxisTypes.includes(vizType)) {
    return null;
  }

  // Time-series types use time scale
  const timeSeriesTypes: VisualizationType[] = ['line_chart', 'area_chart', 'candlestick', 'sparkline', 'heatmap'];
  const isTimeSeries = timeSeriesTypes.includes(vizType);

  return {
    type: 'x',
    label: styling?.x_axis?.label,
    scale: isTimeSeries ? 'time' : (styling?.x_axis?.scale ?? 'category'),
    domain: timeRange ? [timeRange.start, timeRange.end] : undefined,
    format: styling?.x_axis?.format,
    gridLines: styling?.x_axis?.grid_lines ?? true,
  };
}

/**
 * Generates Y-axis configuration
 */
function generateYAxis(
  vizType: VisualizationType,
  metrics: MetricSpec[],
  styling?: StylingSpec
): AxisConfig | null {
  // Types that don't have a traditional Y-axis
  const noYAxisTypes: VisualizationType[] = ['pie_chart', 'donut_chart', 'table', 'metric_card', 'treemap', 'sankey'];
  if (noYAxisTypes.includes(vizType)) {
    return null;
  }

  // Determine scale based on metrics
  const scale = styling?.y_axis?.scale ?? 'linear';

  // Determine domain if min/max provided
  let domain: [number, number] | undefined;
  if (styling?.y_axis?.min !== undefined || styling?.y_axis?.max !== undefined) {
    domain = [
      styling?.y_axis?.min ?? 0,
      styling?.y_axis?.max ?? 100,
    ];
  }

  return {
    type: 'y',
    label: styling?.y_axis?.label ?? (metrics.length === 1 ? metrics[0].label : undefined),
    scale,
    domain,
    format: styling?.y_axis?.format ?? metrics[0]?.format,
    gridLines: styling?.y_axis?.grid_lines ?? true,
  };
}

// =============================================================================
// Styling Resolution
// =============================================================================

interface ResolvedStyling {
  theme: 'light' | 'dark' | 'system' | 'observatory' | 'minimal';
  color_scheme: 'default' | 'categorical' | 'sequential' | 'diverging' | 'status' | 'heatmap';
  legend?: {
    position: 'top' | 'bottom' | 'left' | 'right' | 'none';
    show_values: boolean;
    interactive: boolean;
  };
  tooltip?: {
    enabled: boolean;
    format?: string;
    show_all_series: boolean;
  };
  title?: string;
  subtitle?: string;
  height?: number;
  width?: number;
  responsive?: boolean;
  animations?: boolean;
  x_axis?: {
    label?: string;
    scale?: 'linear' | 'logarithmic' | 'time' | 'category';
    format?: string;
    grid_lines?: boolean;
  };
  y_axis?: {
    label?: string;
    scale?: 'linear' | 'logarithmic' | 'time' | 'category';
    min?: number;
    max?: number;
    format?: string;
    grid_lines?: boolean;
  };
}

/**
 * Resolves styling with defaults from configuration
 */
function resolveStyling(styling?: StylingSpec): ResolvedStyling {
  const config = getConfig();
  const defaults = config.generator;

  return {
    theme: styling?.theme ?? defaults.defaultTheme,
    color_scheme: styling?.color_scheme ?? defaults.defaultColorScheme,
    legend: styling?.legend,
    tooltip: styling?.tooltip,
    title: styling?.title,
    subtitle: styling?.subtitle,
    height: styling?.height,
    width: styling?.width,
    responsive: styling?.responsive ?? defaults.defaultResponsive,
    animations: styling?.animations ?? defaults.defaultAnimations,
    x_axis: styling?.x_axis,
    y_axis: styling?.y_axis,
  };
}

// =============================================================================
// Visualization Type Helpers
// =============================================================================

/**
 * Determines recommended color scheme for a visualization type
 */
export function getRecommendedColorScheme(vizType: VisualizationType): string {
  const schemes: Record<VisualizationType, string> = {
    line_chart: 'categorical',
    area_chart: 'sequential',
    bar_chart: 'categorical',
    stacked_bar_chart: 'sequential',
    pie_chart: 'categorical',
    donut_chart: 'categorical',
    scatter_plot: 'categorical',
    heatmap: 'heatmap',
    table: 'default',
    metric_card: 'status',
    gauge: 'status',
    histogram: 'sequential',
    box_plot: 'categorical',
    candlestick: 'status',
    treemap: 'sequential',
    sankey: 'categorical',
    funnel: 'sequential',
    radar: 'categorical',
    sparkline: 'default',
  };

  return schemes[vizType] ?? 'default';
}

/**
 * Determines recommended aggregation for a metric
 */
export function getRecommendedAggregation(
  vizType: VisualizationType,
  metricIndex: number
): string {
  // For candlestick, we need specific aggregations
  if (vizType === 'candlestick') {
    const candlestickAggs = ['avg', 'max', 'min', 'avg']; // open, high, low, close
    return candlestickAggs[metricIndex] ?? 'avg';
  }

  // For box plots, use statistical aggregations
  if (vizType === 'box_plot') {
    return 'p50';
  }

  // For gauges/metrics, use latest or avg
  if (vizType === 'gauge' || vizType === 'metric_card') {
    return 'avg';
  }

  // Default to avg
  return 'avg';
}

/**
 * Checks if visualization type supports stacking
 */
export function supportsStacking(vizType: VisualizationType): boolean {
  const stackableTypes: VisualizationType[] = [
    'area_chart',
    'stacked_bar_chart',
  ];
  return stackableTypes.includes(vizType);
}

/**
 * Checks if visualization type supports multiple Y-axes
 */
export function supportsMultipleYAxes(vizType: VisualizationType): boolean {
  const multiAxisTypes: VisualizationType[] = [
    'line_chart',
    'area_chart',
    'bar_chart',
  ];
  return multiAxisTypes.includes(vizType);
}

// =============================================================================
// Export for Testing
// =============================================================================

export const __testing = {
  generateSeries,
  generateAxes,
  generateXAxis,
  generateYAxis,
  resolveStyling,
};
