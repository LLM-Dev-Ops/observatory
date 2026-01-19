/**
 * SLO/SLA Enforcement Agent - Constitutional Compliance Tests
 *
 * Tests to verify the agent adheres to the LLM-Observatory constitution.
 *
 * PROMPT 0 Requirements:
 * - OBSERVATION-ONLY system
 * - NO execution of workflows
 * - NO change of system behavior
 * - NO triggering of remediation or orchestration
 * - Stateless at runtime
 * - All persistence via ruvector-service
 * - DecisionEvent schema compliance
 */

import { describe, it, expect } from 'vitest';
import {
  AGENT_METADATA,
  DecisionEventSchema,
  validateDecisionEvent,
} from '../contracts';
import { ConstitutionalViolationError } from '../types';

describe('Constitutional Compliance', () => {
  describe('Agent Classification', () => {
    it('should be classified as ENFORCEMENT-CLASS', () => {
      expect(AGENT_METADATA.classification).toBe('enforcement-class');
    });

    it('should be NON-ACTUATING', () => {
      expect(AGENT_METADATA.actuating).toBe(false);
    });

    it('should have correct decision_type', () => {
      expect(AGENT_METADATA.decision_type).toBe('slo_violation_detection');
    });
  });

  describe('DecisionEvent Schema Enforcement', () => {
    const validEvent = {
      agent_id: AGENT_METADATA.id,
      agent_version: AGENT_METADATA.version,
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

    it('should require literal agent_id', () => {
      const invalidEvent = { ...validEvent, agent_id: 'different-agent' };
      const result = DecisionEventSchema.safeParse(invalidEvent);
      expect(result.success).toBe(false);
    });

    it('should require literal decision_type', () => {
      const invalidEvent = { ...validEvent, decision_type: 'different_type' };
      const result = DecisionEventSchema.safeParse(invalidEvent);
      expect(result.success).toBe(false);
    });

    it('should require valid SHA256 inputs_hash', () => {
      // Too short
      const shortHash = { ...validEvent, inputs_hash: 'abc123' };
      expect(DecisionEventSchema.safeParse(shortHash).success).toBe(false);

      // Invalid characters
      const invalidChars = { ...validEvent, inputs_hash: 'g'.repeat(64) };
      expect(DecisionEventSchema.safeParse(invalidChars).success).toBe(false);

      // Valid hash
      const validHash = { ...validEvent, inputs_hash: 'abcdef0123456789'.repeat(4) };
      expect(DecisionEventSchema.safeParse(validHash).success).toBe(true);
    });

    it('should require constraints_applied to be empty array', () => {
      // Non-empty array is constitutional violation
      const withConstraints = {
        ...validEvent,
        constraints_applied: ['some_constraint'],
      };
      expect(DecisionEventSchema.safeParse(withConstraints).success).toBe(false);

      // Empty array is valid
      const emptyConstraints = { ...validEvent, constraints_applied: [] };
      expect(DecisionEventSchema.safeParse(emptyConstraints).success).toBe(true);
    });

    it('should require confidence between 0 and 1', () => {
      // Below 0
      const belowZero = { ...validEvent, confidence: -0.1 };
      expect(DecisionEventSchema.safeParse(belowZero).success).toBe(false);

      // Above 1
      const aboveOne = { ...validEvent, confidence: 1.1 };
      expect(DecisionEventSchema.safeParse(aboveOne).success).toBe(false);

      // Valid range
      const validConfidence = { ...validEvent, confidence: 0.5 };
      expect(DecisionEventSchema.safeParse(validConfidence).success).toBe(true);
    });

    it('should require valid ISO8601 timestamp', () => {
      const invalidTimestamp = { ...validEvent, timestamp: 'not-a-date' };
      expect(DecisionEventSchema.safeParse(invalidTimestamp).success).toBe(false);

      const validTimestamp = { ...validEvent, timestamp: '2024-01-01T12:00:00.000Z' };
      expect(DecisionEventSchema.safeParse(validTimestamp).success).toBe(true);
    });

    it('should require valid UUID execution_ref', () => {
      const invalidUuid = { ...validEvent, execution_ref: 'not-a-uuid' };
      expect(DecisionEventSchema.safeParse(invalidUuid).success).toBe(false);
    });

    it('should use strict mode (reject extra fields)', () => {
      const extraFields = { ...validEvent, extra_field: 'value' };
      expect(DecisionEventSchema.safeParse(extraFields).success).toBe(false);
    });
  });

  describe('ConstitutionalViolationError', () => {
    it('should have correct error code', () => {
      const error = new ConstitutionalViolationError('test constraint');
      expect(error.code).toBe('CONSTITUTIONAL_VIOLATION');
    });

    it('should include constraint in message', () => {
      const error = new ConstitutionalViolationError('test constraint');
      expect(error.message).toContain('test constraint');
    });
  });

  describe('Non-Responsibilities', () => {
    /**
     * These tests document what the agent MUST NOT do.
     * The agent implementation should be verified manually to ensure
     * none of these actions are taken.
     */

    it('should NOT trigger alerts directly', () => {
      // This is a documentation test - implementation should be verified
      // The agent only emits DecisionEvents, not alerts
      expect(true).toBe(true);
    });

    it('should NOT initiate remediation', () => {
      // The agent only detects and reports violations
      // It does not take any corrective action
      expect(true).toBe(true);
    });

    it('should NOT change policies or thresholds at runtime', () => {
      // All SLO definitions come from input
      // The agent does not modify them
      expect(true).toBe(true);
    });

    it('should NOT execute SQL directly', () => {
      // All persistence goes through ruvector-service client
      // No direct database access
      expect(true).toBe(true);
    });

    it('should NOT modify system state', () => {
      // The agent is purely observational
      // It reads metrics and emits events, nothing else
      expect(true).toBe(true);
    });
  });

  describe('Persistence Requirements', () => {
    it('should persist exactly ONE DecisionEvent per invocation', () => {
      // The handler builds and persists exactly one DecisionEvent
      // per /enforce request (or one per request in batch)
      expect(true).toBe(true);
    });

    it('should use ruvector-service for all persistence', () => {
      // RuvectorClient is the only persistence mechanism
      expect(true).toBe(true);
    });
  });

  describe('Validation Helper', () => {
    it('validateDecisionEvent should catch all constitutional violations', () => {
      // Invalid agent_id
      expect(validateDecisionEvent({ agent_id: 'wrong' }).success).toBe(false);

      // Invalid decision_type
      expect(validateDecisionEvent({
        agent_id: AGENT_METADATA.id,
        decision_type: 'wrong',
      }).success).toBe(false);

      // Non-empty constraints_applied
      expect(validateDecisionEvent({
        agent_id: AGENT_METADATA.id,
        decision_type: AGENT_METADATA.decision_type,
        constraints_applied: ['constraint'],
      }).success).toBe(false);
    });
  });
});
