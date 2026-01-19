import { describe, it, expect, beforeEach } from 'vitest';
import { z } from 'zod';

/**
 * Schema definitions for telemetry event validation
 */
const TelemetryEventSchema = z.object({
  id: z.string().uuid(),
  timestamp: z.date(),
  provider: z.enum(['ANTHROPIC', 'OPENAI', 'GOOGLE', 'OTHER']),
  model: z.string(),
  inputType: z.enum(['TEXT', 'CHAT', 'MULTIMODAL']),
  inputHash: z.string().regex(/^[a-f0-9]{64}$/), // SHA-256 hex
  output: z.object({
    type: z.string(),
    content: z.string(),
  }),
  metadata: z.record(z.unknown()).optional(),
});

const DecisionEventSchema = z.object({
  id: z.string().uuid(),
  timestamp: z.date(),
  agentId: z.string(),
  agentVersion: z.string(),
  decision: z.string(),
  reasoning: z.string(),
  confidence: z.number().min(0).max(1),
  telemetryEventIds: z.array(z.string().uuid()),
  metadata: z.record(z.unknown()).optional(),
});

const NormalizationSchema = z.object({
  provider: z.enum(['ANTHROPIC', 'OPENAI', 'GOOGLE', 'OTHER']),
  timestamp: z.date(),
  inputType: z.enum(['TEXT', 'CHAT', 'MULTIMODAL']),
});

