/**
 * Visualization Spec Agent - Generator Tests
 *
 * Tests for the core visualization spec generation logic
 */

import { describe, it, expect, beforeEach } from 'vitest';
import {
  generateVisualizationSpec,
  generateBatchVisualizationSpecs,
  getRecommendedColorScheme,
  getRecommendedAggregation,
  supportsStacking,
  supportsMultipleYAxes,
} from '../src/generator.js';
import { resetConfig } from '../src/config.js';
import { computeInputHash } from '../contracts/validation.js';
import type { VisualizationRequest } from '../contracts/schemas.js';
import type { GenerationContext } from '../contracts/types.js';

describe('generateVisualizationSpec', () => {
  beforeEach(() => {
    resetConfig();
  });

  const createContext = (request: VisualizationRequest): GenerationContext => ({
    requestId: 'test-request-id',
    executionRef: 'test-execution-ref',
    startTime: Date.now(),
    inputHash: computeInputHash(request),
  });

  it('generates a basic line chart spec', () => {
    const request: VisualizationRequest = {
      data_source: {
        type: 'telemetry_aggregates',
        source_id: 'api-service',
      },
      visualization_type: 'line_chart',
      metrics: [{ field: 'latency_p95' }],
      time_range: {
        start: '2024-01-01T00:00:00Z',
        end: '2024-01-02T00:00:00Z',
      },
    };

    const spec = generateVisualizationSpec(request, createContext(request));

    expect(spec.visualization_type).toBe('line_chart');
    expect(spec.spec_version).toBe('1.0');
    expect(spec.series).toHaveLength(1);
    expect(spec.series[0].field).toBe('latency_p95');
    expect(spec.axes).toHaveLength(2); // x and y
    expect(spec.metadata.deterministic).toBe(true);
  });

  it('generates spec with multiple metrics', () => {
    const request: VisualizationRequest = {
      data_source: {
        type: 'metric_series',
        source_id: 'service',
      },
      visualization_type: 'line_chart',
      metrics: [
        { field: 'latency_p50', label: 'P50', color: '#00FF00' },
        { field: 'latency_p95', label: 'P95', color: '#FFFF00' },
        { field: 'latency_p99', label: 'P99', color: '#FF0000' },
      ],
      time_range: {
        start: '2024-01-01T00:00:00Z',
        end: '2024-01-02T00:00:00Z',
      },
    };

    const spec = generateVisualizationSpec(request, createContext(request));

    expect(spec.series).toHaveLength(3);
    expect(spec.series[0].color).toBe('#00FF00');
    expect(spec.series[1].color).toBe('#FFFF00');
    expect(spec.series[2].color).toBe('#FF0000');
  });

  it('generates bar chart without time axis', () => {
    const request: VisualizationRequest = {
      data_source: {
        type: 'telemetry_aggregates',
        source_id: 'service',
      },
      visualization_type: 'bar_chart',
      metrics: [{ field: 'count', aggregation: 'sum' }],
      dimensions: [{ field: 'endpoint' }],
    };

    const spec = generateVisualizationSpec(request, createContext(request));

    expect(spec.visualization_type).toBe('bar_chart');
    expect(spec.axes.find(a => a.type === 'x')?.scale).toBe('category');
    expect(spec.dimensions).toHaveLength(1);
  });

  it('generates pie chart without axes', () => {
    const request: VisualizationRequest = {
      data_source: {
        type: 'telemetry_aggregates',
        source_id: 'service',
      },
      visualization_type: 'pie_chart',
      metrics: [{ field: 'count' }],
      dimensions: [{ field: 'status' }],
    };

    const spec = generateVisualizationSpec(request, createContext(request));

    expect(spec.visualization_type).toBe('pie_chart');
    // Pie charts don't have traditional axes
    expect(spec.axes.filter(a => a.type === 'x')).toHaveLength(0);
  });

  it('applies custom styling', () => {
    const request: VisualizationRequest = {
      data_source: {
        type: 'telemetry_aggregates',
        source_id: 'service',
      },
      visualization_type: 'line_chart',
      metrics: [{ field: 'latency' }],
      time_range: {
        start: '2024-01-01T00:00:00Z',
        end: '2024-01-02T00:00:00Z',
      },
      styling: {
        theme: 'dark',
        title: 'API Latency Over Time',
        subtitle: 'Last 24 hours',
        height: 400,
        width: 800,
        animations: false,
      },
    };

    const spec = generateVisualizationSpec(request, createContext(request));

    expect(spec.styling.theme).toBe('dark');
    expect(spec.styling.title).toBe('API Latency Over Time');
    expect(spec.styling.subtitle).toBe('Last 24 hours');
    expect(spec.styling.dimensions.height).toBe(400);
    expect(spec.styling.dimensions.width).toBe(800);
    expect(spec.styling.animations).toBe(false);
  });

  it('includes thresholds when provided', () => {
    const request: VisualizationRequest = {
      data_source: {
        type: 'telemetry_aggregates',
        source_id: 'service',
      },
      visualization_type: 'line_chart',
      metrics: [{ field: 'latency' }],
      time_range: {
        start: '2024-01-01T00:00:00Z',
        end: '2024-01-02T00:00:00Z',
      },
      thresholds: [
        { value: 100, label: 'Target', color: '#00FF00', style: 'dashed' },
        { value: 500, label: 'Warning', color: '#FFFF00' },
      ],
    };

    const spec = generateVisualizationSpec(request, createContext(request));

    expect(spec.thresholds).toHaveLength(2);
    expect(spec.thresholds?.[0].value).toBe(100);
    expect(spec.thresholds?.[0].label).toBe('Target');
  });

  it('includes alert zones when provided', () => {
    const request: VisualizationRequest = {
      data_source: {
        type: 'telemetry_aggregates',
        source_id: 'service',
      },
      visualization_type: 'gauge',
      metrics: [{ field: 'cpu_usage' }],
      alert_zones: [
        { min: 0, max: 60, color: '#00FF00', label: 'Healthy' },
        { min: 60, max: 80, color: '#FFFF00', label: 'Warning' },
        { min: 80, max: 100, color: '#FF0000', label: 'Critical' },
      ],
    };

    const spec = generateVisualizationSpec(request, createContext(request));

    expect(spec.alert_zones).toHaveLength(3);
  });

  it('generates deterministic output for same input', () => {
    const request: VisualizationRequest = {
      data_source: {
        type: 'telemetry_aggregates',
        source_id: 'service',
      },
      visualization_type: 'line_chart',
      metrics: [{ field: 'latency' }],
    };

    const context = createContext(request);
    const spec1 = generateVisualizationSpec(request, context);
    const spec2 = generateVisualizationSpec(request, context);

    // The spec_id and generated_at will differ, but structure should be same
    expect(spec1.visualization_type).toBe(spec2.visualization_type);
    expect(spec1.series[0].field).toBe(spec2.series[0].field);
    expect(spec1.styling.theme).toBe(spec2.styling.theme);
  });
});

