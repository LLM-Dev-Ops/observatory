// Copyright 2025 LLM Observatory Contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * Zod schemas for Telemetry Collector Agent contracts.
 *
 * These schemas define the contract layer between:
 * - Raw telemetry input (LlmSpan from crates/core)
 * - Normalized canonical output format
 * - Decision events for ruvector-service persistence
 *
 * CONSTITUTION: This agent is READ-ONLY, NON-ENFORCING, NON-ANALYTICAL.
 * All schemas enforce constitutional constraints.
 */

import { z } from 'zod';

// ============================================================================
// Provider and Status Enums (matching crates/core/src/types.rs and span.rs)
// ============================================================================

/**
 * LLM provider identifiers (matching Rust Provider enum).
 */
export const ProviderSchema = z.enum([
  'openai',
  'anthropic',
  'google',
  'mistral',
  'cohere',
  'self-hosted',
]).or(z.string()); // Custom providers supported

/**
 * Span status following OpenTelemetry conventions.
 */
export const SpanStatusSchema = z.enum(['OK', 'ERROR', 'UNSET']);

// ============================================================================
// Token Usage (matching crates/core/src/types.rs TokenUsage)
// ============================================================================

export const TokenUsageSchema = z.object({
  prompt_tokens: z.number().int().nonnegative(),
  completion_tokens: z.number().int().nonnegative(),
  total_tokens: z.number().int().nonnegative(),
}).strict();

// ============================================================================
// Cost (matching crates/core/src/types.rs Cost)
// ============================================================================

export const CostSchema = z.object({
  amount_usd: z.number().nonnegative(),
  currency: z.string().default('USD'),
  prompt_cost: z.number().nonnegative().optional(),
  completion_cost: z.number().nonnegative().optional(),
}).strict();

// ============================================================================
// Latency (matching crates/core/src/types.rs Latency)
// ============================================================================

export const LatencySchema = z.object({
  total_ms: z.number().int().nonnegative(),
  ttft_ms: z.number().int().nonnegative().optional(),
  start_time: z.string().datetime(), // ISO 8601 UTC
  end_time: z.string().datetime(),   // ISO 8601 UTC
}).strict();

// ============================================================================
// Metadata (matching crates/core/src/types.rs Metadata)
// ============================================================================

export const MetadataSchema = z.object({
  user_id: z.string().optional(),
  session_id: z.string().optional(),
  request_id: z.string().uuid().optional(),
  environment: z.string().optional(),
  tags: z.array(z.string()).default([]),
  attributes: z.record(z.string(), z.string()).default({}),
}).strict();

// ============================================================================
// LLM Input (matching crates/core/src/span.rs LlmInput)
// ============================================================================

const ChatMessageSchema = z.object({
  role: z.string(),
  content: z.string(),
  name: z.string().optional(),
}).strict();

const ContentPartSchema = z.discriminatedUnion('type', [
  z.object({
    type: z.literal('text'),
    text: z.string(),
  }).strict(),
  z.object({
    type: z.literal('image'),
    source: z.string(),
  }).strict(),
  z.object({
    type: z.literal('audio'),
    source: z.string(),
  }).strict(),
]);

export const LlmInputSchema = z.discriminatedUnion('type', [
  z.object({
    type: z.literal('text'),
    prompt: z.string(),
  }).strict(),
  z.object({
    type: z.literal('chat'),
    messages: z.array(ChatMessageSchema),
  }).strict(),
  z.object({
    type: z.literal('multimodal'),
    parts: z.array(ContentPartSchema),
  }).strict(),
]);

// ============================================================================
// LLM Output (matching crates/core/src/span.rs LlmOutput)
// ============================================================================

export const LlmOutputSchema = z.object({
  content: z.string(),
  finish_reason: z.string().optional(),
  metadata: z.record(z.string(), z.unknown()).default({}),
}).strict();

// ============================================================================
// Span Event (matching crates/core/src/span.rs SpanEvent)
// ============================================================================

export const SpanEventSchema = z.object({
  name: z.string(),
  timestamp: z.string().datetime(),
  attributes: z.record(z.string(), z.unknown()).default({}),
}).strict();

// ============================================================================
// Telemetry Event Schema (Raw Input - matching crates/core/src/span.rs LlmSpan)
// ============================================================================

/**
 * Raw telemetry input schema matching Rust LlmSpan structure.
 * This is the INPUT to the Telemetry Collector Agent.
 */
