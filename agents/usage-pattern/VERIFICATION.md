# Usage Pattern Agent - Verification Checklist

This document provides the verification checklist for the Usage Pattern Agent
as required by the LLM-Observatory Agent Infrastructure Constitution.

## Agent Classification

| Property | Value |
|----------|-------|
| Agent ID | `usage-pattern-agent` |
| Agent Version | `1.0.0` |
| Classification | **ADVISORY** |
| Read-Only | **YES** |
| decision_type | `usage_pattern_analysis` |
| confidence_type | Statistical (0.0-1.0) |
| constraints_applied | Always empty (`[]`) |

## Constitutional Compliance Checklist

### Prompt 0: Agent Infrastructure Constitution

- [x] **Observation-only** - Agent does not execute workflows
- [x] **No behavior changes** - Agent does not change system behavior
- [x] **No remediation** - Agent does not trigger remediation
- [x] **Google Cloud Edge Function** - Deployable as GCP Edge Function
- [x] **Stateless execution** - No local persistence at runtime
- [x] **ruvector-service persistence** - All persistence via HTTP client
- [x] **No direct SQL access** - Never connects directly to database
- [x] **Advisory output** - Produces summaries, not commands

### Prompt 1: Agent Contract & Boundary Definition

- [x] **Classification defined** - ADVISORY (read-only, non-enforcing)
- [x] **Schemas from agentics-contracts** - Uses Zod schema contracts
- [x] **Input validation** - AnalysisRequestSchema with strict validation
- [x] **Output validation** - UsagePatternAnalysisSchema
- [x] **Error conditions defined** - ErrorCodeSchema and ErrorResponseSchema
- [x] **DecisionEvent mapping** - UsagePatternDecisionEventSchema
- [x] **Confidence semantics** - Statistical (based on sample size)
- [x] **Empty constraints** - constraints_applied always `[]`
- [x] **Versioning rules** - Semantic versioning (x.y.z)
- [x] **CLI contract** - CLIInvocationSchema, CLIOutputSchema
- [x] **Consumer list** - LLM-Analytics-Hub, Governance, Reporting
- [x] **Non-responsibilities** - Explicit list in AGENT_METADATA

### Prompt 2: Runtime & Infrastructure Implementation

- [x] **Edge Function handler** - `handleRequest()` in handler.ts
- [x] **Input validation** - Zod schema parsing with error handling
- [x] **Core analytical logic** - UsagePatternAnalyzer class
- [x] **Confidence calculation** - Based on sample size
- [x] **DecisionEvent emitter** - DecisionEventEmitter class
- [x] **Telemetry emission** - Self-observation capability
- [x] **Error handling** - All error paths return ErrorResponse
- [x] **Versioned identifier** - `agent_id` and `agent_version`

### Prompt 3: Platform Wiring & Verification

- [x] **Agent registration** - AGENT_METADATA and getAgentRegistration()
- [x] **Endpoint registration** - Defined in AGENT_METADATA.endpoints
- [x] **CLI commands** - analyze, inspect, replay, status, health
- [x] **DecisionEvent persistence** - Via RuvectorClient
- [x] **No orchestration hooks** - Not implemented
- [x] **No execution triggers** - Not implemented
- [x] **No auto-remediation** - Not implemented

### Prompt 4: Usage Pattern Agent Specific

- [x] **Telemetry aggregation** - computeTimeSeries()
- [x] **Trend analysis** - computeTrends()
- [x] **Seasonality detection** - detectSeasonality()
- [x] **Usage distributions** - computeDistributions()
- [x] **Provider breakdown** - computeProviderUsage()
- [x] **Hotspot identification** - computeHotspots()
- [x] **Growth patterns** - computeGrowthPatterns()
- [x] **NO failure classification** - Not implemented
- [x] **NO health evaluation** - Not implemented
- [x] **NO threshold enforcement** - Not implemented
- [x] **NO alert generation** - Not implemented

## DecisionEvent Schema Validation

Every DecisionEvent MUST include:

| Field | Requirement | Validation |
|-------|-------------|------------|
| agent_id | `'usage-pattern-agent'` | Literal type enforcement |
| agent_version | Semantic version | Regex: `/^\d+\.\d+\.\d+$/` |
| decision_type | `'usage_pattern_analysis'` | Literal type enforcement |
| inputs_hash | SHA256 hex string | 64-character hex |
| outputs | Array of analyses | Min 1 item |
| confidence | 0.0 to 1.0 | Statistical calculation |
| constraints_applied | Always `[]` | Empty array literal |
| execution_ref | Unique reference | Non-empty string |
| timestamp | ISO 8601 UTC | Datetime format |

