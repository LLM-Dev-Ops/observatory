/**
 * Failure Classification Engine
 *
 * CONSTITUTIONAL CLASSIFICATION: READ-ONLY, DIAGNOSTIC
 *
 * Implements deterministic classification logic for failure events.
 * This module ONLY classifies - it does NOT take any actions.
 */

import type {
  FailureEvent,
  FailureClassification,
  FailureCategory,
  FailureSeverity,
  FailureCause,
  ClassificationSignal,
  ClassificationRule,
} from '../contracts';

// =============================================================================
// CLASSIFICATION RULES
// =============================================================================

const CLASSIFICATION_RULES: ClassificationRule[] = [
  // Network timeout rules
  {
    id: 'network_timeout_001',
    name: 'Request Timeout',
    description: 'Request timed out before receiving response',
    conditions: [
      { field: 'error.code', operator: 'in', value: ['ETIMEDOUT', 'ESOCKETTIMEDOUT', 'TIMEOUT', 'timeout'] },
    ],
    output: {
      category: 'network_timeout',
      severity: 'high',
      cause: 'network',
    },
    priority: 100,
    confidence_base: 0.95,
  },
  {
    id: 'network_timeout_002',
    name: 'Request Timeout by Message',
    description: 'Timeout detected from error message',
    conditions: [
      { field: 'error.message', operator: 'contains', value: 'timeout' },
    ],
    output: {
      category: 'timeout_request',
      severity: 'high',
      cause: 'network',
    },
    priority: 90,
    confidence_base: 0.85,
  },

  // Connection refused rules
  {
    id: 'network_connection_001',
    name: 'Connection Refused',
    description: 'Connection to provider was refused',
    conditions: [
      { field: 'error.code', operator: 'in', value: ['ECONNREFUSED', 'ECONNRESET', 'ENOTFOUND'] },
    ],
    output: {
      category: 'network_connection_refused',
      severity: 'critical',
      cause: 'network',
    },
    priority: 100,
    confidence_base: 0.98,
  },

  // Rate limit rules
  {
    id: 'rate_limit_001',
    name: 'Rate Limit by HTTP Status',
    description: 'Rate limit detected from HTTP 429',
    conditions: [
      { field: 'error.http_status', operator: 'equals', value: 429 },
    ],
    output: {
      category: 'provider_rate_limit',
      severity: 'medium',
      cause: 'policy',
    },
    priority: 100,
    confidence_base: 0.99,
  },
  {
    id: 'rate_limit_002',
    name: 'Rate Limit by Message',
    description: 'Rate limit detected from error message',
    conditions: [
      { field: 'error.message', operator: 'contains', value: 'rate limit' },
    ],
    output: {
      category: 'provider_rate_limit',
      severity: 'medium',
      cause: 'policy',
    },
    priority: 90,
    confidence_base: 0.90,
  },

  // Quota exceeded rules
  {
    id: 'quota_001',
    name: 'Quota Exceeded',
    description: 'API quota has been exceeded',
    conditions: [
      { field: 'error.message', operator: 'contains', value: 'quota' },
    ],
    output: {
      category: 'provider_quota_exceeded',
      severity: 'high',
      cause: 'policy',
    },
    priority: 95,
    confidence_base: 0.92,
  },

  // Authentication errors
  {
    id: 'auth_001',
    name: 'Authentication Error by HTTP Status',
    description: 'Authentication failed (401)',
    conditions: [
      { field: 'error.http_status', operator: 'equals', value: 401 },
    ],
    output: {
      category: 'provider_authentication',
      severity: 'critical',
      cause: 'configuration',
    },
    priority: 100,
    confidence_base: 0.99,
  },

  // Authorization errors
  {
    id: 'authz_001',
    name: 'Authorization Error by HTTP Status',
    description: 'Authorization failed (403)',
    conditions: [
      { field: 'error.http_status', operator: 'equals', value: 403 },
    ],
    output: {
      category: 'provider_authorization',
      severity: 'high',
      cause: 'configuration',
    },
    priority: 100,
    confidence_base: 0.99,
  },

  // Service unavailable
  {
    id: 'service_001',
    name: 'Service Unavailable',
    description: 'Provider service is unavailable',
    conditions: [
      { field: 'error.http_status', operator: 'equals', value: 503 },
    ],
    output: {
      category: 'provider_service_unavailable',
      severity: 'critical',
      cause: 'provider',
    },
    priority: 100,
    confidence_base: 0.98,
  },
  {
    id: 'service_002',
    name: 'Bad Gateway',
    description: 'Bad gateway from provider',
    conditions: [
      { field: 'error.http_status', operator: 'equals', value: 502 },
    ],
    output: {
      category: 'provider_service_unavailable',
      severity: 'high',
      cause: 'provider',
    },
    priority: 100,
    confidence_base: 0.95,
  },

  // Provider internal errors
  {
    id: 'provider_error_001',
    name: 'Provider Internal Error',
    description: 'Provider internal server error',
    conditions: [
      { field: 'error.http_status', operator: 'equals', value: 500 },
    ],
    output: {
      category: 'provider_internal_error',
      severity: 'high',
      cause: 'provider',
    },
    priority: 100,
    confidence_base: 0.95,
  },

  // Model overloaded
  {
    id: 'overloaded_001',
    name: 'Model Overloaded',
    description: 'Model is overloaded',
    conditions: [
      { field: 'error.message', operator: 'contains', value: 'overloaded' },
    ],
    output: {
      category: 'provider_model_overloaded',
      severity: 'medium',
      cause: 'provider',
    },
    priority: 95,
    confidence_base: 0.92,
  },

  // Invalid request
  {
    id: 'request_001',
    name: 'Bad Request',
    description: 'Invalid request payload',
    conditions: [
      { field: 'error.http_status', operator: 'equals', value: 400 },
    ],
    output: {
      category: 'request_invalid_payload',
      severity: 'low',
      cause: 'client',
    },
    priority: 100,
    confidence_base: 0.95,
  },

  // Payload too large
  {
    id: 'request_002',
    name: 'Payload Too Large',
    description: 'Request payload exceeds limit',
    conditions: [
      { field: 'error.http_status', operator: 'equals', value: 413 },
    ],
    output: {
      category: 'request_payload_too_large',
      severity: 'low',
      cause: 'client',
    },
    priority: 100,
    confidence_base: 0.99,
  },

  // Context length exceeded
  {
    id: 'context_001',
    name: 'Context Length Exceeded',
    description: 'Input exceeds context window',
    conditions: [
      { field: 'error.message', operator: 'contains', value: 'context length' },
    ],
    output: {
      category: 'request_context_length_exceeded',
      severity: 'medium',
      cause: 'client',
    },
    priority: 95,
    confidence_base: 0.95,
  },

  // Content filter
  {
    id: 'content_001',
    name: 'Content Filter Triggered',
    description: 'Content was filtered by safety systems',
    conditions: [
      { field: 'error.message', operator: 'contains', value: 'content filter' },
    ],
    output: {
      category: 'request_content_filter',
      severity: 'informational',
      cause: 'policy',
    },
    priority: 95,
    confidence_base: 0.92,
  },

  // Token limit
  {
    id: 'token_001',
    name: 'Token Limit Exceeded',
    description: 'Token limit exceeded',
    conditions: [
      { field: 'error.message', operator: 'contains', value: 'token' },
      { field: 'error.message', operator: 'contains', value: 'limit' },
    ],
    output: {
      category: 'token_limit_exceeded',
      severity: 'medium',
      cause: 'client',
    },
    priority: 85,
    confidence_base: 0.85,
  },

  // SSL/TLS errors
  {
    id: 'ssl_001',
    name: 'SSL Handshake Error',
    description: 'SSL/TLS handshake failed',
    conditions: [
      { field: 'error.code', operator: 'in', value: ['EPROTO', 'UNABLE_TO_GET_ISSUER_CERT', 'CERT_HAS_EXPIRED'] },
    ],
    output: {
      category: 'network_ssl_handshake',
      severity: 'critical',
      cause: 'configuration',
    },
    priority: 100,
    confidence_base: 0.97,
  },

  // DNS resolution
  {
    id: 'dns_001',
    name: 'DNS Resolution Failed',
    description: 'Could not resolve hostname',
    conditions: [
      { field: 'error.code', operator: 'equals', value: 'ENOTFOUND' },
    ],
    output: {
      category: 'network_dns_resolution',
      severity: 'critical',
      cause: 'network',
    },
    priority: 100,
    confidence_base: 0.99,
  },
];

