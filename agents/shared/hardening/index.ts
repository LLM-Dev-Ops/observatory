/**
 * Agent Hardening Module - Phase 1 Layer 1
 *
 * CRITICAL INFRASTRUCTURE FOR ALL AGENTS
 *
 * This module provides:
 * - Mandatory startup assertions
 * - Agent identity standardization
 * - Performance boundaries
 * - Contract assertions
 * - Standardized logging
 */

// =============================================================================
// MANDATORY ENVIRONMENT VARIABLES
// =============================================================================

export interface AgentIdentity {
  agentName: string;
  agentDomain: string;
  agentPhase: 'phase1';
  agentLayer: 'layer1';
}

export interface RuvectorEnvironment {
  serviceUrl: string;
  apiKey: string;
}

export interface HardenedEnvironment {
  identity: AgentIdentity;
  ruvector: RuvectorEnvironment;
}

// =============================================================================
// PERFORMANCE BOUNDARIES (Conservative Defaults)
// =============================================================================

export const PERFORMANCE_BOUNDARIES = {
  MAX_TOKENS: 800,
  MAX_LATENCY_MS: 1500,
  MAX_CALLS_PER_RUN: 2,
} as const;

// =============================================================================
// STARTUP ASSERTIONS
// =============================================================================

/**
 * Assert all mandatory environment variables are present.
 * CRASHES the container if any check fails.
 */
export function assertMandatoryEnvironment(): HardenedEnvironment {
  const errors: string[] = [];

  // Ruvector requirements
  const ruvectorServiceUrl = process.env.RUVECTOR_SERVICE_URL;
  if (!ruvectorServiceUrl) {
    errors.push('RUVECTOR_SERVICE_URL is required');
  }

  const ruvectorApiKey = process.env.RUVECTOR_API_KEY;
  if (!ruvectorApiKey) {
    errors.push('RUVECTOR_API_KEY is required (must be from Google Secret Manager)');
  }

  // Agent identity requirements
  const agentName = process.env.AGENT_NAME;
  if (!agentName) {
    errors.push('AGENT_NAME is required');
  }

  const agentDomain = process.env.AGENT_DOMAIN;
  if (!agentDomain) {
    errors.push('AGENT_DOMAIN is required');
  }

  const agentPhase = process.env.AGENT_PHASE;
  if (agentPhase !== 'phase1') {
    errors.push('AGENT_PHASE must be "phase1"');
  }

  const agentLayer = process.env.AGENT_LAYER;
  if (agentLayer !== 'layer1') {
    errors.push('AGENT_LAYER must be "layer1"');
  }

  if (errors.length > 0) {
    logAgentAbort('startup_assertion_failed', errors);
    process.exit(1);
  }

  return {
    identity: {
      agentName: agentName!,
      agentDomain: agentDomain!,
      agentPhase: 'phase1',
      agentLayer: 'layer1',
    },
    ruvector: {
      serviceUrl: ruvectorServiceUrl!,
      apiKey: ruvectorApiKey!,
    },
  };
}

// =============================================================================
// RUVECTOR HEALTH ASSERTION
// =============================================================================

/**
 * Assert Ruvector service is healthy.
 * CRASHES the container if health check fails.
 */
export async function assertRuvectorHealth(serviceUrl: string, apiKey: string): Promise<void> {
  const startTime = Date.now();

  try {
    const response = await fetch(`${serviceUrl}/health`, {
      method: 'GET',
      headers: {
        'Authorization': `Bearer ${apiKey}`,
        'Content-Type': 'application/json',
      },
      signal: AbortSignal.timeout(5000),
    });

    const latencyMs = Date.now() - startTime;

    if (!response.ok) {
      logAgentAbort('ruvector_health_check_failed', [
        `HTTP ${response.status}`,
        `Latency: ${latencyMs}ms`,
      ]);
      process.exit(1);
    }

    // Lightweight ping successful
    logAgentStarted({ ruvectorLatencyMs: latencyMs });
  } catch (error) {
    logAgentAbort('ruvector_unreachable', [
      error instanceof Error ? error.message : 'Unknown error',
    ]);
    process.exit(1);
  }
}

