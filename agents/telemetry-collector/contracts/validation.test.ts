// Copyright 2025 LLM Observatory Contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * Tests for Telemetry Collector Agent validation utilities.
 */

import { describe, it, expect, beforeEach } from 'vitest';
import {
  validateTelemetryEvent,
  validateDecisionEvent,
  hashInput,
  hashInputs,
  validateConstitutionalOperation,
} from './validation';
import type { TelemetryEvent, DecisionEvent } from './schemas';

describe('validateTelemetryEvent', () => {
  let validEvent: TelemetryEvent;

  beforeEach(() => {
    validEvent = {
      span_id: 'span_123',
      trace_id: 'trace_456',
      name: 'llm.completion',
      provider: 'openai',
      model: 'gpt-4',
      input: {
        type: 'text',
        prompt: 'Hello, world!',
      },
      latency: {
        total_ms: 1234,
        start_time: '2025-01-19T00:00:00.000Z',
        end_time: '2025-01-19T00:00:01.234Z',
      },
      metadata: {
        tags: [],
        attributes: {},
      },
      status: 'OK',
      attributes: {},
      events: [],
    };
  });

  it('should validate a valid telemetry event', () => {
    const result = validateTelemetryEvent(validEvent);

    expect(result.success).toBe(true);
    expect(result.data).toBeDefined();
    expect(result.data?.span_id).toBe('span_123');
    expect(result.metadata?.validationTimeMs).toBeGreaterThan(0);
    expect(result.metadata?.schemaVersion).toBe('1.0.0');
    expect(result.metadata?.inputHash).toMatch(/^[a-f0-9]{64}$/i);
  });

  it('should return validation errors for invalid event', () => {
    const invalidEvent = {
      span_id: 'span_123',
      // Missing required fields
    };

    const result = validateTelemetryEvent(invalidEvent);

    expect(result.success).toBe(false);
    expect(result.errors).toBeDefined();
    expect(result.errors!.length).toBeGreaterThan(0);
    expect(result.data).toBeUndefined();
  });

  it('should include detailed error information', () => {
    const invalidEvent = {
      ...validEvent,
      token_usage: {
        prompt_tokens: -10, // Invalid: negative
        completion_tokens: 5,
        total_tokens: 15,
      },
    };

    const result = validateTelemetryEvent(invalidEvent);

    expect(result.success).toBe(false);
    expect(result.errors).toBeDefined();

    const error = result.errors!.find(
      (e) => e.path.join('.') === 'token_usage.prompt_tokens'
    );
    expect(error).toBeDefined();
    expect(error?.message).toContain('greater than or equal to 0');
  });

  it('should measure validation time', () => {
    const result = validateTelemetryEvent(validEvent);

    expect(result.metadata?.validationTimeMs).toBeGreaterThan(0);
    expect(result.metadata?.validationTimeMs).toBeLessThan(100); // Should be fast
  });
});

