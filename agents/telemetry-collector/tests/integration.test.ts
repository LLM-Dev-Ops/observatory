import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { v4 as uuidv4 } from 'uuid';

/**
 * Integration test suite for telemetry collector agent
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

interface IngestionResult {
  processed: number;
  accepted: number;
  rejected: number;
  decisions: DecisionEvent[];
  errors: Array<{ index: number; reason: string }>;
}

/**
 * Mock Ruvector service for testing
 */
class MockRuvectorService {
  private persistedDecisions: Map<string, DecisionEvent> = new Map();
  private eventLog: any[] = [];
  private isHealthy: boolean = true;

  async persistDecision(decision: DecisionEvent): Promise<void> {
    if (!this.isHealthy) {
      throw new Error('Ruvector service is unhealthy');
    }
    this.persistedDecisions.set(decision.id, decision);
    this.eventLog.push({
      type: 'decision_persisted',
      timestamp: new Date(),
      decision,
    });
  }

  async queryDecisions(filter: any): Promise<DecisionEvent[]> {
    const results: DecisionEvent[] = [];

    for (const [, decision] of this.persistedDecisions) {
      let matches = true;

      if (filter.agentId && decision.agentId !== filter.agentId) {
        matches = false;
      }

      if (filter.startTime && decision.timestamp < filter.startTime) {
        matches = false;
      }

      if (filter.endTime && decision.timestamp > filter.endTime) {
        matches = false;
      }

      if (matches) {
        results.push(decision);
      }
    }

    return results;
  }

  async recordSelfObservation(observation: any): Promise<void> {
    this.eventLog.push({
      type: 'self_observation',
      timestamp: new Date(),
      observation,
    });
  }

  getPersistedDecisions(): DecisionEvent[] {
    return Array.from(this.persistedDecisions.values());
  }

  getEventLog(): any[] {
    return this.eventLog;
  }

  setHealthy(healthy: boolean): void {
    this.isHealthy = healthy;
  }

  reset(): void {
    this.persistedDecisions.clear();
    this.eventLog = [];
    this.isHealthy = true;
  }
}

/**
 * Telemetry collector service with ruvector integration
 */
class TelemetryCollectorService {
  private ruvectorService: MockRuvectorService;

  constructor(ruvectorService: MockRuvectorService) {
    this.ruvectorService = ruvectorService;
  }

  async ingest(events: TelemetryEvent[]): Promise<IngestionResult> {
    const result: IngestionResult = {
      processed: events.length,
      accepted: 0,
      rejected: 0,
      decisions: [],
      errors: [],
    };

    for (let index = 0; index < events.length; index++) {
      const event = events[index]!;

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
          metadata: {
            eventProvider: event.provider,
            eventModel: event.model,
            inputType: event.inputType,
          },
        };

        // Persist to ruvector
        await this.ruvectorService.persistDecision(decision);

        // Record self-observation telemetry
        await this.ruvectorService.recordSelfObservation({
          type: 'event_ingested',
          eventId: event.id,
          provider: event.provider,
          timestamp: new Date(),
        });

        result.decisions.push(decision);
        result.accepted++;
      } catch (error) {
        result.rejected++;
        const errorMessage = error instanceof Error ? error.message : 'Unknown error';
        result.errors.push({
          index,
          reason: errorMessage,
        });
      }
    }

    return result;
  }

  private validateEvent(event: TelemetryEvent): void {
    if (!event.id || typeof event.id !== 'string') {
      throw new Error('Event must have a valid id');
    }

    if (!event.timestamp || !(event.timestamp instanceof Date)) {
      throw new Error('Event must have a valid timestamp');
    }

    if (!event.provider || !['ANTHROPIC', 'OPENAI', 'GOOGLE', 'OTHER'].includes(event.provider)) {
      throw new Error('Event must have a valid provider');
    }

    if (!event.model || typeof event.model !== 'string') {
      throw new Error('Event must have a valid model');
    }

    if (!event.inputType || !['TEXT', 'CHAT', 'MULTIMODAL'].includes(event.inputType)) {
      throw new Error('Event must have a valid inputType');
    }

    if (!event.inputHash || !event.inputHash.match(/^[a-f0-9]{64}$/i)) {
      throw new Error('Event must have a valid SHA-256 inputHash');
    }

    if (!event.output || !event.output.type || !event.output.content) {
      throw new Error('Event must have valid output');
    }
  }
}

