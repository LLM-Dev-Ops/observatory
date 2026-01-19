/**
 * Failure Classification Agent - Contract Tests
 *
 * These tests validate the contract schemas and constitutional constraints.
 */

import {
  // Schemas
  FailureEventSchema,
  BatchClassificationRequestSchema,
  FailureClassificationSchema,
  BatchClassificationResultSchema,
  DecisionEventSchema,
  ClassificationQuerySchema,
  AnalysisQuerySchema,

  // Validation functions
  validateFailureEvent,
  validateBatchRequest,
  validateClassification,
  validateBatchResult,
  validateDecisionEvent,
  validateConstitutionalOperation,
  assertConstitutionalCompliance,
  hashInput,
  hashInputs,

  // Error classes
  ConstitutionalViolationError,
  ValidationError,

  // Metadata
  AGENT_METADATA,
} from '../contracts';

// =============================================================================
// TEST DATA
// =============================================================================

const validFailureEvent = {
  span_id: 'span-123',
  trace_id: 'trace-456',
  provider: 'openai' as const,
  model: 'gpt-4',
  status: 'ERROR' as const,
  error: {
    code: 'ETIMEDOUT',
    message: 'Request timed out after 30000ms',
    http_status: 504,
  },
  latency: {
    start_time: '2024-01-01T00:00:00.000Z',
    end_time: '2024-01-01T00:00:30.000Z',
    duration_ms: 30000,
  },
  timestamp: '2024-01-01T00:00:30.000Z',
};

const validClassification = {
  span_id: 'span-123',
  trace_id: 'trace-456',
  category: 'network_timeout' as const,
  severity: 'high' as const,
  cause: 'network' as const,
  confidence: 0.95,
  confidence_factors: ['network_timeout_001'],
  classification_signals: [
    {
      signal_type: 'rule:network_timeout_001:error.code',
      signal_value: 'ETIMEDOUT',
      weight: 1.0,
    },
  ],
  recommendations: ['Check network connectivity to provider'],
  classified_at: '2024-01-01T00:00:30.100Z',
  classification_latency_ms: 100,
  schema_version: '1.0.0',
};

const validDecisionEvent = {
  agent_id: 'failure-classification-agent' as const,
  agent_version: '1.0.0',
  decision_type: 'failure_classification' as const,
  inputs_hash: 'a'.repeat(64),
  outputs: [validClassification],
  confidence: 0.95,
  constraints_applied: [] as never[],
  execution_ref: 'exec-789',
  timestamp: '2024-01-01T00:00:30.100Z',
};

// =============================================================================
// SCHEMA VALIDATION TESTS
// =============================================================================

describe('FailureEventSchema', () => {
  it('should validate a valid failure event', () => {
    const result = FailureEventSchema.safeParse(validFailureEvent);
    expect(result.success).toBe(true);
  });

  it('should reject event with non-ERROR status', () => {
    const result = FailureEventSchema.safeParse({
      ...validFailureEvent,
      status: 'OK',
    });
    expect(result.success).toBe(false);
  });

  it('should reject event without span_id', () => {
    const { span_id, ...eventWithoutSpanId } = validFailureEvent;
    const result = FailureEventSchema.safeParse(eventWithoutSpanId);
    expect(result.success).toBe(false);
  });

  it('should reject event with invalid provider', () => {
    const result = FailureEventSchema.safeParse({
      ...validFailureEvent,
      provider: 'invalid_provider',
    });
    expect(result.success).toBe(false);
  });

  it('should apply default values', () => {
    const result = FailureEventSchema.safeParse(validFailureEvent);
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.metadata).toEqual({});
      expect(result.data.events).toEqual([]);
      expect(result.data.attributes).toEqual({});
    }
  });
});

describe('FailureClassificationSchema', () => {
  it('should validate a valid classification', () => {
    const result = FailureClassificationSchema.safeParse(validClassification);
    expect(result.success).toBe(true);
  });

  it('should reject classification with invalid category', () => {
    const result = FailureClassificationSchema.safeParse({
      ...validClassification,
      category: 'invalid_category',
    });
    expect(result.success).toBe(false);
  });

  it('should reject classification with confidence > 1', () => {
    const result = FailureClassificationSchema.safeParse({
      ...validClassification,
      confidence: 1.5,
    });
    expect(result.success).toBe(false);
  });

  it('should reject classification with negative latency', () => {
    const result = FailureClassificationSchema.safeParse({
      ...validClassification,
      classification_latency_ms: -1,
    });
    expect(result.success).toBe(false);
  });
});

