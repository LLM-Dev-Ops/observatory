/**
 * Visualization Spec Agent - Validation Helpers
 *
 * Provides validation utilities for inputs, compatibility checks,
 * and error formatting.
 */

import { createHash } from 'crypto';
import { ZodError, ZodSchema } from 'zod';
import {
  VisualizationRequestSchema,
  BatchVisualizationRequestSchema,
  VisualizationTypeSchema,
  type VisualizationRequest,
  type VisualizationType,
  type MetricSpec,
  type ErrorResponse,
} from './schemas.js';
import { VISUALIZATION_CATEGORIES, type AgentError, type ErrorCode } from './types.js';

// =============================================================================
// Input Validation
// =============================================================================

/**
 * Validates a visualization request against the schema
 */
export function validateRequest(input: unknown): {
  success: true;
  data: VisualizationRequest;
} | {
  success: false;
  error: AgentError;
} {
  const result = VisualizationRequestSchema.safeParse(input);

  if (result.success) {
    return { success: true, data: result.data };
  }

  return {
    success: false,
    error: formatZodError(result.error),
  };
}

/**
 * Validates a batch visualization request
 */
export function validateBatchRequest(input: unknown): {
  success: true;
  data: { requests: VisualizationRequest[]; shared_styling?: unknown; request_id?: string };
} | {
  success: false;
  error: AgentError;
} {
  const result = BatchVisualizationRequestSchema.safeParse(input);

  if (result.success) {
    return { success: true, data: result.data };
  }

  return {
    success: false,
    error: formatZodError(result.error),
  };
}

/**
 * Safely parses input against a schema
 */
export function safeParse<T>(schema: ZodSchema<T>, input: unknown): {
  success: true;
  data: T;
} | {
  success: false;
  error: AgentError;
} {
  const result = schema.safeParse(input);

  if (result.success) {
    return { success: true, data: result.data };
  }

  return {
    success: false,
    error: formatZodError(result.error),
  };
}

// =============================================================================
// Compatibility Checks
// =============================================================================

/**
 * Checks if metrics are compatible with the visualization type
 */
export function checkMetricCompatibility(
  vizType: VisualizationType,
  metrics: MetricSpec[]
): { compatible: true } | { compatible: false; error: AgentError } {
  const typeInfo = getVisualizationTypeInfo(vizType);

  if (metrics.length < typeInfo.minMetrics) {
    return {
      compatible: false,
      error: {
        code: 'INCOMPATIBLE_METRICS',
        message: `Visualization type '${vizType}' requires at least ${typeInfo.minMetrics} metric(s), but ${metrics.length} provided`,
        details: { vizType, minMetrics: typeInfo.minMetrics, providedMetrics: metrics.length },
        recoverable: true,
      },
    };
  }

  if (metrics.length > typeInfo.maxMetrics) {
    return {
      compatible: false,
      error: {
        code: 'INCOMPATIBLE_METRICS',
        message: `Visualization type '${vizType}' supports at most ${typeInfo.maxMetrics} metric(s), but ${metrics.length} provided`,
        details: { vizType, maxMetrics: typeInfo.maxMetrics, providedMetrics: metrics.length },
        recoverable: true,
      },
    };
  }

  return { compatible: true };
}

/**
 * Checks if time range is valid for time-series visualizations
 */
export function checkTimeRangeValidity(
  vizType: VisualizationType,
  timeRange?: { start: string; end: string }
): { valid: true } | { valid: false; error: AgentError } {
  const typeInfo = getVisualizationTypeInfo(vizType);

  // Time-series types require time range
  if (typeInfo.supportsTimeSeries && typeInfo.category === 'time_series' && !timeRange) {
    return {
      valid: false,
      error: {
        code: 'INVALID_TIME_RANGE',
        message: `Visualization type '${vizType}' requires a time_range specification`,
        details: { vizType },
        recoverable: true,
      },
    };
  }

  if (timeRange) {
    const start = new Date(timeRange.start);
    const end = new Date(timeRange.end);
    const durationMs = end.getTime() - start.getTime();
    const maxDurationMs = 365 * 24 * 60 * 60 * 1000; // 1 year max

    if (durationMs > maxDurationMs) {
      return {
        valid: false,
        error: {
          code: 'INVALID_TIME_RANGE',
          message: 'Time range exceeds maximum allowed duration of 1 year',
          details: { durationMs, maxDurationMs },
          recoverable: true,
        },
      };
    }
  }

  return { valid: true };
}

