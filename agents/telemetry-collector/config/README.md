# Telemetry Collector Agent - Deployment Configuration

This directory contains all deployment and configuration files for the Telemetry Collector Agent, a READ-ONLY telemetry ingestion service for the LLM Observatory platform.

## Files Overview

### cloudfunctions.yaml
**Purpose**: Google Cloud Functions deployment configuration

Defines:
- Runtime environment (Node.js 20, 256MB memory)
- Auto-scaling parameters (0-100 instances, 1,000 concurrent connections)
- VPC configuration for private deployment
- Environment variables and secrets management
- OpenTelemetry integration
- Service account and IAM settings

**Deploy with**:
```bash
gcloud functions deploy telemetry-collector-agent \
  --gen2 \
  --runtime nodejs20 \
  --region us-central1 \
  --entry-point handleTelemetryIngestion \
  --config cloudfunctions.yaml
```

### package.json
**Purpose**: Node.js package definition and npm scripts

Defines:
- Package metadata (`@llm-observatory/telemetry-collector-agent@1.0.0`)
- Build scripts (TypeScript compilation)
- Development scripts (watch mode, testing, linting)
- Deployment scripts (Google Cloud Functions)
- Dependencies (OpenTelemetry, zod validation, pino logging, axios)

**Common tasks**:
```bash
npm install                 # Install dependencies
npm run build              # Compile TypeScript to dist/
npm run dev                # Watch mode development
npm run test               # Run tests with coverage
npm run lint               # ESLint with auto-fix
npm run deploy             # Deploy to Google Cloud Functions
npm run logs               # Tail function logs
```

### tsconfig.json
**Purpose**: TypeScript compiler configuration

Defines:
- Target: ES2020 with ES modules
- Strict mode (all strict flags enabled)
- Source maps and declaration files
- ESM-compatible module resolution

**Features**:
- No unused variables/parameters checking
- Exact optional property types
- Unknown in catch variables
- Node.js module resolution

### agent-manifest.json
**Purpose**: Comprehensive agent registration and specification

Defines:
- **Agent Metadata**: ID, version, classification (READ-ONLY)
- **Inputs**: TelemetryEvent structure with examples
- **Outputs**: NormalizedTelemetry and DecisionEvent types
- **Consumers**: Downstream agents (usage-pattern, failure-classification, etc.)
- **Responsibilities**: What the agent does (ingestion, validation, normalization)
- **Non-Responsibilities**: What it does NOT do (aggregation, classification, storage, etc.)
- **HTTP Endpoints**: POST /telemetry/ingest, GET /telemetry/health, GET /telemetry/metrics
- **CLI Commands**: inspect, replay, analyze, status, validate, export
- **Metrics**: Prometheus metrics definitions
- **Dependencies**: schema-registry, ruvector-service
- **SLA Targets**: 99.9% availability, 10ms P50/100ms P99 latency, 1,000 events/sec throughput

### env.example
**Purpose**: Environment variable configuration template

Sections:
- Agent Configuration
- RuVector Service Integration
- OpenTelemetry Observability
- Telemetry Processing (batch size, timeouts)
- Schema Validation
- Retry Strategy
- Logging and Monitoring
- Consumer Registration
- Database (optional persistence)
- Caching Configuration
- Security (API keys, CORS, rate limiting)
- Feature Flags
- Debug Settings
- GCP-Specific Configuration
- Secrets Management References

## Quick Start

### 1. Setup Environment
```bash
cp config/env.example .env.local
# Edit .env.local with your actual values
source .env.local
```

### 2. Build Locally
```bash
npm install
npm run build
npm run test
npm run lint
```

### 3. Deploy to GCP
```bash
gcloud auth login
gcloud config set project llm-observatory
npm run deploy
```

### 4. Monitor Deployment
```bash
# Check deployment status
npm run describe

# View recent logs
npm run logs

# Test the endpoints
curl https://REGION-GCP_PROJECT_ID.cloudfunctions.net/telemetry-collector-agent/telemetry/health
```

## Agent Architecture

### Classification
- **Type**: READ-ONLY (no side effects, no state mutations)
- **Component**: Ingestion Layer
- **Decision Type**: telemetry_ingestion

### Inputs
Accepts TelemetryEvent from:
- OpenAI
- Anthropic
- Google Cloud Vertex AI
- AWS Bedrock
- Custom providers

### Outputs
Emits two types of events:
1. **NormalizedTelemetry**: Validated and normalized events
2. **DecisionEvent**: INGEST/REJECT/QUEUE decisions

### Consumers
Events flow downstream to:
- usage-pattern-agent (HIGH priority, DIRECT routing)
- failure-classification-agent (HIGH priority, DIRECT routing)
- health-check-agent (MEDIUM priority, BATCH routing)
- visualization-spec-agent (MEDIUM priority, BATCH routing)

