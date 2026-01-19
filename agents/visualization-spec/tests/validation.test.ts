/**
 * Visualization Spec Agent - Validation Tests
 *
 * Tests for validation helpers and compatibility checks
 */

import { describe, it, expect } from 'vitest';
import {
  validateRequest,
  validateBatchRequest,
  checkMetricCompatibility,
  checkTimeRangeValidity,
  validateVisualizationType,
  getVisualizationTypeInfo,
  listVisualizationTypes,
  computeInputHash,
  verifyInputHash,
  createAgentError,
  createErrorResponse,
} from '../contracts/validation.js';

describe('validateRequest', () => {
  it('validates a correct request', () => {
    const request = {
      data_source: {
        type: 'telemetry_aggregates',
        source_id: 'test-service',
      },
      visualization_type: 'line_chart',
      metrics: [{ field: 'latency' }],
    };

    const result = validateRequest(request);
    expect(result.success).toBe(true);
  });

  it('returns error for missing required fields', () => {
    const request = {
      data_source: {
        type: 'telemetry_aggregates',
        source_id: 'test',
      },
      // Missing visualization_type and metrics
    };

    const result = validateRequest(request);
    expect(result.success).toBe(false);
    if (!result.success) {
      expect(result.error.code).toBe('SCHEMA_VALIDATION_FAILED');
    }
  });
});

describe('checkMetricCompatibility', () => {
  it('passes for valid metric count', () => {
    const result = checkMetricCompatibility('line_chart', [
      { field: 'metric1' },
      { field: 'metric2' },
    ]);
    expect(result.compatible).toBe(true);
  });

  it('fails for pie chart with multiple metrics', () => {
    const result = checkMetricCompatibility('pie_chart', [
      { field: 'metric1' },
      { field: 'metric2' },
    ]);
    expect(result.compatible).toBe(false);
    if (!result.compatible) {
      expect(result.error.code).toBe('INCOMPATIBLE_METRICS');
    }
  });

  it('fails for radar chart with too few metrics', () => {
    const result = checkMetricCompatibility('radar', [
      { field: 'metric1' },
    ]);
    expect(result.compatible).toBe(false);
  });

  it('fails when metric count exceeds maximum', () => {
    const tooManyMetrics = Array.from({ length: 15 }, (_, i) => ({
      field: `metric${i}`,
    }));

    const result = checkMetricCompatibility('line_chart', tooManyMetrics);
    expect(result.compatible).toBe(false);
  });
});

describe('checkTimeRangeValidity', () => {
  it('passes for valid time range on time series', () => {
    const result = checkTimeRangeValidity('line_chart', {
      start: '2024-01-01T00:00:00Z',
      end: '2024-01-02T00:00:00Z',
    });
    expect(result.valid).toBe(true);
  });

  it('fails for time series without time range', () => {
    const result = checkTimeRangeValidity('line_chart', undefined);
    expect(result.valid).toBe(false);
    if (!result.valid) {
      expect(result.error.code).toBe('INVALID_TIME_RANGE');
    }
  });

  it('passes for non-time-series without time range', () => {
    const result = checkTimeRangeValidity('bar_chart', undefined);
    expect(result.valid).toBe(true);
  });

  it('fails for time range exceeding 1 year', () => {
    const result = checkTimeRangeValidity('line_chart', {
      start: '2022-01-01T00:00:00Z',
      end: '2024-01-01T00:00:00Z',
    });
    expect(result.valid).toBe(false);
  });
});

describe('validateVisualizationType', () => {
  it('validates known types', () => {
    expect(validateVisualizationType('line_chart').valid).toBe(true);
    expect(validateVisualizationType('bar_chart').valid).toBe(true);
    expect(validateVisualizationType('heatmap').valid).toBe(true);
  });

  it('rejects unknown types', () => {
    const result = validateVisualizationType('unknown_chart');
    expect(result.valid).toBe(false);
    if (!result.valid) {
      expect(result.error.code).toBe('UNSUPPORTED_VISUALIZATION_TYPE');
      expect(result.error.details?.supportedTypes).toBeDefined();
    }
  });
});