export const TelemetryEventSchema = z.object({
  span_id: z.string(),
  trace_id: z.string(),
  parent_span_id: z.string().optional(),
  name: z.string(),
  provider: ProviderSchema,
  model: z.string(),
  input: LlmInputSchema,
  output: LlmOutputSchema.optional(),
  token_usage: TokenUsageSchema.optional(),
  cost: CostSchema.optional(),
  latency: LatencySchema,
  metadata: MetadataSchema.default(() => ({ tags: [], attributes: {} })),
  status: SpanStatusSchema.default('UNSET'),
  attributes: z.record(z.string(), z.unknown()).default({}),
  events: z.array(SpanEventSchema).default([]),
}).strict();

// ============================================================================
// Normalized Telemetry Schema (Canonical Output)
// ============================================================================

/**
 * Normalized telemetry schema - canonical output format.
 * This is the OUTPUT from the Telemetry Collector Agent.
 *
 * Normalized fields:
 * - All timestamps are ISO 8601 UTC
 * - All field names are snake_case (matching Rust conventions)
 * - All optional fields explicitly marked
 * - All numeric fields validated for type and range
 */
export const NormalizedTelemetrySchema = z.object({
  // Core identifiers
  span_id: z.string().min(1),
  trace_id: z.string().min(1),
  parent_span_id: z.string().optional(),

  // Operation metadata
  name: z.string().min(1),
  provider: ProviderSchema,
  model: z.string().min(1),

  // Input/Output
  input: LlmInputSchema,
  output: LlmOutputSchema.optional(),

  // Metrics
  token_usage: TokenUsageSchema.optional(),
  cost: CostSchema.optional(),
  latency: LatencySchema,

  // Metadata
  metadata: MetadataSchema,
  status: SpanStatusSchema,

  // OpenTelemetry extensions
  attributes: z.record(z.string(), z.unknown()).default({}),
  events: z.array(SpanEventSchema).default([]),

  // Normalization metadata (added by agent)
  normalized_at: z.string().datetime(), // When normalization occurred
  schema_version: z.string().default('1.0.0'), // Schema version for evolution
}).strict();

// ============================================================================
// Decision Event Schema (for ruvector-service persistence)
// ============================================================================

/**
 * Decision event schema for persistence via ruvector-service.
 *
 * CONSTITUTION CONSTRAINTS:
 * - decision_type: MUST be "telemetry_ingestion" (literal)
 * - confidence: ALWAYS 1.0 (read-only ingestion has perfect confidence)
 * - constraints_applied: ALWAYS [] (read-only agent applies no constraints)
 * - agent_id: Telemetry Collector Agent identifier
 * - agent_version: Agent version for tracking
 */
export const DecisionEventSchema = z.object({
  // Agent metadata
  agent_id: z.string().min(1),
  agent_version: z.string().regex(/^\d+\.\d+\.\d+$/), // Semantic version

  // Decision metadata (CONSTITUTIONAL CONSTRAINTS)
  decision_type: z.literal('telemetry_ingestion'), // MUST be this literal
  confidence: z.literal(1.0), // ALWAYS 1.0 for read-only ingestion
  constraints_applied: z.array(z.never()).length(0), // ALWAYS empty array

  // Input/Output tracking
  inputs_hash: z.string().length(64), // SHA256 hash (hex)
  outputs: z.array(NormalizedTelemetrySchema).min(1),

  // Execution tracking
  execution_ref: z.string().min(1), // Unique execution reference
  timestamp: z.string().datetime(), // ISO 8601 UTC

  // Optional metadata
  metadata: z.record(z.string(), z.unknown()).optional(),
}).strict();

// ============================================================================
// Schema Exports
// ============================================================================

export type TelemetryEvent = z.infer<typeof TelemetryEventSchema>;
export type NormalizedTelemetry = z.infer<typeof NormalizedTelemetrySchema>;
export type DecisionEvent = z.infer<typeof DecisionEventSchema>;
export type TokenUsage = z.infer<typeof TokenUsageSchema>;
export type Cost = z.infer<typeof CostSchema>;
export type Latency = z.infer<typeof LatencySchema>;
export type Metadata = z.infer<typeof MetadataSchema>;
export type LlmInput = z.infer<typeof LlmInputSchema>;
export type LlmOutput = z.infer<typeof LlmOutputSchema>;
export type SpanStatus = z.infer<typeof SpanStatusSchema>;
export type Provider = z.infer<typeof ProviderSchema>;