describe('validateDecisionEvent', () => {
  let validDecision: DecisionEvent;

  beforeEach(() => {
    validDecision = {
      agent_id: 'telemetry-collector',
      agent_version: '1.0.0',
      decision_type: 'telemetry_ingestion',
      confidence: 1.0,
      constraints_applied: [],
      inputs_hash: 'a'.repeat(64),
      outputs: [
        {
          span_id: 'span_123',
          trace_id: 'trace_456',
          name: 'llm.completion',
          provider: 'openai',
          model: 'gpt-4',
          input: { type: 'text', prompt: 'Test' },
          latency: {
            total_ms: 1000,
            start_time: '2025-01-19T00:00:00.000Z',
            end_time: '2025-01-19T00:00:01.000Z',
          },
          metadata: { tags: [], attributes: {} },
          status: 'OK',
          attributes: {},
          events: [],
          normalized_at: '2025-01-19T00:00:01.500Z',
          schema_version: '1.0.0',
        },
      ],
      execution_ref: 'exec_123456',
      timestamp: '2025-01-19T00:00:02.000Z',
    };
  });

  it('should validate a constitutional decision event', () => {
    const result = validateDecisionEvent(validDecision);

    expect(result.success).toBe(true);
    expect(result.data).toBeDefined();
    expect(result.data?.decision_type).toBe('telemetry_ingestion');
    expect(result.data?.confidence).toBe(1.0);
    expect(result.data?.constraints_applied).toEqual([]);
  });

  it('should reject wrong decision_type', () => {
    const invalid = {
      ...validDecision,
      decision_type: 'wrong_type' as any,
    };

    const result = validateDecisionEvent(invalid);

    expect(result.success).toBe(false);
    expect(result.errors).toBeDefined();
    expect(result.errors!.some((e) => e.code === 'invalid_literal')).toBe(true);
  });

  it('should reject confidence other than 1.0', () => {
    const invalid = {
      ...validDecision,
      confidence: 0.95 as any,
    };

    const result = validateDecisionEvent(invalid);

    expect(result.success).toBe(false);
    expect(result.errors).toBeDefined();
    expect(result.errors!.some((e) => e.path.includes('confidence'))).toBe(true);
  });

  it('should reject non-empty constraints_applied', () => {
    const invalid = {
      ...validDecision,
      constraints_applied: ['some_constraint'] as any,
    };

    const result = validateDecisionEvent(invalid);

    expect(result.success).toBe(false);
    expect(result.errors).toBeDefined();
  });

  it('should validate inputs_hash is valid SHA256', () => {
    const invalid = {
      ...validDecision,
      inputs_hash: 'not-a-valid-hash',
    };

    const result = validateDecisionEvent(invalid);

    expect(result.success).toBe(false);
    expect(result.errors).toBeDefined();

    const error = result.errors!.find((e) => e.path.includes('inputs_hash'));
    expect(error).toBeDefined();
    expect(error?.message).toContain('SHA256');
  });

  it('should validate agent_version is semantic version', () => {
    const invalid = {
      ...validDecision,
      agent_version: 'v1.0', // Not X.Y.Z format
    };

    const result = validateDecisionEvent(invalid);

    expect(result.success).toBe(false);
    expect(result.errors).toBeDefined();

    const error = result.errors!.find((e) => e.path.includes('agent_version'));
    expect(error).toBeDefined();
    expect(error?.message).toContain('semantic version');
  });

  it('should enforce constitutional constraints', () => {
    const result = validateDecisionEvent(validDecision);

    expect(result.success).toBe(true);
    expect(result.data?.decision_type).toBe('telemetry_ingestion');
    expect(result.data?.confidence).toBe(1.0);
    expect(result.data?.constraints_applied).toEqual([]);
  });
});

describe('hashInput', () => {
  let event: TelemetryEvent;

  beforeEach(() => {
    event = {
      span_id: 'span_123',
      trace_id: 'trace_456',
      name: 'llm.completion',
      provider: 'openai',
      model: 'gpt-4',
      input: {
        type: 'text',
        prompt: 'Hello, world!',
      },
      latency: {
        total_ms: 1234,
        start_time: '2025-01-19T00:00:00.000Z',
        end_time: '2025-01-19T00:00:01.234Z',
      },
      metadata: {
        tags: ['test'],
        attributes: { key: 'value' },
      },
      status: 'OK',
      attributes: {},
      events: [],
    };
  });

  it('should generate SHA256 hash', () => {
    const hash = hashInput(event);

    expect(hash).toMatch(/^[a-f0-9]{64}$/i);
    expect(hash.length).toBe(64);
  });

  it('should generate consistent hash for same input', () => {
    const hash1 = hashInput(event);
    const hash2 = hashInput(event);

    expect(hash1).toBe(hash2);
  });

  it('should generate different hash for different input', () => {
    const hash1 = hashInput(event);

    const modifiedEvent = { ...event, span_id: 'span_999' };
    const hash2 = hashInput(modifiedEvent);

    expect(hash1).not.toBe(hash2);
  });

  it('should exclude metadata when requested', () => {
    const hash1 = hashInput(event, { includeMetadata: true });
    const hash2 = hashInput(event, { includeMetadata: false });

    expect(hash1).not.toBe(hash2);
  });

  it('should exclude specified fields', () => {
    const hash1 = hashInput(event);
    const hash2 = hashInput(event, { excludeFields: ['attributes', 'events'] });

    expect(hash1).not.toBe(hash2);
  });

  it('should be deterministic with sorted keys', () => {
    const event1 = {
      span_id: 'span_123',
      trace_id: 'trace_456',
      name: 'test',
      provider: 'openai',
      model: 'gpt-4',
      input: { type: 'text' as const, prompt: 'Test' },
      latency: {
        total_ms: 1000,
        start_time: '2025-01-19T00:00:00.000Z',
        end_time: '2025-01-19T00:00:01.000Z',
      },
      metadata: { tags: [], attributes: {} },
      status: 'OK' as const,
      attributes: {},
      events: [],
    };

    // Same data, different key order in object literal
    const event2 = {
      model: 'gpt-4',
      provider: 'openai',
      span_id: 'span_123',
      trace_id: 'trace_456',
      name: 'test',
      input: { type: 'text' as const, prompt: 'Test' },
      latency: {
        total_ms: 1000,
        start_time: '2025-01-19T00:00:00.000Z',
        end_time: '2025-01-19T00:00:01.000Z',
      },
      metadata: { tags: [], attributes: {} },
      status: 'OK' as const,
      attributes: {},
      events: [],
    };

    const hash1 = hashInput(event1);
    const hash2 = hashInput(event2);

    expect(hash1).toBe(hash2); // Should be same despite different key order
  });
});

