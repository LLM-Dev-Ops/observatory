/**
 * Schema definitions for telemetry events and decision events
 * Copyright 2025 LLM Observatory Contributors
 * SPDX-License-Identifier: Apache-2.0
 */

/**
 * Input types supported by the telemetry system
 */
export enum InputType {
  Text = 'text',
  Chat = 'chat',
  Multimodal = 'multimodal',
}

/**
 * Canonical provider names
 */
export enum CanonicalProvider {
  OpenAI = 'openai',
  Anthropic = 'anthropic',
  Google = 'google',
  Mistral = 'mistral',
  Cohere = 'cohere',
  SelfHosted = 'self-hosted',
  Unknown = 'unknown',
}

/**
 * Raw telemetry event as received from clients
 */
export interface TelemetryEvent {
  // Core identifiers
  eventId?: string;
  timestamp?: string | Date;

  // Provider and model
  provider: string;
  model: string;

  // Input data
  inputType: InputType | string;
  input: any; // Can be string, array of messages, or multimodal content

  // Output data
  output?: any;

  // Token usage
  tokenUsage?: {
    promptTokens?: number;
    completionTokens?: number;
    totalTokens?: number;
  };

  // Cost information
  cost?: {
    amountUsd?: number;
    currency?: string;
    promptCost?: number;
    completionCost?: number;
  };

  // Latency metrics
  latency?: {
    totalMs?: number;
    ttftMs?: number;
    startTime?: string | Date;
    endTime?: string | Date;
  };

  // Metadata
  metadata?: {
    userId?: string;
    sessionId?: string;
    requestId?: string;
    environment?: string;
    tags?: string[];
    attributes?: Record<string, string | number | boolean>;
  };

  // Service identifiers
  serviceName?: string;
  serviceVersion?: string;

  // Request parameters
  requestParams?: Record<string, unknown>;

  // Error information
  error?: {
    message?: string;
    type?: string;
    stack?: string;
  };

  // Status
  statusCode?: number;
}

/**
 * Normalized telemetry after processing
 */
export interface NormalizedTelemetry {
  eventId: string;
  timestamp: Date;

  // Normalized provider (canonical format)
  provider: CanonicalProvider;
  originalProvider: string;
  model: string;

  // Input data (normalized)
  inputType: InputType;
  inputHash: string; // SHA-256 hash of inputs for deduplication
  inputSummary?: string; // Optional summary for analytics

  // Output data
  output?: any;
  outputSummary?: string;

  // Token usage (normalized)
  tokenUsage: {
    promptTokens: number;
    completionTokens: number;
    totalTokens: number;
  };

  // Cost (normalized to USD)
  cost: {
    amountUsd: number;
    currency: string;
    promptCost: number;
    completionCost: number;
  };

  // Latency (normalized to milliseconds)
  latency: {
    totalMs: number;
    ttftMs?: number;
    startTime: Date;
    endTime: Date;
  };

  // Metadata (sanitized)
  metadata: {
    userId?: string;
    sessionId?: string;
    requestId?: string;
    environment: string;
    tags: string[];
    attributes: Record<string, string | number | boolean>;
  };

  // Service identifiers (sanitized)
  serviceName: string;
  serviceVersion: string;

  // Error information (if present)
  error?: {
    message: string;
    type: string;
    stack?: string;
  };

  // Status
  statusCode: number;
  success: boolean;
}

/**
 * Decision event for ruvector-service persistence
 * Represents the agent's decision to normalize and persist telemetry
 */
export interface DecisionEvent {
  // Event metadata
  eventId: string;
  eventType: 'telemetry_ingestion';
  timestamp: Date;

  // Agent identity
  agentId: string;
  agentVersion: string;

  // Decision details
  decisionType: 'read_only_observation';
  inputs: TelemetryEvent[];
  outputs: NormalizedTelemetry[];

  // Decision metadata
  confidence: number; // Always 1.0 for read-only agent
  constraintsApplied: string[]; // Empty array per constitution
  executionRef: string; // Unique reference for this execution

  // Processing metrics
  processingTimeMs: number;
  batchSize: number;

  // Self-observation
  selfObservation?: {
    agentTelemetry: {
      ingestionCount: number;
      errorCount: number;
      avgLatencyMs: number;
    };
  };
}

/**
 * Agent telemetry event (self-observation)
 */
export interface AgentTelemetryEvent {
  eventType: 'agent_telemetry';
  agentId: string;
  agentVersion: string;
  timestamp: Date;

  metrics: {
    ingestionCount: number;
    errorCount: number;
    avgLatencyMs: number;
    successRate: number;
  };

  executionRef: string;
}

/**
 * API response for telemetry ingestion
 */
export interface TelemetryIngestionResponse {
  success: boolean;
  processed: number;
  failed: number;
  eventIds: string[];
  errors?: Array<{
    index: number;
    error: string;
  }>;
  executionRef: string;
  processingTimeMs: number;
}

/**
 * Validation error
 */
export interface ValidationError {
  field: string;
  message: string;
  value?: any;
}