describe('generateBatchVisualizationSpecs', () => {
  it('generates multiple specs', () => {
    const requests: VisualizationRequest[] = [
      {
        data_source: { type: 'telemetry_aggregates', source_id: 'svc-1' },
        visualization_type: 'line_chart',
        metrics: [{ field: 'latency' }],
      },
      {
        data_source: { type: 'telemetry_aggregates', source_id: 'svc-2' },
        visualization_type: 'bar_chart',
        metrics: [{ field: 'count' }],
      },
    ];

    const specs = generateBatchVisualizationSpecs(requests);

    expect(specs).toHaveLength(2);
    expect(specs[0].visualization_type).toBe('line_chart');
    expect(specs[1].visualization_type).toBe('bar_chart');
  });

  it('applies shared styling to all specs', () => {
    const requests: VisualizationRequest[] = [
      {
        data_source: { type: 'telemetry_aggregates', source_id: 'svc-1' },
        visualization_type: 'line_chart',
        metrics: [{ field: 'latency' }],
      },
      {
        data_source: { type: 'telemetry_aggregates', source_id: 'svc-2' },
        visualization_type: 'line_chart',
        metrics: [{ field: 'throughput' }],
      },
    ];

    const sharedStyling = {
      theme: 'dark' as const,
      animations: false,
    };

    const specs = generateBatchVisualizationSpecs(requests, sharedStyling);

    expect(specs[0].styling.theme).toBe('dark');
    expect(specs[0].styling.animations).toBe(false);
    expect(specs[1].styling.theme).toBe('dark');
    expect(specs[1].styling.animations).toBe(false);
  });

  it('allows request-specific styling to override shared', () => {
    const requests: VisualizationRequest[] = [
      {
        data_source: { type: 'telemetry_aggregates', source_id: 'svc-1' },
        visualization_type: 'line_chart',
        metrics: [{ field: 'latency' }],
        styling: { theme: 'light' }, // Override
      },
      {
        data_source: { type: 'telemetry_aggregates', source_id: 'svc-2' },
        visualization_type: 'line_chart',
        metrics: [{ field: 'throughput' }],
      },
    ];

    const sharedStyling = {
      theme: 'dark' as const,
    };

    const specs = generateBatchVisualizationSpecs(requests, sharedStyling);

    expect(specs[0].styling.theme).toBe('light'); // Overridden
    expect(specs[1].styling.theme).toBe('dark'); // From shared
  });
});

