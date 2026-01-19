// Copyright 2025 LLM Observatory Contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * Tests for Telemetry Collector Agent contract schemas.
 */

import { describe, it, expect } from 'vitest';
import {
  TelemetryEventSchema,
  NormalizedTelemetrySchema,
  DecisionEventSchema,
  type TelemetryEvent,
  type NormalizedTelemetry,
  type DecisionEvent,
} from './schemas';

describe('TelemetryEventSchema', () => {
  it('should validate a complete telemetry event', () => {
    const validEvent: TelemetryEvent = {
      span_id: 'span_123',
      trace_id: 'trace_456',
      parent_span_id: 'span_000',
      name: 'llm.completion',
      provider: 'openai',
      model: 'gpt-4',
      input: {
        type: 'text',
        prompt: 'Hello, world!',
      },
      output: {
        content: 'Hi there!',
        finish_reason: 'stop',
        metadata: {},
      },
      token_usage: {
        prompt_tokens: 10,
        completion_tokens: 5,
        total_tokens: 15,
      },
      cost: {
        amount_usd: 0.0015,
        currency: 'USD',
        prompt_cost: 0.001,
        completion_cost: 0.0005,
      },
      latency: {
        total_ms: 1234,
        ttft_ms: 500,
        start_time: '2025-01-19T00:00:00.000Z',
        end_time: '2025-01-19T00:00:01.234Z',
      },
      metadata: {
        user_id: 'user_123',
        session_id: 'session_456',
        environment: 'production',
        tags: ['test', 'api'],
        attributes: { key: 'value' },
      },
      status: 'OK',
      attributes: { custom: 'attribute' },
      events: [
        {
          name: 'stream.start',
          timestamp: '2025-01-19T00:00:00.100Z',
          attributes: {},
        },
      ],
    };

    const result = TelemetryEventSchema.safeParse(validEvent);
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.span_id).toBe('span_123');
      expect(result.data.provider).toBe('openai');
    }
  });

  it('should validate minimal telemetry event', () => {
    const minimalEvent = {
      span_id: 'span_123',
      trace_id: 'trace_456',
      name: 'llm.completion',
      provider: 'anthropic',
      model: 'claude-3-opus',
      input: {
        type: 'text',
        prompt: 'Test',
      },
      latency: {
        total_ms: 1000,
        start_time: '2025-01-19T00:00:00.000Z',
        end_time: '2025-01-19T00:00:01.000Z',
      },
    };

    const result = TelemetryEventSchema.safeParse(minimalEvent);
    expect(result.success).toBe(true);
  });

  it('should validate chat input type', () => {
    const event = {
      span_id: 'span_123',
      trace_id: 'trace_456',
      name: 'llm.chat',
      provider: 'openai',
      model: 'gpt-4',
      input: {
        type: 'chat',
        messages: [
          { role: 'system', content: 'You are helpful' },
          { role: 'user', content: 'Hello' },
          { role: 'assistant', content: 'Hi there!' },
        ],
      },
      latency: {
        total_ms: 1000,
        start_time: '2025-01-19T00:00:00.000Z',
        end_time: '2025-01-19T00:00:01.000Z',
      },
    };

    const result = TelemetryEventSchema.safeParse(event);
    expect(result.success).toBe(true);
  });

  it('should validate multimodal input type', () => {
    const event = {
      span_id: 'span_123',
      trace_id: 'trace_456',
      name: 'llm.multimodal',
      provider: 'google',
      model: 'gemini-pro-vision',
      input: {
        type: 'multimodal',
        parts: [
          { type: 'text', text: 'What is in this image?' },
          { type: 'image', source: 'data:image/png;base64,...' },
        ],
      },
      latency: {
        total_ms: 2000,
        start_time: '2025-01-19T00:00:00.000Z',
        end_time: '2025-01-19T00:00:02.000Z',
      },
    };

    const result = TelemetryEventSchema.safeParse(event);
    expect(result.success).toBe(true);
  });

  it('should reject invalid provider', () => {
    const event = {
      span_id: 'span_123',
      trace_id: 'trace_456',
      name: 'llm.completion',
      provider: 123, // Invalid: number instead of string
      model: 'test-model',
      input: { type: 'text', prompt: 'Test' },
      latency: {
        total_ms: 1000,
        start_time: '2025-01-19T00:00:00.000Z',
        end_time: '2025-01-19T00:00:01.000Z',
      },
    };

    const result = TelemetryEventSchema.safeParse(event);
    expect(result.success).toBe(false);
  });

  it('should reject missing required fields', () => {
    const event = {
      span_id: 'span_123',
      // Missing trace_id, name, provider, model, input, latency
    };

    const result = TelemetryEventSchema.safeParse(event);
    expect(result.success).toBe(false);
  });

  it('should reject negative token counts', () => {
    const event = {
      span_id: 'span_123',
      trace_id: 'trace_456',
      name: 'llm.completion',
      provider: 'openai',
      model: 'gpt-4',
      input: { type: 'text', prompt: 'Test' },
      token_usage: {
        prompt_tokens: -10, // Invalid: negative
        completion_tokens: 5,
        total_tokens: 15,
      },
      latency: {
        total_ms: 1000,
        start_time: '2025-01-19T00:00:00.000Z',
        end_time: '2025-01-19T00:00:01.000Z',
      },
    };

    const result = TelemetryEventSchema.safeParse(event);
    expect(result.success).toBe(false);
  });

  it('should accept custom provider string', () => {
    const event = {
      span_id: 'span_123',
      trace_id: 'trace_456',
      name: 'llm.completion',
      provider: 'my-custom-provider',
      model: 'custom-model-v1',
      input: { type: 'text', prompt: 'Test' },
      latency: {
        total_ms: 1000,
        start_time: '2025-01-19T00:00:00.000Z',
        end_time: '2025-01-19T00:00:01.000Z',
      },
    };

    const result = TelemetryEventSchema.safeParse(event);
    expect(result.success).toBe(true);
  });
});