describe('DecisionEventSchema', () => {
  it('should validate a valid decision event', () => {
    const result = DecisionEventSchema.safeParse(validDecisionEvent);
    expect(result.success).toBe(true);
  });

  it('should reject decision event with wrong agent_id', () => {
    const result = DecisionEventSchema.safeParse({
      ...validDecisionEvent,
      agent_id: 'wrong-agent-id',
    });
    expect(result.success).toBe(false);
  });

  it('should reject decision event with wrong decision_type', () => {
    const result = DecisionEventSchema.safeParse({
      ...validDecisionEvent,
      decision_type: 'wrong_type',
    });
    expect(result.success).toBe(false);
  });

  it('should reject decision event with non-empty constraints_applied', () => {
    const result = DecisionEventSchema.safeParse({
      ...validDecisionEvent,
      constraints_applied: ['some_constraint'],
    });
    expect(result.success).toBe(false);
  });

  it('should reject decision event with invalid version format', () => {
    const result = DecisionEventSchema.safeParse({
      ...validDecisionEvent,
      agent_version: 'v1.0',
    });
    expect(result.success).toBe(false);
  });

  it('should reject decision event with wrong hash length', () => {
    const result = DecisionEventSchema.safeParse({
      ...validDecisionEvent,
      inputs_hash: 'short_hash',
    });
    expect(result.success).toBe(false);
  });
});

// =============================================================================
// VALIDATION FUNCTION TESTS
// =============================================================================

describe('validateFailureEvent', () => {
  it('should return success for valid event', () => {
    const result = validateFailureEvent(validFailureEvent);
    expect(result.success).toBe(true);
    expect(result.data).toBeDefined();
  });

  it('should return errors for invalid event', () => {
    const result = validateFailureEvent({ invalid: 'data' });
    expect(result.success).toBe(false);
    expect(result.errors).toBeDefined();
    expect(result.errors!.length).toBeGreaterThan(0);
  });
});

describe('validateDecisionEvent', () => {
  it('should return success for valid decision event', () => {
    const result = validateDecisionEvent(validDecisionEvent);
    expect(result.success).toBe(true);
  });

  it('should enforce constitutional constraints on agent_id', () => {
    const result = validateDecisionEvent({
      ...validDecisionEvent,
      agent_id: 'wrong-agent',
    });
    expect(result.success).toBe(false);
    expect(result.errors![0].code).toBe('invalid_literal');
  });

  it('should enforce constitutional constraints on constraints_applied', () => {
    const result = validateDecisionEvent({
      ...validDecisionEvent,
      constraints_applied: ['constraint'],
    });
    expect(result.success).toBe(false);
  });
});

// =============================================================================
// CONSTITUTIONAL CONSTRAINT TESTS
// =============================================================================

describe('validateConstitutionalOperation', () => {
  it('should allow read operations', () => {
    const result = validateConstitutionalOperation('read_data');
    expect(result.success).toBe(true);
  });

  it('should reject sql_execute', () => {
    const result = validateConstitutionalOperation('sql_execute');
    expect(result.success).toBe(false);
    expect(result.errors![0].code).toBe('constitutional_violation');
  });

  it('should reject orchestration_trigger', () => {
    const result = validateConstitutionalOperation('orchestration_trigger');
    expect(result.success).toBe(false);
  });

  it('should reject alert_trigger', () => {
    const result = validateConstitutionalOperation('alert_trigger');
    expect(result.success).toBe(false);
  });

  it('should reject remediation_trigger', () => {
    const result = validateConstitutionalOperation('remediation_trigger');
    expect(result.success).toBe(false);
  });
});

describe('assertConstitutionalCompliance', () => {
  it('should pass for read-only operations', () => {
    expect(() => {
      assertConstitutionalCompliance({
        operation: 'classify_failure',
        modifiesState: false,
        triggersAction: false,
        accessesStorage: 'none',
      });
    }).not.toThrow();
  });

  it('should throw for state modification', () => {
    expect(() => {
      assertConstitutionalCompliance({
        operation: 'modify_state',
        modifiesState: true,
        triggersAction: false,
        accessesStorage: 'none',
      });
    }).toThrow(ConstitutionalViolationError);
  });

  it('should throw for action triggering', () => {
    expect(() => {
      assertConstitutionalCompliance({
        operation: 'trigger_action',
        modifiesState: false,
        triggersAction: true,
        accessesStorage: 'none',
      });
    }).toThrow(ConstitutionalViolationError);
  });

  it('should throw for direct storage writes', () => {
    expect(() => {
      assertConstitutionalCompliance({
        operation: 'write_storage',
        modifiesState: false,
        triggersAction: false,
        accessesStorage: 'write',
      });
    }).toThrow(ConstitutionalViolationError);
  });

  it('should throw for prohibited operations', () => {
    expect(() => {
      assertConstitutionalCompliance({
        operation: 'sql_execute',
        modifiesState: false,
        triggersAction: false,
        accessesStorage: 'none',
      });
    }).toThrow(ConstitutionalViolationError);
  });
});

// =============================================================================
// HASHING TESTS
// =============================================================================