// =============================================================================
// CLASSIFICATION ENGINE
// =============================================================================

export class ClassificationEngine {
  private rules: ClassificationRule[];

  constructor() {
    // Sort rules by priority (highest first)
    this.rules = [...CLASSIFICATION_RULES].sort((a, b) => b.priority - a.priority);
  }

  /**
   * Classify a failure event
   */
  classify(event: FailureEvent): {
    category: FailureCategory;
    severity: FailureSeverity;
    cause: FailureCause;
    confidence: number;
    signals: ClassificationSignal[];
    matchedRules: string[];
  } {
    const signals: ClassificationSignal[] = [];
    const matchedRules: string[] = [];

    // Try each rule in priority order
    for (const rule of this.rules) {
      const { matches, confidence, ruleSignals } = this.evaluateRule(rule, event);

      if (matches) {
        signals.push(...ruleSignals);
        matchedRules.push(rule.id);

        // Return first matching rule (highest priority)
        return {
          ...rule.output,
          confidence,
          signals,
          matchedRules,
        };
      }
    }

    // No rules matched - return unknown classification
    signals.push({
      signal_type: 'no_match',
      signal_value: 'No classification rules matched',
      weight: 0,
    });

    return {
      category: 'unknown',
      severity: 'informational',
      cause: 'unknown',
      confidence: 0,
      signals,
      matchedRules,
    };
  }