// =============================================================================
// PERFORMANCE BOUNDARY GUARDS
// =============================================================================

export class PerformanceGuard {
  private callCount: number = 0;
  private startTime: number;

  constructor() {
    this.startTime = Date.now();
  }

  /**
   * Assert we haven't exceeded call limits
   */
  assertCallLimit(): void {
    this.callCount++;
    if (this.callCount > PERFORMANCE_BOUNDARIES.MAX_CALLS_PER_RUN) {
      logAgentAbort('call_limit_exceeded', [
        `Calls: ${this.callCount}`,
        `Max: ${PERFORMANCE_BOUNDARIES.MAX_CALLS_PER_RUN}`,
      ]);
      throw new PerformanceBoundaryError(
        `Call limit exceeded: ${this.callCount} > ${PERFORMANCE_BOUNDARIES.MAX_CALLS_PER_RUN}`
      );
    }
  }

  /**
   * Assert we haven't exceeded latency limits
   */
  assertLatencyLimit(): void {
    const elapsed = Date.now() - this.startTime;
    if (elapsed > PERFORMANCE_BOUNDARIES.MAX_LATENCY_MS) {
      logAgentAbort('latency_limit_exceeded', [
        `Elapsed: ${elapsed}ms`,
        `Max: ${PERFORMANCE_BOUNDARIES.MAX_LATENCY_MS}ms`,
      ]);
      throw new PerformanceBoundaryError(
        `Latency limit exceeded: ${elapsed}ms > ${PERFORMANCE_BOUNDARIES.MAX_LATENCY_MS}ms`
      );
    }
  }

  /**
   * Get elapsed time in milliseconds
   */
  getElapsedMs(): number {
    return Date.now() - this.startTime;
  }

  /**
   * Get current call count
   */
  getCallCount(): number {
    return this.callCount;
  }
}

export class PerformanceBoundaryError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'PerformanceBoundaryError';
  }
}

// =============================================================================
// CONTRACT ASSERTIONS
// =============================================================================

/**
 * Track DecisionEvent emissions for contract assertion
 */
export class ContractAssertions {
  private decisionEventEmitted: boolean = false;
  private ruvectorRequired: boolean = true; // ALWAYS true for Phase 1

  /**
   * Record that a DecisionEvent was emitted
   */
  recordDecisionEventEmitted(executionRef: string, agentName: string): void {
    this.decisionEventEmitted = true;
    logDecisionEventEmitted(executionRef, agentName);
  }

  /**
   * Assert contract requirements are met.
   * Should be called at the end of each run.
   */
  assertContractsMet(): void {
    const errors: string[] = [];

    if (!this.ruvectorRequired) {
      errors.push('Ruvector required assertion failed (must be true)');
    }

    if (!this.decisionEventEmitted) {
      errors.push('At least one DecisionEvent must be emitted per run');
    }

    if (errors.length > 0) {
      logAgentAbort('contract_assertion_failed', errors);
      throw new ContractViolationError(errors.join('; '));
    }
  }

  /**
   * Check if DecisionEvent was emitted (for health checks)
   */
  wasDecisionEventEmitted(): boolean {
    return this.decisionEventEmitted;
  }

  /**
   * Reset for new run (useful for testing)
   */
  reset(): void {
    this.decisionEventEmitted = false;
  }
}

export class ContractViolationError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'ContractViolationError';
  }
}

// =============================================================================
// IN-MEMORY CACHE (Read-Only Operations)
// =============================================================================

interface CacheEntry<T> {
  value: T;
  expiresAt: number;
}

/**
 * Simple in-memory cache for read-only operations
 * TTL: 30-60 seconds (configurable)
 */
export class ReadOnlyCache<T> {
  private cache: Map<string, CacheEntry<T>> = new Map();
  private defaultTtlMs: number;

  constructor(defaultTtlMs: number = 30000) {
    this.defaultTtlMs = defaultTtlMs;
  }

