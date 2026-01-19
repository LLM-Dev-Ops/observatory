# Telemetry Collector Agent - Runtime Layer

**Version**: 1.0.0
**Status**: Production Ready
**Deployment**: Google Cloud Edge Function

## Overview

The Telemetry Collector Agent is a **read-only, non-enforcing, non-analytical** agent that ingests LLM telemetry events, normalizes them to canonical schema, and persists them via the ruvector-service.

## Constitutional Constraints

This agent adheres to strict constitutional rules:

- ✅ **READ-ONLY**: Only observes and records telemetry
- ✅ **NON-ENFORCING**: Does not apply policies or constraints
- ✅ **NON-ANALYTICAL**: No analysis or decision-making
- ✅ **STATELESS**: No local persistence or state
- ✅ **DETERMINISTIC**: Predictable behavior for all inputs
- ✅ **ASYNC WRITES**: Non-blocking persistence to ruvector-service only

## Architecture

```
┌─────────────────┐
│  HTTP Request   │
│  (Telemetry)    │
└────────┬────────┘
         │
         ▼
┌─────────────────────────────────────────┐
│         handler.ts                      │
│  • Parse & validate request             │
│  • Route to appropriate handler         │
│  • Create response                      │
└────────┬────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────┐
│        normalizer.ts                    │
│  • Validate events                      │
│  • Normalize to canonical schema        │
│  • Calculate inputs_hash (SHA-256)      │
│  • Sanitize service identifiers         │
└────────┬────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────┐
│         emitter.ts                      │
│  • Create DecisionEvent                 │
│  • Set confidence = 1.0                 │
│  • Set constraints_applied = []         │
│  • Generate execution_ref               │
└────────┬────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────┐
│       telemetry.ts                      │
│  • Track metrics (allowed per const.)   │
│  • Emit self-observation telemetry      │
│  • Record latency, error rates          │
└────────┬────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────┐
│     ruvector-client.ts                  │
│  • Async, non-blocking writes           │
│  • Retry logic with exponential backoff │
│  • Connection pooling                   │
└────────┬────────────────────────────────┘
         │
         ▼
┌─────────────────┐
│ ruvector-service│
│   (Persistence) │
└─────────────────┘
```

## File Structure

```
src/
├── index.ts              # Entry point, Cloud Function export
├── handler.ts            # Main HTTP handler
├── normalizer.ts         # Event normalization
├── emitter.ts            # DecisionEvent creation
├── telemetry.ts          # Self-observation telemetry
├── config.ts             # Configuration management
├── ruvector-client.ts    # Ruvector service HTTP client
└── types/
    ├── schemas.ts        # Event and decision schemas
    ├── ruvector.ts       # Ruvector client types
    └── index.ts          # Type exports
```

## API Endpoints

### POST /ingest

Ingest telemetry events (single or batch).

**Request Body** (single event):
```json
{
  "provider": "openai",
  "model": "gpt-4",
  "inputType": "chat",
  "input": [{"role": "user", "content": "Hello"}],
  "output": {"content": "Hi there!"},
  "tokenUsage": {
    "promptTokens": 10,
    "completionTokens": 5,
    "totalTokens": 15
  },
  "cost": {
    "amountUsd": 0.0003
  },
  "metadata": {
    "userId": "user-123",
    "environment": "production"
  }
}
```

**Request Body** (batch):
```json
[
  { "provider": "openai", "model": "gpt-4", ... },
  { "provider": "anthropic", "model": "claude-3", ... }
]
```

**Response**:
```json
{
  "success": true,
  "processed": 2,
  "failed": 0,
  "eventIds": ["evt_abc123", "evt_def456"],
  "executionRef": "telemetry-collector-agent:1234567890:abc123",
  "processingTimeMs": 45
}
```

### GET /health

Health check endpoint.

