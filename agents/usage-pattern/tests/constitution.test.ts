/**
 * Constitutional Compliance Tests for Usage Pattern Agent.
 *
 * These tests verify that the agent strictly adheres to the
 * LLM-Observatory Agent Infrastructure Constitution (Prompt 0).
 *
 * CONSTITUTIONAL REQUIREMENTS:
 * 1. READ-ONLY operation
 * 2. ADVISORY classification
 * 3. No orchestration triggering
 * 4. No alert generation
 * 5. No policy/threshold enforcement
 * 6. Exactly ONE DecisionEvent per invocation
 * 7. All persistence through ruvector-service only
 */

import { describe, it, expect } from 'vitest';
import {
  UsagePatternDecisionEventSchema,
  AnalysisRequestSchema,
  UsagePatternAnalysisSchema,
  ErrorCodeSchema,
} from '../contracts/schemas.js';
import { AGENT_METADATA, getAgentRegistration } from '../src/index.js';
import {
  validateConstitutionalConstraints,
} from '../src/decision-emitter.js';
import { validateConfig, getDefaultConfig } from '../src/config.js';

describe('Constitutional Compliance', () => {
  describe('Prompt 0: Agent Infrastructure Constitution', () => {
    it('should be classified as observation-only', () => {
      expect(AGENT_METADATA.read_only).toBe(true);
      expect(AGENT_METADATA.classification).toBe('advisory');
    });

    it('should NOT trigger orchestration', () => {
      expect(AGENT_METADATA.non_responsibilities).toContain('orchestration_trigger');
    });

    it('should NOT trigger remediation', () => {
      expect(AGENT_METADATA.non_responsibilities).toContain('remediation_trigger');
    });

    it('should NOT trigger alerts directly', () => {
      expect(AGENT_METADATA.non_responsibilities).toContain('alert_generation');
    });

    it('should NOT modify routing', () => {
      expect(AGENT_METADATA.non_responsibilities).toContain('routing_modification');
    });

    it('should NOT modify policies', () => {
      expect(AGENT_METADATA.non_responsibilities).toContain('policy_modification');
    });

    it('should NOT modify thresholds at runtime', () => {
      expect(AGENT_METADATA.non_responsibilities).toContain('threshold_enforcement');
    });

    it('should be deployable as Google Edge Function', () => {
      expect(AGENT_METADATA.deployment.platform).toBe('google-cloud');
      expect(AGENT_METADATA.deployment.type).toBe('edge-function');
    });

    it('should be stateless at runtime', () => {
      const config = getDefaultConfig();
      // Configuration is loaded from environment, not persisted
      expect(config).toBeDefined();
      // No state storage fields
      expect((config as any).stateStorage).toBeUndefined();
      expect((config as any).persistentState).toBeUndefined();
    });
  });

  describe('Prompt 1: Agent Contract & Boundary Definition', () => {
    it('should be classified as ADVISORY', () => {
      expect(AGENT_METADATA.classification).toBe('advisory');
      expect(AGENT_METADATA.advisory).toBe(true);
    });

    it('should have decision_type of "usage_pattern_analysis"', () => {
      expect(AGENT_METADATA.decision_type).toBe('usage_pattern_analysis');
    });

    it('should have STATISTICAL confidence semantics', () => {
      expect(AGENT_METADATA.confidence_type).toBe('statistical');
    });

    it('should have empty constraints_applied', () => {
      expect(AGENT_METADATA.constraints_applied).toEqual([]);
    });

    it('should define all capabilities', () => {
      const expectedCapabilities = [
        'telemetry_aggregation',
        'trend_analysis',
        'seasonality_detection',
        'distribution_statistics',
        'provider_usage_breakdown',
        'hotspot_identification',
        'growth_pattern_analysis',
      ];

      for (const cap of expectedCapabilities) {
        expect(AGENT_METADATA.capabilities).toContain(cap);
      }
    });

    it('should define explicit non-responsibilities', () => {
      const requiredNonResponsibilities = [
        'failure_classification',
        'health_evaluation',
        'threshold_enforcement',
        'alert_generation',
        'orchestration_trigger',
      ];

      for (const nonResp of requiredNonResponsibilities) {
        expect(AGENT_METADATA.non_responsibilities).toContain(nonResp);
      }
    });

    it('should define primary consumers', () => {
      expect(AGENT_METADATA.primary_consumers).toContain('llm-analytics-hub');
      expect(AGENT_METADATA.primary_consumers).toContain('governance-dashboards');
      expect(AGENT_METADATA.primary_consumers).toContain('platform-usage-reporting');
    });

    it('should define CLI commands', () => {
      expect(AGENT_METADATA.cli_commands).toContain('analyze');
      expect(AGENT_METADATA.cli_commands).toContain('inspect');
      expect(AGENT_METADATA.cli_commands).toContain('replay');
      expect(AGENT_METADATA.cli_commands).toContain('status');
      expect(AGENT_METADATA.cli_commands).toContain('health');
    });
  });

  describe('Decision Event Schema Constraints', () => {
    it('should enforce agent_id literal', () => {
      const validEvent = createValidDecisionEvent();
      const result = UsagePatternDecisionEventSchema.safeParse(validEvent);
      expect(result.success).toBe(true);

      const invalidEvent = { ...validEvent, agent_id: 'wrong-agent' };
      const invalidResult = UsagePatternDecisionEventSchema.safeParse(invalidEvent);
      expect(invalidResult.success).toBe(false);
    });

    it('should enforce decision_type literal', () => {
      const validEvent = createValidDecisionEvent();
      const result = UsagePatternDecisionEventSchema.safeParse(validEvent);
      expect(result.success).toBe(true);

      const invalidEvent = { ...validEvent, decision_type: 'wrong_type' };
      const invalidResult = UsagePatternDecisionEventSchema.safeParse(invalidEvent);
      expect(invalidResult.success).toBe(false);
    });

    it('should enforce classification literal', () => {
      const validEvent = createValidDecisionEvent();
      const result = UsagePatternDecisionEventSchema.safeParse(validEvent);
      expect(result.success).toBe(true);

      const invalidEvent = { ...validEvent, classification: 'enforcement' };
      const invalidResult = UsagePatternDecisionEventSchema.safeParse(invalidEvent);
      expect(invalidResult.success).toBe(false);
    });

    it('should enforce empty constraints_applied', () => {
      const validEvent = createValidDecisionEvent();
      const result = UsagePatternDecisionEventSchema.safeParse(validEvent);
      expect(result.success).toBe(true);

      const invalidEvent = { ...validEvent, constraints_applied: ['some_constraint'] };
      const invalidResult = UsagePatternDecisionEventSchema.safeParse(invalidEvent);
      expect(invalidResult.success).toBe(false);
    });

    it('should require confidence between 0 and 1', () => {
      const validEvent = createValidDecisionEvent();
      const result = UsagePatternDecisionEventSchema.safeParse(validEvent);
      expect(result.success).toBe(true);

      const invalidEvent1 = { ...validEvent, confidence: -0.1 };
      expect(UsagePatternDecisionEventSchema.safeParse(invalidEvent1).success).toBe(false);

      const invalidEvent2 = { ...validEvent, confidence: 1.1 };
      expect(UsagePatternDecisionEventSchema.safeParse(invalidEvent2).success).toBe(false);
    });

    it('should require valid SHA256 inputs_hash', () => {
      const validEvent = createValidDecisionEvent();
      expect(validEvent.inputs_hash.length).toBe(64);
      expect(validEvent.inputs_hash).toMatch(/^[a-f0-9]{64}$/);
    });

    it('should require semantic version for agent_version', () => {
      const validEvent = createValidDecisionEvent();
      const result = UsagePatternDecisionEventSchema.safeParse(validEvent);
      expect(result.success).toBe(true);

      const invalidEvent = { ...validEvent, agent_version: 'v1.0' };
      const invalidResult = UsagePatternDecisionEventSchema.safeParse(invalidEvent);
      expect(invalidResult.success).toBe(false);
    });
  });

  describe('Constitutional Constraint Validation', () => {
    it('should validate compliant decision events', () => {
      const event = createValidDecisionEvent();
      const result = validateConstitutionalConstraints(event as any);
      expect(result.valid).toBe(true);
      expect(result.violations).toEqual([]);
    });

    it('should detect agent_id violations', () => {
      const event = { ...createValidDecisionEvent(), agent_id: 'wrong-agent' };
      const result = validateConstitutionalConstraints(event as any);
      expect(result.valid).toBe(false);
      expect(result.violations.some((v) => v.includes('agent_id'))).toBe(true);
    });

    it('should detect decision_type violations', () => {
      const event = { ...createValidDecisionEvent(), decision_type: 'wrong' };
      const result = validateConstitutionalConstraints(event as any);
      expect(result.valid).toBe(false);
      expect(result.violations.some((v) => v.includes('decision_type'))).toBe(true);
    });

    it('should detect classification violations', () => {
      const event = { ...createValidDecisionEvent(), classification: 'enforcement' };
      const result = validateConstitutionalConstraints(event as any);
      expect(result.valid).toBe(false);
      expect(result.violations.some((v) => v.includes('classification'))).toBe(true);
    });

    it('should detect non-empty constraints_applied', () => {
      const event = { ...createValidDecisionEvent(), constraints_applied: ['x'] };
      const result = validateConstitutionalConstraints(event as any);
      expect(result.valid).toBe(false);
      expect(result.violations.some((v) => v.includes('constraints_applied'))).toBe(true);
    });
  });

  describe('Configuration Validation', () => {
    it('should validate default configuration', () => {
      const config = getDefaultConfig();
      const errors = validateConfig(config);
      expect(errors).toEqual([]);
    });

    it('should reject invalid agent_version format', () => {
      const config = getDefaultConfig();
      config.agentVersion = 'invalid';
      const errors = validateConfig(config);
      expect(errors.some((e) => e.includes('semantic version'))).toBe(true);
    });

    it('should reject invalid confidence threshold', () => {
      const config = getDefaultConfig();
      config.confidenceThreshold = 1.5;
      const errors = validateConfig(config);
      expect(errors.some((e) => e.includes('confidenceThreshold'))).toBe(true);
    });
  });

  describe('Agent Registration', () => {
    it('should produce valid registration metadata', () => {
      const registration = getAgentRegistration();

      expect(registration.agent_id).toBe('usage-pattern-agent');
      expect(registration.classification).toBe('advisory');
      expect(registration.read_only).toBe(true);
      expect(registration.decision_type).toBe('usage_pattern_analysis');
      expect(registration.registered_at).toBeDefined();
    });
  });
});

