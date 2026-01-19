# LLM Observatory CLI Commands

All agents are callable via `agentics-cli`. Each CLI command routes to the correct agent endpoint dynamically.

## CLI Configuration

The CLI resolves the service URL dynamically via environment:

```bash
# Set Observatory service URL
export LLM_OBSERVATORY_URL=https://llm-observatory-<region>-<project>.a.run.app

# Or use agentics-cli config
agentics-cli config set observatory.url $LLM_OBSERVATORY_URL
```

---

## 1. Telemetry Collector Agent

### Commands

```bash
# Inspect ingestion status and metrics
agentics-cli observatory telemetry inspect [--provider=<provider>]

# Ingest telemetry events
agentics-cli observatory telemetry ingest --input=<file.json>
agentics-cli observatory telemetry ingest --provider=openai --event-type=completion --payload='{"model":"gpt-4"}'

# Replay telemetry events from a time range
agentics-cli observatory telemetry replay --from=<timestamp> --to=<timestamp>

# Analyze ingestion patterns
agentics-cli observatory telemetry analyze [--provider=<provider>] [--time-window=<hours>]

# Check agent health
agentics-cli observatory telemetry status [--detailed]

# Validate event against schema
agentics-cli observatory telemetry validate --input=<file.json>

# Export metrics
agentics-cli observatory telemetry export [--format=json|csv] [--output=<file>]
```

### Example Invocation

```bash
$ agentics-cli observatory telemetry status --detailed

{
  "agent": {
    "id": "telemetry-collector-agent",
    "version": "1.0.0",
    "status": "healthy"
  },
  "metrics": {
    "events_ingested_total": 15420,
    "events_rejected_total": 23,
    "avg_latency_ms": 12
  },
  "ruvector": {
    "healthy": true,
    "latency_ms": 8
  }
}
```

---

## 2. Usage Pattern Agent

### Commands

```bash
# Analyze usage patterns
agentics-cli observatory usage analyze --time-range=<range> [--dimensions=provider,model]

# Get pattern summary
agentics-cli observatory usage summary [--period=24h]

# Check agent health
agentics-cli observatory usage status
```

### Example Invocation

```bash
$ agentics-cli observatory usage analyze --time-range="24h" --dimensions=provider,model

{
  "success": true,
  "patterns": [
    {
      "provider": "openai",
      "model": "gpt-4-turbo",
      "request_count": 5420,
      "avg_tokens": 1250,
      "trend": "increasing"
    }
  ],
  "processing_time_ms": 45
}
```

---

## 3. Failure Classification Agent

### Commands

```bash
# Classify failure events
agentics-cli observatory failure classify --input=<file.json>

# Get classification summary
agentics-cli observatory failure summary [--time-range=<range>]

# Analyze failure patterns
agentics-cli observatory failure analyze [--provider=<provider>]

# Check agent health
agentics-cli observatory failure status
```

### Example Invocation

```bash
$ agentics-cli observatory failure classify --input=errors.json

{
  "success": true,
  "classifications": [
    {
      "error_id": "err-123",
      "category": "rate_limit",
      "severity": "warning",
      "confidence": 0.95
    }
  ],
  "execution_ref": "abc-123-def"
}
```

---

## 4. Health Check Agent

### Commands

```bash
# Evaluate health for targets
agentics-cli observatory health evaluate --targets=<targets.json>

# Get health summary
agentics-cli observatory health summary [--scope=all|degraded|unhealthy]

# Check agent health
agentics-cli observatory health status
```

### Example Invocation

```bash
$ agentics-cli observatory health evaluate --targets='[{"type":"provider","id":"openai"}]'

{
  "success": true,
  "evaluations": [
    {
      "target": {"type": "provider", "id": "openai"},
      "status": "healthy",
      "indicators": {
        "availability": 0.999,
        "latency_p99": 245
      }
    }
  ]
}
```

---

## 5. SLO Enforcement Agent

### Commands

```bash
# Evaluate SLOs
agentics-cli observatory slo enforce --input=<slos.json> --metrics=<metrics.json>

# Query violations
agentics-cli observatory slo violations [--severity=critical|warning] [--time-range=<range>]

# Get analysis
agentics-cli observatory slo analysis [--group-by=slo|provider]

# Replay a decision
agentics-cli observatory slo replay --execution-ref=<ref>

# Check agent health
agentics-cli observatory slo status
```

### Example Invocation

```bash
$ agentics-cli observatory slo violations --severity=critical --time-range=24h

{
  "success": true,
  "violations": [
    {
      "slo_id": "latency-p99",
      "breach_type": "threshold",
      "severity": "critical",
      "detected_at": "2025-01-19T10:30:00Z",
      "current_value": 520,
      "threshold": 500
    }
  ],
  "count": 1
}
```

---

## 6. Post-Mortem Generator Agent

### Commands

```bash
# Generate post-mortem report
agentics-cli observatory postmortem generate --incident=<id> [--include-metrics]

# List recent incidents
agentics-cli observatory postmortem list [--time-range=<range>]

# Check agent health
agentics-cli observatory postmortem status
```

### Example Invocation

```bash
$ agentics-cli observatory postmortem generate --incident=INC-2025-0119 --include-metrics

{
  "success": true,
  "postmortem": {
    "incident_id": "INC-2025-0119",
    "title": "Elevated Error Rates on OpenAI Provider",
    "timeline": [...],
    "root_cause": "Rate limiting due to traffic spike",
    "impact": {...},
    "recommendations": [...]
  }
}
```

---

## 7. Visualization Spec Agent

### Commands

```bash
# Generate dashboard spec
agentics-cli observatory viz generate --type=<dashboard-type> --sources=<sources.json>

# List available dashboard types
agentics-cli observatory viz types

# Check agent health
agentics-cli observatory viz status
```

### Example Invocation

```bash
$ agentics-cli observatory viz generate --type=provider-health

{
  "success": true,
  "spec": {
    "dashboard_type": "provider-health",
    "panels": [
      {
        "title": "Request Volume",
        "type": "timeseries",
        "query": "sum(rate(requests_total[5m])) by (provider)"
      }
    ]
  }
}
```

---

## Common Options

All commands support:

| Option | Description |
|--------|-------------|
| `--format=json\|yaml\|table` | Output format (default: json) |
| `--output=<file>` | Write output to file |
| `--verbose` | Enable verbose output |
| `--quiet` | Suppress non-essential output |
| `--dry-run` | Preview action without executing |

## Environment Variables

| Variable | Description |
|----------|-------------|
| `LLM_OBSERVATORY_URL` | Observatory service URL |
| `AGENTICS_API_KEY` | API key for authentication |
| `AGENTICS_ENV` | Environment (dev/staging/prod) |