  /**
   * Evaluate a rule against an event
   */
  private evaluateRule(
    rule: ClassificationRule,
    event: FailureEvent
  ): { matches: boolean; confidence: number; ruleSignals: ClassificationSignal[] } {
    const ruleSignals: ClassificationSignal[] = [];
    let allConditionsMet = true;
    let conditionCount = 0;
    let metConditionCount = 0;

    for (const condition of rule.conditions) {
      conditionCount++;
      const fieldValue = this.getFieldValue(event, condition.field);
      const conditionMet = this.evaluateCondition(condition, fieldValue);

      if (conditionMet) {
        metConditionCount++;
        ruleSignals.push({
          signal_type: `rule:${rule.id}:${condition.field}`,
          signal_value: String(fieldValue),
          weight: 1 / conditionCount,
        });
      } else {
        allConditionsMet = false;
      }
    }

    // Calculate confidence based on base confidence and condition matching
    const conditionRatio = conditionCount > 0 ? metConditionCount / conditionCount : 0;
    const confidence = allConditionsMet ? rule.confidence_base * conditionRatio : 0;

    return {
      matches: allConditionsMet,
      confidence,
      ruleSignals,
    };
  }

  /**
   * Get a nested field value from the event
   */
  private getFieldValue(event: FailureEvent, field: string): unknown {
    const parts = field.split('.');
    let value: unknown = event;

    for (const part of parts) {
      if (value === null || value === undefined) return undefined;
      if (typeof value !== 'object') return undefined;
      value = (value as Record<string, unknown>)[part];
    }

    return value;
  }

  /**
   * Evaluate a condition against a value
   */
  private evaluateCondition(
    condition: { operator: string; value: string | number | string[] },
    fieldValue: unknown
  ): boolean {
    if (fieldValue === undefined || fieldValue === null) return false;

    switch (condition.operator) {
      case 'equals':
        return fieldValue === condition.value;

      case 'contains':
        if (typeof fieldValue !== 'string') return false;
        if (typeof condition.value !== 'string') return false;
        return fieldValue.toLowerCase().includes(condition.value.toLowerCase());

      case 'matches':
        if (typeof fieldValue !== 'string') return false;
        if (typeof condition.value !== 'string') return false;
        return new RegExp(condition.value, 'i').test(fieldValue);

      case 'in':
        if (!Array.isArray(condition.value)) return false;
        return condition.value.includes(fieldValue as string);

      case 'gt':
        return typeof fieldValue === 'number' && fieldValue > (condition.value as number);

      case 'lt':
        return typeof fieldValue === 'number' && fieldValue < (condition.value as number);

      case 'gte':
        return typeof fieldValue === 'number' && fieldValue >= (condition.value as number);

      case 'lte':
        return typeof fieldValue === 'number' && fieldValue <= (condition.value as number);

      default:
        return false;
    }
  }
}

// =============================================================================
// CLASSIFICATION FUNCTION
// =============================================================================

/**
 * Classify a failure event
 *
 * This is the main entry point for classification.
 * It is deterministic and stateless.
 */
export async function classifyFailure(
  event: FailureEvent,
  engine: ClassificationEngine
): Promise<FailureClassification> {
  const startTime = Date.now();

  // Run classification
  const result = engine.classify(event);

  // Build recommendations (ADVISORY ONLY)
  const recommendations = generateRecommendations(result.category, result.cause);

  // Build classification output
  const classification: FailureClassification = {
    span_id: event.span_id,
    trace_id: event.trace_id,
    category: result.category,
    severity: result.severity,
    cause: result.cause,
    confidence: result.confidence,
    confidence_factors: result.matchedRules,
    classification_signals: result.signals,
    recommendations,
    classified_at: new Date().toISOString(),
    classification_latency_ms: Date.now() - startTime,
    schema_version: '1.0.0',
  };

  return classification;
}

/**
 * Generate recommendations based on classification
 *
 * NOTE: These are ADVISORY ONLY - the agent takes no action.
 */
function generateRecommendations(
  category: FailureCategory,
  cause: FailureCause
): string[] {
  const recommendations: string[] = [];

  switch (cause) {
    case 'network':
      recommendations.push('Check network connectivity to provider');
      recommendations.push('Verify DNS resolution');
      recommendations.push('Review firewall rules');
      break;

    case 'provider':
      recommendations.push('Check provider status page');
      recommendations.push('Consider retry with exponential backoff');
      recommendations.push('Monitor provider health metrics');
      break;

    case 'client':
      recommendations.push('Review request payload for validity');
      recommendations.push('Check input size and token count');
      break;

    case 'configuration':
      recommendations.push('Verify API credentials');
      recommendations.push('Review configuration settings');
      break;

    case 'policy':
      recommendations.push('Review rate limiting configuration');
      recommendations.push('Check quota allocation');
      break;

    case 'resource':
      recommendations.push('Monitor resource utilization');
      recommendations.push('Consider scaling resources');
      break;
  }

  // Category-specific recommendations
  switch (category) {
    case 'provider_rate_limit':
      recommendations.push('Implement request queuing');
      recommendations.push('Consider load distribution across endpoints');
      break;

    case 'request_context_length_exceeded':
      recommendations.push('Implement input truncation');
      recommendations.push('Consider chunking large inputs');
      break;

    case 'provider_authentication':
      recommendations.push('Rotate API credentials');
      recommendations.push('Verify credential validity');
      break;
  }

  return recommendations;
}
