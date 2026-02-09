// Copyright 2025 LLM Observatory Contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * Agentic execution context types and helpers for LLM Observatory.
 *
 * This module provides client-side types for the execution context layer
 * that tracks agentic execution flow. This is orthogonal to the existing
 * OpenTelemetry-based LLM tracing -- LLM tracing tracks model calls,
 * execution context tracks which repo/agents are doing work.
 *
 * @example
 * ```typescript
 * const tracker = new ExecutionTracker({
 *   executionId: 'exec-123',
 *   parentSpanId: 'caller-span-456',
 *   repoName: 'llm-observatory',
 * });
 *
 * const agentSpan = tracker.startAgentSpan('analyzer-agent');
 * // ... agent does work ...
 * tracker.attachArtifact(agentSpan.spanId, {
 *   name: 'analysis_report',
 *   contentType: 'application/json',
 *   data: JSON.stringify(report),
 * });
 * tracker.completeAgentSpan(agentSpan.spanId);
 *
 * const result = tracker.finalize(); // validates and returns ExecutionResult
 * ```
 */

import { createHash, randomUUID } from 'crypto';

// ─── Header Constants ───

/** HTTP header names for execution context propagation. */
export const ExecutionHeaders = {
  /** Header carrying the execution ID. */
  EXECUTION_ID: 'x-execution-id',
  /** Header carrying the parent span ID from the caller. */
  PARENT_SPAN_ID: 'x-execution-parent-span-id',
  /** Header carrying the repo name (optional override). */
  REPO_NAME: 'x-execution-repo-name',
} as const;

// ─── Enums ───

/** Discriminates repo-level vs agent-level spans. */
export enum ExecutionSpanKind {
  Repo = 'repo',
  Agent = 'agent',
}

/** Status of an execution span. */
export enum ExecutionSpanStatus {
  Running = 'RUNNING',
  Completed = 'COMPLETED',
  Failed = 'FAILED',
  Cancelled = 'CANCELLED',
}

// ─── Interfaces ───

/** Artifact content: either inline data or a reference URI. */
export type ArtifactContent =
  | { contentLocation: 'inline'; data: string }
  | { contentLocation: 'reference'; uri: string };

/** An artifact produced by an agent and attached to its span. */
export interface Artifact {
  artifactId: string;
  agentSpanId: string;
  name: string;
  contentType: string;
  contentHash: string;
  sizeBytes: number;
  content: ArtifactContent;
  createdAt: string;
  metadata?: Record<string, unknown>;
}

/** A timestamped event within an execution span. */
export interface ExecutionEvent {
  name: string;
  timestamp: string;
  attributes?: Record<string, unknown>;
}

/** A single execution span (repo-level or agent-level). */
export interface ExecutionSpan {
  spanId: string;
  executionId: string;
  parentSpanId: string;
  kind: ExecutionSpanKind;
  repoName: string;
  agentName?: string;
  status: ExecutionSpanStatus;
  startTime: string;
  endTime?: string;
  durationMs?: number;
  artifacts: Artifact[];
  events: ExecutionEvent[];
  attributes: Record<string, unknown>;
  errorMessage?: string;
}

/** Execution context extracted from / injected into HTTP headers. */
export interface ExecutionContext {
  executionId: string;
  parentSpanId: string;
  repoSpanId?: string;
  repoName: string;
}

/** The output contract for a completed execution. */
export interface ExecutionResult {
  executionId: string;
  repoSpan: ExecutionSpan;
  agentSpans: ExecutionSpan[];
  valid: boolean;
  validationErrors: string[];
  totalArtifacts: number;
  totalDurationMs?: number;
}

/** Configuration for the execution context system. */
export interface ExecutionConfig {
  /** Repository name (used in repo-level spans). */
  repoName: string;
  /** Whether to enforce execution context on all operations (default: true). */
  enforce?: boolean;
}

// ─── UUID Helper ───

function generateUUID(): string {
  // crypto.randomUUID() is available since Node 19+
  // For Node 16-18 compatibility, fall back to randomBytes
  if (typeof randomUUID === 'function') {
    return randomUUID();
  }
  // Fallback: generate v4-like UUID from random bytes
  const bytes = createHash('sha256')
    .update(String(Date.now()) + String(Math.random()))
    .digest();
  const hex = bytes.toString('hex').slice(0, 32);
  return [
    hex.slice(0, 8),
    hex.slice(8, 12),
    '4' + hex.slice(13, 16),
    ((parseInt(hex[16], 16) & 0x3) | 0x8).toString(16) + hex.slice(17, 20),
    hex.slice(20, 32),
  ].join('-');
}

