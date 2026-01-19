# LLM Observatory - Production Deployment Manifest

## Service Topology

### Unified Service Name
```
llm-observatory
```

### Service Classification
```
OBSERVATION-ONLY
```

### Agent Endpoints (All via Unified Service)

| Agent | Endpoint | Method | Description |
|-------|----------|--------|-------------|
| Telemetry Collector | `/api/v1/telemetry/ingest` | POST | Ingest telemetry events |
| Telemetry Collector | `/api/v1/telemetry/health` | GET | Agent health check |
| Usage Pattern | `/api/v1/usage/analyze` | POST | Analyze usage patterns |
| Usage Pattern | `/api/v1/usage/health` | GET | Agent health check |
| Failure Classification | `/api/v1/failure/classify` | POST | Classify failures |
| Failure Classification | `/api/v1/failure/health` | GET | Agent health check |
| Health Check | `/api/v1/health-check/evaluate` | POST | Evaluate health |
| Health Check | `/api/v1/health-check/health` | GET | Agent health check |
| SLO Enforcement | `/api/v1/slo/enforce` | POST | Evaluate SLOs |
| SLO Enforcement | `/api/v1/slo/violations` | GET | Query violations |
| SLO Enforcement | `/api/v1/slo/health` | GET | Agent health check |
| Post-Mortem Generator | `/api/v1/postmortem/generate` | POST | Generate post-mortem |
| Post-Mortem Generator | `/api/v1/postmortem/health` | GET | Agent health check |
| Visualization Spec | `/api/v1/visualization/generate` | POST | Generate viz spec |
| Visualization Spec | `/api/v1/visualization/health` | GET | Agent health check |

### Confirmations

- [x] **No agent is deployed as a standalone service** - All agents share unified runtime
- [x] **Shared runtime** - Single Cloud Run service
- [x] **Shared configuration** - Single environment configuration
- [x] **Shared telemetry stack** - Single logging/metrics/tracing

---

## Environment Configuration

### Required Environment Variables

| Variable | Description | Source |
|----------|-------------|--------|
| `RUVECTOR_SERVICE_URL` | RuVector service endpoint | Environment |
| `RUVECTOR_API_KEY` | RuVector API authentication | Secret Manager |
| `PLATFORM_ENV` | Environment (dev/staging/prod) | Environment |
| `SERVICE_NAME` | Service identifier | Environment |
| `SERVICE_VERSION` | Deployment version | Environment |

### Confirmations

- [x] **No hardcoded service names** - All from environment
- [x] **No embedded credentials** - All via Secret Manager
- [x] **All dependencies via environment** - Dynamic resolution

---

## Google SQL / Memory Wiring

### Confirmations

- [x] **LLM-Observatory does NOT connect directly to Google SQL**
- [x] **ALL DecisionEvents written via ruvector-service**
- [x] **Schema compatible with agentics-contracts**
- [x] **Append-only persistence behavior**
- [x] **Idempotent writes with retry safety**

### RuVector Client Implementation
- Location: `deploy/service/src/ruvector-client.ts`
- All persistence goes through `persistDecisionEvent()` method
- Idempotency key: `X-Idempotency-Key` header with execution_ref
- Retry logic: Exponential backoff, max 3 retries

---

## Cloud Build & Deployment

### Deployment Command
```bash
# Using deployment script
./deploy/scripts/deploy.sh prod

# Or using Cloud Build
gcloud builds submit --config=deploy/cloudbuild.yaml \
  --substitutions=_ENV=prod,_REGION=us-central1
```

### IAM Service Account
- Name: `llm-observatory-sa`
- Roles (Least Privilege):
  - `roles/run.invoker`
  - `roles/secretmanager.secretAccessor`
  - `roles/logging.logWriter`
  - `roles/monitoring.metricWriter`
  - `roles/cloudtrace.agent`

### Networking
- **Public ingestion** - Unauthenticated access allowed
- **Internal access** - Via service account for ruvector-service

---

## CLI Activation

### CLI Commands Per Agent

See `deploy/docs/CLI_COMMANDS.md` for full documentation.

All commands follow pattern:
```bash
agentics-cli observatory <agent> <command> [options]
```

### Configuration
```bash
export LLM_OBSERVATORY_URL=https://llm-observatory-<region>-<project>.a.run.app
```

---

## Platform & Core Integration

### Confirmations

- [x] **LLM-Analytics-Hub consumes Observatory outputs** - Via ruvector-service queries
- [x] **Agentics Dev Platform dashboards visualize data** - Via standard metrics
- [x] **Governance views consume SLO/SLA violations** - Via DecisionEvents
- [x] **Core bundles consume without rewiring** - Read from ruvector-service
- [x] **LLM-Observatory does NOT influence execution** - Observation only

---

## Deployment Artifacts

| File | Purpose |
|------|---------|
| `deploy/Dockerfile` | Multi-stage Docker build |
| `deploy/cloudbuild.yaml` | Cloud Build configuration |
| `deploy/service/` | Unified service source code |
| `deploy/env.example` | Environment template |
| `deploy/service-account.yaml` | IAM configuration |
| `deploy/scripts/deploy.sh` | Deployment script |
| `deploy/docs/CLI_COMMANDS.md` | CLI documentation |
| `deploy/docs/VERIFICATION_CHECKLIST.md` | Post-deploy verification |
| `deploy/docs/ROLLBACK_PROCEDURES.md` | Failure handling |

---

## Summary

**LLM Observatory is now ready for production deployment.**

- Single unified Cloud Run service
- 7 agents sharing runtime
- All persistence via ruvector-service
- No direct SQL access
- Observation-only (no orchestration)
- CLI activation verified
