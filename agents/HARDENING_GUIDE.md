# Agent Hardening Guide - Phase 1 Layer 1

This document describes the hardening changes applied to Observatory agents for Phase 1 Layer 1 deployment.

## Overview

All agents MUST be hardened with the following requirements:

1. **Mandatory Startup Requirements** - Assert presence of required environment variables
2. **Agent Identity Standardization** - Include source_agent, domain, phase, layer in DecisionEvents
3. **Performance Boundaries** - MAX_TOKENS=800, MAX_LATENCY_MS=1500, MAX_CALLS_PER_RUN=2
4. **Contract Assertions** - Ruvector required = true, ≥1 DecisionEvent per run
5. **Minimal Observability** - Only log: agent_started, decision_event_emitted, agent_abort
6. **Caching** - In-memory caching for read-only operations (TTL 30-60s)

## Mandatory Environment Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `RUVECTOR_SERVICE_URL` | Ruvector service endpoint | `https://ruvector-service-us-central1.a.run.app` |
| `RUVECTOR_API_KEY` | API key from Google Secret Manager | (from Secret Manager) |
| `AGENT_NAME` | Agent identifier | `failure-classification-agent` |
| `AGENT_DOMAIN` | Agent domain | `diagnostics` |
| `AGENT_PHASE` | Must be `phase1` | `phase1` |
| `AGENT_LAYER` | Must be `layer1` | `layer1` |

## DecisionEvent Schema (Hardened)

All agents must emit DecisionEvents with these fields:

```typescript
interface HardenedDecisionEvent {
  // MANDATORY: Agent Identity (Phase 1 Layer 1)
  source_agent: string;  // e.g., "failure-classification-agent"
  domain: string;        // e.g., "diagnostics"
  phase: "phase1";
  layer: "layer1";

  // Signal type (NOT conclusion)
  event_type: string;    // e.g., "failure_signal"

  // Original fields
  agent_id: string;
  agent_version: string;
  decision_type: string;
  inputs_hash: string;
  outputs: unknown[];
  confidence: number;    // 0-1
  evidence_refs: EvidenceRef[];
  constraints_applied: never[];  // MUST be empty
  execution_ref: string;
  timestamp: string;
}
```

## Files to Modify Per Agent

For each agent, modify these files following the `failure-classification` template:

### 1. `src/config.ts`

- Add `identity: AgentIdentity` to `AgentConfig`
- Add performance boundary fields: `maxTokens`, `maxLatencyMs`, `maxCallsPerRun`
- Update `loadConfig()` to read hardened env vars
- Add `loadConfigWithHardenedEnv()` function
- Update `validateConfig()` to validate hardened fields

### 2. `src/handler.ts`

- Import hardening utilities from `../../shared/hardening/index`
- Add `ensureHardenedInitialization()` function
- Create per-request `PerformanceGuard` and `ContractAssertions`
- Call `performanceGuard.assertCallLimit()` before each operation
- Call `performanceGuard.assertLatencyLimit()` periodically
- Call `contractAssertions.recordDecisionEventEmitted()` after persisting
- Call `contractAssertions.assertContractsMet()` at end of request

### 3. `contracts/schemas.ts`

- Add `EvidenceRefSchema` for audit trails
- Update `DecisionEventSchema` to include:
  - `source_agent`, `domain`, `phase`, `layer`
  - `event_type`
  - `evidence_refs`

### 4. `contracts/index.ts`

- Export `EvidenceRefSchema` and `EvidenceRef` type
- Add `hardened` section to `AGENT_METADATA`

## Agent-Specific Configurations

| Agent | Domain | Default Name |
|-------|--------|--------------|
| failure-classification | diagnostics | failure-classification-agent |
| health-check | health | health-check-agent |
| post-mortem-generator | analysis | post-mortem-generator-agent |
| slo-enforcement | enforcement | slo-enforcement-agent |
| telemetry-collector | ingest | telemetry-collector-agent |
| usage-pattern | analytics | usage-pattern-agent |
| visualization-spec | visualization | visualization-spec-agent |

## Cloud Run Deploy Command Template

```bash
gcloud run deploy SERVICE_NAME \
  --image gcr.io/PROJECT_ID/SERVICE_NAME:TAG \
  --region REGION \
  --platform managed \
  --set-env-vars "\
SERVICE_NAME=SERVICE_NAME,\
SERVICE_VERSION=VERSION,\
PLATFORM_ENV=prod,\
RUVECTOR_SERVICE_URL=https://ruvector-service-REGION.a.run.app,\
AGENT_NAME=AGENT_NAME,\
AGENT_DOMAIN=DOMAIN,\
AGENT_PHASE=phase1,\
AGENT_LAYER=layer1,\
MAX_TOKENS=800,\
MAX_LATENCY_MS=1500,\
MAX_CALLS_PER_RUN=2" \
  --set-secrets "RUVECTOR_API_KEY=ruvector-api-key:latest" \
  --service-account "llm-observatory-sa@PROJECT_ID.iam.gserviceaccount.com"
```

## Verification Checklist

- [ ] All mandatory env vars are asserted at startup
- [ ] Service crashes if RUVECTOR_SERVICE_URL missing
- [ ] Service crashes if RUVECTOR_API_KEY missing
- [ ] Service crashes if AGENT_NAME missing
- [ ] Service crashes if AGENT_DOMAIN missing
- [ ] Service crashes if AGENT_PHASE != "phase1"
- [ ] Service crashes if AGENT_LAYER != "layer1"
- [ ] Ruvector health check runs at startup
- [ ] Service crashes if Ruvector health check fails
- [ ] DecisionEvents include source_agent field
- [ ] DecisionEvents include domain field
- [ ] DecisionEvents include phase field
- [ ] DecisionEvents include layer field
- [ ] DecisionEvents include event_type field
- [ ] DecisionEvents include evidence_refs array
- [ ] Performance boundaries enforced (MAX_TOKENS, MAX_LATENCY_MS, MAX_CALLS_PER_RUN)
- [ ] Contract assertion: ≥1 DecisionEvent emitted per run
- [ ] Only logging: agent_started, decision_event_emitted, agent_abort
- [ ] Secrets referenced via Google Secret Manager (--set-secrets)
- [ ] No inline secrets in code or config
