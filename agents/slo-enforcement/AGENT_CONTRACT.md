# SLO/SLA Enforcement Agent Contract

## Agent Identity

| Property | Value |
|----------|-------|
| **Agent ID** | `slo-enforcement-agent` |
| **Version** | `1.0.0` |
| **Classification** | ENFORCEMENT-CLASS |
| **Actuating** | NO (NON-ACTUATING) |
| **Decision Type** | `slo_violation_detection` |

## Purpose Statement

Detect violations of defined Service Level Objectives (SLOs) and Service Level Agreements (SLAs) using telemetry-derived metrics. This agent evaluates metrics against policy-defined thresholds, detects breaches and near-breaches, emits structured violation events, and persists violation history for governance and audit.

## Classification Details

### ENFORCEMENT-CLASS (NON-ACTUATING)

This agent is classified as **ENFORCEMENT-CLASS** because it:
- Evaluates compliance against defined policies (SLOs/SLAs)
- Detects and reports violations
- Generates recommendations (advisory only)

This agent is **NON-ACTUATING** because it:
- Does NOT trigger alerts directly
- Does NOT initiate remediation
- Does NOT change policies or thresholds at runtime
- Does NOT modify system state in any way

## Input Schemas

### SloEnforcementRequest

```typescript
{
  slo_definitions: SloDefinition[];  // 1-100 SLO definitions
  metrics: TelemetryMetric[];        // 1-1000 metrics
  evaluation_time: string;           // ISO8601 datetime
  include_historical_context?: boolean;
  correlation_id?: string;
}
```

### SloDefinition

```typescript
{
  slo_id: string;              // Unique identifier
  name: string;                // Human-readable name
  indicator: SloIndicatorType; // e.g., 'latency_p95', 'error_rate'
  operator: SloOperator;       // 'lt', 'lte', 'gt', 'gte', 'eq', 'neq'
  threshold: number;           // Threshold value
  window: TimeWindow;          // '1m', '5m', '15m', '1h', '24h', etc.
  enabled: boolean;
  is_sla: boolean;
  sla_penalty_tier?: number;   // 1-5 if is_sla
  warning_threshold_percentage?: number; // Default 80
  provider?: string;           // Filter by provider
  model?: string;              // Filter by model
  environment?: string;        // Filter by environment
}
```

### TelemetryMetric

```typescript
{
  metric_id: string;           // UUID
  indicator: SloIndicatorType;
  value: number;
  window: TimeWindow;
  timestamp: string;           // ISO8601
  sample_count?: number;
  provider?: string;
  model?: string;
  environment?: string;
}
```

## Output Schemas

### EnforcementResult

```typescript
{
  violations: SloViolation[];
  slo_statuses: SloStatus[];
  evaluation_time: string;
  metrics_evaluated: number;
  slos_evaluated: number;
  processing_time_ms: number;
}
```

### SloViolation

```typescript
{
  violation_id: string;        // UUID
  slo_id: string;
  slo_name: string;
  breach_type: 'slo_breach' | 'sla_breach' | 'near_breach' | 'consecutive_breach';
  severity: 'critical' | 'high' | 'medium' | 'low';
  indicator: SloIndicatorType;
  metric_context: MetricContext;
  is_sla: boolean;
  sla_penalty_tier?: number;
  detected_at: string;
  window: TimeWindow;
  recommendation?: string;     // Advisory only
}
```

## DecisionEvent Mapping

Every invocation produces exactly ONE DecisionEvent with this structure:

```typescript
{
  agent_id: 'slo-enforcement-agent',           // Literal
  agent_version: '1.0.0',                      // Semantic version
  decision_type: 'slo_violation_detection',    // Literal
  inputs_hash: string,                         // SHA256 of inputs
  outputs: {
    violations: SloViolation[];
    slo_statuses: SloStatus[];
    metrics_evaluated: number;
    slos_evaluated: number;
  },
  confidence: number,                          // 0.0-1.0 (analytical)
  constraints_applied: [],                     // MUST be empty
  execution_ref: string,                       // UUID
  timestamp: string                            // ISO8601 UTC
}
```

### Confidence Semantics

