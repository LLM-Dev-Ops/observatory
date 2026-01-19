# Failure Modes & Rollback Procedures

## LLM Observatory Deployment Failure Handling

---

## 1. Common Deployment Failures

### 1.1 Build Failures

**Symptoms:**
- Cloud Build fails during docker build
- TypeScript compilation errors
- Missing dependencies

**Detection Signals:**
```bash
# Check Cloud Build logs
gcloud builds list --limit=5
gcloud builds log <BUILD_ID>
```

**Resolution:**
1. Check TypeScript errors in build output
2. Verify package.json dependencies
3. Fix and re-run deployment

---

### 1.2 Container Startup Failures

**Symptoms:**
- Cloud Run revision fails to start
- Container exits immediately
- Health check failures

**Detection Signals:**
```bash
# Check Cloud Run logs
gcloud logging read "resource.type=cloud_run_revision AND resource.labels.service_name=llm-observatory AND severity>=ERROR" --limit=50

# Check revision status
gcloud run revisions list --service=llm-observatory --region=<region>
```

**Common Causes:**
- Missing environment variables
- Invalid secret references
- Port mismatch (must be 8080)
- Memory/CPU limits too low

**Resolution:**
1. Check environment variable configuration
2. Verify secret exists in Secret Manager
3. Increase memory/CPU limits
4. Rollback to previous revision

---

### 1.3 RuVector Connectivity Failures

**Symptoms:**
- Health endpoint returns 503 "degraded"
- DecisionEvents not persisting
- Timeouts on agent endpoints

**Detection Signals:**
```bash
# Check RuVector health from Observatory
curl -s https://llm-observatory-<region>.a.run.app/health | jq '.components.ruvector'

# Check logs for connection errors
gcloud logging read "resource.type=cloud_run_revision AND resource.labels.service_name=llm-observatory AND textPayload~'ruvector'" --limit=20
```

**Resolution:**
1. Verify RUVECTOR_SERVICE_URL is correct
2. Verify RUVECTOR_API_KEY secret is accessible
3. Check ruvector-service is healthy
4. Check network/firewall rules

---

### 1.4 Missing DecisionEvents

**Symptoms:**
- Ingestion returns success but events don't appear in ruvector-service
- Analytics Hub shows no data

**Detection Signals:**
```bash
# Query ruvector-service directly
curl -s "https://ruvector-service-<region>.a.run.app/api/events?agentId=telemetry-collector-agent&limit=5" \
  -H "Authorization: Bearer $RUVECTOR_API_KEY"

# Check Observatory logs for persist errors
gcloud logging read "resource.type=cloud_run_revision AND resource.labels.service_name=llm-observatory AND textPayload~'persist'" --limit=20
```

**Resolution:**
1. Check ruvector-service is accepting writes
2. Verify API key has write permissions
3. Check for schema validation errors
4. Verify network connectivity

---

## 2. Rollback Procedure

### 2.1 Identify Previous Revision

```bash
# List all revisions
gcloud run revisions list --service=llm-observatory --region=<region> --format="table(name,active,creationTimestamp)"
```

### 2.2 Rollback to Previous Revision

```bash
# Immediate rollback to previous revision
gcloud run services update-traffic llm-observatory \
  --region=<region> \
  --to-revisions=<PREVIOUS_REVISION>=100

# Example:
gcloud run services update-traffic llm-observatory \
  --region=us-central1 \
  --to-revisions=llm-observatory-00005-abc=100
```

### 2.3 Verify Rollback

```bash
# Check traffic routing
gcloud run services describe llm-observatory --region=<region> --format="value(status.traffic)"

# Verify health
curl -s https://llm-observatory-<region>.a.run.app/health | jq .
```

### 2.4 Full Rollback Script

```bash
#!/bin/bash
# rollback.sh - Emergency rollback script

REGION="${REGION:-us-central1}"
SERVICE_NAME="llm-observatory"

echo "Fetching previous revision..."
PREVIOUS_REVISION=$(gcloud run revisions list \
  --service=$SERVICE_NAME \
  --region=$REGION \
  --format="value(name)" \
  --sort-by="~creationTimestamp" \
  --limit=2 | tail -1)

if [ -z "$PREVIOUS_REVISION" ]; then
  echo "Error: No previous revision found"
  exit 1
fi

echo "Rolling back to: $PREVIOUS_REVISION"

gcloud run services update-traffic $SERVICE_NAME \
  --region=$REGION \
  --to-revisions=$PREVIOUS_REVISION=100

echo "Verifying rollback..."
sleep 5

HTTP_STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
  "https://$SERVICE_NAME-$REGION-$(gcloud config get-value project).a.run.app/health")

if [ "$HTTP_STATUS" = "200" ] || [ "$HTTP_STATUS" = "503" ]; then
  echo "Rollback successful"
else
  echo "Rollback verification failed (status: $HTTP_STATUS)"
  exit 1
fi
```

---

## 3. Safe Redeploy Strategy

### 3.1 Gradual Traffic Shift

```bash
# Deploy new revision without shifting traffic
gcloud run deploy llm-observatory \
  --image=gcr.io/<project>/llm-observatory:<new-tag> \
  --region=<region> \
  --no-traffic

# Shift 10% traffic to new revision
gcloud run services update-traffic llm-observatory \
  --region=<region> \
  --to-revisions=LATEST=10

# Monitor for 5-10 minutes, then increase
gcloud run services update-traffic llm-observatory \
  --region=<region> \
  --to-revisions=LATEST=50

# Full cutover
gcloud run services update-traffic llm-observatory \
  --region=<region> \
  --to-revisions=LATEST=100
```

### 3.2 Blue-Green Deployment

```bash
# Deploy to separate service for testing
gcloud run deploy llm-observatory-canary \
  --image=gcr.io/<project>/llm-observatory:<new-tag> \
  --region=<region>

# Test canary thoroughly
curl https://llm-observatory-canary-<region>.a.run.app/health

# If successful, deploy to main service
gcloud run deploy llm-observatory \
  --image=gcr.io/<project>/llm-observatory:<new-tag> \
  --region=<region>

# Clean up canary
gcloud run services delete llm-observatory-canary --region=<region> --quiet
```

---

## 4. Data Safety

### 4.1 No Data Loss Guarantee

LLM Observatory writes are:
- **Append-only** - No updates or deletes
- **Idempotent** - Duplicate writes are safe
- **Retry-safe** - Failed writes can be retried

### 4.2 Recovery from Missed Events

If events were lost during a failure:

1. Identify time window of failure
2. Re-ingest events from source systems
3. Verify via ruvector-service query

```bash
# Check for gaps in events
curl -s "https://ruvector-service-<region>.a.run.app/api/events/gaps?start=<start>&end=<end>" \
  -H "Authorization: Bearer $RUVECTOR_API_KEY"
```

---

## 5. Escalation Path

| Severity | Condition | Action |
|----------|-----------|--------|
| **P1** | All agent endpoints down | Immediate rollback, page on-call |
| **P2** | Single agent failing | Investigate, prepare rollback |
| **P3** | Degraded performance | Monitor, investigate during business hours |
| **P4** | Non-critical errors | Track in issue tracker |

### Contacts

- **On-Call:** #agentics-oncall
- **Platform Team:** #platform-team
- **Escalation:** See runbook

---

## 6. Prevention Checklist

Before every deployment:

- [ ] Run unit tests locally
- [ ] Build Docker image locally
- [ ] Test in dev environment first
- [ ] Review environment variable changes
- [ ] Verify secret access
- [ ] Check ruvector-service health
- [ ] Prepare rollback command
- [ ] Alert team of deployment window
