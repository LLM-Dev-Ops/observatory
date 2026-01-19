/**
 * Failure Classification Agent - Classifier Tests
 *
 * These tests validate the classification engine and its deterministic behavior.
 */

import { ClassificationEngine, classifyFailure } from '../src/classifier';
import type { FailureEvent } from '../contracts';

// =============================================================================
// TEST DATA FACTORIES
// =============================================================================

function createFailureEvent(overrides: Partial<FailureEvent> = {}): FailureEvent {
  return {
    span_id: 'span-test-123',
    trace_id: 'trace-test-456',
    provider: 'openai',
    model: 'gpt-4',
    status: 'ERROR',
    error: {
      message: 'An error occurred',
    },
    latency: {
      start_time: '2024-01-01T00:00:00.000Z',
      end_time: '2024-01-01T00:00:05.000Z',
      duration_ms: 5000,
    },
    timestamp: '2024-01-01T00:00:05.000Z',
    metadata: {},
    events: [],
    attributes: {},
    ...overrides,
  } as FailureEvent;
}

// =============================================================================
// CLASSIFICATION ENGINE TESTS
// =============================================================================

describe('ClassificationEngine', () => {
  let engine: ClassificationEngine;

  beforeEach(() => {
    engine = new ClassificationEngine();
  });

  describe('Network Timeout Classification', () => {
    it('should classify ETIMEDOUT as network_timeout', () => {
      const event = createFailureEvent({
        error: { code: 'ETIMEDOUT', message: 'Request timed out' },
      });

      const result = engine.classify(event);

      expect(result.category).toBe('network_timeout');
      expect(result.severity).toBe('high');
      expect(result.cause).toBe('network');
      expect(result.confidence).toBeGreaterThan(0.9);
    });

    it('should classify timeout message as timeout_request', () => {
      const event = createFailureEvent({
        error: { message: 'Connection timeout after 30 seconds' },
      });

      const result = engine.classify(event);

      expect(result.category).toBe('timeout_request');
      expect(result.cause).toBe('network');
    });
  });

  describe('Connection Error Classification', () => {
    it('should classify ECONNREFUSED as network_connection_refused', () => {
      const event = createFailureEvent({
        error: { code: 'ECONNREFUSED', message: 'Connection refused' },
      });

      const result = engine.classify(event);

      expect(result.category).toBe('network_connection_refused');
      expect(result.severity).toBe('critical');
      expect(result.cause).toBe('network');
    });

    it('should classify ECONNRESET as network_connection_refused', () => {
      const event = createFailureEvent({
        error: { code: 'ECONNRESET', message: 'Connection reset' },
      });

      const result = engine.classify(event);

      expect(result.category).toBe('network_connection_refused');
    });
  });

  describe('Rate Limit Classification', () => {
    it('should classify HTTP 429 as provider_rate_limit', () => {
      const event = createFailureEvent({
        error: { http_status: 429, message: 'Too many requests' },
      });

      const result = engine.classify(event);

      expect(result.category).toBe('provider_rate_limit');
      expect(result.severity).toBe('medium');
      expect(result.cause).toBe('policy');
    });

    it('should classify rate limit message', () => {
      const event = createFailureEvent({
        error: { message: 'Rate limit exceeded, please try again later' },
      });

      const result = engine.classify(event);

      expect(result.category).toBe('provider_rate_limit');
    });
  });

  describe('Authentication Classification', () => {
    it('should classify HTTP 401 as provider_authentication', () => {
      const event = createFailureEvent({
        error: { http_status: 401, message: 'Unauthorized' },
      });

      const result = engine.classify(event);

      expect(result.category).toBe('provider_authentication');
      expect(result.severity).toBe('critical');
      expect(result.cause).toBe('configuration');
    });

    it('should classify HTTP 403 as provider_authorization', () => {
      const event = createFailureEvent({
        error: { http_status: 403, message: 'Forbidden' },
      });

      const result = engine.classify(event);

      expect(result.category).toBe('provider_authorization');
      expect(result.severity).toBe('high');
    });
  });

  describe('Provider Error Classification', () => {
    it('should classify HTTP 500 as provider_internal_error', () => {
      const event = createFailureEvent({
        error: { http_status: 500, message: 'Internal server error' },
      });

      const result = engine.classify(event);

      expect(result.category).toBe('provider_internal_error');
      expect(result.cause).toBe('provider');
    });

    it('should classify HTTP 503 as provider_service_unavailable', () => {
      const event = createFailureEvent({
        error: { http_status: 503, message: 'Service unavailable' },
      });

      const result = engine.classify(event);

      expect(result.category).toBe('provider_service_unavailable');
      expect(result.severity).toBe('critical');
    });

    it('should classify HTTP 502 as provider_service_unavailable', () => {
      const event = createFailureEvent({
        error: { http_status: 502, message: 'Bad gateway' },
      });

      const result = engine.classify(event);

      expect(result.category).toBe('provider_service_unavailable');
    });

    it('should classify overloaded message', () => {
      const event = createFailureEvent({
        error: { message: 'The model is currently overloaded' },
      });

      const result = engine.classify(event);

      expect(result.category).toBe('provider_model_overloaded');
      expect(result.cause).toBe('provider');
    });
  });

  describe('Request Error Classification', () => {
    it('should classify HTTP 400 as request_invalid_payload', () => {
      const event = createFailureEvent({
        error: { http_status: 400, message: 'Bad request' },
      });

      const result = engine.classify(event);

      expect(result.category).toBe('request_invalid_payload');
      expect(result.severity).toBe('low');
      expect(result.cause).toBe('client');
    });

    it('should classify HTTP 413 as request_payload_too_large', () => {
      const event = createFailureEvent({
        error: { http_status: 413, message: 'Payload too large' },
      });

      const result = engine.classify(event);

      expect(result.category).toBe('request_payload_too_large');
    });

    it('should classify context length message', () => {
      const event = createFailureEvent({
        error: { message: 'This model has a maximum context length of 128k tokens' },
      });

      const result = engine.classify(event);

      expect(result.category).toBe('request_context_length_exceeded');
      expect(result.cause).toBe('client');
    });
  });

  describe('Content Filter Classification', () => {
    it('should classify content filter message', () => {
      const event = createFailureEvent({
        error: { message: 'Content filter triggered due to policy violation' },
      });

      const result = engine.classify(event);

      expect(result.category).toBe('request_content_filter');
      expect(result.severity).toBe('informational');
      expect(result.cause).toBe('policy');
    });
  });

  describe('SSL/DNS Classification', () => {
    it('should classify SSL errors', () => {
      const event = createFailureEvent({
        error: { code: 'EPROTO', message: 'SSL handshake failed' },
      });

      const result = engine.classify(event);

      expect(result.category).toBe('network_ssl_handshake');
      expect(result.cause).toBe('configuration');
    });

    it('should classify DNS errors', () => {
      const event = createFailureEvent({
        error: { code: 'ENOTFOUND', message: 'DNS lookup failed' },
      });

      const result = engine.classify(event);

      expect(result.category).toBe('network_dns_resolution');
      expect(result.severity).toBe('critical');
    });
  });

  describe('Unknown Classification', () => {
    it('should classify unrecognized errors as unknown', () => {
      const event = createFailureEvent({
        error: { message: 'Something completely unexpected happened' },
      });

      const result = engine.classify(event);

      expect(result.category).toBe('unknown');
      expect(result.severity).toBe('informational');
      expect(result.cause).toBe('unknown');
      expect(result.confidence).toBe(0);
    });
  });

  describe('Signal Generation', () => {
    it('should generate classification signals', () => {
      const event = createFailureEvent({
        error: { http_status: 429, message: 'Rate limit exceeded' },
      });

      const result = engine.classify(event);

      expect(result.signals.length).toBeGreaterThan(0);
      expect(result.signals[0].signal_type).toContain('rule:');
    });

    it('should include matched rule in signals', () => {
      const event = createFailureEvent({
        error: { code: 'ETIMEDOUT', message: 'Timeout' },
      });

      const result = engine.classify(event);

      expect(result.matchedRules).toContain('network_timeout_001');
    });
  });
});