describe('Prompt 4: Usage Pattern Agent Specific Requirements', () => {
  it('should analyze aggregated telemetry', () => {
    expect(AGENT_METADATA.capabilities).toContain('telemetry_aggregation');
  });

  it('should identify usage trends', () => {
    expect(AGENT_METADATA.capabilities).toContain('trend_analysis');
  });

  it('should detect seasonality', () => {
    expect(AGENT_METADATA.capabilities).toContain('seasonality_detection');
  });

  it('should compute usage distributions', () => {
    expect(AGENT_METADATA.capabilities).toContain('distribution_statistics');
  });

  it('should break down provider usage', () => {
    expect(AGENT_METADATA.capabilities).toContain('provider_usage_breakdown');
  });

  it('should identify hotspots', () => {
    expect(AGENT_METADATA.capabilities).toContain('hotspot_identification');
  });

  it('should analyze growth patterns', () => {
    expect(AGENT_METADATA.capabilities).toContain('growth_pattern_analysis');
  });

  it('should NOT classify failures', () => {
    expect(AGENT_METADATA.non_responsibilities).toContain('failure_classification');
  });

  it('should NOT evaluate health', () => {
    expect(AGENT_METADATA.non_responsibilities).toContain('health_evaluation');
  });

  it('should NOT enforce thresholds', () => {
    expect(AGENT_METADATA.non_responsibilities).toContain('threshold_enforcement');
  });

  it('should NOT generate alerts', () => {
    expect(AGENT_METADATA.non_responsibilities).toContain('alert_generation');
  });
});