describe('Contracts - Schema Validation', () => {
  describe('TelemetryEventSchema', () => {
    it('should validate a complete telemetry event', () => {
      const validEvent = {
        id: '550e8400-e29b-41d4-a716-446655440000',
        timestamp: new Date('2026-01-19T10:00:00Z'),
        provider: 'ANTHROPIC',
        model: 'claude-opus-4.5',
        inputType: 'TEXT',
        inputHash: 'a'.repeat(64),
        output: {
          type: 'text',
          content: 'Response content',
        },
      };

      const result = TelemetryEventSchema.safeParse(validEvent);
      expect(result.success).toBe(true);
    });

    it('should reject event with invalid UUID', () => {
      const invalidEvent = {
        id: 'not-a-uuid',
        timestamp: new Date(),
        provider: 'ANTHROPIC',
        model: 'claude-opus-4.5',
        inputType: 'TEXT',
        inputHash: 'a'.repeat(64),
        output: {
          type: 'text',
          content: 'Response content',
        },
      };

      const result = TelemetryEventSchema.safeParse(invalidEvent);
      expect(result.success).toBe(false);
    });

    it('should reject event with invalid provider', () => {
      const invalidEvent = {
        id: '550e8400-e29b-41d4-a716-446655440000',
        timestamp: new Date(),
        provider: 'INVALID_PROVIDER',
        model: 'claude-opus-4.5',
        inputType: 'TEXT',
        inputHash: 'a'.repeat(64),
        output: {
          type: 'text',
          content: 'Response content',
        },
      };

      const result = TelemetryEventSchema.safeParse(invalidEvent);
      expect(result.success).toBe(false);
    });

    it('should reject event with invalid inputHash format', () => {
      const invalidEvent = {
        id: '550e8400-e29b-41d4-a716-446655440000',
        timestamp: new Date(),
        provider: 'ANTHROPIC',
        model: 'claude-opus-4.5',
        inputType: 'TEXT',
        inputHash: 'not-valid-hex', // Invalid SHA-256 hex
        output: {
          type: 'text',
          content: 'Response content',
        },
      };

      const result = TelemetryEventSchema.safeParse(invalidEvent);
      expect(result.success).toBe(false);
    });

    it('should reject event with invalid inputType', () => {
      const invalidEvent = {
        id: '550e8400-e29b-41d4-a716-446655440000',
        timestamp: new Date(),
        provider: 'ANTHROPIC',
        model: 'claude-opus-4.5',
        inputType: 'INVALID_TYPE',
        inputHash: 'a'.repeat(64),
        output: {
          type: 'text',
          content: 'Response content',
        },
      };

      const result = TelemetryEventSchema.safeParse(invalidEvent);
      expect(result.success).toBe(false);
    });

    it('should reject event with missing output.type', () => {
      const invalidEvent = {
        id: '550e8400-e29b-41d4-a716-446655440000',
        timestamp: new Date(),
        provider: 'ANTHROPIC',
        model: 'claude-opus-4.5',
        inputType: 'TEXT',
        inputHash: 'a'.repeat(64),
        output: {
          content: 'Response content',
        },
      };

      const result = TelemetryEventSchema.safeParse(invalidEvent);
      expect(result.success).toBe(false);
    });

    it('should accept optional metadata', () => {
      const eventWithMetadata = {
        id: '550e8400-e29b-41d4-a716-446655440000',
        timestamp: new Date(),
        provider: 'ANTHROPIC',
        model: 'claude-opus-4.5',
        inputType: 'TEXT',
        inputHash: 'a'.repeat(64),
        output: {
          type: 'text',
          content: 'Response content',
        },
        metadata: {
          customField: 'customValue',
          tokenCount: 150,
        },
      };

      const result = TelemetryEventSchema.safeParse(eventWithMetadata);
      expect(result.success).toBe(true);
    });
  });

  describe('DecisionEventSchema', () => {
    it('should validate a complete decision event', () => {
      const validDecision = {
        id: '550e8400-e29b-41d4-a716-446655440001',
        timestamp: new Date('2026-01-19T10:00:00Z'),
        agentId: 'agent-telemetry-collector-v1',
        agentVersion: '1.0.0',
        decision: 'ACCEPT_VALID_EVENT',
        reasoning: 'Event passed all validation checks',
        confidence: 0.95,
        telemetryEventIds: [
          '550e8400-e29b-41d4-a716-446655440000',
          '550e8400-e29b-41d4-a716-446655440002',
        ],
      };

      const result = DecisionEventSchema.safeParse(validDecision);
      expect(result.success).toBe(true);
    });

    it('should reject decision with confidence > 1', () => {
      const invalidDecision = {
        id: '550e8400-e29b-41d4-a716-446655440001',
        timestamp: new Date(),
        agentId: 'agent-telemetry-collector-v1',
        agentVersion: '1.0.0',
        decision: 'ACCEPT_VALID_EVENT',
        reasoning: 'Event passed all validation checks',
        confidence: 1.5, // Invalid: > 1
        telemetryEventIds: ['550e8400-e29b-41d4-a716-446655440000'],
      };

      const result = DecisionEventSchema.safeParse(invalidDecision);
      expect(result.success).toBe(false);
    });

    it('should reject decision with confidence < 0', () => {
      const invalidDecision = {
        id: '550e8400-e29b-41d4-a716-446655440001',
        timestamp: new Date(),
        agentId: 'agent-telemetry-collector-v1',
        agentVersion: '1.0.0',
        decision: 'ACCEPT_VALID_EVENT',
        reasoning: 'Event passed all validation checks',
        confidence: -0.5, // Invalid: < 0
        telemetryEventIds: ['550e8400-e29b-41d4-a716-446655440000'],
      };

      const result = DecisionEventSchema.safeParse(invalidDecision);
      expect(result.success).toBe(false);
    });

    it('should accept confidence = 0', () => {
      const validDecision = {
        id: '550e8400-e29b-41d4-a716-446655440001',
        timestamp: new Date(),
        agentId: 'agent-telemetry-collector-v1',
        agentVersion: '1.0.0',
        decision: 'REJECT_EVENT',
        reasoning: 'Event validation failed',
        confidence: 0,
        telemetryEventIds: [],
      };

      const result = DecisionEventSchema.safeParse(validDecision);
      expect(result.success).toBe(true);
    });

    it('should accept confidence = 1', () => {
      const validDecision = {
        id: '550e8400-e29b-41d4-a716-446655440001',
        timestamp: new Date(),
        agentId: 'agent-telemetry-collector-v1',
        agentVersion: '1.0.0',
        decision: 'ACCEPT_VALID_EVENT',
        reasoning: 'Event passed all validation checks',
        confidence: 1,
        telemetryEventIds: ['550e8400-e29b-41d4-a716-446655440000'],
      };

      const result = DecisionEventSchema.safeParse(validDecision);
      expect(result.success).toBe(true);
    });

    it('should reject decision with invalid telemetryEventIds', () => {
      const invalidDecision = {
        id: '550e8400-e29b-41d4-a716-446655440001',
        timestamp: new Date(),
        agentId: 'agent-telemetry-collector-v1',
        agentVersion: '1.0.0',
        decision: 'ACCEPT_VALID_EVENT',
        reasoning: 'Event passed all validation checks',
        confidence: 0.95,
        telemetryEventIds: ['not-a-uuid'],
      };

      const result = DecisionEventSchema.safeParse(invalidDecision);
      expect(result.success).toBe(false);
    });

    it('should accept optional metadata', () => {
      const validDecision = {
        id: '550e8400-e29b-41d4-a716-446655440001',
        timestamp: new Date(),
        agentId: 'agent-telemetry-collector-v1',
        agentVersion: '1.0.0',
        decision: 'ACCEPT_VALID_EVENT',
        reasoning: 'Event passed all validation checks',
        confidence: 0.95,
        telemetryEventIds: ['550e8400-e29b-41d4-a716-446655440000'],
        metadata: {
          processingTimeMs: 25,
          validationRules: ['schema', 'hash', 'timestamp'],
        },
      };

      const result = DecisionEventSchema.safeParse(validDecision);
      expect(result.success).toBe(true);
    });

    it('should accept empty telemetryEventIds array', () => {
      const validDecision = {
        id: '550e8400-e29b-41d4-a716-446655440001',
        timestamp: new Date(),
        agentId: 'agent-telemetry-collector-v1',
        agentVersion: '1.0.0',
        decision: 'REJECT_EVENT',
        reasoning: 'Event validation failed',
        confidence: 0.9,
        telemetryEventIds: [],
      };

      const result = DecisionEventSchema.safeParse(validDecision);
      expect(result.success).toBe(true);
    });
  });

  describe('Deterministic Hash Input', () => {
    it('should produce consistent SHA-256 hashes for identical inputs', () => {
      // In real implementation, would use crypto
      const input1 = 'test input content';
      const input2 = 'test input content';

      // Simulate deterministic hashing
      const hash1 = computeHash(input1);
      const hash2 = computeHash(input2);

      expect(hash1).toBe(hash2);
    });

    it('should produce different hashes for different inputs', () => {
      const input1 = 'test input 1';
      const input2 = 'test input 2';

      const hash1 = computeHash(input1);
      const hash2 = computeHash(input2);

      expect(hash1).not.toBe(hash2);
    });

    it('should handle empty string input', () => {
      const hash = computeHash('');
      expect(hash).toMatch(/^[a-f0-9]{64}$/);
    });

    it('should handle large string input', () => {
      const largeInput = 'x'.repeat(10000);
      const hash = computeHash(largeInput);
      expect(hash).toMatch(/^[a-f0-9]{64}$/);
    });

    it('should handle special characters', () => {
      const specialInput = '!@#$%^&*()_+-=[]{}|;:\'",.<>?/~`';
      const hash = computeHash(specialInput);
      expect(hash).toMatch(/^[a-f0-9]{64}$/);
    });

    it('should be case-sensitive', () => {
      const hash1 = computeHash('TestInput');
      const hash2 = computeHash('testinput');

      expect(hash1).not.toBe(hash2);
    });
  });

  describe('Timestamp Normalization', () => {
    it('should normalize timestamp to UTC', () => {
      const date = new Date('2026-01-19T10:30:45.123Z');
      const normalized = normalizeTimestamp(date);

      expect(normalized.getUTCHours()).toBe(date.getUTCHours());
      expect(normalized.getUTCMinutes()).toBe(date.getUTCMinutes());
      expect(normalized.getUTCSeconds()).toBe(date.getUTCSeconds());
    });

    it('should preserve milliseconds', () => {
      const date = new Date('2026-01-19T10:30:45.123Z');
      const normalized = normalizeTimestamp(date);

      expect(normalized.getUTCMilliseconds()).toBe(123);
    });

    it('should handle dates from different timezones', () => {
      // Create a date in local time
      const date = new Date(2026, 0, 19, 10, 30, 45, 123);
      const normalized = normalizeTimestamp(date);

      // Should be a valid Date object
      expect(normalized instanceof Date).toBe(true);
      expect(normalized.getTime()).toBeLessThanOrEqual(Date.now() + 86400000); // Within 1 day
    });

    it('should handle edge case dates', () => {
      const edgeDates = [
        new Date('2026-01-01T00:00:00Z'),
        new Date('2026-12-31T23:59:59.999Z'),
        new Date(0), // Unix epoch
      ];

      for (const date of edgeDates) {
        const normalized = normalizeTimestamp(date);
        expect(normalized instanceof Date).toBe(true);
        expect(!isNaN(normalized.getTime())).toBe(true);
      }
    });

    it('should reject invalid date inputs', () => {
      expect(() => normalizeTimestamp(new Date('invalid'))).toThrow();
    });
  });

  describe('Provider Name Normalization', () => {
    it('should normalize provider names to uppercase', () => {
      const testCases = [
        { input: 'anthropic', expected: 'ANTHROPIC' },
        { input: 'ANTHROPIC', expected: 'ANTHROPIC' },
        { input: 'Anthropic', expected: 'ANTHROPIC' },
        { input: 'openai', expected: 'OPENAI' },
        { input: 'google', expected: 'GOOGLE' },
      ];

      for (const { input, expected } of testCases) {
        const normalized = normalizeProvider(input);
        expect(normalized).toBe(expected);
      }
    });

    it('should map common provider aliases', () => {
      const testCases = [
        { input: 'claude', expected: 'ANTHROPIC' },
        { input: 'gpt', expected: 'OPENAI' },
        { input: 'palm', expected: 'GOOGLE' },
      ];

      for (const { input, expected } of testCases) {
        const normalized = normalizeProvider(input);
        expect(normalized).toBe(expected);
      }
    });

    it('should default to OTHER for unknown providers', () => {
      const normalized = normalizeProvider('unknown_provider');
      expect(normalized).toBe('OTHER');
    });
  });
});

/**
 * Helper functions (would be in actual implementation)
 */
function computeHash(input: string): string {
  // Simulate SHA-256 hex output
  // In real implementation, would use crypto.createHash('sha256')
  let hash = 0;
  for (let i = 0; i < input.length; i++) {
    const char = input.charCodeAt(i);
    hash = (hash << 5) - hash + char;
    hash = hash & hash;
  }
  return Math.abs(hash).toString(16).padEnd(64, '0').substring(0, 64);
}

function normalizeTimestamp(date: Date): Date {
  if (!(date instanceof Date) || isNaN(date.getTime())) {
    throw new Error('Invalid date input');
  }
  // Return as-is, JavaScript Dates are always in UTC internally
  return new Date(date.getTime());
}

function normalizeProvider(provider: string): string {
  const upper = provider.toUpperCase();

  // Map aliases
  const aliasMap: Record<string, string> = {
    CLAUDE: 'ANTHROPIC',
    GPT: 'OPENAI',
    PALM: 'GOOGLE',
  };

  if (aliasMap[upper]) {
    return aliasMap[upper];
  }

  // Accept known providers
  if (['ANTHROPIC', 'OPENAI', 'GOOGLE'].includes(upper)) {
    return upper;
  }

  return 'OTHER';
}