/**
 * Validates visualization type is supported
 */
export function validateVisualizationType(type: string): {
  valid: true;
  type: VisualizationType;
} | {
  valid: false;
  error: AgentError;
} {
  const result = VisualizationTypeSchema.safeParse(type);

  if (result.success) {
    return { valid: true, type: result.data };
  }

  const supportedTypes = VisualizationTypeSchema.options;
  return {
    valid: false,
    error: {
      code: 'UNSUPPORTED_VISUALIZATION_TYPE',
      message: `Visualization type '${type}' is not supported`,
      details: { providedType: type, supportedTypes },
      recoverable: true,
    },
  };
}

// =============================================================================
// Type Information
// =============================================================================

interface VisualizationTypeInfo {
  type: VisualizationType;
  category: 'time_series' | 'categorical' | 'comparative' | 'hierarchical' | 'single_value' | 'tabular';
  supportsTimeSeries: boolean;
  supportsDimensions: boolean;
  minMetrics: number;
  maxMetrics: number;
  description: string;
}

const TYPE_INFO: Record<VisualizationType, VisualizationTypeInfo> = {
  line_chart: { type: 'line_chart', category: 'time_series', supportsTimeSeries: true, supportsDimensions: true, minMetrics: 1, maxMetrics: 10, description: 'Line chart for time-series data' },
  area_chart: { type: 'area_chart', category: 'time_series', supportsTimeSeries: true, supportsDimensions: true, minMetrics: 1, maxMetrics: 10, description: 'Area chart for cumulative time-series' },
  bar_chart: { type: 'bar_chart', category: 'categorical', supportsTimeSeries: false, supportsDimensions: true, minMetrics: 1, maxMetrics: 5, description: 'Bar chart for categorical comparison' },
  stacked_bar_chart: { type: 'stacked_bar_chart', category: 'categorical', supportsTimeSeries: false, supportsDimensions: true, minMetrics: 2, maxMetrics: 10, description: 'Stacked bar chart for part-to-whole' },
  pie_chart: { type: 'pie_chart', category: 'categorical', supportsTimeSeries: false, supportsDimensions: true, minMetrics: 1, maxMetrics: 1, description: 'Pie chart for proportional data' },
  donut_chart: { type: 'donut_chart', category: 'categorical', supportsTimeSeries: false, supportsDimensions: true, minMetrics: 1, maxMetrics: 1, description: 'Donut chart for proportional data' },
  scatter_plot: { type: 'scatter_plot', category: 'comparative', supportsTimeSeries: false, supportsDimensions: true, minMetrics: 2, maxMetrics: 4, description: 'Scatter plot for correlation analysis' },
  heatmap: { type: 'heatmap', category: 'comparative', supportsTimeSeries: true, supportsDimensions: true, minMetrics: 1, maxMetrics: 1, description: 'Heatmap for density visualization' },
  table: { type: 'table', category: 'tabular', supportsTimeSeries: false, supportsDimensions: true, minMetrics: 1, maxMetrics: 20, description: 'Tabular data display' },
  metric_card: { type: 'metric_card', category: 'single_value', supportsTimeSeries: false, supportsDimensions: false, minMetrics: 1, maxMetrics: 1, description: 'Single metric display card' },
  gauge: { type: 'gauge', category: 'single_value', supportsTimeSeries: false, supportsDimensions: false, minMetrics: 1, maxMetrics: 1, description: 'Gauge for threshold-based metrics' },
  histogram: { type: 'histogram', category: 'comparative', supportsTimeSeries: false, supportsDimensions: false, minMetrics: 1, maxMetrics: 1, description: 'Histogram for distribution analysis' },
  box_plot: { type: 'box_plot', category: 'comparative', supportsTimeSeries: false, supportsDimensions: true, minMetrics: 1, maxMetrics: 5, description: 'Box plot for statistical distribution' },
  candlestick: { type: 'candlestick', category: 'time_series', supportsTimeSeries: true, supportsDimensions: false, minMetrics: 4, maxMetrics: 4, description: 'Candlestick for OHLC data' },
  treemap: { type: 'treemap', category: 'hierarchical', supportsTimeSeries: false, supportsDimensions: true, minMetrics: 1, maxMetrics: 2, description: 'Treemap for hierarchical proportions' },
  sankey: { type: 'sankey', category: 'hierarchical', supportsTimeSeries: false, supportsDimensions: true, minMetrics: 1, maxMetrics: 1, description: 'Sankey diagram for flow visualization' },
  funnel: { type: 'funnel', category: 'categorical', supportsTimeSeries: false, supportsDimensions: true, minMetrics: 1, maxMetrics: 1, description: 'Funnel for conversion analysis' },
  radar: { type: 'radar', category: 'comparative', supportsTimeSeries: false, supportsDimensions: true, minMetrics: 3, maxMetrics: 10, description: 'Radar chart for multi-dimensional comparison' },
  sparkline: { type: 'sparkline', category: 'time_series', supportsTimeSeries: true, supportsDimensions: false, minMetrics: 1, maxMetrics: 1, description: 'Compact inline chart' },
};