// Helper function

function createValidDecisionEvent() {
  return {
    agent_id: 'usage-pattern-agent',
    agent_version: '1.0.0',
    decision_type: 'usage_pattern_analysis',
    confidence: 0.85,
    constraints_applied: [],
    classification: 'advisory',
    inputs_hash: 'a'.repeat(64),
    outputs: [
      {
        analysis_id: '550e8400-e29b-41d4-a716-446655440000',
        analyzed_at: '2025-01-19T00:00:00Z',
        time_window: {
          start: '2025-01-18T00:00:00Z',
          end: '2025-01-19T00:00:00Z',
          granularity: 'hour',
        },
        summary: {
          total_requests: 1000,
          total_tokens: 500000,
          total_cost_usd: 50.0,
          unique_users: 100,
          unique_sessions: 200,
          unique_providers: 3,
          unique_models: 5,
          error_rate: 0.02,
          avg_requests_per_user: 10,
        },
        time_series: [],
        distributions: {},
        provider_usage: [],
        hotspots: [],
        growth_patterns: [],
        overall_confidence: 0.85,
        sample_size: 1000,
        schema_version: '1.0.0',
      },
    ],
    execution_ref: 'exec-12345',
    timestamp: '2025-01-19T00:00:00Z',
    processing_metrics: {
      events_analyzed: 1000,
      processing_time_ms: 1500,
    },
  };
}