describe('hashInputs', () => {
  let events: TelemetryEvent[];

  beforeEach(() => {
    events = [
      {
        span_id: 'span_1',
        trace_id: 'trace_1',
        name: 'test',
        provider: 'openai',
        model: 'gpt-4',
        input: { type: 'text', prompt: 'Test 1' },
        latency: {
          total_ms: 1000,
          start_time: '2025-01-19T00:00:00.000Z',
          end_time: '2025-01-19T00:00:01.000Z',
        },
        metadata: { tags: [], attributes: {} },
        status: 'OK',
        attributes: {},
        events: [],
      },
      {
        span_id: 'span_2',
        trace_id: 'trace_2',
        name: 'test',
        provider: 'anthropic',
        model: 'claude-3',
        input: { type: 'text', prompt: 'Test 2' },
        latency: {
          total_ms: 2000,
          start_time: '2025-01-19T00:00:00.000Z',
          end_time: '2025-01-19T00:00:02.000Z',
        },
        metadata: { tags: [], attributes: {} },
        status: 'OK',
        attributes: {},
        events: [],
      },
    ];
  });

  it('should generate hash for multiple inputs', () => {
    const hash = hashInputs(events);

    expect(hash).toMatch(/^[a-f0-9]{64}$/i);
    expect(hash.length).toBe(64);
  });

  it('should be consistent for same inputs', () => {
    const hash1 = hashInputs(events);
    const hash2 = hashInputs(events);

    expect(hash1).toBe(hash2);
  });

  it('should be different for different order', () => {
    const hash1 = hashInputs(events);
    const hash2 = hashInputs([events[1], events[0]]);

    expect(hash1).not.toBe(hash2); // Order matters
  });

  it('should handle single input', () => {
    const hash = hashInputs([events[0]]);

    expect(hash).toMatch(/^[a-f0-9]{64}$/i);
  });
});

describe('validateConstitutionalOperation', () => {
  it('should accept read-only operations', () => {
    const operations = [
      { type: 'telemetry_read' },
      { type: 'normalize' },
      { type: 'validate' },
      { type: 'hash_input' },
    ];

    for (const op of operations) {
      const result = validateConstitutionalOperation(op);
      expect(result.success).toBe(true);
    }
  });

  it('should reject SQL execution', () => {
    const result = validateConstitutionalOperation({ type: 'sql_execute' });

    expect(result.success).toBe(false);
    expect(result.errors).toBeDefined();
    expect(result.errors![0].code).toBe('CONSTITUTIONAL_VIOLATION');
    expect(result.errors![0].message).toContain('READ-ONLY');
  });

  it('should reject orchestration triggers', () => {
    const result = validateConstitutionalOperation({ type: 'orchestration_trigger' });

    expect(result.success).toBe(false);
    expect(result.errors![0].code).toBe('CONSTITUTIONAL_VIOLATION');
  });

  it('should reject state modifications', () => {
    const result = validateConstitutionalOperation({ type: 'state_modify' });

    expect(result.success).toBe(false);
    expect(result.errors![0].code).toBe('CONSTITUTIONAL_VIOLATION');
  });

  it('should reject constraint applications', () => {
    const result = validateConstitutionalOperation({ type: 'constraint_apply' });

    expect(result.success).toBe(false);
    expect(result.errors![0].code).toBe('CONSTITUTIONAL_VIOLATION');
  });

  it('should reject retry triggers', () => {
    const result = validateConstitutionalOperation({ type: 'retry_trigger' });

    expect(result.success).toBe(false);
    expect(result.errors![0].code).toBe('CONSTITUTIONAL_VIOLATION');
  });

  it('should reject alert triggers', () => {
    const result = validateConstitutionalOperation({ type: 'alert_trigger' });

    expect(result.success).toBe(false);
    expect(result.errors![0].code).toBe('CONSTITUTIONAL_VIOLATION');
  });
});
