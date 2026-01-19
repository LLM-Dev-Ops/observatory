/**
 * Visualization Spec Agent - Schema Tests
 *
 * Tests for Zod schema validation
 */

import { describe, it, expect } from 'vitest';
import {
  VisualizationRequestSchema,
  BatchVisualizationRequestSchema,
  VisualizationSpecSchema,
  VisualizationDecisionEventSchema,
  VisualizationTypeSchema,
  DataSourceSpecSchema,
  MetricSpecSchema,
  TimeRangeSpecSchema,
  AGENT_ID,
  AGENT_VERSION,
  AGENT_CLASSIFICATION,
  DECISION_TYPE,
} from '../contracts/schemas.js';

describe('VisualizationTypeSchema', () => {
  it('accepts valid visualization types', () => {
    const validTypes = [
      'line_chart',
      'bar_chart',
      'pie_chart',
      'table',
      'heatmap',
      'gauge',
      'scatter_plot',
    ];

    for (const type of validTypes) {
      const result = VisualizationTypeSchema.safeParse(type);
      expect(result.success).toBe(true);
    }
  });

  it('rejects invalid visualization types', () => {
    const result = VisualizationTypeSchema.safeParse('invalid_chart');
    expect(result.success).toBe(false);
  });
});

describe('DataSourceSpecSchema', () => {
  it('accepts valid data source spec', () => {
    const validSpec = {
      type: 'telemetry_aggregates',
      source_id: 'my-service',
      time_field: 'timestamp',
    };

    const result = DataSourceSpecSchema.safeParse(validSpec);
    expect(result.success).toBe(true);
  });

  it('accepts data source with filters', () => {
    const specWithFilters = {
      type: 'metric_series',
      source_id: 'api-metrics',
      filters: {
        environment: 'production',
        status: 200,
        enabled: true,
      },
    };

    const result = DataSourceSpecSchema.safeParse(specWithFilters);
    expect(result.success).toBe(true);
  });

  it('rejects invalid data source type', () => {
    const invalidSpec = {
      type: 'invalid_type',
      source_id: 'test',
    };

    const result = DataSourceSpecSchema.safeParse(invalidSpec);
    expect(result.success).toBe(false);
  });

  it('rejects empty source_id', () => {
    const invalidSpec = {
      type: 'telemetry_aggregates',
      source_id: '',
    };

    const result = DataSourceSpecSchema.safeParse(invalidSpec);
    expect(result.success).toBe(false);
  });
});

describe('MetricSpecSchema', () => {
  it('accepts minimal metric spec', () => {
    const minimalMetric = {
      field: 'latency_p99',
    };

    const result = MetricSpecSchema.safeParse(minimalMetric);
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.aggregation).toBe('avg'); // default
    }
  });

  it('accepts full metric spec', () => {
    const fullMetric = {
      field: 'error_rate',
      label: 'Error Rate',
      aggregation: 'sum',
      format: '0.00%',
      color: '#FF0000',
      unit: '%',
    };

    const result = MetricSpecSchema.safeParse(fullMetric);
    expect(result.success).toBe(true);
  });

  it('rejects invalid color format', () => {
    const invalidMetric = {
      field: 'test',
      color: 'red', // should be hex
    };

    const result = MetricSpecSchema.safeParse(invalidMetric);
    expect(result.success).toBe(false);
  });

  it('rejects invalid aggregation', () => {
    const invalidMetric = {
      field: 'test',
      aggregation: 'invalid',
    };

    const result = MetricSpecSchema.safeParse(invalidMetric);
    expect(result.success).toBe(false);
  });
});