describe('hashInput', () => {
  it('should produce consistent hashes', () => {
    const hash1 = hashInput(validFailureEvent);
    const hash2 = hashInput(validFailureEvent);
    expect(hash1).toBe(hash2);
  });

  it('should produce 64-character hex hash', () => {
    const hash = hashInput(validFailureEvent);
    expect(hash.length).toBe(64);
    expect(/^[a-f0-9]+$/.test(hash)).toBe(true);
  });

  it('should produce different hashes for different inputs', () => {
    const hash1 = hashInput(validFailureEvent);
    const hash2 = hashInput({ ...validFailureEvent, span_id: 'different' });
    expect(hash1).not.toBe(hash2);
  });
});

describe('hashInputs', () => {
  it('should produce consistent hashes for arrays', () => {
    const hash1 = hashInputs([validFailureEvent, validFailureEvent]);
    const hash2 = hashInputs([validFailureEvent, validFailureEvent]);
    expect(hash1).toBe(hash2);
  });

  it('should produce different hashes for different arrays', () => {
    const hash1 = hashInputs([validFailureEvent]);
    const hash2 = hashInputs([validFailureEvent, validFailureEvent]);
    expect(hash1).not.toBe(hash2);
  });
});

// =============================================================================
// AGENT METADATA TESTS
// =============================================================================

describe('AGENT_METADATA', () => {
  it('should have correct agent id', () => {
    expect(AGENT_METADATA.id).toBe('failure-classification-agent');
  });

  it('should have READ-ONLY classification', () => {
    expect(AGENT_METADATA.classification).toBe('READ-ONLY');
  });

  it('should have correct decision_type', () => {
    expect(AGENT_METADATA.decision_type).toBe('failure_classification');
  });

  it('should list prohibited operations', () => {
    expect(AGENT_METADATA.prohibited_operations).toContain('sql_execute');
    expect(AGENT_METADATA.prohibited_operations).toContain('orchestration_trigger');
    expect(AGENT_METADATA.prohibited_operations).toContain('alert_trigger');
  });

  it('should list downstream consumers', () => {
    expect(AGENT_METADATA.downstream_consumers).toContain('post-mortem-generator-agent');
    expect(AGENT_METADATA.downstream_consumers).toContain('governance-audit-views');
  });
});

// =============================================================================
// BATCH VALIDATION TESTS
// =============================================================================

describe('BatchClassificationRequestSchema', () => {
  it('should validate valid batch request', () => {
    const result = BatchClassificationRequestSchema.safeParse({
      events: [validFailureEvent],
    });
    expect(result.success).toBe(true);
  });

  it('should reject empty events array', () => {
    const result = BatchClassificationRequestSchema.safeParse({
      events: [],
    });
    expect(result.success).toBe(false);
  });

  it('should reject batch with more than 1000 events', () => {
    const events = Array(1001).fill(validFailureEvent);
    const result = BatchClassificationRequestSchema.safeParse({ events });
    expect(result.success).toBe(false);
  });
});

describe('BatchClassificationResultSchema', () => {
  it('should validate valid batch result', () => {
    const result = BatchClassificationResultSchema.safeParse({
      classifications: [validClassification],
      batch_id: 'batch-123',
      total_events: 1,
      classified_count: 1,
      failed_count: 0,
      processing_time_ms: 150,
    });
    expect(result.success).toBe(true);
  });

  it('should reject result with negative counts', () => {
    const result = BatchClassificationResultSchema.safeParse({
      classifications: [validClassification],
      batch_id: 'batch-123',
      total_events: -1,
      classified_count: 1,
      failed_count: 0,
      processing_time_ms: 150,
    });
    expect(result.success).toBe(false);
  });
});

// =============================================================================
// QUERY SCHEMA TESTS
// =============================================================================

describe('ClassificationQuerySchema', () => {
  it('should validate empty query with defaults', () => {
    const result = ClassificationQuerySchema.safeParse({});
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.limit).toBe(100);
      expect(result.data.offset).toBe(0);
      expect(result.data.sort_by).toBe('timestamp');
      expect(result.data.sort_order).toBe('desc');
    }
  });

  it('should reject limit > 1000', () => {
    const result = ClassificationQuerySchema.safeParse({ limit: 1001 });
    expect(result.success).toBe(false);
  });

  it('should reject invalid sort_by', () => {
    const result = ClassificationQuerySchema.safeParse({ sort_by: 'invalid' });
    expect(result.success).toBe(false);
  });
});

describe('AnalysisQuerySchema', () => {
  it('should validate with defaults', () => {
    const result = AnalysisQuerySchema.safeParse({});
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.group_by).toBe('category');
      expect(result.data.time_window_hours).toBe(24);
    }
  });

  it('should accept valid group_by values', () => {
    const validGroupBy = ['category', 'severity', 'cause', 'provider', 'model'];
    for (const groupBy of validGroupBy) {
      const result = AnalysisQuerySchema.safeParse({ group_by: groupBy });
      expect(result.success).toBe(true);
    }
  });

  it('should reject invalid group_by', () => {
    const result = AnalysisQuerySchema.safeParse({ group_by: 'invalid' });
    expect(result.success).toBe(false);
  });
});
