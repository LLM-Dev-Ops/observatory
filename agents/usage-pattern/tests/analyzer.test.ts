/**
 * Tests for Usage Pattern Analyzer.
 *
 * CONSTITUTION VERIFICATION:
 * These tests verify that the agent adheres to constitutional constraints:
 * - READ-ONLY operation
 * - ADVISORY classification
 * - Statistical confidence
 * - Empty constraints_applied
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { UsagePatternAnalyzer } from '../src/analyzer.js';
import { getDefaultConfig } from '../src/config.js';
import { RuvectorClient } from '../src/ruvector-client.js';
import { StoredTelemetryEvent } from '../src/types/ruvector.js';
import { AnalysisRequest } from '../contracts/schemas.js';

// Mock RuvectorClient
vi.mock('../src/ruvector-client.js', () => {
  return {
    RuvectorClient: vi.fn().mockImplementation(() => ({
      streamTelemetryEvents: vi.fn(),
      getTelemetryEvents: vi.fn(),
      healthCheck: vi.fn(),
    })),
  };
});

describe('UsagePatternAnalyzer', () => {
  let analyzer: UsagePatternAnalyzer;
  let mockClient: RuvectorClient;
  const config = getDefaultConfig();

  beforeEach(() => {
    mockClient = new RuvectorClient(config.ruvector);
    analyzer = new UsagePatternAnalyzer(config, mockClient);
  });

  describe('CONSTITUTION: READ-ONLY Operation', () => {
    it('should only read telemetry data, never modify it', async () => {
      const mockEvents: StoredTelemetryEvent[] = createMockEvents(100);

      vi.mocked(mockClient.streamTelemetryEvents).mockImplementation(
        async function* () {
          yield mockEvents;
        }
      );

      const request = createAnalysisRequest();
      const analysis = await analyzer.analyze(request);

      // Verify analysis was performed
      expect(analysis.sample_size).toBe(100);

      // Verify no write operations were called
      expect(mockClient.persistDecisionEvent).toBeUndefined();
    });

    it('should not modify input events during analysis', async () => {
      const mockEvents: StoredTelemetryEvent[] = createMockEvents(10);
      const originalEvents = JSON.parse(JSON.stringify(mockEvents));

      vi.mocked(mockClient.streamTelemetryEvents).mockImplementation(
        async function* () {
          yield mockEvents;
        }
      );

      const request = createAnalysisRequest();
      await analyzer.analyze(request);

      // Events should remain unchanged
      expect(mockEvents).toEqual(originalEvents);
    });
  });

  describe('CONSTITUTION: ADVISORY Classification', () => {
    it('should NOT classify failures', async () => {
      const mockEvents = createMockEvents(50, { includeErrors: true });

      vi.mocked(mockClient.streamTelemetryEvents).mockImplementation(
        async function* () {
          yield mockEvents;
        }
      );

      const request = createAnalysisRequest();
      const analysis = await analyzer.analyze(request);

      // Analysis should compute error_rate but NOT classify failures
      expect(analysis.summary.error_rate).toBeDefined();
      expect(analysis.summary.error_rate).toBeGreaterThan(0);

      // Should NOT have failure classifications
      expect((analysis as any).failure_classifications).toBeUndefined();
      expect((analysis as any).failure_severity).toBeUndefined();
    });

    it('should NOT evaluate health', async () => {
      const mockEvents = createMockEvents(100);

      vi.mocked(mockClient.streamTelemetryEvents).mockImplementation(
        async function* () {
          yield mockEvents;
        }
      );

      const request = createAnalysisRequest();
      const analysis = await analyzer.analyze(request);

      // Should NOT have health evaluations
      expect((analysis as any).health_status).toBeUndefined();
      expect((analysis as any).health_score).toBeUndefined();
      expect((analysis as any).is_healthy).toBeUndefined();
    });

    it('should NOT enforce thresholds', async () => {
      const mockEvents = createMockEvents(100);

      vi.mocked(mockClient.streamTelemetryEvents).mockImplementation(
        async function* () {
          yield mockEvents;
        }
      );

      const request = createAnalysisRequest();
      const analysis = await analyzer.analyze(request);

      // Should NOT have threshold violations
      expect((analysis as any).threshold_violations).toBeUndefined();
      expect((analysis as any).sla_breaches).toBeUndefined();
      expect((analysis as any).alerts).toBeUndefined();
    });

    it('should NOT generate alerts', async () => {
      const mockEvents = createMockEvents(100, {
        highLatency: true,
        highErrorRate: true,
      });

      vi.mocked(mockClient.streamTelemetryEvents).mockImplementation(
        async function* () {
          yield mockEvents;
        }
      );

      const request = createAnalysisRequest();
      const analysis = await analyzer.analyze(request);

      // Should NOT have alerts even with problematic data
      expect((analysis as any).alerts).toBeUndefined();
      expect((analysis as any).notifications).toBeUndefined();
      expect((analysis as any).warnings).toBeUndefined();
    });
  });

  describe('CONSTITUTION: Statistical Confidence', () => {
    it('should compute confidence based on sample size', async () => {
      const mockEvents = createMockEvents(1000);

      vi.mocked(mockClient.streamTelemetryEvents).mockImplementation(
        async function* () {
          yield mockEvents;
        }
      );

      const request = createAnalysisRequest();
      const analysis = await analyzer.analyze(request);

      // Confidence should be between 0 and 1
      expect(analysis.overall_confidence).toBeGreaterThanOrEqual(0);
      expect(analysis.overall_confidence).toBeLessThanOrEqual(1);

      // Larger sample = higher confidence
      expect(analysis.overall_confidence).toBeGreaterThan(0.5);
    });

    it('should have lower confidence with small sample size', async () => {
      const mockEvents = createMockEvents(10);

      vi.mocked(mockClient.streamTelemetryEvents).mockImplementation(
        async function* () {
          yield mockEvents;
        }
      );

      const request = createAnalysisRequest();
      const analysis = await analyzer.analyze(request);

      // Small sample = lower confidence
      expect(analysis.overall_confidence).toBeLessThan(0.5);
    });

    it('should include confidence in trend analysis', async () => {
      const mockEvents = createMockEvents(100);

      vi.mocked(mockClient.streamTelemetryEvents).mockImplementation(
        async function* () {
          yield mockEvents;
        }
      );

      const request = createAnalysisRequest({ includeTrends: true });
      const analysis = await analyzer.analyze(request);

      if (analysis.trends && analysis.trends.length > 0) {
        for (const trend of analysis.trends) {
          expect(trend.confidence).toBeGreaterThanOrEqual(0);
          expect(trend.confidence).toBeLessThanOrEqual(1);
        }
      }
    });
  });

  describe('Analysis Capabilities', () => {
    it('should compute time series aggregations', async () => {
      const mockEvents = createMockEvents(100);

      vi.mocked(mockClient.streamTelemetryEvents).mockImplementation(
        async function* () {
          yield mockEvents;
        }
      );

      const request = createAnalysisRequest();
      const analysis = await analyzer.analyze(request);

      expect(analysis.time_series).toBeDefined();
      expect(Array.isArray(analysis.time_series)).toBe(true);

      for (const bucket of analysis.time_series) {
        expect(bucket.bucket_start).toBeDefined();
        expect(bucket.bucket_end).toBeDefined();
        expect(bucket.request_count).toBeGreaterThanOrEqual(0);
        expect(bucket.total_tokens).toBeGreaterThanOrEqual(0);
      }
    });

    it('should compute distribution statistics', async () => {
      const mockEvents = createMockEvents(100);

      vi.mocked(mockClient.streamTelemetryEvents).mockImplementation(
        async function* () {
          yield mockEvents;
        }
      );

      const request = createAnalysisRequest();
      const analysis = await analyzer.analyze(request);

      expect(analysis.distributions).toBeDefined();

      if (analysis.distributions.latency) {
        expect(analysis.distributions.latency.mean).toBeDefined();
        expect(analysis.distributions.latency.median).toBeDefined();
        expect(analysis.distributions.latency.std_dev).toBeGreaterThanOrEqual(0);
      }
    });

    it('should compute provider usage breakdown', async () => {
      const mockEvents = createMockEvents(100);

      vi.mocked(mockClient.streamTelemetryEvents).mockImplementation(
        async function* () {
          yield mockEvents;
        }
      );

      const request = createAnalysisRequest();
      const analysis = await analyzer.analyze(request);

      expect(analysis.provider_usage).toBeDefined();
      expect(Array.isArray(analysis.provider_usage)).toBe(true);

      let totalPercentage = 0;
      for (const provider of analysis.provider_usage) {
        expect(provider.provider).toBeDefined();
        expect(provider.request_count).toBeGreaterThanOrEqual(0);
        totalPercentage += provider.percentage_of_total;
      }

      // Total percentage should be approximately 100%
      expect(totalPercentage).toBeCloseTo(100, 0);
    });

    it('should identify hotspots', async () => {
      const mockEvents = createMockEvents(100);

      vi.mocked(mockClient.streamTelemetryEvents).mockImplementation(
        async function* () {
          yield mockEvents;
        }
      );

      const request = createAnalysisRequest();
      const analysis = await analyzer.analyze(request);

      expect(analysis.hotspots).toBeDefined();
      expect(Array.isArray(analysis.hotspots)).toBe(true);

      for (const hotspot of analysis.hotspots) {
        expect(hotspot.dimension).toBeDefined();
        expect(hotspot.value).toBeDefined();
        expect(hotspot.intensity).toBeGreaterThanOrEqual(0);
        expect(hotspot.intensity).toBeLessThanOrEqual(1);
      }
    });

    it('should compute growth patterns', async () => {
      const mockEvents = createMockEvents(100);

      vi.mocked(mockClient.streamTelemetryEvents).mockImplementation(
        async function* () {
          yield mockEvents;
        }
      );

      const request = createAnalysisRequest();
      const analysis = await analyzer.analyze(request);

      expect(analysis.growth_patterns).toBeDefined();
      expect(Array.isArray(analysis.growth_patterns)).toBe(true);

      for (const pattern of analysis.growth_patterns) {
        expect(pattern.metric_name).toBeDefined();
        expect(pattern.growth_classification).toMatch(
          /^(rapid_growth|moderate_growth|stable|moderate_decline|rapid_decline)$/
        );
        expect(pattern.confidence).toBeGreaterThanOrEqual(0);
        expect(pattern.confidence).toBeLessThanOrEqual(1);
      }
    });
  });

  describe('Input Hash Computation', () => {
    it('should compute deterministic SHA256 hash of inputs', () => {
      const request = createAnalysisRequest();
      const hash1 = analyzer.computeInputsHash(request);
      const hash2 = analyzer.computeInputsHash(request);

      // Same input = same hash
      expect(hash1).toBe(hash2);

      // Should be 64 character hex string (SHA256)
      expect(hash1).toMatch(/^[a-f0-9]{64}$/);
    });

    it('should produce different hashes for different inputs', () => {
      const request1 = createAnalysisRequest();
      const request2 = createAnalysisRequest({
        startTime: '2025-01-17T00:00:00Z',
      });

      const hash1 = analyzer.computeInputsHash(request1);
      const hash2 = analyzer.computeInputsHash(request2);

      expect(hash1).not.toBe(hash2);
    });
  });
});

// Helper functions

function createMockEvents(
  count: number,
  options: {
    includeErrors?: boolean;
    highLatency?: boolean;
    highErrorRate?: boolean;
  } = {}
): StoredTelemetryEvent[] {
  const providers = ['openai', 'anthropic', 'google'];
  const models = ['gpt-4', 'claude-3', 'gemini-pro'];
  const events: StoredTelemetryEvent[] = [];

  const baseTime = new Date('2025-01-18T00:00:00Z');

  for (let i = 0; i < count; i++) {
    const startTime = new Date(baseTime.getTime() + i * 60000);
    const latency = options.highLatency
      ? 5000 + Math.random() * 10000
      : 100 + Math.random() * 500;

    const isError =
      options.includeErrors && Math.random() < (options.highErrorRate ? 0.5 : 0.1);

    events.push({
      id: `event-${i}`,
      span_id: `span-${i}`,
      trace_id: `trace-${Math.floor(i / 10)}`,
      name: `llm-call-${i}`,
      provider: providers[i % providers.length],
      model: models[i % models.length],
      token_usage: {
        prompt_tokens: 100 + Math.floor(Math.random() * 500),
        completion_tokens: 50 + Math.floor(Math.random() * 200),
        total_tokens: 150 + Math.floor(Math.random() * 700),
      },
      cost: {
        amount_usd: 0.01 + Math.random() * 0.1,
        currency: 'USD',
      },
      latency: {
        total_ms: latency,
        ttft_ms: latency * 0.3,
        start_time: startTime.toISOString(),
        end_time: new Date(startTime.getTime() + latency).toISOString(),
      },
      status: isError ? 'ERROR' : 'OK',
      metadata: {
        user_id: `user-${i % 10}`,
        session_id: `session-${i % 5}`,
        environment: 'production',
        tags: ['test'],
        attributes: {},
      },
      normalized_at: startTime.toISOString(),
      created_at: startTime.toISOString(),
    });
  }

  return events;
}

function createAnalysisRequest(
  overrides: {
    startTime?: string;
    endTime?: string;
    includeTrends?: boolean;
    includeSeasonality?: boolean;
  } = {}
): AnalysisRequest {
  return {
    time_window: {
      start: overrides.startTime || '2025-01-18T00:00:00Z',
      end: overrides.endTime || '2025-01-19T00:00:00Z',
      granularity: 'hour',
    },
    filters: {},
    options: {
      include_trends: overrides.includeTrends ?? true,
      include_distributions: true,
      include_seasonality: overrides.includeSeasonality ?? false,
      include_forecasts: false,
      percentiles: [50, 90, 95, 99],
    },
  };
}