describe('TimeRangeSpecSchema', () => {
  it('accepts valid time range', () => {
    const validRange = {
      start: '2024-01-01T00:00:00Z',
      end: '2024-01-02T00:00:00Z',
    };

    const result = TimeRangeSpecSchema.safeParse(validRange);
    expect(result.success).toBe(true);
  });

  it('accepts time range with granularity', () => {
    const rangeWithGranularity = {
      start: '2024-01-01T00:00:00Z',
      end: '2024-01-02T00:00:00Z',
      granularity: '1h',
      timezone: 'America/New_York',
    };

    const result = TimeRangeSpecSchema.safeParse(rangeWithGranularity);
    expect(result.success).toBe(true);
  });

  it('rejects when start is after end', () => {
    const invalidRange = {
      start: '2024-01-02T00:00:00Z',
      end: '2024-01-01T00:00:00Z',
    };

    const result = TimeRangeSpecSchema.safeParse(invalidRange);
    expect(result.success).toBe(false);
  });

  it('rejects invalid datetime format', () => {
    const invalidRange = {
      start: 'not-a-date',
      end: '2024-01-01T00:00:00Z',
    };

    const result = TimeRangeSpecSchema.safeParse(invalidRange);
    expect(result.success).toBe(false);
  });
});

describe('VisualizationRequestSchema', () => {
  it('accepts minimal valid request', () => {
    const minimalRequest = {
      data_source: {
        type: 'telemetry_aggregates',
        source_id: 'my-service',
      },
      visualization_type: 'line_chart',
      metrics: [{ field: 'latency' }],
    };

    const result = VisualizationRequestSchema.safeParse(minimalRequest);
    expect(result.success).toBe(true);
  });

  it('accepts full request with all optional fields', () => {
    const fullRequest = {
      data_source: {
        type: 'metric_series',
        source_id: 'api-service',
        filters: { env: 'prod' },
      },
      visualization_type: 'line_chart',
      metrics: [
        { field: 'latency_p95', label: 'P95 Latency', aggregation: 'p95' },
        { field: 'latency_p99', label: 'P99 Latency', aggregation: 'p99' },
      ],
      dimensions: [{ field: 'endpoint', label: 'Endpoint' }],
      time_range: {
        start: '2024-01-01T00:00:00Z',
        end: '2024-01-02T00:00:00Z',
        granularity: '5m',
      },
      styling: {
        theme: 'observatory',
        title: 'API Latency',
      },
      thresholds: [
        { value: 100, label: 'Target', color: '#00FF00' },
      ],
      request_id: '123e4567-e89b-12d3-a456-426614174000',
    };

    const result = VisualizationRequestSchema.safeParse(fullRequest);
    expect(result.success).toBe(true);
  });

  it('rejects request with empty metrics array', () => {
    const invalidRequest = {
      data_source: {
        type: 'telemetry_aggregates',
        source_id: 'test',
      },
      visualization_type: 'line_chart',
      metrics: [],
    };

    const result = VisualizationRequestSchema.safeParse(invalidRequest);
    expect(result.success).toBe(false);
  });

  it('rejects request with too many metrics', () => {
    const tooManyMetrics = Array.from({ length: 15 }, (_, i) => ({
      field: `metric_${i}`,
    }));

    const invalidRequest = {
      data_source: {
        type: 'telemetry_aggregates',
        source_id: 'test',
      },
      visualization_type: 'line_chart',
      metrics: tooManyMetrics,
    };

    const result = VisualizationRequestSchema.safeParse(invalidRequest);
    expect(result.success).toBe(false);
  });
});

describe('BatchVisualizationRequestSchema', () => {
  it('accepts valid batch request', () => {
    const batchRequest = {
      requests: [
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
      ],
      shared_styling: {
        theme: 'dark',
      },
    };

    const result = BatchVisualizationRequestSchema.safeParse(batchRequest);
    expect(result.success).toBe(true);
  });

  it('rejects batch with too many requests', () => {
    const tooManyRequests = Array.from({ length: 25 }, () => ({
      data_source: { type: 'telemetry_aggregates', source_id: 'test' },
      visualization_type: 'line_chart',
      metrics: [{ field: 'metric' }],
    }));

    const invalidBatch = { requests: tooManyRequests };
    const result = BatchVisualizationRequestSchema.safeParse(invalidBatch);
    expect(result.success).toBe(false);
  });
});

