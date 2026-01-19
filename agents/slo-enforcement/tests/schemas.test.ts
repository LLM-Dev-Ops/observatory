/**
 * SLO/SLA Enforcement Agent - Schema Tests
 *
 * Tests for contract schemas and validation.
 */

import { describe, it, expect } from 'vitest';
import {
  SloDefinitionSchema,
  TelemetryMetricSchema,
  SloEnforcementRequestSchema,
  DecisionEventSchema,
  SloViolationSchema,
  ViolationQuerySchema,
  AGENT_METADATA,
  validateSloEnforcementRequest,
  validateDecisionEvent,
} from '../contracts';

describe('Schemas', () => {
  describe('SloDefinitionSchema', () => {
    it('should validate a valid SLO definition', () => {
      const slo = {
        slo_id: 'latency-p95',
        name: 'P95 Latency SLO',
        indicator: 'latency_p95',
        operator: 'lt',
        threshold: 500,
        window: '5m',
        enabled: true,
        is_sla: false,
        warning_threshold_percentage: 80,
      };

      const result = SloDefinitionSchema.safeParse(slo);
      expect(result.success).toBe(true);
    });

    it('should reject invalid indicator', () => {
      const slo = {
        slo_id: 'test',
        name: 'Test',
        indicator: 'invalid_indicator',
        operator: 'lt',
        threshold: 100,
        window: '5m',
      };

      const result = SloDefinitionSchema.safeParse(slo);
      expect(result.success).toBe(false);
    });

    it('should reject invalid operator', () => {
      const slo = {
        slo_id: 'test',
        name: 'Test',
        indicator: 'latency_p95',
        operator: 'invalid',
        threshold: 100,
        window: '5m',
      };

      const result = SloDefinitionSchema.safeParse(slo);
      expect(result.success).toBe(false);
    });

    it('should apply defaults for optional fields', () => {
      const slo = {
        slo_id: 'test',
        name: 'Test',
        indicator: 'latency_p95',
        operator: 'lt',
        threshold: 100,
        window: '5m',
      };

      const result = SloDefinitionSchema.safeParse(slo);
      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data.enabled).toBe(true);
        expect(result.data.is_sla).toBe(false);
        expect(result.data.warning_threshold_percentage).toBe(80);
      }
    });
  });

  describe('TelemetryMetricSchema', () => {
    it('should validate a valid metric', () => {
      const metric = {
        metric_id: '550e8400-e29b-41d4-a716-446655440000',
        indicator: 'latency_p95',
        value: 250.5,
        window: '5m',
        timestamp: new Date().toISOString(),
        sample_count: 100,
      };

      const result = TelemetryMetricSchema.safeParse(metric);
      expect(result.success).toBe(true);
    });

    it('should reject invalid UUID', () => {
      const metric = {
        metric_id: 'not-a-uuid',
        indicator: 'latency_p95',
        value: 250.5,
        window: '5m',
        timestamp: new Date().toISOString(),
      };

      const result = TelemetryMetricSchema.safeParse(metric);
      expect(result.success).toBe(false);
    });

    it('should reject invalid timestamp', () => {
      const metric = {
        metric_id: '550e8400-e29b-41d4-a716-446655440000',
        indicator: 'latency_p95',
        value: 250.5,
        window: '5m',
        timestamp: 'not-a-timestamp',
      };

      const result = TelemetryMetricSchema.safeParse(metric);
      expect(result.success).toBe(false);
    });
  });

  describe('DecisionEventSchema', () => {
    it('should validate a valid decision event', () => {
      const event = {
        agent_id: AGENT_METADATA.id,
        agent_version: '1.0.0',
        decision_type: AGENT_METADATA.decision_type,
        inputs_hash: 'a'.repeat(64),
        outputs: {
          violations: [],
          slo_statuses: [],
          metrics_evaluated: 0,
          slos_evaluated: 0,
        },
        confidence: 0.95,
        constraints_applied: [],
        execution_ref: '550e8400-e29b-41d4-a716-446655440000',
        timestamp: new Date().toISOString(),
      };

      const result = DecisionEventSchema.safeParse(event);
      expect(result.success).toBe(true);
    });

    it('should reject wrong agent_id', () => {
      const event = {
        agent_id: 'wrong-agent-id',
        agent_version: '1.0.0',
        decision_type: AGENT_METADATA.decision_type,
        inputs_hash: 'a'.repeat(64),
        outputs: {
          violations: [],
          slo_statuses: [],
          metrics_evaluated: 0,
          slos_evaluated: 0,
        },
        confidence: 0.95,
        constraints_applied: [],
        execution_ref: '550e8400-e29b-41d4-a716-446655440000',
        timestamp: new Date().toISOString(),
      };

      const result = DecisionEventSchema.safeParse(event);
      expect(result.success).toBe(false);
    });

    it('should reject wrong decision_type', () => {
      const event = {
        agent_id: AGENT_METADATA.id,
        agent_version: '1.0.0',
        decision_type: 'wrong_decision_type',
        inputs_hash: 'a'.repeat(64),
        outputs: {
          violations: [],
          slo_statuses: [],
          metrics_evaluated: 0,
          slos_evaluated: 0,
        },
        confidence: 0.95,
        constraints_applied: [],
        execution_ref: '550e8400-e29b-41d4-a716-446655440000',
        timestamp: new Date().toISOString(),
      };

      const result = DecisionEventSchema.safeParse(event);
      expect(result.success).toBe(false);
    });

    it('should reject non-empty constraints_applied (constitutional violation)', () => {
      const event = {
        agent_id: AGENT_METADATA.id,
        agent_version: '1.0.0',
        decision_type: AGENT_METADATA.decision_type,
        inputs_hash: 'a'.repeat(64),
        outputs: {
          violations: [],
          slo_statuses: [],
          metrics_evaluated: 0,
          slos_evaluated: 0,
        },
        confidence: 0.95,
        constraints_applied: ['some-constraint'], // CONSTITUTIONAL VIOLATION
        execution_ref: '550e8400-e29b-41d4-a716-446655440000',
        timestamp: new Date().toISOString(),
      };

      const result = DecisionEventSchema.safeParse(event);
      expect(result.success).toBe(false);
    });

    it('should reject invalid inputs_hash', () => {
      const event = {
        agent_id: AGENT_METADATA.id,
        agent_version: '1.0.0',
        decision_type: AGENT_METADATA.decision_type,
        inputs_hash: 'not-a-valid-sha256',
        outputs: {
          violations: [],
          slo_statuses: [],
          metrics_evaluated: 0,
          slos_evaluated: 0,
        },
        confidence: 0.95,
        constraints_applied: [],
        execution_ref: '550e8400-e29b-41d4-a716-446655440000',
        timestamp: new Date().toISOString(),
      };

      const result = DecisionEventSchema.safeParse(event);
      expect(result.success).toBe(false);
    });

    it('should reject confidence out of range', () => {
      const event = {
        agent_id: AGENT_METADATA.id,
        agent_version: '1.0.0',
        decision_type: AGENT_METADATA.decision_type,
        inputs_hash: 'a'.repeat(64),
        outputs: {
          violations: [],
          slo_statuses: [],
          metrics_evaluated: 0,
          slos_evaluated: 0,
        },
        confidence: 1.5, // Out of range
        constraints_applied: [],
        execution_ref: '550e8400-e29b-41d4-a716-446655440000',
        timestamp: new Date().toISOString(),
      };

      const result = DecisionEventSchema.safeParse(event);
      expect(result.success).toBe(false);
    });
  });

  describe('SloViolationSchema', () => {
    it('should validate a valid violation', () => {
      const violation = {
        violation_id: '550e8400-e29b-41d4-a716-446655440000',
        slo_id: 'latency-p95',
        slo_name: 'P95 Latency SLO',
        breach_type: 'slo_breach',
        severity: 'high',
        indicator: 'latency_p95',
        metric_context: {
          current_value: 750,
          threshold_value: 500,
          deviation_percentage: 50,
          trend: 'stable',
          samples_in_window: 100,
          previous_breaches_in_window: 0,
        },
        is_sla: false,
        detected_at: new Date().toISOString(),
        window: '5m',
      };

      const result = SloViolationSchema.safeParse(violation);
      expect(result.success).toBe(true);
    });
  });

  describe('ViolationQuerySchema', () => {
    it('should apply defaults', () => {
      const query = {};
      const result = ViolationQuerySchema.safeParse(query);

      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data.limit).toBe(100);
        expect(result.data.offset).toBe(0);
        expect(result.data.sort_by).toBe('detected_at');
        expect(result.data.sort_order).toBe('desc');
      }
    });

    it('should reject limit over maximum', () => {
      const query = { limit: 5000 };
      const result = ViolationQuerySchema.safeParse(query);
      expect(result.success).toBe(false);
    });
  });

  describe('Validation helpers', () => {
    it('validateSloEnforcementRequest should return errors on invalid input', () => {
      const result = validateSloEnforcementRequest({});
      expect(result.success).toBe(false);
      expect(result.errors).toBeDefined();
    });

    it('validateDecisionEvent should return errors on invalid input', () => {
      const result = validateDecisionEvent({});
      expect(result.success).toBe(false);
      expect(result.errors).toBeDefined();
    });
  });
});
