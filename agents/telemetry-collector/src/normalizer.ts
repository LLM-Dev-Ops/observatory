/**
 * Telemetry normalization module
 * Converts raw telemetry events to canonical schema
 * Copyright 2025 LLM Observatory Contributors
 * SPDX-License-Identifier: Apache-2.0
 */

import { createHash } from 'crypto';
import {
  TelemetryEvent,
  NormalizedTelemetry,
  CanonicalProvider,
  InputType,
  ValidationError,
} from './types/schemas.js';

/**
 * Provider name mapping to canonical format
 */
const PROVIDER_MAPPING: Record<string, CanonicalProvider> = {
  openai: CanonicalProvider.OpenAI,
  'openai-chat': CanonicalProvider.OpenAI,
  'gpt-3.5-turbo': CanonicalProvider.OpenAI,
  'gpt-4': CanonicalProvider.OpenAI,
  anthropic: CanonicalProvider.Anthropic,
  claude: CanonicalProvider.Anthropic,
  google: CanonicalProvider.Google,
  'google-ai': CanonicalProvider.Google,
  gemini: CanonicalProvider.Google,
  mistral: CanonicalProvider.Mistral,
  cohere: CanonicalProvider.Cohere,
  'self-hosted': CanonicalProvider.SelfHosted,
  ollama: CanonicalProvider.SelfHosted,
  'local-model': CanonicalProvider.SelfHosted,
};

/**
 * Normalize a single telemetry event
 */
export function normalizeEvent(raw: TelemetryEvent): NormalizedTelemetry {
  // Validate required fields
  validateEvent(raw);

  // Generate event ID if not provided
  const eventId = raw.eventId || generateEventId();

  // Parse and normalize timestamp
  const timestamp = normalizeTimestamp(raw.timestamp);

  // Normalize provider to canonical format
  const provider = normalizeProvider(raw.provider);
  const originalProvider = raw.provider;

  // Normalize input type
  const inputType = normalizeInputType(raw.inputType);

  // Calculate input hash for deduplication
  const inputHash = calculateInputHash(raw.input);

  // Normalize token usage
  const tokenUsage = normalizeTokenUsage(raw.tokenUsage);

  // Normalize cost
  const cost = normalizeCost(raw.cost);

  // Normalize latency
  const latency = normalizeLatency(raw.latency);

  // Normalize metadata
  const metadata = normalizeMetadata(raw.metadata);

  // Sanitize service identifiers
  const serviceName = sanitizeServiceName(raw.serviceName);
  const serviceVersion = raw.serviceVersion || '1.0.0';

  // Normalize error if present
  const error = raw.error
    ? {
        message: raw.error.message || 'Unknown error',
        type: raw.error.type || 'UnknownError',
        stack: raw.error.stack,
      }
    : undefined;

  // Determine success status
  const statusCode = raw.statusCode || (error ? 500 : 200);
  const success = statusCode >= 200 && statusCode < 400;

  return {
    eventId,
    timestamp,
    provider,
    originalProvider,
    model: raw.model,
    inputType,
    inputHash,
    output: raw.output,
    tokenUsage,
    cost,
    latency,
    metadata,
    serviceName,
    serviceVersion,
    error,
    statusCode,
    success,
  };
}

/**
 * Validate required fields in telemetry event
 */
function validateEvent(event: TelemetryEvent): void {
  const errors: ValidationError[] = [];

  if (!event.provider) {
    errors.push({ field: 'provider', message: 'Provider is required' });
  }

  if (!event.model) {
    errors.push({ field: 'model', message: 'Model is required' });
  }

  if (!event.inputType) {
    errors.push({ field: 'inputType', message: 'Input type is required' });
  }

  if (event.input === undefined || event.input === null) {
    errors.push({ field: 'input', message: 'Input is required' });
  }

  if (errors.length > 0) {
    throw new ValidationEventError('Invalid telemetry event', errors);
  }
}

/**
 * Normalize timestamp to UTC Date object
 */
function normalizeTimestamp(timestamp?: string | Date): Date {
  if (!timestamp) {
    return new Date();
  }

  if (timestamp instanceof Date) {
    return timestamp;
  }

  const parsed = new Date(timestamp);
  if (isNaN(parsed.getTime())) {
    return new Date();
  }

  return parsed;
}