describe('VisualizationDecisionEventSchema', () => {
  it('validates correct decision event', () => {
    const validEvent = {
      agent_id: AGENT_ID,
      agent_version: AGENT_VERSION,
      decision_type: DECISION_TYPE,
      confidence: 0.95,
      constraints_applied: [],
      classification: AGENT_CLASSIFICATION,
      inputs_hash: 'a'.repeat(64),
      outputs: [
        {
          spec_id: '123e4567-e89b-12d3-a456-426614174000',
          spec_version: '1.0',
          visualization_type: 'line_chart',
          data_source: { type: 'telemetry_aggregates', source_id: 'test' },
          series: [
            { id: 'series-0', name: 'Latency', field: 'latency', aggregation: 'avg', color: '#3B82F6' },
          ],
          axes: [
            { type: 'x', scale: 'time', gridLines: true },
            { type: 'y', scale: 'linear', gridLines: true },
          ],
          styling: {
            theme: 'observatory',
            color_scheme: 'default',
            legend: { position: 'bottom', show_values: false, interactive: true },
            tooltip: { enabled: true, show_all_series: true },
            dimensions: { responsive: true },
            animations: true,
          },
          metadata: {
            generated_at: '2024-01-01T00:00:00Z',
            generator_version: AGENT_VERSION,
            input_hash: 'b'.repeat(64),
            deterministic: true,
          },
        },
      ],
      execution_ref: 'exec-123',
      timestamp: '2024-01-01T00:00:00Z',
      processing_metrics: {
        parsing_ms: 1,
        validation_ms: 2,
        generation_ms: 10,
        total_ms: 15,
        specs_generated: 1,
      },
    };

    const result = VisualizationDecisionEventSchema.safeParse(validEvent);
    expect(result.success).toBe(true);
  });

  it('rejects event with non-empty constraints_applied', () => {
    const invalidEvent = {
      agent_id: AGENT_ID,
      agent_version: AGENT_VERSION,
      decision_type: DECISION_TYPE,
      confidence: 0.95,
      constraints_applied: ['some_constraint'], // Must be empty for READ-ONLY
      classification: AGENT_CLASSIFICATION,
      inputs_hash: 'a'.repeat(64),
      outputs: [],
      execution_ref: 'exec-123',
      timestamp: '2024-01-01T00:00:00Z',
      processing_metrics: {
        parsing_ms: 1,
        validation_ms: 2,
        generation_ms: 10,
        total_ms: 15,
        specs_generated: 0,
      },
    };

    const result = VisualizationDecisionEventSchema.safeParse(invalidEvent);
    expect(result.success).toBe(false);
  });

  it('rejects event with wrong agent_id', () => {
    const invalidEvent = {
      agent_id: 'wrong-agent-id',
      agent_version: AGENT_VERSION,
      decision_type: DECISION_TYPE,
      confidence: 0.95,
      constraints_applied: [],
      classification: AGENT_CLASSIFICATION,
      inputs_hash: 'a'.repeat(64),
      outputs: [],
      execution_ref: 'exec-123',
      timestamp: '2024-01-01T00:00:00Z',
      processing_metrics: {
        parsing_ms: 1,
        validation_ms: 2,
        generation_ms: 10,
        total_ms: 15,
        specs_generated: 0,
      },
    };

    const result = VisualizationDecisionEventSchema.safeParse(invalidEvent);
    expect(result.success).toBe(false);
  });

  it('rejects event with confidence out of range', () => {
    const invalidEvent = {
      agent_id: AGENT_ID,
      agent_version: AGENT_VERSION,
      decision_type: DECISION_TYPE,
      confidence: 1.5, // Must be 0-1
      constraints_applied: [],
      classification: AGENT_CLASSIFICATION,
      inputs_hash: 'a'.repeat(64),
      outputs: [],
      execution_ref: 'exec-123',
      timestamp: '2024-01-01T00:00:00Z',
      processing_metrics: {
        parsing_ms: 1,
        validation_ms: 2,
        generation_ms: 10,
        total_ms: 15,
        specs_generated: 0,
      },
    };

    const result = VisualizationDecisionEventSchema.safeParse(invalidEvent);
    expect(result.success).toBe(false);
  });
});