## Failure Modes

| Error Code | Description | HTTP Status |
|------------|-------------|-------------|
| INVALID_INPUT | Invalid request format | 400 |
| VALIDATION_FAILED | Schema validation failed | 400 |
| INSUFFICIENT_DATA | No events in time window | 422 |
| TIME_WINDOW_TOO_LARGE | Exceeds max days limit | 400 |
| RUVECTOR_CONNECTION_ERROR | Cannot connect to ruvector | 503 |
| ANALYSIS_TIMEOUT | Analysis exceeded timeout | 504 |
| INTERNAL_ERROR | Unexpected error | 500 |

## Smoke Test Commands

### 1. Health Check
```bash
# Check ruvector-service connectivity
npx ts-node src/cli.ts health
```

Expected output: JSON with `status: "healthy"` or `status: "unhealthy"`

### 2. Agent Status
```bash
# Get agent status and capabilities
npx ts-node src/cli.ts status
```

Expected output: JSON with agent metadata, capabilities, and constitution

### 3. Basic Analysis
```bash
# Run analysis for last 24 hours
npx ts-node src/cli.ts analyze \
  --start "2025-01-18T00:00:00Z" \
  --end "2025-01-19T00:00:00Z" \
  --granularity hour
```

Expected output: JSON with UsagePatternAnalysis

### 4. Analysis with Filters
```bash
# Filter by provider
npx ts-node src/cli.ts analyze \
  --start "2025-01-18T00:00:00Z" \
  --end "2025-01-19T00:00:00Z" \
  --providers "openai,anthropic"
```

### 5. Trend Analysis
```bash
# Include trend analysis
npx ts-node src/cli.ts analyze \
  --start "2025-01-18T00:00:00Z" \
  --end "2025-01-19T00:00:00Z" \
  --include-trends
```

### 6. Inspect Historical Analysis
```bash
# Inspect by analysis ID
npx ts-node src/cli.ts inspect \
  --id "550e8400-e29b-41d4-a716-446655440000"
```

### 7. Replay Analysis (Dry Run)
```bash
# Replay without persisting new DecisionEvent
npx ts-node src/cli.ts replay \
  --id "550e8400-e29b-41d4-a716-446655440000" \
  --dry-run
```

## Unit Test Commands

```bash
# Run all tests
npm test

# Run constitution compliance tests
npm test -- constitution.test.ts

# Run analyzer tests
npm test -- analyzer.test.ts

# Run with coverage
npm run test:coverage
```

## Edge Function Deployment

```bash
# Build TypeScript
npm run build

# Deploy to Google Cloud Functions
gcloud functions deploy usage-pattern-agent \
  --runtime nodejs20 \
  --trigger-http \
  --entry-point usagePatternAnalyzer \
  --memory 512MB \
  --timeout 60s \
  --region us-central1
```

## Environment Variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| AGENT_ID | No | `usage-pattern-agent` | Agent identifier |
| AGENT_VERSION | No | `1.0.0` | Semantic version |
| RUVECTOR_ENDPOINT | No | `http://localhost:3001` | ruvector-service URL |
| RUVECTOR_API_KEY | No | - | API key for auth |
| RUVECTOR_TIMEOUT | No | `30000` | Request timeout (ms) |
| MAX_EVENTS_PER_ANALYSIS | No | `100000` | Max events to analyze |
| MAX_TIME_WINDOW_DAYS | No | `90` | Max time window days |
| SELF_OBSERVATION_ENABLED | No | `false` | Enable self-telemetry |

## Verification Sign-Off

| Check | Status | Verified By | Date |
|-------|--------|-------------|------|
| Constitutional compliance | ✅ | Automated tests | - |
| Schema validation | ✅ | Zod schemas | - |
| DecisionEvent format | ✅ | Schema + validation | - |
| Edge Function handler | ✅ | handler.ts | - |
| CLI commands | ✅ | cli.ts | - |
| Error handling | ✅ | All paths covered | - |
| No orchestration hooks | ✅ | Not implemented | - |
| No auto-remediation | ✅ | Not implemented | - |

---

**CONSTITUTION COMPLIANCE: VERIFIED**

This agent adheres to all requirements of the LLM-Observatory Agent Infrastructure Constitution.