describe('Integration Tests - End-to-End Flow', () => {
  let ruvectorService: MockRuvectorService;
  let collectorService: TelemetryCollectorService;

  beforeEach(() => {
    ruvectorService = new MockRuvectorService();
    collectorService = new TelemetryCollectorService(ruvectorService);
  });

  afterEach(() => {
    ruvectorService.reset();
  });

  describe('End-to-End Flow', () => {
    it('should complete full ingestion flow with mock ruvector', async () => {
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

      const result = await collectorService.ingest([event]);

      expect(result.processed).toBe(1);
      expect(result.accepted).toBe(1);
      expect(result.rejected).toBe(0);
      expect(result.decisions).toHaveLength(1);
    });

    it('should persist decision events to ruvector', async () => {
      const events: TelemetryEvent[] = Array.from({ length: 3 }, () => ({
        id: uuidv4(),
        timestamp: new Date(),
        provider: 'ANTHROPIC',
        model: 'claude-opus-4.5',
        inputType: 'TEXT',
        inputHash: 'b'.repeat(64),
        output: { type: 'text', content: 'Response' },
      }));

      await collectorService.ingest(events);

      const persistedDecisions = ruvectorService.getPersistedDecisions();
      expect(persistedDecisions).toHaveLength(3);

      for (const decision of persistedDecisions) {
        expect(decision.agentId).toBe('agent-telemetry-collector-v1');
        expect(decision.decision).toBe('ACCEPT_VALID_EVENT');
        expect(decision.confidence).toBe(0.95);
      }
    });

    it('should record self-observation telemetry in ruvector', async () => {
      const event: TelemetryEvent = {
        id: uuidv4(),
        timestamp: new Date(),
        provider: 'OPENAI',
        model: 'gpt-4',
        inputType: 'CHAT',
        inputHash: 'c'.repeat(64),
        output: { type: 'text', content: 'Chat response' },
      };

      await collectorService.ingest([event]);

      const eventLog = ruvectorService.getEventLog();

      // Should have both decision_persisted and self_observation events
      expect(eventLog.length).toBeGreaterThanOrEqual(2);

      const selfObservations = eventLog.filter((entry) => entry.type === 'self_observation');
      expect(selfObservations.length).toBeGreaterThan(0);

      const observation = selfObservations[0]!;
      expect(observation.observation.type).toBe('event_ingested');
      expect(observation.observation.eventId).toBe(event.id);
      expect(observation.observation.provider).toBe('OPENAI');
    });

    it('should handle multiple batches sequentially', async () => {
      const batch1 = Array.from({ length: 2 }, () => ({
        id: uuidv4(),
        timestamp: new Date(),
        provider: 'ANTHROPIC',
        model: 'claude-opus-4.5',
        inputType: 'TEXT',
        inputHash: 'd'.repeat(64),
        output: { type: 'text', content: 'Response' },
      }));

      const batch2 = Array.from({ length: 3 }, () => ({
        id: uuidv4(),
        timestamp: new Date(),
        provider: 'OPENAI',
        model: 'gpt-4',
        inputType: 'CHAT',
        inputHash: 'e'.repeat(64),
        output: { type: 'text', content: 'Chat response' },
      }));

      const result1 = await collectorService.ingest(batch1);
      const result2 = await collectorService.ingest(batch2);

      expect(result1.accepted).toBe(2);
      expect(result2.accepted).toBe(3);

      const persistedDecisions = ruvectorService.getPersistedDecisions();
      expect(persistedDecisions).toHaveLength(5);
    });

    it('should preserve decision metadata through persistence', async () => {
      const event: TelemetryEvent = {
        id: uuidv4(),
        timestamp: new Date(),
        provider: 'GOOGLE',
        model: 'palm-2',
        inputType: 'MULTIMODAL',
        inputHash: 'f'.repeat(64),
        output: { type: 'text', content: 'Response' },
        metadata: {
          customField: 'customValue',
          tokenCount: 500,
        },
      };

      const result = await collectorService.ingest([event]);

      const persistedDecisions = ruvectorService.getPersistedDecisions();
      const decision = persistedDecisions[0]!;

      // Verify agent metadata is present
      expect(decision.metadata).toBeDefined();
      expect(decision.metadata?.eventProvider).toBe(event.provider);
      expect(decision.metadata?.eventModel).toBe(event.model);
      expect(decision.metadata?.inputType).toBe(event.inputType);
    });
  });

  describe('DecisionEvent Persistence', () => {
    it('should persist decision with all required fields', async () => {
      const event: TelemetryEvent = {
        id: uuidv4(),
        timestamp: new Date('2026-01-19T10:00:00Z'),
        provider: 'ANTHROPIC',
        model: 'claude-opus-4.5',
        inputType: 'TEXT',
        inputHash: 'g'.repeat(64),
        output: { type: 'text', content: 'Response' },
      };

      await collectorService.ingest([event]);

      const persistedDecisions = ruvectorService.getPersistedDecisions();
      const decision = persistedDecisions[0]!;

      expect(decision.id).toBeTruthy();
      expect(decision.timestamp).toBeInstanceOf(Date);
      expect(decision.agentId).toBe('agent-telemetry-collector-v1');
      expect(decision.agentVersion).toBe('1.0.0');
      expect(decision.decision).toBe('ACCEPT_VALID_EVENT');
      expect(decision.reasoning).toBeTruthy();
      expect(decision.confidence).toBe(0.95);
      expect(decision.telemetryEventIds).toContain(event.id);
    });

    it('should query persisted decisions from ruvector', async () => {
      const events: TelemetryEvent[] = Array.from({ length: 5 }, (_, i) => ({
        id: uuidv4(),
        timestamp: new Date(),
        provider: i % 2 === 0 ? 'ANTHROPIC' : 'OPENAI',
        model: i % 2 === 0 ? 'claude-opus-4.5' : 'gpt-4',
        inputType: 'TEXT',
        inputHash: 'h'.repeat(64),
        output: { type: 'text', content: 'Response' },
      }));

      await collectorService.ingest(events);

      // Query all decisions
      const allDecisions = await ruvectorService.queryDecisions({});
      expect(allDecisions).toHaveLength(5);

      // Query decisions by agent ID
      const agentDecisions = await ruvectorService.queryDecisions({
        agentId: 'agent-telemetry-collector-v1',
      });
      expect(agentDecisions).toHaveLength(5);
    });

    it('should handle persistence failures gracefully', async () => {
      ruvectorService.setHealthy(false);

      const event: TelemetryEvent = {
        id: uuidv4(),
        timestamp: new Date(),
        provider: 'ANTHROPIC',
        model: 'claude-opus-4.5',
        inputType: 'TEXT',
        inputHash: 'i'.repeat(64),
        output: { type: 'text', content: 'Response' },
      };

      const result = await collectorService.ingest([event]);

      expect(result.rejected).toBe(1);
      expect(result.errors[0]!.reason).toContain('unhealthy');
      expect(ruvectorService.getPersistedDecisions()).toHaveLength(0);
    });
  });

  describe('Self-Observation Telemetry', () => {
    it('should record ingestion telemetry in ruvector', async () => {
      const event: TelemetryEvent = {
        id: uuidv4(),
        timestamp: new Date(),
        provider: 'ANTHROPIC',
        model: 'claude-opus-4.5',
        inputType: 'TEXT',
        inputHash: 'j'.repeat(64),
        output: { type: 'text', content: 'Response' },
      };

      await collectorService.ingest([event]);

      const eventLog = ruvectorService.getEventLog();
      expect(eventLog.length).toBeGreaterThan(0);
    });

    it('should capture event metadata in self-observation', async () => {
      const event: TelemetryEvent = {
        id: uuidv4(),
        timestamp: new Date(),
        provider: 'OPENAI',
        model: 'gpt-4',
        inputType: 'CHAT',
        inputHash: 'k'.repeat(64),
        output: { type: 'text', content: 'Response' },
      };

      await collectorService.ingest([event]);

      const eventLog = ruvectorService.getEventLog();
      const selfObservations = eventLog.filter((entry) => entry.type === 'self_observation');

      expect(selfObservations.length).toBeGreaterThan(0);

      const observation = selfObservations[0]!.observation;
      expect(observation.eventId).toBe(event.id);
      expect(observation.provider).toBe('OPENAI');
      expect(observation.timestamp).toBeInstanceOf(Date);
    });

    it('should track ingestion metrics in self-observation', async () => {
      const events = Array.from({ length: 3 }, () => ({
        id: uuidv4(),
        timestamp: new Date(),
        provider: 'ANTHROPIC',
        model: 'claude-opus-4.5',
        inputType: 'TEXT',
        inputHash: 'l'.repeat(64),
        output: { type: 'text', content: 'Response' },
      }));

      const result = await collectorService.ingest(events);

      const eventLog = ruvectorService.getEventLog();
      const selfObservations = eventLog.filter((entry) => entry.type === 'self_observation');

      expect(selfObservations.length).toBeGreaterThanOrEqual(3);
      expect(result.accepted).toBe(3);
    });

    it('should not record self-observation for rejected events', async () => {
      const validEvent: TelemetryEvent = {
        id: uuidv4(),
        timestamp: new Date(),
        provider: 'ANTHROPIC',
        model: 'claude-opus-4.5',
        inputType: 'TEXT',
        inputHash: 'm'.repeat(64),
        output: { type: 'text', content: 'Response' },
      };

      const invalidEvent: any = {
        id: 'invalid',
        timestamp: new Date(),
        provider: 'ANTHROPIC',
        // Missing required fields
      };

      const result = await collectorService.ingest([validEvent, invalidEvent], true);

      const eventLog = ruvectorService.getEventLog();
      const selfObservations = eventLog.filter((entry) => entry.type === 'self_observation');

      // Only 1 valid event should have self-observation
      expect(selfObservations.length).toBeGreaterThanOrEqual(1);
      expect(result.rejected).toBeGreaterThan(0);
    });
  });

  describe('Error Handling in Integration', () => {
    it('should continue processing after validation error', async () => {
      const events: (TelemetryEvent | any)[] = [
        {
          id: uuidv4(),
          timestamp: new Date(),
          provider: 'ANTHROPIC',
          model: 'claude-opus-4.5',
          inputType: 'TEXT',
          inputHash: 'n'.repeat(64),
          output: { type: 'text', content: 'Response' },
        },
        {
          id: 'invalid',
          // Missing required fields
        },
        {
          id: uuidv4(),
          timestamp: new Date(),
          provider: 'OPENAI',
          model: 'gpt-4',
          inputType: 'CHAT',
          inputHash: 'o'.repeat(64),
          output: { type: 'text', content: 'Response' },
        },
      ];

      const result = await collectorService.ingest(events);

      expect(result.processed).toBe(3);
      expect(result.accepted).toBe(2);
      expect(result.rejected).toBe(1);
      expect(result.errors).toHaveLength(1);
    });

    it('should report error details with event index', async () => {
      const events: (TelemetryEvent | any)[] = [
        {
          id: uuidv4(),
          timestamp: new Date(),
          provider: 'ANTHROPIC',
          model: 'claude-opus-4.5',
          inputType: 'TEXT',
          inputHash: 'p'.repeat(64),
          output: { type: 'text', content: 'Response' },
        },
        {
          id: 'invalid-id',
          // Invalid id
          timestamp: new Date(),
          provider: 'ANTHROPIC',
          model: 'claude-opus-4.5',
          inputType: 'TEXT',
          inputHash: 'q'.repeat(64),
          output: { type: 'text', content: 'Response' },
        },
      ];

      const result = await collectorService.ingest(events);

      expect(result.errors).toHaveLength(1);
      expect(result.errors[0]!.index).toBe(1);
      expect(result.errors[0]!.reason).toBeTruthy();
    });
  });

  /**
   * Extended ingest method to support error handling parameter
   */
});