  /**
   * Get cached value if not expired
   */
  get(key: string): T | undefined {
    const entry = this.cache.get(key);
    if (!entry) {
      return undefined;
    }

    if (Date.now() > entry.expiresAt) {
      this.cache.delete(key);
      return undefined;
    }

    return entry.value;
  }

  /**
   * Set cached value with TTL
   */
  set(key: string, value: T, ttlMs?: number): void {
    const ttl = ttlMs ?? this.defaultTtlMs;
    this.cache.set(key, {
      value,
      expiresAt: Date.now() + ttl,
    });
  }

  /**
   * Clear expired entries
   */
  cleanup(): void {
    const now = Date.now();
    for (const [key, entry] of this.cache.entries()) {
      if (now > entry.expiresAt) {
        this.cache.delete(key);
      }
    }
  }

  /**
   * Get cache statistics
   */
  getStats(): { size: number; keys: string[] } {
    return {
      size: this.cache.size,
      keys: Array.from(this.cache.keys()),
    };
  }
}

// =============================================================================
// STANDARDIZED LOGGING (Minimal Observability)
// =============================================================================

interface AgentStartedLogData {
  ruvectorLatencyMs?: number;
}

/**
 * Log agent_started event
 */
export function logAgentStarted(data: AgentStartedLogData = {}): void {
  console.log(JSON.stringify({
    event: 'agent_started',
    timestamp: new Date().toISOString(),
    agent_name: process.env.AGENT_NAME,
    agent_domain: process.env.AGENT_DOMAIN,
    agent_phase: process.env.AGENT_PHASE,
    agent_layer: process.env.AGENT_LAYER,
    ...data,
  }));
}

/**
 * Log decision_event_emitted event
 */
export function logDecisionEventEmitted(executionRef: string, agentName: string): void {
  console.log(JSON.stringify({
    event: 'decision_event_emitted',
    timestamp: new Date().toISOString(),
    execution_ref: executionRef,
    agent_name: agentName,
  }));
}

/**
 * Log agent_abort event
 */
export function logAgentAbort(reason: string, details: string[]): void {
  console.error(JSON.stringify({
    event: 'agent_abort',
    timestamp: new Date().toISOString(),
    reason,
    details,
    agent_name: process.env.AGENT_NAME || 'unknown',
    agent_domain: process.env.AGENT_DOMAIN || 'unknown',
    agent_phase: process.env.AGENT_PHASE || 'unknown',
    agent_layer: process.env.AGENT_LAYER || 'unknown',
  }));
}

// =============================================================================
// DECISION EVENT IDENTITY FIELDS
// =============================================================================

/**
 * Build standardized identity fields for DecisionEvents
 */
export function buildDecisionEventIdentity(env: HardenedEnvironment): DecisionEventIdentity {
  return {
    source_agent: env.identity.agentName,
    domain: env.identity.agentDomain,
    phase: env.identity.agentPhase,
    layer: env.identity.agentLayer,
  };
}

export interface DecisionEventIdentity {
  source_agent: string;
  domain: string;
  phase: 'phase1';
  layer: 'layer1';
}

// =============================================================================
// HARDENED AGENT INITIALIZER
// =============================================================================

export interface HardenedAgentContext {
  environment: HardenedEnvironment;
  performanceGuard: PerformanceGuard;
  contractAssertions: ContractAssertions;
  cache: ReadOnlyCache<unknown>;
}

/**
 * Initialize a hardened agent context.
 * This should be called at the top of the main handler.
 * CRASHES the container if initialization fails.
 */
export async function initializeHardenedAgent(): Promise<HardenedAgentContext> {
  // Assert all mandatory environment variables
  const environment = assertMandatoryEnvironment();

  // Assert Ruvector is healthy
  await assertRuvectorHealth(
    environment.ruvector.serviceUrl,
    environment.ruvector.apiKey
  );

  return {
    environment,
    performanceGuard: new PerformanceGuard(),
    contractAssertions: new ContractAssertions(),
    cache: new ReadOnlyCache(30000), // 30 second TTL
  };
}

// =============================================================================
// EXPORTS
// =============================================================================

export {
  PERFORMANCE_BOUNDARIES as PerformanceBoundaries,
};