describe('NormalizedTelemetrySchema', () => {
  it('should validate normalized telemetry', () => {
    const normalized: NormalizedTelemetry = {
      span_id: 'span_123',
      trace_id: 'trace_456',
      name: 'llm.completion',
      provider: 'openai',
      model: 'gpt-4',
      input: { type: 'text', prompt: 'Hello' },
      output: { content: 'Hi', finish_reason: 'stop', metadata: {} },
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
    };

    const result = NormalizedTelemetrySchema.safeParse(normalized);
    expect(result.success).toBe(true);
  });

  it('should require normalized_at timestamp', () => {
    const normalized = {
      span_id: 'span_123',
      trace_id: 'trace_456',
      name: 'llm.completion',
      provider: 'openai',
      model: 'gpt-4',
      input: { type: 'text', prompt: 'Hello' },
      latency: {
        total_ms: 1000,
        start_time: '2025-01-19T00:00:00.000Z',
        end_time: '2025-01-19T00:00:01.000Z',
      },
      metadata: { tags: [], attributes: {} },
      status: 'OK',
      // Missing normalized_at
    };

    const result = NormalizedTelemetrySchema.safeParse(normalized);
    expect(result.success).toBe(false);
  });
});

describe('DecisionEventSchema', () => {
  it('should validate constitutional decision event', () => {
    const decision: DecisionEvent = {
      agent_id: 'telemetry-collector',
      agent_version: '1.0.0',
      decision_type: 'telemetry_ingestion',
      confidence: 1.0,
      constraints_applied: [],
      inputs_hash: 'a'.repeat(64), // Valid SHA256 hash
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

    const result = DecisionEventSchema.safeParse(decision);
    expect(result.success).toBe(true);
  });

  it('should enforce decision_type literal', () => {
    const decision = {
      agent_id: 'telemetry-collector',
      agent_version: '1.0.0',
      decision_type: 'wrong_type', // Invalid: not the literal
      confidence: 1.0,
      constraints_applied: [],
      inputs_hash: 'a'.repeat(64),
      outputs: [],
      execution_ref: 'exec_123',
      timestamp: '2025-01-19T00:00:00.000Z',
    };

    const result = DecisionEventSchema.safeParse(decision);
    expect(result.success).toBe(false);
  });

  it('should enforce confidence literal (1.0)', () => {
    const decision = {
      agent_id: 'telemetry-collector',
      agent_version: '1.0.0',
      decision_type: 'telemetry_ingestion',
      confidence: 0.95, // Invalid: not 1.0
      constraints_applied: [],
      inputs_hash: 'a'.repeat(64),
      outputs: [],
      execution_ref: 'exec_123',
      timestamp: '2025-01-19T00:00:00.000Z',
    };

    const result = DecisionEventSchema.safeParse(decision);
    expect(result.success).toBe(false);
  });

  it('should enforce empty constraints_applied array', () => {
    const decision = {
      agent_id: 'telemetry-collector',
      agent_version: '1.0.0',
      decision_type: 'telemetry_ingestion',
      confidence: 1.0,
      constraints_applied: ['some_constraint'], // Invalid: not empty
      inputs_hash: 'a'.repeat(64),
      outputs: [],
      execution_ref: 'exec_123',
      timestamp: '2025-01-19T00:00:00.000Z',
    };

    const result = DecisionEventSchema.safeParse(decision);
    expect(result.success).toBe(false);
  });

  it('should validate inputs_hash is 64 hex chars', () => {
    const decision = {
      agent_id: 'telemetry-collector',
      agent_version: '1.0.0',
      decision_type: 'telemetry_ingestion',
      confidence: 1.0,
      constraints_applied: [],
      inputs_hash: 'a'.repeat(63), // Invalid: only 63 chars
      outputs: [],
      execution_ref: 'exec_123',
      timestamp: '2025-01-19T00:00:00.000Z',
    };

    const result = DecisionEventSchema.safeParse(decision);
    expect(result.success).toBe(false);
  });

  it('should require at least one output', () => {
    const decision = {
      agent_id: 'telemetry-collector',
      agent_version: '1.0.0',
      decision_type: 'telemetry_ingestion',
      confidence: 1.0,
      constraints_applied: [],
      inputs_hash: 'a'.repeat(64),
      outputs: [], // Invalid: empty array
      execution_ref: 'exec_123',
      timestamp: '2025-01-19T00:00:00.000Z',
    };

    const result = DecisionEventSchema.safeParse(decision);
    expect(result.success).toBe(false);
  });

  it('should validate semantic version format', () => {
    const decision = {
      agent_id: 'telemetry-collector',
      agent_version: 'v1.0', // Invalid: not X.Y.Z format
      decision_type: 'telemetry_ingestion',
      confidence: 1.0,
      constraints_applied: [],
      inputs_hash: 'a'.repeat(64),
      outputs: [
        {
          span_id: 'span_123',
          trace_id: 'trace_456',
          name: 'test',
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
          normalized_at: '2025-01-19T00:00:01.000Z',
          schema_version: '1.0.0',
        },
      ],
      execution_ref: 'exec_123',
      timestamp: '2025-01-19T00:00:00.000Z',
    };

    const result = DecisionEventSchema.safeParse(decision);
    expect(result.success).toBe(false);
  });
});