// =============================================================================
// CLASSIFY FAILURE FUNCTION TESTS
// =============================================================================

describe('classifyFailure', () => {
  let engine: ClassificationEngine;

  beforeEach(() => {
    engine = new ClassificationEngine();
  });

  it('should return complete classification object', async () => {
    const event = createFailureEvent({
      error: { http_status: 500, message: 'Internal error' },
    });

    const classification = await classifyFailure(event, engine);

    expect(classification.span_id).toBe(event.span_id);
    expect(classification.trace_id).toBe(event.trace_id);
    expect(classification.category).toBeDefined();
    expect(classification.severity).toBeDefined();
    expect(classification.cause).toBeDefined();
    expect(classification.confidence).toBeDefined();
    expect(classification.classified_at).toBeDefined();
    expect(classification.classification_latency_ms).toBeGreaterThanOrEqual(0);
    expect(classification.schema_version).toBe('1.0.0');
  });

  it('should generate recommendations', async () => {
    const event = createFailureEvent({
      error: { http_status: 429, message: 'Rate limited' },
    });

    const classification = await classifyFailure(event, engine);

    expect(classification.recommendations).toBeDefined();
    expect(classification.recommendations.length).toBeGreaterThan(0);
  });

  it('should be deterministic', async () => {
    const event = createFailureEvent({
      error: { code: 'ECONNREFUSED', message: 'Connection refused' },
    });

    const classification1 = await classifyFailure(event, engine);
    const classification2 = await classifyFailure(event, engine);

    // Remove timing-dependent fields
    const normalize = (c: any) => ({
      ...c,
      classified_at: undefined,
      classification_latency_ms: undefined,
    });

    expect(normalize(classification1)).toEqual(normalize(classification2));
  });
});

// =============================================================================
// RULE PRIORITY TESTS
// =============================================================================

describe('Rule Priority', () => {
  let engine: ClassificationEngine;

  beforeEach(() => {
    engine = new ClassificationEngine();
  });

  it('should prefer higher priority rules', () => {
    // HTTP status 429 (priority 100) should take precedence over
    // rate limit message (priority 90) when both match
    const event = createFailureEvent({
      error: {
        http_status: 429,
        message: 'Rate limit exceeded',
      },
    });

    const result = engine.classify(event);

    // The HTTP status rule should be matched
    expect(result.matchedRules).toContain('rate_limit_001');
  });
});

// =============================================================================
// EDGE CASE TESTS
// =============================================================================

describe('Edge Cases', () => {
  let engine: ClassificationEngine;

  beforeEach(() => {
    engine = new ClassificationEngine();
  });

  it('should handle null error code gracefully', () => {
    const event = createFailureEvent({
      error: { message: 'Error with no code' },
    });

    expect(() => engine.classify(event)).not.toThrow();
  });

  it('should handle empty error message', () => {
    const event = createFailureEvent({
      error: { message: '' },
    });

    const result = engine.classify(event);
    expect(result.category).toBe('unknown');
  });

  it('should handle case-insensitive matching', () => {
    const event = createFailureEvent({
      error: { message: 'RATE LIMIT EXCEEDED' },
    });

    const result = engine.classify(event);
    expect(result.category).toBe('provider_rate_limit');
  });
});