Confidence is **analytical**, calculated from:
- **Sample size factor** (30%): Larger samples = higher confidence
- **Data freshness factor** (30%): More recent data = higher confidence
- **Consistency factor** (25%): Less volatility = higher confidence
- **Coverage factor** (15%): More SLOs evaluated = higher confidence

Minimum confidence: 0.5

### constraints_applied

**MUST always be empty array** (`[]`).

This agent is NON-ACTUATING and does not apply any constraints to the system. Any non-empty value is a constitutional violation.

## CLI Contract

### Commands

```bash
# Evaluate SLOs from files
slo-enforce evaluate --slos <file> --metrics <file> [--output <file>] [--format json|table]

# Query violations
slo-enforce query [--slo-id <id>] [--severity <level>] [--start <datetime>] [--end <datetime>] [--limit <n>] [--format json|table]

# Replay a decision event
slo-enforce replay --execution-ref <uuid>

# Get aggregated analysis
slo-enforce analyze --start <datetime> --end <datetime> [--group-by <field>] [--format json|table]

# Health check
slo-enforce health
```

### HTTP Endpoints

| Method | Path | Description |
|--------|------|-------------|
| POST | `/enforce` | Evaluate SLOs against metrics (single) |
| POST | `/enforce/batch` | Batch evaluation (up to 50) |
| GET | `/violations` | Query violations |
| GET | `/analysis` | Get aggregated analysis |
| GET | `/health` | Health check |
| GET | `/replay/:id` | Replay/inspect decision |
| GET | `/` | Agent info |

## Primary Consumers

- **LLM-Governance-Dashboard**: Displays violation history and trends
- **LLM-Policy-Engine**: Consumes violation events for policy decisions
- **Incident review workflows**: Historical analysis and audit

## Explicit Non-Responsibilities

This agent MUST NEVER:

1. **Trigger alerts directly** - Only emits DecisionEvents; alerting is handled by consumers
2. **Initiate remediation** - Only detects; remediation is handled by other systems
3. **Change policies or thresholds at runtime** - All definitions come from input
4. **Modify system state** - Purely observational
5. **Execute SQL directly** - All persistence via ruvector-service
6. **Connect to databases directly** - Only through ruvector-service client
7. **Invoke other agents** - Observatory agents are isolated
8. **Influence live execution** - Read-only observation

## Failure Modes

| Condition | Error Code | HTTP Status | Behavior |
|-----------|------------|-------------|----------|
| Invalid input schema | `VALIDATION_ERROR` | 400 | Reject request, no persistence |
| Constitutional violation | `CONSTITUTIONAL_VIOLATION` | 500 | Reject request, log error |
| RuVector unavailable | `RUVECTOR_ERROR` | 502 | Retry with backoff, then fail |
| Internal error | `INTERNAL_ERROR` | 500 | Log error, return safe response |

### Error Response Format

```typescript
{
  success: false,
  error: {
    code: string;
    message: string;
    details?: unknown;
  },
  metadata: {
    execution_ref: string;
    processing_time_ms: number;
    agent_id: string;
    agent_version: string;
  }
}
```

## Versioning Rules

- **Major version**: Breaking changes to input/output schemas
- **Minor version**: New optional features, backward compatible
- **Patch version**: Bug fixes, no schema changes

Version is embedded in:
- `agent_version` field in DecisionEvent
- `X-Agent-Version` response header
- Agent manifest

## Persistence

### Data Persisted to ruvector-service

- Complete DecisionEvent (including all violations)
- SLO statuses at evaluation time
- Confidence score and factors
- Processing metadata

### Data NOT Persisted

- Raw input metrics (ephemeral)
- Intermediate calculation state
- Historical context lookups (derived from stored data)

## Constitutional Compliance Checklist

- [x] Classification defined (ENFORCEMENT-CLASS, NON-ACTUATING)
- [x] Schemas from agentics-contracts only
- [x] All inputs/outputs validated
- [x] Error conditions defined
- [x] decision_type is literal (`slo_violation_detection`)
- [x] confidence is analytical (0.0-1.0)
- [x] constraints_applied is always empty
- [x] Persistence via ruvector-service only
- [x] DecisionEvent schema validated before persistence
- [x] CLI invocation defined
- [x] Core bundle consumers identified
- [x] Non-responsibilities explicitly defined
- [x] Failure modes documented