### Non-Responsibilities
This agent explicitly does NOT:
- Aggregate events (temporal, spatial, or dimensional)
- Interpret event semantics
- Classify events into categories
- Perform anomaly detection or alerting
- Correlate events across time or providers
- Store or persist events
- Cache events or results
- Sample or filter events
- Compute derived metrics
- Make authorization or access control decisions

## HTTP Endpoints

### POST /telemetry/ingest
Ingest telemetry events

**Rate Limit**: 1,000 requests/minute per IP

**Request**:
```json
{
  "provider": "openai",
  "eventType": "completion_created",
  "timestamp": "2025-01-19T10:30:00Z",
  "payload": {
    "modelId": "gpt-4-turbo",
    "tokensUsed": 150,
    "cost": 0.45
  }
}
```

**Response**:
```json
{
  "normalizedId": "550e8400-e29b-41d4-a716-446655440000",
  "status": "INGESTED",
  "message": "Event ingested successfully"
}
```

### GET /telemetry/health
Health check endpoint

**Response**:
```json
{
  "status": "healthy",
  "timestamp": "2025-01-19T10:30:00Z",
  "eventsIngested": 15234,
  "eventsRejected": 12,
  "uptime": "72h 45m"
}
```

### GET /telemetry/metrics
Prometheus-compatible metrics endpoint

**Format**: text/plain (Prometheus format)

## CLI Commands

### Inspect
View ingestion status and metrics
```bash
npx @llm-observatory/cli telemetry-collector inspect [--provider=<provider>]
```

### Replay
Replay telemetry events from a time range
```bash
npx @llm-observatory/cli telemetry-collector replay --from=<timestamp> --to=<timestamp>
```

### Analyze
Analyze ingestion patterns and anomalies
```bash
npx @llm-observatory/cli telemetry-collector analyze [--provider=<provider>] [--time-window=<hours>]
```

### Status
Check agent health and status
```bash
npx @llm-observatory/cli telemetry-collector status [--detailed]
```

### Validate
Validate a telemetry event against schema
```bash
npx @llm-observatory/cli telemetry-collector validate --input=<file.json>
```

### Export
Export ingestion metrics to file
```bash
npx @llm-observatory/cli telemetry-collector export [--format=json|csv] [--output=<file>]
```

## Monitoring & Observability

### Metrics
Prometheus metrics available at GET /telemetry/metrics:
- `telemetry_events_ingested_total` - Total events ingested by provider and type
- `telemetry_ingestion_latency_ms` - Ingestion latency histogram
- `telemetry_queue_depth` - Current queue depth per consumer
- `telemetry_schema_validation_errors_total` - Schema validation errors

### Logging
- **Log Level**: Configurable via LOG_LEVEL environment variable
- **Format**: JSON (structured logging via pino)
- **Destination**: Console, file, or Google Cloud Logging

### Tracing
OpenTelemetry traces for:
- telemetry-ingestion
- schema-validation
- payload-normalization
- consumer-dispatch

## Secrets Management

Secrets are stored in Google Secret Manager:
- RUVECTOR_API_KEY
- OTEL_AUTH_TOKEN
- DATABASE_URL (if using persistent storage)
- REDIS_PASSWORD (if using redis caching)
- INGEST_API_KEY (if authentication enabled)

Update secrets:
```bash
gcloud secrets versions add ruvector-api-key --data-file=- < key.txt
```

## SLA Targets

- **Availability**: 99.9%
- **Latency (P50)**: 10ms
- **Latency (P99)**: 100ms
- **Throughput**: 1,000 events/second
- **Max Payload Size**: 10MB

## Deployment Checklist

Before Deployment:
- [ ] All environment variables configured
- [ ] Secrets stored in Google Secret Manager
- [ ] Service account IAM roles configured
- [ ] VPC connector set up (if using VPC)
- [ ] Schema registry endpoint accessible
- [ ] RuVector service endpoint accessible

Build & Deploy:
- [ ] npm install
- [ ] npm run build
- [ ] npm run test
- [ ] npm run lint
- [ ] npm run deploy

Post-Deployment:
- [ ] Verify health endpoint responding
- [ ] Verify metrics endpoint responding
- [ ] Check logs for errors
- [ ] Test telemetry ingestion
- [ ] Verify downstream consumers receiving events
- [ ] Set up alerting for SLA violations

## Support

For issues or questions:
- Check logs: `npm run logs`
- Review agent manifest: See agent-manifest.json
- Check GCP Cloud Functions: `npm run describe`
- Report issues: https://github.com/globalbusinessadvisors/llm-observatory/issues

---

**Last Updated**: January 19, 2025
**Agent Version**: 1.0.0
**Status**: Production Ready
