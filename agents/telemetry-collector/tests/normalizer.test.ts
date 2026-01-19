import { describe, it, expect } from 'vitest';

/**
 * Normalizer implementation for testing
 */
class InputNormalizer {
  /**
   * Normalize provider name to standard format
   */
  static normalizeProvider(provider: string): string {
    if (!provider || typeof provider !== 'string') {
      throw new Error('Provider must be a non-empty string');
    }

    const upper = provider.trim().toUpperCase();

    // Map common aliases
    const aliasMap: Record<string, string> = {
      CLAUDE: 'ANTHROPIC',
      ANTHROPIC_API: 'ANTHROPIC',
      'ANTHROPIC-API': 'ANTHROPIC',
      GPT: 'OPENAI',
      'GPT-3': 'OPENAI',
      'GPT-4': 'OPENAI',
      OPENAI_API: 'OPENAI',
      'OPENAI-API': 'OPENAI',
      GOOGLE_AI: 'GOOGLE',
      GOOGLE_PALM: 'GOOGLE',
      PALM: 'GOOGLE',
      'PALM-2': 'GOOGLE',
      BARD: 'GOOGLE',
    };

    if (aliasMap[upper]) {
      return aliasMap[upper];
    }

    // Accept known providers
    const knownProviders = ['ANTHROPIC', 'OPENAI', 'GOOGLE'];
    if (knownProviders.includes(upper)) {
      return upper;
    }

    return 'OTHER';
  }

  /**
   * Normalize input type to standard format
   */
  static normalizeInputType(
    inputType: string | undefined
  ): 'TEXT' | 'CHAT' | 'MULTIMODAL' {
    if (!inputType || typeof inputType !== 'string') {
      // Default to TEXT if not provided
      return 'TEXT';
    }

    const upper = inputType.trim().toUpperCase();

    // Map common aliases
    const aliasMap: Record<string, string> = {
      'PLAIN-TEXT': 'TEXT',
      'RAW-TEXT': 'TEXT',
      MESSAGE: 'CHAT',
      CONVERSATION: 'CHAT',
      DIALOG: 'CHAT',
      IMAGE: 'MULTIMODAL',
      AUDIO: 'MULTIMODAL',
      VIDEO: 'MULTIMODAL',
      'MIXED-MEDIA': 'MULTIMODAL',
    };

    if (aliasMap[upper]) {
      const normalized = aliasMap[upper];
      if (normalized === 'TEXT' || normalized === 'CHAT' || normalized === 'MULTIMODAL') {
        return normalized;
      }
    }

    // Check known types
    if (upper === 'TEXT' || upper === 'CHAT' || upper === 'MULTIMODAL') {
      return upper as 'TEXT' | 'CHAT' | 'MULTIMODAL';
    }

    // Default to TEXT for unknown types
    return 'TEXT';
  }

  /**
   * Normalize timestamp to UTC
   */
  static normalizeTimestamp(timestamp: Date | string | number): Date {
    if (timestamp instanceof Date) {
      if (isNaN(timestamp.getTime())) {
        throw new Error('Invalid date object');
      }
      return new Date(timestamp.getTime());
    }

    if (typeof timestamp === 'string') {
      const parsed = new Date(timestamp);
      if (isNaN(parsed.getTime())) {
        throw new Error(`Invalid date string: ${timestamp}`);
      }
      return parsed;
    }

    if (typeof timestamp === 'number') {
      const date = new Date(timestamp);
      if (isNaN(date.getTime())) {
        throw new Error(`Invalid timestamp number: ${timestamp}`);
      }
      return date;
    }

    throw new Error('Timestamp must be a Date, string, or number');
  }

  /**
   * Normalize provider model name
   */
  static normalizeModelName(model: string): string {
    if (!model || typeof model !== 'string') {
      throw new Error('Model name must be a non-empty string');
    }

    // Remove whitespace and normalize
    return model.trim().toLowerCase();
  }

  /**
   * Normalize and validate input hash
   */
  static normalizeHash(hash: string): string {
    if (!hash || typeof hash !== 'string') {
      throw new Error('Hash must be a non-empty string');
    }

    const trimmed = hash.trim().toLowerCase();

    // Validate SHA-256 format (64 hex characters)
    if (!trimmed.match(/^[a-f0-9]{64}$/)) {
      throw new Error('Hash must be a valid SHA-256 hex string (64 characters)');
    }

    return trimmed;
  }