/**
 * Gets metadata about a visualization type
 */
export function getVisualizationTypeInfo(type: VisualizationType): VisualizationTypeInfo {
  return TYPE_INFO[type];
}

/**
 * Lists visualization types by category
 */
export function listVisualizationTypes(category?: keyof typeof VISUALIZATION_CATEGORIES | 'all'): VisualizationTypeInfo[] {
  if (!category || category === 'all') {
    return Object.values(TYPE_INFO);
  }

  const typesInCategory = VISUALIZATION_CATEGORIES[category] || [];
  return typesInCategory.map(t => TYPE_INFO[t as VisualizationType]).filter(Boolean);
}

// =============================================================================
// Error Formatting
// =============================================================================

/**
 * Formats a Zod error into an AgentError
 */
export function formatZodError(error: ZodError): AgentError {
  const issues = error.issues.map(issue => ({
    path: issue.path.join('.'),
    message: issue.message,
    code: issue.code,
  }));

  return {
    code: 'SCHEMA_VALIDATION_FAILED',
    message: `Validation failed: ${issues.map(i => `${i.path}: ${i.message}`).join('; ')}`,
    details: { issues },
    recoverable: true,
  };
}

/**
 * Creates an error response object
 */
export function createErrorResponse(error: AgentError, requestId?: string): ErrorResponse {
  return {
    success: false,
    error: {
      code: error.code,
      message: error.message,
      details: error.details,
    },
    request_id: requestId,
  };
}

/**
 * Creates an AgentError from code and message
 */
export function createAgentError(
  code: ErrorCode,
  message: string,
  details?: Record<string, unknown>
): AgentError {
  const recoverableCodes: ErrorCode[] = [
    'INVALID_DATA_SOURCE',
    'UNSUPPORTED_VISUALIZATION_TYPE',
    'INCOMPATIBLE_METRICS',
    'INVALID_TIME_RANGE',
    'RUVECTOR_UNAVAILABLE',
    'SCHEMA_VALIDATION_FAILED',
  ];

  return {
    code,
    message,
    details,
    recoverable: recoverableCodes.includes(code),
  };
}

// =============================================================================
// Hashing
// =============================================================================

/**
 * Computes SHA256 hash of input for determinism verification
 */
export function computeInputHash(input: unknown): string {
  const normalized = JSON.stringify(input, Object.keys(input as object).sort());
  return createHash('sha256').update(normalized).digest('hex');
}

/**
 * Verifies that two inputs produce the same hash
 */
export function verifyInputHash(input: unknown, expectedHash: string): boolean {
  return computeInputHash(input) === expectedHash;
}
