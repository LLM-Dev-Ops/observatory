# Post-Deployment Verification Checklist

## LLM Observatory Production Deployment Verification

Execute this checklist after every deployment to verify the system is operational.

---

## 1. Service Liveness

### 1.1 Health Endpoint
```bash
# Expected: HTTP 200 with status "healthy" or HTTP 503 with status "degraded"
curl -s https://llm-observatory-<region>-<project>.a.run.app/health | jq .
```

**Expected Output:**
```json
{
  "status": "healthy",
  "service": "llm-observatory",
  "version": "<commit-sha>",
  "environment": "prod",
  "classification": "OBSERVATION-ONLY"
}
```

- [ ] Health endpoint returns 200 or 503
- [ ] `status` is "healthy" or "degraded" (not "unhealthy")
- [ ] `classification` is "OBSERVATION-ONLY"

### 1.2 Readiness Endpoint
```bash
curl -s https://llm-observatory-<region>-<project>.a.run.app/ready | jq .
```

- [ ] Readiness endpoint returns 200
- [ ] All checks show `passed: true`

---

## 2. Agent Endpoints Respond

### 2.1 Telemetry Collector
```bash
curl -s https://llm-observatory-<region>-<project>.a.run.app/api/v1/telemetry/health | jq .
```
- [ ] Returns 200 with agent info

### 2.2 Usage Pattern
```bash
curl -s https://llm-observatory-<region>-<project>.a.run.app/api/v1/usage/health | jq .
```
- [ ] Returns 200 with agent info

### 2.3 Failure Classification
```bash
curl -s https://llm-observatory-<region>-<project>.a.run.app/api/v1/failure/health | jq .
```
- [ ] Returns 200 with agent info

### 2.4 Health Check Agent
```bash
curl -s https://llm-observatory-<region>-<project>.a.run.app/api/v1/health-check/health | jq .
```
- [ ] Returns 200 with agent info

### 2.5 SLO Enforcement
```bash
curl -s https://llm-observatory-<region>-<project>.a.run.app/api/v1/slo/health | jq .
```
- [ ] Returns 200 with agent info

### 2.6 Post-Mortem Generator
```bash
curl -s https://llm-observatory-<region>-<project>.a.run.app/api/v1/postmortem/health | jq .
```
- [ ] Returns 200 with agent info

### 2.7 Visualization Spec
```bash
curl -s https://llm-observatory-<region>-<project>.a.run.app/api/v1/visualization/health | jq .
```
- [ ] Returns 200 with agent info

---

## 3. Telemetry Ingestion Functions

### 3.1 Test Telemetry Ingestion
```bash
curl -X POST https://llm-observatory-<region>-<project>.a.run.app/api/v1/telemetry/ingest \
  -H "Content-Type: application/json" \
  -d '{
    "provider": "openai",
    "eventType": "completion_created",
    "timestamp": "'$(date -u +%Y-%m-%dT%H:%M:%SZ)'",
    "payload": {"model": "gpt-4", "tokens": 100}
  }' | jq .
```

- [ ] Returns 200 with `success: true`
- [ ] `processed` count equals 1
- [ ] `execution_ref` is present

---

## 4. DecisionEvents in RuVector-Service

### 4.1 Verify Event Persistence
```bash
# Query ruvector-service for recent events from Observatory
curl -s "https://ruvector-service-<region>-<project>.a.run.app/api/events?agentId=telemetry-collector-agent&limit=5" \
  -H "Authorization: Bearer $RUVECTOR_API_KEY" | jq .
```

- [ ] DecisionEvents appear in ruvector-service
- [ ] `agent_id` matches expected agent
- [ ] `decision_type` is correct
- [ ] `constraints_applied` is empty array (constitutional requirement)

---

## 5. Observatory Dashboards Populate

### 5.1 Check LLM-Analytics-Hub
```bash
# Verify Analytics Hub receives Observatory data
curl -s "https://llm-analytics-hub-<region>-<project>.a.run.app/api/observatory/status" | jq .
```

- [ ] Analytics Hub acknowledges Observatory connection
- [ ] Recent data points visible

### 5.2 Check Governance Views
```bash
# Verify SLO violations appear in governance
curl -s "https://llm-analytics-hub-<region>-<project>.a.run.app/api/governance/violations?source=observatory" | jq .
```

- [ ] Governance views consume Observatory data

---

## 6. CLI Inspection Commands

### 6.1 CLI Health Check
```bash
agentics-cli observatory telemetry status --detailed
```

- [ ] CLI command executes successfully
- [ ] Returns agent status and metrics

### 6.2 CLI Replay
```bash
# Get an execution_ref from step 3.1
agentics-cli observatory slo replay --execution-ref=<ref>
```

- [ ] Replay returns original decision event

---

## 7. Architectural Compliance

### 7.1 No Direct SQL Access
```bash
# Verify no SQL connections from Observatory
gcloud sql instances list --format="table(name,connectionName)"
# Check Cloud Run service has NO Cloud SQL connections
gcloud run services describe llm-observatory --region=<region> --format="value(spec.template.spec.containers[0].env)"
```

- [ ] No `CLOUD_SQL_CONNECTION_NAME` environment variable
- [ ] No `cloudsql-instances` annotation
- [ ] No Cloud SQL IAM bindings for service account

### 7.2 No Workflow Execution
```bash
# Verify no Workflows API calls
gcloud logging read "resource.type=cloud_run_revision AND resource.labels.service_name=llm-observatory AND protoPayload.methodName~workflows" --limit=10
```

- [ ] No Workflows API calls logged

### 7.3 Contracts Compliance
- [ ] All DecisionEvents match `agentics-contracts` schema
- [ ] `constraints_applied` is always empty array
- [ ] `inputs_hash` is valid SHA256

---

## 8. Performance Verification

### 8.1 Latency Check
```bash
# Run 10 requests and check latency
for i in {1..10}; do
  curl -s -o /dev/null -w "%{time_total}\n" https://llm-observatory-<region>-<project>.a.run.app/health
done
```

- [ ] Average latency < 100ms
- [ ] P99 latency < 500ms

### 8.2 Error Rate
```bash
# Check error logs
gcloud logging read "resource.type=cloud_run_revision AND resource.labels.service_name=llm-observatory AND severity>=ERROR" --limit=10 --format=json
```

- [ ] No unexpected errors
- [ ] Error rate < 0.1%

---

## Verification Summary

| Category | Status |
|----------|--------|
| Service Liveness | ☐ Pass / ☐ Fail |
| Agent Endpoints | ☐ Pass / ☐ Fail |
| Telemetry Ingestion | ☐ Pass / ☐ Fail |
| DecisionEvent Persistence | ☐ Pass / ☐ Fail |
| Dashboard Integration | ☐ Pass / ☐ Fail |
| CLI Commands | ☐ Pass / ☐ Fail |
| Architectural Compliance | ☐ Pass / ☐ Fail |
| Performance | ☐ Pass / ☐ Fail |

**Deployment Verified By:** _______________
**Date:** _______________
**Version:** _______________