  /**
   * Normalize output content
   */
  static normalizeOutput(output: any): { type: string; content: string } {
    if (!output || typeof output !== 'object') {
      throw new Error('Output must be an object');
    }

    const type = (output.type || '').toString().trim().toLowerCase();
    const content = (output.content || '').toString().trim();

    if (!type) {
      throw new Error('Output must have a type');
    }

    if (!content) {
      throw new Error('Output must have content');
    }

    return { type, content };
  }

  /**
   * Normalize entire event
   */
  static normalizeEvent(event: any): any {
    return {
      id: event.id,
      timestamp: this.normalizeTimestamp(event.timestamp),
      provider: this.normalizeProvider(event.provider),
      model: this.normalizeModelName(event.model),
      inputType: this.normalizeInputType(event.inputType),
      inputHash: this.normalizeHash(event.inputHash),
      output: this.normalizeOutput(event.output),
      metadata: event.metadata,
    };
  }
}

describe('Normalizer - Input Normalization Tests', () => {
  describe('Provider Name Normalization', () => {
    it('should normalize provider name to uppercase', () => {
      const testCases = [
        { input: 'anthropic', expected: 'ANTHROPIC' },
        { input: 'ANTHROPIC', expected: 'ANTHROPIC' },
        { input: 'Anthropic', expected: 'ANTHROPIC' },
        { input: 'AnThRoPiC', expected: 'ANTHROPIC' },
        { input: 'openai', expected: 'OPENAI' },
        { input: 'OPENAI', expected: 'OPENAI' },
        { input: 'google', expected: 'GOOGLE' },
        { input: 'GOOGLE', expected: 'GOOGLE' },
      ];

      for (const { input, expected } of testCases) {
        expect(InputNormalizer.normalizeProvider(input)).toBe(expected);
      }
    });

    it('should handle provider aliases', () => {
      const testCases = [
        { input: 'claude', expected: 'ANTHROPIC' },
        { input: 'Claude', expected: 'ANTHROPIC' },
        { input: 'gpt', expected: 'OPENAI' },
        { input: 'GPT-4', expected: 'OPENAI' },
        { input: 'palm', expected: 'GOOGLE' },
        { input: 'bard', expected: 'GOOGLE' },
        { input: 'google-palm', expected: 'GOOGLE' },
      ];

      for (const { input, expected } of testCases) {
        expect(InputNormalizer.normalizeProvider(input)).toBe(expected);
      }
    });

    it('should normalize provider with whitespace', () => {
      expect(InputNormalizer.normalizeProvider('  anthropic  ')).toBe('ANTHROPIC');
      expect(InputNormalizer.normalizeProvider('\topenai\n')).toBe('OPENAI');
    });

    it('should default to OTHER for unknown providers', () => {
      const testCases = [
        'unknown_provider',
        'random',
        'test-provider',
        'xyz',
        '',
      ];

      for (const input of testCases) {
        if (input === '') continue; // Skip empty string (will throw)
        expect(InputNormalizer.normalizeProvider(input)).toBe('OTHER');
      }
    });

    it('should throw on empty string', () => {
      expect(() => InputNormalizer.normalizeProvider('')).toThrow();
    });

    it('should throw on non-string input', () => {
      expect(() => InputNormalizer.normalizeProvider(null as any)).toThrow();
      expect(() => InputNormalizer.normalizeProvider(undefined as any)).toThrow();
    });
  });

  describe('Input Type Handling', () => {
    it('should normalize input type to standard format', () => {
      const testCases = [
        { input: 'text', expected: 'TEXT' },
        { input: 'TEXT', expected: 'TEXT' },
        { input: 'chat', expected: 'CHAT' },
        { input: 'CHAT', expected: 'CHAT' },
        { input: 'multimodal', expected: 'MULTIMODAL' },
        { input: 'MULTIMODAL', expected: 'MULTIMODAL' },
      ];

      for (const { input, expected } of testCases) {
        expect(InputNormalizer.normalizeInputType(input)).toBe(expected);
      }
    });

    it('should handle input type aliases', () => {
      const testCases = [
        { input: 'plain-text', expected: 'TEXT' },
        { input: 'raw-text', expected: 'TEXT' },
        { input: 'message', expected: 'CHAT' },
        { input: 'conversation', expected: 'CHAT' },
        { input: 'dialog', expected: 'CHAT' },
        { input: 'image', expected: 'MULTIMODAL' },
        { input: 'audio', expected: 'MULTIMODAL' },
        { input: 'video', expected: 'MULTIMODAL' },
      ];

      for (const { input, expected } of testCases) {
        expect(InputNormalizer.normalizeInputType(input)).toBe(expected);
      }
    });

    it('should default to TEXT for missing input type', () => {
      expect(InputNormalizer.normalizeInputType(undefined)).toBe('TEXT');
      expect(InputNormalizer.normalizeInputType('')).toBe('TEXT');
    });

    it('should default to TEXT for unknown input type', () => {
      expect(InputNormalizer.normalizeInputType('unknown')).toBe('TEXT');
      expect(InputNormalizer.normalizeInputType('xyz')).toBe('TEXT');
    });

    it('should handle input type with whitespace', () => {
      expect(InputNormalizer.normalizeInputType('  text  ')).toBe('TEXT');
      expect(InputNormalizer.normalizeInputType('\nchat\n')).toBe('CHAT');
    });

    it('should handle mixed case input types', () => {
      expect(InputNormalizer.normalizeInputType('TeXt')).toBe('TEXT');
      expect(InputNormalizer.normalizeInputType('ChAt')).toBe('CHAT');
      expect(InputNormalizer.normalizeInputType('MuLtImOdAl')).toBe('MULTIMODAL');
    });
  });

  describe('Timestamp UTC Conversion', () => {
    it('should preserve UTC timestamps', () => {
      const utcDate = new Date('2026-01-19T10:30:45.123Z');
      const normalized = InputNormalizer.normalizeTimestamp(utcDate);

      expect(normalized.getUTCHours()).toBe(utcDate.getUTCHours());
      expect(normalized.getUTCMinutes()).toBe(utcDate.getUTCMinutes());
      expect(normalized.getUTCSeconds()).toBe(utcDate.getUTCSeconds());
      expect(normalized.getUTCMilliseconds()).toBe(utcDate.getUTCMilliseconds());
    });

    it('should handle ISO string timestamps', () => {
      const isoString = '2026-01-19T10:30:45.123Z';
      const normalized = InputNormalizer.normalizeTimestamp(isoString);

      expect(normalized).toBeInstanceOf(Date);
      expect(normalized.toISOString()).toBe(isoString);
    });

    it('should handle Unix timestamp numbers', () => {
      const unixTime = 1737274245123; // 2025-01-19T10:30:45.123Z
      const normalized = InputNormalizer.normalizeTimestamp(unixTime);

      expect(normalized).toBeInstanceOf(Date);
      expect(normalized.getTime()).toBe(unixTime);
    });

    it('should handle various date string formats', () => {
      const testCases = [
        '2026-01-19',
        '2026-01-19T10:30:45Z',
        '2026-01-19T10:30:45.123Z',
        'Jan 19, 2026',
      ];

      for (const dateString of testCases) {
        const normalized = InputNormalizer.normalizeTimestamp(dateString);
        expect(normalized).toBeInstanceOf(Date);
        expect(!isNaN(normalized.getTime())).toBe(true);
      }
    });

    it('should handle edge case dates', () => {
      const testCases = [
        new Date('2026-01-01T00:00:00Z'), // Start of year
        new Date('2026-12-31T23:59:59.999Z'), // End of year
        new Date(0), // Unix epoch
      ];

      for (const date of testCases) {
        const normalized = InputNormalizer.normalizeTimestamp(date);
        expect(normalized).toBeInstanceOf(Date);
        expect(!isNaN(normalized.getTime())).toBe(true);
      }
    });

    it('should throw on invalid date object', () => {
      const invalidDate = new Date('invalid');
      expect(() => InputNormalizer.normalizeTimestamp(invalidDate)).toThrow();
    });

    it('should throw on invalid date string', () => {
      expect(() => InputNormalizer.normalizeTimestamp('not-a-date')).toThrow();
    });

    it('should throw on invalid timestamp number', () => {
      expect(() => InputNormalizer.normalizeTimestamp(NaN)).toThrow();
      expect(() => InputNormalizer.normalizeTimestamp(Infinity)).toThrow();
    });

    it('should throw on invalid type', () => {
      expect(() => InputNormalizer.normalizeTimestamp({} as any)).toThrow();
      expect(() => InputNormalizer.normalizeTimestamp([] as any)).toThrow();
    });
  });

  describe('Edge Cases', () => {
    it('should normalize model name to lowercase', () => {
      expect(InputNormalizer.normalizeModelName('Claude-Opus-4.5')).toBe(
        'claude-opus-4.5'
      );
      expect(InputNormalizer.normalizeModelName('GPT-4')).toBe('gpt-4');
      expect(InputNormalizer.normalizeModelName('PALM-2')).toBe('palm-2');
    });

    it('should handle model name with whitespace', () => {
      expect(InputNormalizer.normalizeModelName('  claude-opus-4.5  ')).toBe(
        'claude-opus-4.5'
      );
    });

    it('should normalize SHA-256 hash to lowercase', () => {
      const hash = 'A'.repeat(64);
      const normalized = InputNormalizer.normalizeHash(hash);
      expect(normalized).toBe('a'.repeat(64));
    });

    it('should validate hash format', () => {
      expect(() => InputNormalizer.normalizeHash('invalid-hash')).toThrow();
      expect(() => InputNormalizer.normalizeHash('a'.repeat(63))).toThrow(); // Too short
      expect(() => InputNormalizer.normalizeHash('a'.repeat(65))).toThrow(); // Too long
      expect(() => InputNormalizer.normalizeHash('z'.repeat(64))).toThrow(); // Invalid hex
    });

    it('should normalize output object', () => {
      const output = {
        type: '  TEXT  ',
        content: '  Response content  ',
      };

      const normalized = InputNormalizer.normalizeOutput(output);
      expect(normalized.type).toBe('text');
      expect(normalized.content).toBe('Response content');
    });

    it('should handle missing output fields gracefully', () => {
      expect(() => InputNormalizer.normalizeOutput({ type: '' })).toThrow();
      expect(() => InputNormalizer.normalizeOutput({ content: 'test' })).toThrow();
      expect(() => InputNormalizer.normalizeOutput({})).toThrow();
    });

    it('should normalize complete event', () => {
      const event = {
        id: '550e8400-e29b-41d4-a716-446655440000',
        timestamp: '2026-01-19T10:30:45Z',
        provider: '  ANTHROPIC  ',
        model: '  Claude-Opus-4.5  ',
        inputType: '  text  ',
        inputHash: 'a'.repeat(64),
        output: {
          type: '  TEXT  ',
          content: '  Response  ',
        },
        metadata: { custom: 'value' },
      };

      const normalized = InputNormalizer.normalizeEvent(event);

      expect(normalized.id).toBe(event.id);
      expect(normalized.provider).toBe('ANTHROPIC');
      expect(normalized.model).toBe('claude-opus-4.5');
      expect(normalized.inputType).toBe('TEXT');
      expect(normalized.output.type).toBe('text');
      expect(normalized.output.content).toBe('Response');
    });

    it('should preserve timestamp precision', () => {
      const originalDate = new Date('2026-01-19T10:30:45.123Z');
      const normalized = InputNormalizer.normalizeTimestamp(originalDate);

      expect(normalized.getTime()).toBe(originalDate.getTime());
    });

    it('should handle very long content strings', () => {
      const longContent = 'x'.repeat(10000);
      const output = {
        type: 'text',
        content: longContent,
      };

      const normalized = InputNormalizer.normalizeOutput(output);
      expect(normalized.content).toBe(longContent);
    });

    it('should handle special characters in content', () => {
      const specialContent = '!@#$%^&*()_+-=[]{}|;:\'",.<>?/~`';
      const output = {
        type: 'text',
        content: specialContent,
      };

      const normalized = InputNormalizer.normalizeOutput(output);
      expect(normalized.content).toBe(specialContent);
    });

    it('should handle unicode characters', () => {
      const unicodeContent = 'Hello 世界 مرحبا мир 🌍';
      const output = {
        type: 'text',
        content: unicodeContent,
      };

      const normalized = InputNormalizer.normalizeOutput(output);
      expect(normalized.content).toBe(unicodeContent);
    });
  });
});
