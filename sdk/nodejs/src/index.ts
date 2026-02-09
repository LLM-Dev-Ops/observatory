// Copyright 2025 LLM Observatory Contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * LLM Observatory Node.js SDK
 *
 * High-performance observability for LLM applications with OpenTelemetry.
 *
 * @example
 * ```typescript
 * import { initObservatory, instrumentOpenAI } from '@llm-observatory/sdk';
 * import OpenAI from 'openai';
 *
 * // Initialize observatory
 * await initObservatory({
 *   serviceName: 'my-llm-app',
 *   otlpEndpoint: 'http://localhost:4317',
 * });
 *
 * // Instrument OpenAI client
 * const openai = new OpenAI({ apiKey: process.env.OPENAI_API_KEY });
 * instrumentOpenAI(openai);
 *
 * // Make LLM calls - automatically traced and cost-tracked
 * const response = await openai.chat.completions.create({
 *   model: 'gpt-4',
 *   messages: [{ role: 'user', content: 'Hello!' }],
 * });
 * ```
 *
 * @packageDocumentation
 */

// Main Observatory class and initialization
export {
  LLMObservatory,
  initObservatory,
  getObservatory,
  shutdownObservatory,
} from './observatory';

// Instrumentation
export { instrumentOpenAI, traced } from './instrument';

// Tracing utilities
export {
  LLMTracer,
  createTracer,
  withSpan,
  traced as tracedDecorator,
  LLMSemanticAttributes,
} from './tracing';

// Cost calculation
export { PricingEngine, pricingDB } from './cost';

// Types
export {
  // Enums
  Provider,
  ErrorType,
  StreamingEvent,

  // Interfaces
  TokenUsage,
  Cost,
  Latency,
  Metadata,
  ObservatoryConfig,
  InstrumentOpenAIOptions,
  LLMSpanData,
  StreamingChunk,
  MiddlewareOptions,
  Pricing,
  SamplingDecision,
  ModelInfo,

  // Error class
  ObservatoryError,
} from './types';

// Execution context (Agentics Foundational Execution Unit)
export {
  ExecutionTracker,
  extractExecutionContext,
  injectExecutionHeaders,
  ExecutionHeaders,
  ExecutionSpanKind,
  ExecutionSpanStatus,
} from './execution';

export type {
  ExecutionContext,
  ExecutionSpan,
  ExecutionResult,
  ExecutionEvent,
  Artifact,
  ArtifactContent,
  ExecutionConfig,
} from './execution';

// Import everything for default export
import {
  initObservatory as init,
  getObservatory as get,
  shutdownObservatory as shutdown,
} from './observatory';
import { instrumentOpenAI as instrument } from './instrument';
import { PricingEngine as Pricing } from './cost';

/**
 * Package version
 */
export const VERSION = '0.1.0';

/**
 * Default export for convenience
 */
export default {
  initObservatory: init,
  getObservatory: get,
  shutdownObservatory: shutdown,
  instrumentOpenAI: instrument,
  PricingEngine: Pricing,
  VERSION,
};