describe('getRecommendedColorScheme', () => {
  it('returns categorical for line charts', () => {
    expect(getRecommendedColorScheme('line_chart')).toBe('categorical');
  });

  it('returns sequential for area charts', () => {
    expect(getRecommendedColorScheme('area_chart')).toBe('sequential');
  });

  it('returns heatmap for heatmaps', () => {
    expect(getRecommendedColorScheme('heatmap')).toBe('heatmap');
  });

  it('returns status for gauges', () => {
    expect(getRecommendedColorScheme('gauge')).toBe('status');
  });
});

describe('getRecommendedAggregation', () => {
  it('returns p50 for box plots', () => {
    expect(getRecommendedAggregation('box_plot', 0)).toBe('p50');
  });

  it('returns appropriate aggregations for candlestick', () => {
    expect(getRecommendedAggregation('candlestick', 0)).toBe('avg'); // open
    expect(getRecommendedAggregation('candlestick', 1)).toBe('max'); // high
    expect(getRecommendedAggregation('candlestick', 2)).toBe('min'); // low
    expect(getRecommendedAggregation('candlestick', 3)).toBe('avg'); // close
  });

  it('returns avg as default', () => {
    expect(getRecommendedAggregation('line_chart', 0)).toBe('avg');
  });
});

describe('supportsStacking', () => {
  it('returns true for stackable types', () => {
    expect(supportsStacking('area_chart')).toBe(true);
    expect(supportsStacking('stacked_bar_chart')).toBe(true);
  });

  it('returns false for non-stackable types', () => {
    expect(supportsStacking('line_chart')).toBe(false);
    expect(supportsStacking('pie_chart')).toBe(false);
  });
});

describe('supportsMultipleYAxes', () => {
  it('returns true for multi-axis types', () => {
    expect(supportsMultipleYAxes('line_chart')).toBe(true);
    expect(supportsMultipleYAxes('area_chart')).toBe(true);
    expect(supportsMultipleYAxes('bar_chart')).toBe(true);
  });

  it('returns false for single-axis types', () => {
    expect(supportsMultipleYAxes('pie_chart')).toBe(false);
    expect(supportsMultipleYAxes('gauge')).toBe(false);
  });
});