// ─── Execution Tracker Class ───

/**
 * Manages execution context and spans within a single execution.
 *
 * Creates a repo-level span on construction, allows starting/completing
 * agent spans, attaching artifacts, and finalizing with validation.
 */
export class ExecutionTracker {
  private repoSpan: ExecutionSpan;
  private agentSpans: Map<string, ExecutionSpan> = new Map();
  private readonly executionId: string;
  private readonly repoName: string;

  constructor(context: ExecutionContext) {
    if (!context.executionId) {
      throw new Error('executionId is required');
    }
    if (!context.parentSpanId) {
      throw new Error('parentSpanId is required');
    }
    if (!context.repoName) {
      throw new Error('repoName is required');
    }

    this.executionId = context.executionId;
    this.repoName = context.repoName;

    const repoSpanId = context.repoSpanId || generateUUID();
    this.repoSpan = {
      spanId: repoSpanId,
      executionId: context.executionId,
      parentSpanId: context.parentSpanId,
      kind: ExecutionSpanKind.Repo,
      repoName: context.repoName,
      status: ExecutionSpanStatus.Running,
      startTime: new Date().toISOString(),
      artifacts: [],
      events: [],
      attributes: {},
    };
  }

  /** Get the repo span ID (use as parent for agent spans). */
  getRepoSpanId(): string {
    return this.repoSpan.spanId;
  }

  /** Get the execution context for propagation to downstream calls. */
  getContext(): ExecutionContext {
    return {
      executionId: this.executionId,
      parentSpanId: this.repoSpan.spanId,
      repoSpanId: this.repoSpan.spanId,
      repoName: this.repoName,
    };
  }

  /** Get HTTP headers for propagating execution context to downstream services. */
  getHeaders(): Record<string, string> {
    return {
      [ExecutionHeaders.EXECUTION_ID]: this.executionId,
      [ExecutionHeaders.PARENT_SPAN_ID]: this.repoSpan.spanId,
      [ExecutionHeaders.REPO_NAME]: this.repoName,
    };
  }

  /** Start a new agent-level span. Each agent MUST have its own span. */
  startAgentSpan(
    agentName: string,
    attributes?: Record<string, unknown>
  ): ExecutionSpan {
    if (!agentName) {
      throw new Error('agentName is required for agent spans');
    }

    const spanId = generateUUID();
    const span: ExecutionSpan = {
      spanId,
      executionId: this.executionId,
      parentSpanId: this.repoSpan.spanId,
      kind: ExecutionSpanKind.Agent,
      repoName: this.repoName,
      agentName,
      status: ExecutionSpanStatus.Running,
      startTime: new Date().toISOString(),
      artifacts: [],
      events: [],
      attributes: attributes || {},
    };
    this.agentSpans.set(spanId, span);
    return { ...span };
  }

  /** Mark an agent span as completed. */
  completeAgentSpan(spanId: string): void {
    const span = this.agentSpans.get(spanId);
    if (!span) {
      throw new Error(`Agent span ${spanId} not found`);
    }
    const now = new Date();
    span.endTime = now.toISOString();
    span.durationMs = now.getTime() - new Date(span.startTime).getTime();
    span.status = ExecutionSpanStatus.Completed;
  }

  /** Mark an agent span as failed. */
  failAgentSpan(spanId: string, errorMessage: string): void {
    const span = this.agentSpans.get(spanId);
    if (!span) {
      throw new Error(`Agent span ${spanId} not found`);
    }
    const now = new Date();
    span.endTime = now.toISOString();
    span.durationMs = now.getTime() - new Date(span.startTime).getTime();
    span.status = ExecutionSpanStatus.Failed;
    span.errorMessage = errorMessage;
  }

  /** Attach an artifact to an agent span. */
  attachArtifact(
    agentSpanId: string,
    artifact: {
      name: string;
      contentType: string;
      data?: string;
      uri?: string;
      metadata?: Record<string, unknown>;
    }
  ): Artifact {
    const span = this.agentSpans.get(agentSpanId);
    if (!span) {
      throw new Error(`Agent span ${agentSpanId} not found`);
    }
    if (span.kind !== ExecutionSpanKind.Agent) {
      throw new Error('Artifacts can only be attached to agent spans');
    }

    const rawContent = artifact.data || artifact.uri || '';
    const contentHash = createHash('sha256').update(rawContent).digest('hex');

    const content: ArtifactContent = artifact.data
      ? { contentLocation: 'inline', data: artifact.data }
      : { contentLocation: 'reference', uri: artifact.uri || '' };

    const result: Artifact = {
      artifactId: generateUUID(),
      agentSpanId,
      name: artifact.name,
      contentType: artifact.contentType,
      contentHash,
      sizeBytes: Buffer.byteLength(rawContent, 'utf-8'),
      content,
      createdAt: new Date().toISOString(),
      metadata: artifact.metadata,
    };

    span.artifacts.push(result);
    return result;
  }

