import { describe, it, expect, beforeEach, vi } from 'vitest';
import { v4 as uuidv4 } from 'uuid';

/**
 * Mock types for testing
 */
interface TelemetryEvent {
  id: string;
  timestamp: Date;
  provider: string;
  model: string;
  inputType: string;
  inputHash: string;
  output: {
    type: string;
    content: string;
  };
  metadata?: Record<string, unknown>;
}

interface DecisionEvent {
  id: string;
  timestamp: Date;
  agentId: string;
  agentVersion: string;
  decision: string;
  reasoning: string;
  confidence: number;
  telemetryEventIds: string[];
  metadata?: Record<string, unknown>;
}

interface IngestRequest {
  events: TelemetryEvent[];
  options?: {
    failFast?: boolean;
    continueOnError?: boolean;
  };
}

interface IngestResponse {
  processed: number;
  accepted: number;
  rejected: number;
  decisions: DecisionEvent[];
  errors: Array<{ index: number; reason: string }>;
}

/**
 * Handler implementation for testing
 */
class TelemetryHandler {
  private ruvectorClient: MockRuvectorClient;

  constructor() {
    this.ruvectorClient = new MockRuvectorClient();
  }

  async ingest(request: IngestRequest): Promise<IngestResponse> {
    const response: IngestResponse = {
      processed: request.events.length,
      accepted: 0,
      rejected: 0,
      decisions: [],
      errors: [],
    };

    const failFast = request.options?.failFast ?? false;

    for (let index = 0; index < request.events.length; index++) {
      const event = request.events[index]!;

      try {
        // Validate event
        this.validateEvent(event);

        // Create decision event
        const decision: DecisionEvent = {
          id: uuidv4(),
          timestamp: new Date(),
          agentId: 'agent-telemetry-collector-v1',
          agentVersion: '1.0.0',
          decision: 'ACCEPT_VALID_EVENT',
          reasoning: 'Event passed all validation checks',
          confidence: 0.95,
          telemetryEventIds: [event.id],
        };

        // Persist decision event to ruvector
        const persistResult = await this.ruvectorClient.persistDecision(decision);
        if (!persistResult.success) {
          throw new Error(`Failed to persist decision: ${persistResult.error}`);
        }

        response.decisions.push(decision);
        response.accepted++;
      } catch (error) {
        response.rejected++;
        const errorMessage = error instanceof Error ? error.message : 'Unknown error';
        response.errors.push({
          index,
          reason: errorMessage,
        });

        if (failFast) {
          break;
        }
      }
    }

    return response;
  }

  async ingestBatch(events: TelemetryEvent[]): Promise<IngestResponse> {
    return this.ingest({ events });
  }

  private validateEvent(event: TelemetryEvent): void {
    // Check required fields
    if (!event.id || typeof event.id !== 'string') {
      throw new Error('Event must have a valid id');
    }

    if (!event.timestamp || !(event.timestamp instanceof Date)) {
      throw new Error('Event must have a valid timestamp');
    }

    if (!event.provider || typeof event.provider !== 'string') {
      throw new Error('Event must have a valid provider');
    }

    if (!event.model || typeof event.model !== 'string') {
      throw new Error('Event must have a valid model');
    }

    if (!event.inputType || typeof event.inputType !== 'string') {
      throw new Error('Event must have a valid inputType');
    }

    if (!event.inputHash || typeof event.inputHash !== 'string') {
      throw new Error('Event must have a valid inputHash');
    }

    // Validate hash format (SHA-256 hex)
    if (!event.inputHash.match(/^[a-f0-9]{64}$/i)) {
      throw new Error('Event inputHash must be a valid SHA-256 hex string');
    }

    // Validate output
    if (!event.output || typeof event.output !== 'object') {
      throw new Error('Event must have a valid output object');
    }

    if (!event.output.type || typeof event.output.type !== 'string') {
      throw new Error('Event output must have a type');
    }

    if (!event.output.content || typeof event.output.content !== 'string') {
      throw new Error('Event output must have content');
    }

    // Validate provider is known
    const validProviders = ['ANTHROPIC', 'OPENAI', 'GOOGLE', 'OTHER'];
    if (!validProviders.includes(event.provider.toUpperCase())) {
      throw new Error(`Event provider must be one of: ${validProviders.join(', ')}`);
    }

    // Validate inputType is known
    const validInputTypes = ['TEXT', 'CHAT', 'MULTIMODAL'];
    if (!validInputTypes.includes(event.inputType.toUpperCase())) {
      throw new Error(`Event inputType must be one of: ${validInputTypes.join(', ')}`);
    }

    // Validate timestamp is not in future (with 1 minute tolerance for clock skew)
    const oneMinuteMs = 60000;
    if (event.timestamp.getTime() > Date.now() + oneMinuteMs) {
      throw new Error('Event timestamp cannot be in the future');
    }
  }