**Response**:
```json
{
  "status": "healthy",
  "agent": {
    "id": "telemetry-collector-agent",
    "version": "1.0.0",
    "uptime": 3600
  },
  "ruvector": {
    "healthy": true,
    "endpoint": "http://localhost:3001",
    "latencyMs": 12
  },
  "metrics": {
    "ingestionCount": 1000,
    "errorCount": 5,
    "avgLatencyMs": 42,
    "successRate": 0.995,
    "uptimeSeconds": 3600
  },
  "timestamp": "2025-01-19T12:00:00.000Z"
}
```

## Environment Variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `AGENT_ID` | No | `telemetry-collector` | Agent identifier |
| `AGENT_VERSION` | No | `1.0.0` | Agent version |
| `RUVECTOR_ENDPOINT` | No | `http://localhost:3001` | Ruvector service URL |
| `RUVECTOR_API_KEY` | No | - | Ruvector API key |
| `RUVECTOR_TIMEOUT` | No | `30000` | Request timeout (ms) |
| `RUVECTOR_RETRY_ATTEMPTS` | No | `3` | Max retry attempts |
| `RUVECTOR_RETRY_DELAY_MS` | No | `1000` | Initial retry delay |
| `RUVECTOR_MAX_RETRY_DELAY_MS` | No | `10000` | Max retry delay |
| `RUVECTOR_CONNECTION_POOL_SIZE` | No | `5` | Connection pool size |
| `SELF_OBSERVATION_ENABLED` | No | `false` | Enable self-observation |
| `BATCH_SIZE` | No | `10` | Max batch size |
| `TIMEOUT_MS` | No | `30000` | Request timeout |

## Build & Deploy

### Build
```bash
npm install
npm run build
```

### Deploy to Google Cloud Functions
```bash
gcloud functions deploy telemetry-collector \
  --runtime nodejs20 \
  --trigger-http \
  --allow-unauthenticated \
  --entry-point telemetryCollector \
  --set-env-vars RUVECTOR_ENDPOINT=https://ruvector.example.com
```

### Local Development
```bash
npm run dev  # Watch mode
```

## Key Features

### 1. **Normalization**
- Converts provider names to canonical format
- Validates and normalizes timestamps to UTC
- Calculates SHA-256 hash of inputs for deduplication
- Sanitizes service identifiers

### 2. **DecisionEvent Structure**
```typescript
{
  eventId: "dec_abc123",
  eventType: "telemetry_ingestion",
  timestamp: Date,
  agentId: "telemetry-collector-agent",
  agentVersion: "1.0.0",
  decisionType: "read_only_observation",
  inputs: TelemetryEvent[],
  outputs: NormalizedTelemetry[],
  confidence: 1.0,              // Always 1.0 (read-only)
  constraintsApplied: [],       // Always empty (non-enforcing)
  executionRef: "agent:ts:rand",
  processingTimeMs: 45,
  batchSize: 2
}
```

### 3. **Self-Observation**
Per constitution, self-observation is allowed:
- Tracks ingestion count, error count, latency
- Emits agent telemetry events
- OpenTelemetry-compatible patterns

### 4. **Error Handling**
- Graceful degradation
- Retry logic with exponential backoff
- Detailed error responses
- Structured logging

## Testing

```bash
# Run tests
npm test

# Test ingestion endpoint
curl -X POST http://localhost:8080/ingest \
  -H "Content-Type: application/json" \
  -d '{
    "provider": "openai",
    "model": "gpt-4",
    "inputType": "text",
    "input": "Hello, world!"
  }'

# Test health endpoint
curl http://localhost:8080/health
```

## Performance

- **Cold Start**: <500ms
- **Warm Request**: <50ms (excluding ruvector persistence)
- **Batch Processing**: 100 events/request max
- **Concurrent Connections**: 5 (configurable)
- **Memory**: ~128MB

## Security

- No secrets in code (environment variables only)
- Input validation on all requests
- Sanitization of service identifiers
- CORS support for cross-origin requests
- Rate limiting (via Cloud Functions)

## License

Apache-2.0 - See LICENSE file for details

## Contributing

This is a constitutional agent. Any changes must preserve the READ-ONLY, NON-ENFORCING, NON-ANALYTICAL constraints.