  /** Record an event on an agent span. */
  recordEvent(
    spanId: string,
    name: string,
    attributes?: Record<string, unknown>
  ): void {
    const span = this.agentSpans.get(spanId);
    if (!span) {
      throw new Error(`Agent span ${spanId} not found`);
    }
    span.events.push({
      name,
      timestamp: new Date().toISOString(),
      attributes,
    });
  }

  /**
   * Finalize the execution, completing the repo span and validating.
   *
   * On success: repo span marked COMPLETED if all agent spans completed.
   * On failure: repo span marked FAILED, all spans still returned.
   * Validation ensures the execution graph invariants hold.
   */
  finalize(error?: string): ExecutionResult {
    const now = new Date();
    const agentSpans = Array.from(this.agentSpans.values()).sort(
      (a, b) => new Date(a.startTime).getTime() - new Date(b.startTime).getTime()
    );

    // Determine repo span status
    const anyFailed = agentSpans.some(
      (s) => s.status === ExecutionSpanStatus.Failed
    );
    if (error) {
      this.repoSpan.status = ExecutionSpanStatus.Failed;
      this.repoSpan.errorMessage = error;
    } else if (anyFailed) {
      this.repoSpan.status = ExecutionSpanStatus.Failed;
      this.repoSpan.errorMessage = 'One or more agent spans failed';
    } else {
      this.repoSpan.status = ExecutionSpanStatus.Completed;
    }
    this.repoSpan.endTime = now.toISOString();
    this.repoSpan.durationMs =
      now.getTime() - new Date(this.repoSpan.startTime).getTime();

    // Validate
    const validationErrors: string[] = [];

    if (!this.repoSpan.parentSpanId) {
      validationErrors.push(
        'Repo span is missing parent_span_id from caller'
      );
    }

    if (agentSpans.length === 0) {
      validationErrors.push(
        'No agent spans emitted -- execution has no evidence of agent work'
      );
    }

    for (const agent of agentSpans) {
      if (agent.parentSpanId !== this.repoSpan.spanId) {
        validationErrors.push(
          `Agent span ${agent.spanId} has parentSpanId ${agent.parentSpanId} ` +
            `but expected repo span ${this.repoSpan.spanId}`
        );
      }
    }

    const seenIds = new Set<string>();
    for (const agent of agentSpans) {
      if (seenIds.has(agent.spanId)) {
        validationErrors.push(`Duplicate agent spanId: ${agent.spanId}`);
      }
      seenIds.add(agent.spanId);
    }

    const totalArtifacts = agentSpans.reduce(
      (sum, s) => sum + s.artifacts.length,
      0
    );

    return {
      executionId: this.executionId,
      repoSpan: { ...this.repoSpan },
      agentSpans: agentSpans.map((s) => ({ ...s })),
      valid: validationErrors.length === 0,
      validationErrors,
      totalArtifacts,
      totalDurationMs: this.repoSpan.durationMs,
    };
  }
}

/**
 * Extract execution context from incoming HTTP headers.
 * Returns null if required headers are missing.
 */
export function extractExecutionContext(
  headers: Record<string, string | string[] | undefined>,
  defaultRepoName: string
): ExecutionContext | null {
  const executionId = getHeader(headers, ExecutionHeaders.EXECUTION_ID);
  const parentSpanId = getHeader(headers, ExecutionHeaders.PARENT_SPAN_ID);

  if (!executionId || !parentSpanId) {
    return null;
  }

  const repoName =
    getHeader(headers, ExecutionHeaders.REPO_NAME) || defaultRepoName;

  return {
    executionId,
    parentSpanId,
    repoName,
  };
}

/**
 * Inject execution context into outgoing HTTP headers.
 */
export function injectExecutionHeaders(
  context: ExecutionContext,
  headers: Record<string, string> = {}
): Record<string, string> {
  headers[ExecutionHeaders.EXECUTION_ID] = context.executionId;
  headers[ExecutionHeaders.PARENT_SPAN_ID] = context.parentSpanId;
  if (context.repoName) {
    headers[ExecutionHeaders.REPO_NAME] = context.repoName;
  }
  return headers;
}

function getHeader(
  headers: Record<string, string | string[] | undefined>,
  name: string
): string | undefined {
  const value = headers[name] || headers[name.toLowerCase()];
  if (Array.isArray(value)) return value[0];
  return value as string | undefined;
}
