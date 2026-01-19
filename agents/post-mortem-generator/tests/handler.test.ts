/**
 * Post-Mortem Generator Agent - Handler Tests
 *
 * Verification tests for constitutional compliance and core functionality.
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import {
  PostMortemRequestSchema,
  DecisionEventSchema,
  PostMortemReportSchema,
  AGENT_METADATA,
  type PostMortemRequest,
} from '../contracts/schemas.js';
import { generatePostMortem, type GeneratorInput } from '../src/generator.js';
import {
  createDecisionEvent,
  validateDecisionEventCompliance,
  ConstitutionalViolationError,
  verifyConstitutionalCompliance,
} from '../src/emitter.js';
import type {
  AggregatedFailureData,
  AggregatedHealthData,
  AggregatedTelemetryData,
} from '../src/types/ruvector.js';

// =============================================================================
// TEST DATA
// =============================================================================

function createMockFailureData(): AggregatedFailureData {
  return {
    total_failures: 50,
    by_category: new Map([
      ['provider_rate_limit', 25],
      ['network_timeout', 15],
      ['request_invalid_payload', 10],
    ]),
    by_severity: new Map([
      ['high', 20],
      ['medium', 25],
      ['low', 5],
    ]),
    by_cause: new Map([
      ['provider', 30],
      ['network', 15],
      ['client', 5],
    ]),
    by_provider: new Map([
      ['openai', { count: 30, models: new Set(['gpt-4', 'gpt-3.5-turbo']), first_occurrence: '2024-01-01T00:00:00Z', last_occurrence: '2024-01-01T01:00:00Z' }],
      ['anthropic', { count: 20, models: new Set(['claude-3-opus']), first_occurrence: '2024-01-01T00:30:00Z', last_occurrence: '2024-01-01T01:00:00Z' }],
    ]),
    time_series: [
      { timestamp: '2024-01-01T00:00:00Z', count: 10, error_rate: 0.1 },
      { timestamp: '2024-01-01T00:30:00Z', count: 30, error_rate: 0.3 },
      { timestamp: '2024-01-01T01:00:00Z', count: 10, error_rate: 0.1 },
    ],
  };
}

function createMockHealthData(): AggregatedHealthData {
  return {
    health_transitions: [
      {
        timestamp: '2024-01-01T00:15:00Z',
        target_id: 'api-service',
        target_type: 'service',
        from_state: 'healthy',
        to_state: 'degraded',
        duration_in_previous_state_ms: 900000,
      },
      {
        timestamp: '2024-01-01T00:45:00Z',
        target_id: 'api-service',
        target_type: 'service',
        from_state: 'degraded',
        to_state: 'unhealthy',
        duration_in_previous_state_ms: 1800000,
      },
      {
        timestamp: '2024-01-01T01:15:00Z',
        target_id: 'api-service',
        target_type: 'service',
        from_state: 'unhealthy',
        to_state: 'healthy',
        duration_in_previous_state_ms: 1800000,
      },
    ],
    state_durations: new Map([
      ['healthy', 3600000],
      ['degraded', 1800000],
      ['unhealthy', 1800000],
    ]),
    current_states: new Map([
      ['api-service', 'healthy'],
    ]),
  };
}

function createMockTelemetryData(): AggregatedTelemetryData {
  return {
    total_requests: 500,
    total_errors: 50,
    error_rate: 0.1,
    latency_stats: {
      min_ms: 50,
      max_ms: 5000,
      avg_ms: 500,
      p50_ms: 400,
      p95_ms: 2000,
      p99_ms: 4000,
    },
    by_provider: new Map([
      ['openai', { request_count: 300, error_count: 30, models: new Set(['gpt-4']) }],
      ['anthropic', { request_count: 200, error_count: 20, models: new Set(['claude-3-opus']) }],
    ]),
    peak_error_rate: {
      value: 0.35,
      timestamp: '2024-01-01T00:30:00Z',
    },
  };
}

function createMockRequest(): PostMortemRequest {
  return {
    time_range: {
      start_time: '2024-01-01T00:00:00Z',
      end_time: '2024-01-01T02:00:00Z',
    },
    options: {
      include_timeline: true,
      include_classification_breakdown: true,
      include_health_transitions: true,
      include_contributing_factors: true,
      include_statistics: true,
    },
    incident_id: 'INC-2024-001',
  };
}

function createMockGeneratorInput(): GeneratorInput {
  return {
    request: createMockRequest(),
    failureData: createMockFailureData(),
    healthData: createMockHealthData(),
    telemetryData: createMockTelemetryData(),
    failureClassifications: [],
    healthEvaluations: [],
  };
}

// =============================================================================
// SCHEMA VALIDATION TESTS
// =============================================================================

describe('Schema Validation', () => {
  describe('PostMortemRequestSchema', () => {
    it('should validate a valid request', () => {
      const request = createMockRequest();
      const result = PostMortemRequestSchema.safeParse(request);
      expect(result.success).toBe(true);
    });

    it('should reject request without time_range', () => {
      const request = { options: {} };
      const result = PostMortemRequestSchema.safeParse(request);
      expect(result.success).toBe(false);
    });

    it('should reject invalid datetime format', () => {
      const request = {
        time_range: {
          start_time: 'invalid-date',
          end_time: '2024-01-01T00:00:00Z',
        },
      };
      const result = PostMortemRequestSchema.safeParse(request);
      expect(result.success).toBe(false);
    });
  });

  describe('PostMortemReportSchema', () => {
    it('should validate a generated report', () => {
      const input = createMockGeneratorInput();
      const result = generatePostMortem(input);
      const parseResult = PostMortemReportSchema.safeParse(result.report);
      expect(parseResult.success).toBe(true);
    });
  });
});

// =============================================================================
// GENERATOR TESTS
// =============================================================================

describe('Post-Mortem Generator', () => {
  it('should generate a complete report', () => {
    const input = createMockGeneratorInput();
    const result = generatePostMortem(input);

    expect(result.report).toBeDefined();
    expect(result.report.report_id).toBeDefined();
    expect(result.report.generated_at).toBeDefined();
    expect(result.report.time_range).toEqual(input.request.time_range);
    expect(result.report.incident_id).toBe('INC-2024-001');
  });

  it('should include summary', () => {
    const input = createMockGeneratorInput();
    const result = generatePostMortem(input);

    expect(result.report.summary).toBeDefined();
    expect(result.report.summary.title).toBeDefined();
    expect(result.report.summary.description).toBeDefined();
    expect(result.report.summary.impact_level).toBeDefined();
    expect(result.report.summary.status).toBeDefined();
  });

  it('should include classification breakdown when requested', () => {
    const input = createMockGeneratorInput();
    const result = generatePostMortem(input);

    expect(result.report.classification_breakdown).toBeDefined();
    expect(result.report.classification_breakdown?.by_category).toBeDefined();
    expect(result.report.classification_breakdown?.by_severity).toBeDefined();
    expect(result.report.classification_breakdown?.by_cause).toBeDefined();
    expect(result.report.classification_breakdown?.by_provider).toBeDefined();
  });

  it('should include health transitions when requested', () => {
    const input = createMockGeneratorInput();
    const result = generatePostMortem(input);

    expect(result.report.health_transitions).toBeDefined();
    expect(result.report.health_transitions?.length).toBeGreaterThan(0);
  });

  it('should include contributing factors when requested', () => {
    const input = createMockGeneratorInput();
    const result = generatePostMortem(input);

    expect(result.report.contributing_factors).toBeDefined();
    expect(result.report.contributing_factors?.length).toBeGreaterThan(0);
  });

  it('should include statistics when requested', () => {
    const input = createMockGeneratorInput();
    const result = generatePostMortem(input);

    expect(result.report.statistics).toBeDefined();
    expect(result.report.statistics?.total_failures).toBe(50);
    expect(result.report.statistics?.total_requests).toBe(500);
    expect(result.report.statistics?.error_rate).toBe(0.1);
  });

  it('should include data quality assessment', () => {
    const input = createMockGeneratorInput();
    const result = generatePostMortem(input);

    expect(result.report.data_quality).toBeDefined();
    expect(result.report.data_quality?.completeness).toBeGreaterThan(0);
    expect(result.report.data_quality?.completeness).toBeLessThanOrEqual(1);
  });

  it('should calculate confidence based on data quality', () => {
    const input = createMockGeneratorInput();
    const result = generatePostMortem(input);

    expect(result.confidence).toBeGreaterThan(0);
    expect(result.confidence).toBeLessThanOrEqual(1);
  });

  it('should skip sections when not requested', () => {
    const input = createMockGeneratorInput();
    input.request.options = {
      include_timeline: false,
      include_classification_breakdown: false,
      include_health_transitions: false,
      include_contributing_factors: false,
      include_statistics: false,
    };

    const result = generatePostMortem(input);

    expect(result.report.timeline).toBeUndefined();
    expect(result.report.classification_breakdown).toBeUndefined();
    expect(result.report.health_transitions).toBeUndefined();
    expect(result.report.contributing_factors).toBeUndefined();
    expect(result.report.statistics).toBeUndefined();
  });
});

// =============================================================================
// DECISION EVENT TESTS
// =============================================================================

describe('Decision Event', () => {
  it('should create a valid decision event', () => {
    const input = createMockGeneratorInput();
    const generatorResult = generatePostMortem(input);

    const decisionEvent = createDecisionEvent({
      request: input.request,
      reports: [generatorResult.report],
      confidence: generatorResult.confidence,
      executionRef: 'test-execution-123',
    });

    const parseResult = DecisionEventSchema.safeParse(decisionEvent);
    expect(parseResult.success).toBe(true);
  });

  it('should have correct agent metadata', () => {
    const input = createMockGeneratorInput();
    const generatorResult = generatePostMortem(input);

    const decisionEvent = createDecisionEvent({
      request: input.request,
      reports: [generatorResult.report],
      confidence: generatorResult.confidence,
      executionRef: 'test-execution-123',
    });

    expect(decisionEvent.agent_id).toBe(AGENT_METADATA.id);
    expect(decisionEvent.agent_version).toBe(AGENT_METADATA.version);
    expect(decisionEvent.decision_type).toBe('postmortem_generation');
  });

  it('should have empty constraints_applied (constitutional requirement)', () => {
    const input = createMockGeneratorInput();
    const generatorResult = generatePostMortem(input);

    const decisionEvent = createDecisionEvent({
      request: input.request,
      reports: [generatorResult.report],
      confidence: generatorResult.confidence,
      executionRef: 'test-execution-123',
    });

    expect(decisionEvent.constraints_applied).toEqual([]);
    expect(decisionEvent.constraints_applied.length).toBe(0);
  });

  it('should have valid inputs_hash', () => {
    const input = createMockGeneratorInput();
    const generatorResult = generatePostMortem(input);

    const decisionEvent = createDecisionEvent({
      request: input.request,
      reports: [generatorResult.report],
      confidence: generatorResult.confidence,
      executionRef: 'test-execution-123',
    });

    expect(decisionEvent.inputs_hash).toMatch(/^[a-f0-9]{64}$/);
  });

  it('should produce deterministic inputs_hash for same input', () => {
    const input = createMockGeneratorInput();
    const generatorResult = generatePostMortem(input);

    const event1 = createDecisionEvent({
      request: input.request,
      reports: [generatorResult.report],
      confidence: generatorResult.confidence,
      executionRef: 'test-1',
    });

    const event2 = createDecisionEvent({
      request: input.request,
      reports: [generatorResult.report],
      confidence: generatorResult.confidence,
      executionRef: 'test-2',
    });

    expect(event1.inputs_hash).toBe(event2.inputs_hash);
  });
});

// =============================================================================
// CONSTITUTIONAL COMPLIANCE TESTS
// =============================================================================

describe('Constitutional Compliance', () => {
  describe('validateDecisionEventCompliance', () => {
    it('should pass for valid decision event', () => {
      const input = createMockGeneratorInput();
      const generatorResult = generatePostMortem(input);

      const decisionEvent = createDecisionEvent({
        request: input.request,
        reports: [generatorResult.report],
        confidence: generatorResult.confidence,
        executionRef: 'test-execution-123',
      });

      const compliance = validateDecisionEventCompliance(decisionEvent);
      expect(compliance.valid).toBe(true);
      expect(compliance.violations).toHaveLength(0);
    });

    it('should fail for wrong agent_id', () => {
      const input = createMockGeneratorInput();
      const generatorResult = generatePostMortem(input);

      const decisionEvent = createDecisionEvent({
        request: input.request,
        reports: [generatorResult.report],
        confidence: generatorResult.confidence,
        executionRef: 'test-execution-123',
      });

      // Mutate for testing
      (decisionEvent as any).agent_id = 'wrong-agent';

      const compliance = validateDecisionEventCompliance(decisionEvent as any);
      expect(compliance.valid).toBe(false);
      expect(compliance.violations.some((v) => v.includes('agent_id'))).toBe(true);
    });

    it('should fail for non-empty constraints_applied', () => {
      const input = createMockGeneratorInput();
      const generatorResult = generatePostMortem(input);

      const decisionEvent = createDecisionEvent({
        request: input.request,
        reports: [generatorResult.report],
        confidence: generatorResult.confidence,
        executionRef: 'test-execution-123',
      });

      // Mutate for testing
      (decisionEvent as any).constraints_applied = ['some-constraint'];

      const compliance = validateDecisionEventCompliance(decisionEvent as any);
      expect(compliance.valid).toBe(false);
      expect(compliance.violations.some((v) => v.includes('CONSTITUTIONAL VIOLATION'))).toBe(true);
    });
  });

  describe('verifyConstitutionalCompliance', () => {
    it('should allow read-only operations', () => {
      expect(() => {
        verifyConstitutionalCompliance('read_data', {});
      }).not.toThrow();

      expect(() => {
        verifyConstitutionalCompliance('generate_report', {});
      }).not.toThrow();
    });

    it('should reject trigger_alert', () => {
      expect(() => {
        verifyConstitutionalCompliance('trigger_alert', {});
      }).toThrow(ConstitutionalViolationError);
    });

    it('should reject initiate_remediation', () => {
      expect(() => {
        verifyConstitutionalCompliance('initiate_remediation', {});
      }).toThrow(ConstitutionalViolationError);
    });

    it('should reject modify_state', () => {
      expect(() => {
        verifyConstitutionalCompliance('modify_state', {});
      }).toThrow(ConstitutionalViolationError);
    });

    it('should reject execute_sql', () => {
      expect(() => {
        verifyConstitutionalCompliance('execute_sql', {});
      }).toThrow(ConstitutionalViolationError);
    });

    it('should reject invoke_agent', () => {
      expect(() => {
        verifyConstitutionalCompliance('invoke_agent', {});
      }).toThrow(ConstitutionalViolationError);
    });

    it('should reject recommend_remediation', () => {
      expect(() => {
        verifyConstitutionalCompliance('recommend_remediation', {});
      }).toThrow(ConstitutionalViolationError);
    });
  });
});

// =============================================================================
// AGENT METADATA TESTS
// =============================================================================

describe('Agent Metadata', () => {
  it('should have correct agent ID', () => {
    expect(AGENT_METADATA.id).toBe('post-mortem-generator-agent');
  });

  it('should have correct version format', () => {
    expect(AGENT_METADATA.version).toMatch(/^\d+\.\d+\.\d+$/);
  });

  it('should be classified as READ-ONLY', () => {
    expect(AGENT_METADATA.classification.type).toBe('READ-ONLY');
  });

  it('should be classified as ANALYTICAL', () => {
    expect(AGENT_METADATA.classification.subtype).toBe('ANALYTICAL');
  });

  it('should not be enforcement class', () => {
    expect(AGENT_METADATA.classification.enforcement).toBe(false);
  });

  it('should not be advisory class', () => {
    expect(AGENT_METADATA.classification.advisory).toBe(false);
  });

  it('should have correct decision_type', () => {
    expect(AGENT_METADATA.decision_type).toBe('postmortem_generation');
  });
});

// =============================================================================
// EDGE CASES
// =============================================================================

describe('Edge Cases', () => {
  it('should handle empty failure data', () => {
    const input = createMockGeneratorInput();
    input.failureData = {
      total_failures: 0,
      by_category: new Map(),
      by_severity: new Map(),
      by_cause: new Map(),
      by_provider: new Map(),
      time_series: [],
    };

    const result = generatePostMortem(input);
    expect(result.report).toBeDefined();
    expect(result.report.statistics?.total_failures).toBe(0);
  });

  it('should handle empty health data', () => {
    const input = createMockGeneratorInput();
    input.healthData = {
      health_transitions: [],
      state_durations: new Map(),
      current_states: new Map(),
    };

    const result = generatePostMortem(input);
    expect(result.report).toBeDefined();
    expect(result.report.health_transitions).toEqual([]);
  });

  it('should handle empty telemetry data', () => {
    const input = createMockGeneratorInput();
    input.telemetryData = {
      total_requests: 0,
      total_errors: 0,
      error_rate: 0,
      latency_stats: {
        min_ms: 0,
        max_ms: 0,
        avg_ms: 0,
        p50_ms: 0,
        p95_ms: 0,
        p99_ms: 0,
      },
      by_provider: new Map(),
      peak_error_rate: {
        value: 0,
        timestamp: new Date().toISOString(),
      },
    };

    const result = generatePostMortem(input);
    expect(result.report).toBeDefined();
    expect(result.report.statistics?.total_requests).toBe(0);
  });

  it('should have lower confidence with incomplete data', () => {
    const input = createMockGeneratorInput();
    input.failureData.total_failures = 0;
    input.healthData.health_transitions = [];
    input.telemetryData.total_requests = 0;

    const result = generatePostMortem(input);
    expect(result.confidence).toBeLessThan(0.5);
  });
});