  getRuvectorClient(): MockRuvectorClient {
    return this.ruvectorClient;
  }
}

/**
 * Mock Ruvector client
 */
class MockRuvectorClient {
  private decisions: DecisionEvent[] = [];
  private persistDelay: number = 0;
  private shouldFail: boolean = false;
  private failureReason: string = '';

  async persistDecision(
    decision: DecisionEvent
  ): Promise<{ success: boolean; error?: string }> {
    if (this.persistDelay > 0) {
      await new Promise((resolve) => setTimeout(resolve, this.persistDelay));
    }

    if (this.shouldFail) {
      return { success: false, error: this.failureReason };
    }

    this.decisions.push(decision);
    return { success: true };
  }

  getDecisions(): DecisionEvent[] {
    return this.decisions;
  }

  setFailure(shouldFail: boolean, reason: string = 'Simulated failure'): void {
    this.shouldFail = shouldFail;
    this.failureReason = reason;
  }

  setPersistDelay(delayMs: number): void {
    this.persistDelay = delayMs;
  }

  reset(): void {
    this.decisions = [];
    this.shouldFail = false;
    this.failureReason = '';
    this.persistDelay = 0;
  }
}

describe('Handler - Edge Function Tests', () => {
  let handler: TelemetryHandler;
  let ruvectorClient: MockRuvectorClient;

  beforeEach(() => {
    handler = new TelemetryHandler();
    ruvectorClient = handler.getRuvectorClient();
    ruvectorClient.reset();
  });

  describe('Successful Ingestion', () => {
    it('should accept and process a valid telemetry event', async () => {
      const event: TelemetryEvent = {
        id: uuidv4(),
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

      const response = await handler.ingest({ events: [event] });

      expect(response.processed).toBe(1);
      expect(response.accepted).toBe(1);
      expect(response.rejected).toBe(0);
      expect(response.errors).toHaveLength(0);
      expect(response.decisions).toHaveLength(1);
    });

    it('should create decision event with correct structure', async () => {
      const event: TelemetryEvent = {
        id: uuidv4(),
        timestamp: new Date(),
        provider: 'OPENAI',
        model: 'gpt-4',
        inputType: 'CHAT',
        inputHash: 'b'.repeat(64),
        output: {
          type: 'text',
          content: 'Chat response',
        },
      };

      const response = await handler.ingest({ events: [event] });

      expect(response.decisions).toHaveLength(1);
      const decision = response.decisions[0]!;

      expect(decision.id).toBeTruthy();
      expect(decision.timestamp).toBeInstanceOf(Date);
      expect(decision.agentId).toBe('agent-telemetry-collector-v1');
      expect(decision.agentVersion).toBe('1.0.0');
      expect(decision.decision).toBe('ACCEPT_VALID_EVENT');
      expect(decision.confidence).toBe(0.95);
      expect(decision.telemetryEventIds).toContain(event.id);
    });

    it('should persist decision event to ruvector', async () => {
      const event: TelemetryEvent = {
        id: uuidv4(),
        timestamp: new Date(),
        provider: 'GOOGLE',
        model: 'palm-2',
        inputType: 'MULTIMODAL',
        inputHash: 'c'.repeat(64),
        output: {
          type: 'text',
          content: 'Response',
        },
      };

      await handler.ingest({ events: [event] });

      const persistedDecisions = ruvectorClient.getDecisions();
      expect(persistedDecisions).toHaveLength(1);
      expect(persistedDecisions[0]!.telemetryEventIds).toContain(event.id);
    });

    it('should handle events with optional metadata', async () => {
      const event: TelemetryEvent = {
        id: uuidv4(),
        timestamp: new Date(),
        provider: 'ANTHROPIC',
        model: 'claude-opus-4.5',
        inputType: 'TEXT',
        inputHash: 'd'.repeat(64),
        output: {
          type: 'text',
          content: 'Response',
        },
        metadata: {
          tokenCount: 250,
          processingTimeMs: 125,
          customField: 'customValue',
        },
      };

      const response = await handler.ingest({ events: [event] });

      expect(response.accepted).toBe(1);
      expect(response.decisions).toHaveLength(1);
    });
  });

  describe('Batch Processing', () => {
    it('should process multiple events in batch', async () => {
      const events: TelemetryEvent[] = Array.from({ length: 5 }, () => ({
        id: uuidv4(),
        timestamp: new Date(),
        provider: 'ANTHROPIC',
        model: 'claude-opus-4.5',
        inputType: 'TEXT',
        inputHash: 'e'.repeat(64),
        output: {
          type: 'text',
          content: 'Response',
        },
      }));

      const response = await handler.ingestBatch(events);

      expect(response.processed).toBe(5);
      expect(response.accepted).toBe(5);
      expect(response.rejected).toBe(0);
      expect(response.decisions).toHaveLength(5);
    });

    it('should process partial batch with some invalid events', async () => {
      const events: (TelemetryEvent | Partial<TelemetryEvent>)[] = [
        {
          id: uuidv4(),
          timestamp: new Date(),
          provider: 'ANTHROPIC',
          model: 'claude-opus-4.5',
          inputType: 'TEXT',
          inputHash: 'f'.repeat(64),
          output: { type: 'text', content: 'Response' },
        },
        {
          id: 'invalid-id',
          timestamp: new Date(),
          provider: 'ANTHROPIC',
          model: 'claude-opus-4.5',
          inputType: 'TEXT',
          inputHash: 'g'.repeat(64),
          output: { type: 'text', content: 'Response' },
        },
        {
          id: uuidv4(),
          timestamp: new Date(),
          provider: 'OPENAI',
          model: 'gpt-4',
          inputType: 'CHAT',
          inputHash: 'h'.repeat(64),
          output: { type: 'text', content: 'Response' },
        },
      ];

      const response = await handler.ingest({
        events: events as TelemetryEvent[],
        options: { continueOnError: true },
      });

      expect(response.processed).toBe(3);
      expect(response.accepted).toBe(2);
      expect(response.rejected).toBe(1);
      expect(response.errors).toHaveLength(1);
      expect(response.errors[0]!.index).toBe(1);
    });

    it('should respect failFast option', async () => {
      const events: (TelemetryEvent | Partial<TelemetryEvent>)[] = [
        {
          id: uuidv4(),
          timestamp: new Date(),
          provider: 'ANTHROPIC',
          model: 'claude-opus-4.5',
          inputType: 'TEXT',
          inputHash: 'i'.repeat(64),
          output: { type: 'text', content: 'Response' },
        },
        {
          // Missing required field
          id: uuidv4(),
          timestamp: new Date(),
          provider: 'ANTHROPIC',
          model: 'claude-opus-4.5',
          inputType: 'TEXT',
          // Missing inputHash
          output: { type: 'text', content: 'Response' },
        },
        {
          id: uuidv4(),
          timestamp: new Date(),
          provider: 'OPENAI',
          model: 'gpt-4',
          inputType: 'CHAT',
          inputHash: 'j'.repeat(64),
          output: { type: 'text', content: 'Response' },
        },
      ];

      const response = await handler.ingest({
        events: events as TelemetryEvent[],
        options: { failFast: true },
      });

      // Should stop at first error
      expect(response.accepted).toBe(1);
      expect(response.rejected).toBe(1);
      // Third event should not be processed
      expect(response.processed).toBe(2);
    });

    it('should handle empty batch', async () => {
      const response = await handler.ingest({ events: [] });

      expect(response.processed).toBe(0);
      expect(response.accepted).toBe(0);
      expect(response.rejected).toBe(0);
      expect(response.decisions).toHaveLength(0);
      expect(response.errors).toHaveLength(0);
    });
  });

  describe('Error Handling', () => {
    it('should reject malformed JSON input', async () => {
      const malformedEvent: any = {
        // Missing required fields
        provider: 'ANTHROPIC',
      };

      const response = await handler.ingest({
        events: [malformedEvent],
      });

      expect(response.rejected).toBe(1);
      expect(response.errors).toHaveLength(1);
      expect(response.errors[0]!.reason).toBeTruthy();
    });

    it('should reject event with invalid hash format', async () => {
      const event: Partial<TelemetryEvent> = {
        id: uuidv4(),
        timestamp: new Date(),
        provider: 'ANTHROPIC',
        model: 'claude-opus-4.5',
        inputType: 'TEXT',
        inputHash: 'not-valid-hex', // Invalid format
        output: { type: 'text', content: 'Response' },
      };

      const response = await handler.ingest({
        events: [event as TelemetryEvent],
      });

      expect(response.rejected).toBe(1);
      expect(response.errors[0]!.reason).toContain('SHA-256');
    });

    it('should reject event with invalid provider', async () => {
      const event: Partial<TelemetryEvent> = {
        id: uuidv4(),
        timestamp: new Date(),
        provider: 'INVALID_PROVIDER',
        model: 'claude-opus-4.5',
        inputType: 'TEXT',
        inputHash: 'k'.repeat(64),
        output: { type: 'text', content: 'Response' },
      };

      const response = await handler.ingest({
        events: [event as TelemetryEvent],
      });

      expect(response.rejected).toBe(1);
      expect(response.errors[0]!.reason).toContain('provider');
    });

    it('should reject event with invalid inputType', async () => {
      const event: Partial<TelemetryEvent> = {
        id: uuidv4(),
        timestamp: new Date(),
        provider: 'ANTHROPIC',
        model: 'claude-opus-4.5',
        inputType: 'INVALID_TYPE',
        inputHash: 'l'.repeat(64),
        output: { type: 'text', content: 'Response' },
      };

      const response = await handler.ingest({
        events: [event as TelemetryEvent],
      });

      expect(response.rejected).toBe(1);
      expect(response.errors[0]!.reason).toContain('inputType');
    });

    it('should reject event with future timestamp', async () => {
      const futureDate = new Date();
      futureDate.setMinutes(futureDate.getMinutes() + 5);

      const event: TelemetryEvent = {
        id: uuidv4(),
        timestamp: futureDate,
        provider: 'ANTHROPIC',
        model: 'claude-opus-4.5',
        inputType: 'TEXT',
        inputHash: 'm'.repeat(64),
        output: { type: 'text', content: 'Response' },
      };

      const response = await handler.ingest({
        events: [event],
      });

      expect(response.rejected).toBe(1);
      expect(response.errors[0]!.reason).toContain('future');
    });

    it('should handle ruvector persistence failures', async () => {
      ruvectorClient.setFailure(true, 'Connection timeout');

      const event: TelemetryEvent = {
        id: uuidv4(),
        timestamp: new Date(),
        provider: 'ANTHROPIC',
        model: 'claude-opus-4.5',
        inputType: 'TEXT',
        inputHash: 'n'.repeat(64),
        output: { type: 'text', content: 'Response' },
      };

      const response = await handler.ingest({
        events: [event],
      });

      expect(response.rejected).toBe(1);
      expect(response.errors[0]!.reason).toContain('persist');
    });
  });

  describe('Validation Failures', () => {
    it('should reject event with missing id', async () => {
      const event: Partial<TelemetryEvent> = {
        timestamp: new Date(),
        provider: 'ANTHROPIC',
        model: 'claude-opus-4.5',
        inputType: 'TEXT',
        inputHash: 'o'.repeat(64),
        output: { type: 'text', content: 'Response' },
      };

      const response = await handler.ingest({
        events: [event as TelemetryEvent],
      });

      expect(response.rejected).toBe(1);
      expect(response.errors[0]!.reason).toContain('id');
    });

    it('should reject event with missing timestamp', async () => {
      const event: Partial<TelemetryEvent> = {
        id: uuidv4(),
        provider: 'ANTHROPIC',
        model: 'claude-opus-4.5',
        inputType: 'TEXT',
        inputHash: 'p'.repeat(64),
        output: { type: 'text', content: 'Response' },
      };

      const response = await handler.ingest({
        events: [event as TelemetryEvent],
      });

      expect(response.rejected).toBe(1);
      expect(response.errors[0]!.reason).toContain('timestamp');
    });

    it('should reject event with missing output', async () => {
      const event: Partial<TelemetryEvent> = {
        id: uuidv4(),
        timestamp: new Date(),
        provider: 'ANTHROPIC',
        model: 'claude-opus-4.5',
        inputType: 'TEXT',
        inputHash: 'q'.repeat(64),
      };

      const response = await handler.ingest({
        events: [event as TelemetryEvent],
      });

      expect(response.rejected).toBe(1);
      expect(response.errors[0]!.reason).toContain('output');
    });

    it('should reject event with invalid output structure', async () => {
      const event: Partial<TelemetryEvent> = {
        id: uuidv4(),
        timestamp: new Date(),
        provider: 'ANTHROPIC',
        model: 'claude-opus-4.5',
        inputType: 'TEXT',
        inputHash: 'r'.repeat(64),
        output: {
          content: 'Response',
          // Missing type
        } as any,
      };

      const response = await handler.ingest({
        events: [event as TelemetryEvent],
      });

      expect(response.rejected).toBe(1);
      expect(response.errors[0]!.reason).toContain('type');
    });
  });

  describe('Deterministic Output', () => {
    it('should produce identical decision events for identical inputs', async () => {
      const event: TelemetryEvent = {
        id: uuidv4(),
        timestamp: new Date('2026-01-19T10:00:00Z'),
        provider: 'ANTHROPIC',
        model: 'claude-opus-4.5',
        inputType: 'TEXT',
        inputHash: 's'.repeat(64),
        output: { type: 'text', content: 'Response' },
      };

      const response1 = await handler.ingest({ events: [event] });
      const response2 = await handler.ingest({ events: [event] });

      expect(response1.accepted).toBe(response2.accepted);
      expect(response1.decisions[0]!.decision).toBe(response2.decisions[0]!.decision);
      expect(response1.decisions[0]!.reasoning).toBe(response2.decisions[0]!.reasoning);
      expect(response1.decisions[0]!.confidence).toBe(response2.decisions[0]!.confidence);
    });

    it('should use consistent agent metadata in decisions', async () => {
      const events: TelemetryEvent[] = Array.from({ length: 3 }, () => ({
        id: uuidv4(),
        timestamp: new Date(),
        provider: 'ANTHROPIC',
        model: 'claude-opus-4.5',
        inputType: 'TEXT',
        inputHash: 't'.repeat(64),
        output: { type: 'text', content: 'Response' },
      }));

      const response = await handler.ingest({ events });

      // All decisions should have same agent metadata
      for (const decision of response.decisions) {
        expect(decision.agentId).toBe('agent-telemetry-collector-v1');
        expect(decision.agentVersion).toBe('1.0.0');
      }
    });
  });
});