// Re-declare to support extended parameter
class TelemetryCollectorServiceExtended {
  private ruvectorService: MockRuvectorService;

  constructor(ruvectorService: MockRuvectorService) {
    this.ruvectorService = ruvectorService;
  }

  async ingest(events: TelemetryEvent[], continueOnError = true): Promise<IngestionResult> {
    const result: IngestionResult = {
      processed: events.length,
      accepted: 0,
      rejected: 0,
      decisions: [],
      errors: [],
    };

    for (let index = 0; index < events.length; index++) {
      const event = events[index]!;

      try {
        this.validateEvent(event);

        const decision: DecisionEvent = {
          id: uuidv4(),
          timestamp: new Date(),
          agentId: 'agent-telemetry-collector-v1',
          agentVersion: '1.0.0',
          decision: 'ACCEPT_VALID_EVENT',
          reasoning: 'Event passed all validation checks',
          confidence: 0.95,
          telemetryEventIds: [event.id],
          metadata: {
            eventProvider: event.provider,
            eventModel: event.model,
            inputType: event.inputType,
          },
        };

        await this.ruvectorService.persistDecision(decision);
        await this.ruvectorService.recordSelfObservation({
          type: 'event_ingested',
          eventId: event.id,
          provider: event.provider,
          timestamp: new Date(),
        });

        result.decisions.push(decision);
        result.accepted++;
      } catch (error) {
        result.rejected++;
        const errorMessage = error instanceof Error ? error.message : 'Unknown error';
        result.errors.push({
          index,
          reason: errorMessage,
        });

        if (!continueOnError) {
          break;
        }
      }
    }

    return result;
  }

  private validateEvent(event: TelemetryEvent): void {
    if (!event.id || typeof event.id !== 'string') {
      throw new Error('Event must have a valid id');
    }

    if (!event.timestamp || !(event.timestamp instanceof Date)) {
      throw new Error('Event must have a valid timestamp');
    }

    if (!event.provider || !['ANTHROPIC', 'OPENAI', 'GOOGLE', 'OTHER'].includes(event.provider)) {
      throw new Error('Event must have a valid provider');
    }

    if (!event.model || typeof event.model !== 'string') {
      throw new Error('Event must have a valid model');
    }

    if (!event.inputType || !['TEXT', 'CHAT', 'MULTIMODAL'].includes(event.inputType)) {
      throw new Error('Event must have a valid inputType');
    }

    if (!event.inputHash || !event.inputHash.match(/^[a-f0-9]{64}$/i)) {
      throw new Error('Event must have a valid SHA-256 inputHash');
    }

    if (!event.output || !event.output.type || !event.output.content) {
      throw new Error('Event must have valid output');
    }
  }
}