describe('getVisualizationTypeInfo', () => {
  it('returns correct info for line chart', () => {
    const info = getVisualizationTypeInfo('line_chart');
    expect(info.category).toBe('time_series');
    expect(info.supportsTimeSeries).toBe(true);
    expect(info.minMetrics).toBe(1);
    expect(info.maxMetrics).toBe(10);
  });

  it('returns correct info for pie chart', () => {
    const info = getVisualizationTypeInfo('pie_chart');
    expect(info.category).toBe('categorical');
    expect(info.supportsTimeSeries).toBe(false);
    expect(info.minMetrics).toBe(1);
    expect(info.maxMetrics).toBe(1);
  });

  it('returns correct info for candlestick', () => {
    const info = getVisualizationTypeInfo('candlestick');
    expect(info.category).toBe('time_series');
    expect(info.minMetrics).toBe(4); // open, high, low, close
    expect(info.maxMetrics).toBe(4);
  });
});

describe('listVisualizationTypes', () => {
  it('lists all types when category is all', () => {
    const types = listVisualizationTypes('all');
    expect(types.length).toBeGreaterThan(10);
  });

  it('lists only time series types', () => {
    const types = listVisualizationTypes('time_series');
    expect(types.every(t => t.category === 'time_series')).toBe(true);
  });

  it('lists only categorical types', () => {
    const types = listVisualizationTypes('categorical');
    expect(types.every(t => t.category === 'categorical')).toBe(true);
  });
});

describe('computeInputHash', () => {
  it('produces consistent hash for same input', () => {
    const input = { a: 1, b: 'test', c: [1, 2, 3] };
    const hash1 = computeInputHash(input);
    const hash2 = computeInputHash(input);
    expect(hash1).toBe(hash2);
  });

  it('produces 64-character hex hash', () => {
    const hash = computeInputHash({ test: 'data' });
    expect(hash).toMatch(/^[a-f0-9]{64}$/);
  });

  it('produces different hash for different input', () => {
    const hash1 = computeInputHash({ a: 1 });
    const hash2 = computeInputHash({ a: 2 });
    expect(hash1).not.toBe(hash2);
  });

  it('normalizes key order', () => {
    const hash1 = computeInputHash({ a: 1, b: 2 });
    const hash2 = computeInputHash({ b: 2, a: 1 });
    expect(hash1).toBe(hash2);
  });
});

describe('verifyInputHash', () => {
  it('returns true for matching hash', () => {
    const input = { test: 'data' };
    const hash = computeInputHash(input);
    expect(verifyInputHash(input, hash)).toBe(true);
  });

  it('returns false for non-matching hash', () => {
    const input = { test: 'data' };
    expect(verifyInputHash(input, 'invalid_hash')).toBe(false);
  });
});

describe('createAgentError', () => {
  it('creates error with correct code', () => {
    const error = createAgentError('INVALID_DATA_SOURCE', 'Invalid source');
    expect(error.code).toBe('INVALID_DATA_SOURCE');
    expect(error.message).toBe('Invalid source');
    expect(error.recoverable).toBe(true);
  });

  it('marks internal errors as non-recoverable', () => {
    const error = createAgentError('INTERNAL_ERROR', 'Something went wrong');
    expect(error.recoverable).toBe(false);
  });

  it('includes details when provided', () => {
    const error = createAgentError('SCHEMA_VALIDATION_FAILED', 'Failed', { field: 'test' });
    expect(error.details).toEqual({ field: 'test' });
  });
});

describe('createErrorResponse', () => {
  it('creates error response from agent error', () => {
    const error = createAgentError('INVALID_DATA_SOURCE', 'Invalid');
    const response = createErrorResponse(error, 'request-123');

    expect(response.success).toBe(false);
    expect(response.error.code).toBe('INVALID_DATA_SOURCE');
    expect(response.request_id).toBe('request-123');
  });

  it('works without request ID', () => {
    const error = createAgentError('INTERNAL_ERROR', 'Error');
    const response = createErrorResponse(error);

    expect(response.success).toBe(false);
    expect(response.request_id).toBeUndefined();
  });
});