/**
 * Normalize provider name to canonical format
 */
function normalizeProvider(provider: string): CanonicalProvider {
  const normalized = provider.toLowerCase().trim();
  return PROVIDER_MAPPING[normalized] || CanonicalProvider.Unknown;
}

/**
 * Normalize input type
 */
function normalizeInputType(inputType: string | InputType): InputType {
  const normalized = inputType.toLowerCase().trim();

  switch (normalized) {
    case 'text':
    case InputType.Text:
      return InputType.Text;
    case 'chat':
    case InputType.Chat:
      return InputType.Chat;
    case 'multimodal':
    case InputType.Multimodal:
      return InputType.Multimodal;
    default:
      return InputType.Text;
  }
}

/**
 * Calculate SHA-256 hash of input for deduplication
 */
export function calculateInputHash(input: any): string {
  const normalized = typeof input === 'string' ? input : JSON.stringify(input);
  return createHash('sha256').update(normalized).digest('hex');
}

/**
 * Normalize token usage
 */
function normalizeTokenUsage(
  usage?: TelemetryEvent['tokenUsage']
): NormalizedTelemetry['tokenUsage'] {
  return {
    promptTokens: usage?.promptTokens || 0,
    completionTokens: usage?.completionTokens || 0,
    totalTokens:
      usage?.totalTokens || (usage?.promptTokens || 0) + (usage?.completionTokens || 0),
  };
}

/**
 * Normalize cost to USD
 */
function normalizeCost(cost?: TelemetryEvent['cost']): NormalizedTelemetry['cost'] {
  return {
    amountUsd: cost?.amountUsd || 0,
    currency: cost?.currency || 'USD',
    promptCost: cost?.promptCost || 0,
    completionCost: cost?.completionCost || 0,
  };
}

/**
 * Normalize latency metrics
 */
function normalizeLatency(
  latency?: TelemetryEvent['latency']
): NormalizedTelemetry['latency'] {
  const now = new Date();
  const startTime = latency?.startTime ? normalizeTimestamp(latency.startTime) : now;
  const endTime = latency?.endTime ? normalizeTimestamp(latency.endTime) : now;
  const totalMs =
    latency?.totalMs !== undefined ? latency.totalMs : endTime.getTime() - startTime.getTime();

  return {
    totalMs,
    ttftMs: latency?.ttftMs,
    startTime,
    endTime,
  };
}

/**
 * Normalize and sanitize metadata
 */
function normalizeMetadata(
  metadata?: TelemetryEvent['metadata']
): NormalizedTelemetry['metadata'] {
  return {
    userId: metadata?.userId,
    sessionId: metadata?.sessionId,
    requestId: metadata?.requestId,
    environment: metadata?.environment || 'production',
    tags: metadata?.tags || [],
    attributes: metadata?.attributes || {},
  };
}

/**
 * Sanitize service name (remove special characters)
 */
function sanitizeServiceName(serviceName?: string): string {
  if (!serviceName) {
    return 'unknown-service';
  }

  return serviceName
    .replace(/[^a-zA-Z0-9-_]/g, '-')
    .toLowerCase()
    .substring(0, 255);
}

/**
 * Generate unique event ID
 */
function generateEventId(): string {
  const timestamp = Date.now().toString(36);
  const random = Math.random().toString(36).substring(2, 15);
  return `evt_${timestamp}_${random}`;
}

/**
 * Validation error class
 */
export class ValidationEventError extends Error {
  public readonly errors: ValidationError[];

  constructor(message: string, errors: ValidationError[]) {
    super(message);
    this.name = 'ValidationEventError';
    this.errors = errors;
    Error.captureStackTrace(this, this.constructor);
  }
}

/**
 * Batch normalize multiple events
 */
export function normalizeEvents(events: TelemetryEvent[]): {
  normalized: NormalizedTelemetry[];
  errors: Array<{ index: number; error: string }>;
} {
  const normalized: NormalizedTelemetry[] = [];
  const errors: Array<{ index: number; error: string }> = [];

  for (let i = 0; i < events.length; i++) {
    try {
      const normalizedEvent = normalizeEvent(events[i]);
      normalized.push(normalizedEvent);
    } catch (error) {
      errors.push({
        index: i,
        error: error instanceof Error ? error.message : String(error),
      });
    }
  }

  return { normalized, errors };
}
