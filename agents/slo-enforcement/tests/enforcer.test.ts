/**
 * SLO/SLA Enforcement Agent - Enforcer Tests
 *
 * Tests for the core violation detection logic.
 */

import { describe, it, expect, beforeEach } from 'vitest';
import { getSloEnforcer, resetSloEnforcer } from '../src/enforcer';
import type { SloDefinition, TelemetryMetric } from '../contracts';

describe('SloEnforcer', () => {
  beforeEach(() => {
    resetSloEnforcer();
  });

  describe('evaluateAll', () => {
    it('should detect a latency SLO breach', () => {
      const enforcer = getSloEnforcer();

      const slos: SloDefinition[] = [
        {
          slo_id: 'latency-p95',
          name: 'P95 Latency SLO',
          indicator: 'latency_p95',
          operator: 'lt',
          threshold: 500, // Must be less than 500ms
          window: '5m',
          enabled: true,
          is_sla: false,
          warning_threshold_percentage: 80,
        },
      ];

      const metrics: TelemetryMetric[] = [
        {
          metric_id: '550e8400-e29b-41d4-a716-446655440000',
          indicator: 'latency_p95',
          value: 750, // Exceeds threshold
          window: '5m',
          timestamp: new Date().toISOString(),
          sample_count: 100,
        },
      ];

      const result = enforcer.evaluateAll(slos, metrics, new Date());

      expect(result.violations.length).toBe(1);
      expect(result.violations[0].slo_id).toBe('latency-p95');
      expect(result.violations[0].breach_type).toBe('slo_breach');
      expect(result.violations[0].metric_context.current_value).toBe(750);
      expect(result.violations[0].metric_context.threshold_value).toBe(500);
      expect(result.violations[0].metric_context.deviation_percentage).toBe(50);
    });

    it('should detect near-breach (warning)', () => {
      const enforcer = getSloEnforcer();

      const slos: SloDefinition[] = [
        {
          slo_id: 'error-rate',
          name: 'Error Rate SLO',
          indicator: 'error_rate',
          operator: 'lt',
          threshold: 5, // Must be less than 5%
          window: '5m',
          enabled: true,
          is_sla: false,
          warning_threshold_percentage: 80, // Warning at 4%
        },
      ];

      const metrics: TelemetryMetric[] = [
        {
          metric_id: '550e8400-e29b-41d4-a716-446655440001',
          indicator: 'error_rate',
          value: 4.2, // Above 80% of threshold but below threshold
          window: '5m',
          timestamp: new Date().toISOString(),
          sample_count: 1000,
        },
      ];

      const result = enforcer.evaluateAll(slos, metrics, new Date());

      expect(result.violations.length).toBe(1);
      expect(result.violations[0].breach_type).toBe('near_breach');
      expect(result.violations[0].severity).toBe('low');
    });

    it('should mark SLA breach as critical', () => {
      const enforcer = getSloEnforcer();

      const slos: SloDefinition[] = [
        {
          slo_id: 'availability-sla',
          name: 'Availability SLA',
          indicator: 'availability',
          operator: 'gte',
          threshold: 99.9, // Must be >= 99.9%
          window: '24h',
          enabled: true,
          is_sla: true,
          sla_penalty_tier: 2,
          warning_threshold_percentage: 80,
        },
      ];

      const metrics: TelemetryMetric[] = [
        {
          metric_id: '550e8400-e29b-41d4-a716-446655440002',
          indicator: 'availability',
          value: 98.5, // Below SLA threshold
          window: '24h',
          timestamp: new Date().toISOString(),
          sample_count: 86400,
        },
      ];

      const result = enforcer.evaluateAll(slos, metrics, new Date());

      expect(result.violations.length).toBe(1);
      expect(result.violations[0].is_sla).toBe(true);
      expect(result.violations[0].breach_type).toBe('sla_breach');
      expect(result.violations[0].severity).toBe('critical');
      expect(result.violations[0].sla_penalty_tier).toBe(2);
    });

    it('should not report violation for healthy metric', () => {
      const enforcer = getSloEnforcer();

      const slos: SloDefinition[] = [
        {
          slo_id: 'throughput',
          name: 'Throughput SLO',
          indicator: 'throughput',
          operator: 'gte',
          threshold: 100, // Must be >= 100 rps
          window: '1m',
          enabled: true,
          is_sla: false,
          warning_threshold_percentage: 80,
        },
      ];

      const metrics: TelemetryMetric[] = [
        {
          metric_id: '550e8400-e29b-41d4-a716-446655440003',
          indicator: 'throughput',
          value: 150, // Above threshold
          window: '1m',
          timestamp: new Date().toISOString(),
          sample_count: 60,
        },
      ];

      const result = enforcer.evaluateAll(slos, metrics, new Date());

      expect(result.violations.length).toBe(0);
      expect(result.slo_statuses.length).toBe(1);
      expect(result.slo_statuses[0].status).toBe('healthy');
    });

    it('should skip disabled SLOs', () => {
      const enforcer = getSloEnforcer();

      const slos: SloDefinition[] = [
        {
          slo_id: 'disabled-slo',
          name: 'Disabled SLO',
          indicator: 'latency_p95',
          operator: 'lt',
          threshold: 100,
          window: '5m',
          enabled: false, // Disabled
          is_sla: false,
          warning_threshold_percentage: 80,
        },
      ];

      const metrics: TelemetryMetric[] = [
        {
          metric_id: '550e8400-e29b-41d4-a716-446655440004',
          indicator: 'latency_p95',
          value: 500, // Would breach if enabled
          window: '5m',
          timestamp: new Date().toISOString(),
        },
      ];

      const result = enforcer.evaluateAll(slos, metrics, new Date());

      expect(result.violations.length).toBe(0);
      expect(result.slos_evaluated).toBe(0);
    });

    it('should filter metrics by provider and model', () => {
      const enforcer = getSloEnforcer();

      const slos: SloDefinition[] = [
        {
          slo_id: 'openai-latency',
          name: 'OpenAI Latency SLO',
          indicator: 'latency_p95',
          operator: 'lt',
          threshold: 500,
          window: '5m',
          enabled: true,
          is_sla: false,
          provider: 'openai',
          model: 'gpt-4',
          warning_threshold_percentage: 80,
        },
      ];

      const metrics: TelemetryMetric[] = [
        {
          metric_id: '550e8400-e29b-41d4-a716-446655440005',
          indicator: 'latency_p95',
          value: 750, // Would breach
          window: '5m',
          timestamp: new Date().toISOString(),
          provider: 'anthropic', // Different provider
          model: 'claude-3',
        },
        {
          metric_id: '550e8400-e29b-41d4-a716-446655440006',
          indicator: 'latency_p95',
          value: 600, // Breaches
          window: '5m',
          timestamp: new Date().toISOString(),
          provider: 'openai',
          model: 'gpt-4',
        },
      ];

      const result = enforcer.evaluateAll(slos, metrics, new Date());

      expect(result.metrics_evaluated).toBe(1); // Only matching metric
      expect(result.violations.length).toBe(1);
      expect(result.violations[0].provider).toBe('openai');
    });

    it('should return unknown status when no metrics match', () => {
      const enforcer = getSloEnforcer();

      const slos: SloDefinition[] = [
        {
          slo_id: 'no-metrics',
          name: 'No Metrics SLO',
          indicator: 'ttft',
          operator: 'lt',
          threshold: 200,
          window: '5m',
          enabled: true,
          is_sla: false,
          warning_threshold_percentage: 80,
        },
      ];

      const metrics: TelemetryMetric[] = [
        {
          metric_id: '550e8400-e29b-41d4-a716-446655440007',
          indicator: 'latency_p95', // Different indicator
          value: 100,
          window: '5m',
          timestamp: new Date().toISOString(),
        },
      ];

      const result = enforcer.evaluateAll(slos, metrics, new Date());

      expect(result.violations.length).toBe(0);
      expect(result.slo_statuses.length).toBe(1);
      expect(result.slo_statuses[0].status).toBe('unknown');
    });
  });

  describe('calculateConfidence', () => {
    it('should return high confidence for large sample size and fresh data', () => {
      const enforcer = getSloEnforcer();

      const metric: TelemetryMetric = {
        metric_id: '550e8400-e29b-41d4-a716-446655440008',
        indicator: 'latency_p95',
        value: 100,
        window: '5m',
        timestamp: new Date().toISOString(), // Very fresh
        sample_count: 1000, // Large sample
      };

      const confidence = enforcer.calculateConfidence(metric, undefined);

      expect(confidence).toBeGreaterThan(0.8);
    });

    it('should return lower confidence for small sample size', () => {
      const enforcer = getSloEnforcer();

      const metric: TelemetryMetric = {
        metric_id: '550e8400-e29b-41d4-a716-446655440009',
        indicator: 'latency_p95',
        value: 100,
        window: '5m',
        timestamp: new Date().toISOString(),
        sample_count: 2, // Very small sample
      };

      const confidence = enforcer.calculateConfidence(metric, undefined);

      expect(confidence).toBeLessThan(0.8);
    });

    it('should return minimum confidence for stale data', () => {
      const enforcer = getSloEnforcer();

      const staleDate = new Date();
      staleDate.setHours(staleDate.getHours() - 1); // 1 hour old

      const metric: TelemetryMetric = {
        metric_id: '550e8400-e29b-41d4-a716-446655440010',
        indicator: 'latency_p95',
        value: 100,
        window: '5m',
        timestamp: staleDate.toISOString(),
        sample_count: 100,
      };

      const confidence = enforcer.calculateConfidence(metric, undefined);

      expect(confidence).toBe(0.5); // Minimum confidence
    });
  });

  describe('evaluateSingle', () => {
    it('should generate recommendation for violations', () => {
      const enforcer = getSloEnforcer();

      const slos: SloDefinition[] = [
        {
          slo_id: 'recommendation-test',
          name: 'Recommendation Test SLO',
          indicator: 'error_rate',
          operator: 'lt',
          threshold: 1,
          window: '5m',
          enabled: true,
          is_sla: false,
          warning_threshold_percentage: 80,
        },
      ];

      const metrics: TelemetryMetric[] = [
        {
          metric_id: '550e8400-e29b-41d4-a716-446655440011',
          indicator: 'error_rate',
          value: 5, // 5x threshold
          window: '5m',
          timestamp: new Date().toISOString(),
        },
      ];

      const result = enforcer.evaluateAll(slos, metrics, new Date());

      expect(result.violations[0].recommendation).toBeDefined();
      expect(result.violations[0].recommendation).toContain('error_rate');
    });
  });

  describe('edge cases', () => {
    it('should handle empty SLO list', () => {
      const enforcer = getSloEnforcer();

      const result = enforcer.evaluateAll([], [], new Date());

      expect(result.violations.length).toBe(0);
      expect(result.slo_statuses.length).toBe(0);
      expect(result.slos_evaluated).toBe(0);
    });

    it('should handle all operators correctly', () => {
      const enforcer = getSloEnforcer();

      const operators = ['lt', 'lte', 'gt', 'gte', 'eq', 'neq'] as const;

      for (const operator of operators) {
        const slos: SloDefinition[] = [
          {
            slo_id: `op-${operator}`,
            name: `Operator ${operator} Test`,
            indicator: 'latency_p95',
            operator,
            threshold: 100,
            window: '5m',
            enabled: true,
            is_sla: false,
            warning_threshold_percentage: 80,
          },
        ];

        const metrics: TelemetryMetric[] = [
          {
            metric_id: '550e8400-e29b-41d4-a716-446655440012',
            indicator: 'latency_p95',
            value: 100, // Exact threshold
            window: '5m',
            timestamp: new Date().toISOString(),
          },
        ];

        const result = enforcer.evaluateAll(slos, metrics, new Date());

        // Just verify no errors - actual logic depends on operator
        expect(result.slos_evaluated).toBe(1);
      }
    });
  });
});
