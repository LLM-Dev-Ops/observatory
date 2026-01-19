# Telemetry Collector Agent - Verification Checklist

## Overview

This checklist provides manual verification steps to ensure the Telemetry Collector Agent meets all specification requirements. Follow each section in order, documenting results and any issues encountered.

**Last Updated:** 2026-01-19
**Agent Version:** 1.0.0

---

## Pre-Verification Setup

Before starting verification, ensure:

- [ ] Agent is built and running locally
- [ ] Ruvector service is accessible (mocked or real)
- [ ] Test database/storage is initialized
- [ ] Network connectivity verified to all backends
- [ ] Environment variables configured correctly

**Setup Command:**
```bash
npm run build
npm run start:dev
npm run test -- --run  # Run all tests first
```

---

## 1. Event Ingestion - Valid Events

### 1.1 Accept Valid TEXT Events
- [ ] Send valid TEXT event with all required fields
- [ ] Verify HTTP 200/201 response
- [ ] Confirm event_id in response
- [ ] Check event stored in decision log

**Test Command:**
```bash
curl -X POST http://localhost:8080/telemetry/ingest \
  -H "Content-Type: application/json" \
  -d '{
    "events": [{
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "timestamp": "2026-01-19T10:00:00Z",
      "provider": "ANTHROPIC",
      "model": "claude-opus-4.5",
      "inputType": "TEXT",
      "inputHash": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "output": {
        "type": "text",
        "content": "Test response content"
      }
    }]
  }'
```

**Expected Result:**
- HTTP 200 or 201
- Response contains `accepted: 1`
- DecisionEvent persisted to ruvector

### 1.2 Accept Valid CHAT Events
- [ ] Send valid CHAT event with dialogue content
- [ ] Verify HTTP 200/201 response
- [ ] Confirm event processed and decision created

**Test Command:**
```bash
curl -X POST http://localhost:8080/telemetry/ingest \
  -H "Content-Type: application/json" \
  -d '{
    "events": [{
      "id": "550e8400-e29b-41d4-a716-446655440001",
      "timestamp": "2026-01-19T10:00:00Z",
      "provider": "OPENAI",
      "model": "gpt-4",
      "inputType": "CHAT",
      "inputHash": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "output": {
        "type": "text",
        "content": "This is a chat response"
      }
    }]
  }'
```

**Expected Result:**
- HTTP 200 or 201
- Response contains `accepted: 1`
- Event metadata preserved

### 1.3 Accept Valid MULTIMODAL Events
- [ ] Send valid MULTIMODAL event with mixed content
- [ ] Verify event processed successfully
- [ ] Check metadata preserved through persistence

**Test Command:**
```bash
curl -X POST http://localhost:8080/telemetry/ingest \
  -H "Content-Type: application/json" \
  -d '{
    "events": [{
      "id": "550e8400-e29b-41d4-a716-446655440002",
      "timestamp": "2026-01-19T10:00:00Z",
      "provider": "GOOGLE",
      "model": "palm-2",
      "inputType": "MULTIMODAL",
      "inputHash": "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
      "output": {
        "type": "text",
        "content": "Multimodal analysis result"
      }
    }]
  }'
```

**Expected Result:**
- HTTP 200 or 201
- DecisionEvent with confidence score
- Metadata intact

### 1.4 Accept Events with Optional Metadata
- [ ] Send event with custom metadata fields
- [ ] Verify metadata preserved in DecisionEvent
- [ ] Confirm all standard metadata fields present

**Test Command:**
```bash
curl -X POST http://localhost:8080/telemetry/ingest \
  -H "Content-Type: application/json" \
  -d '{
    "events": [{
      "id": "550e8400-e29b-41d4-a716-446655440003",
      "timestamp": "2026-01-19T10:00:00Z",
      "provider": "ANTHROPIC",
      "model": "claude-opus-4.5",
      "inputType": "TEXT",
      "inputHash": "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
      "output": {
        "type": "text",
        "content": "Response with metadata"
      },
      "metadata": {
        "tokenCount": 250,
        "processingTime": 125,
        "customField": "customValue"
      }
    }]
  }'
```

**Expected Result:**
- HTTP 200 or 201
- Metadata accessible in DecisionEvent
- No errors processing additional fields

---

## 2. Event Ingestion - Invalid Events

### 2.1 Reject Malformed JSON
- [ ] Send malformed JSON payload
- [ ] Verify HTTP error response (400 or 422)
- [ ] Check error message is descriptive

**Test Command:**
```bash
curl -X POST http://localhost:8080/telemetry/ingest \
  -H "Content-Type: application/json" \
  -d '{invalid json'
```

**Expected Result:**
- HTTP 400 or 422
- Error message indicates JSON parsing error
- No partial processing

### 2.2 Reject Missing Required Fields
- [ ] Send event missing `id` field
- [ ] Send event missing `timestamp` field
- [ ] Send event missing `provider` field
- [ ] Verify rejection with error message for each

**Test Case: Missing ID**
```bash
curl -X POST http://localhost:8080/telemetry/ingest \
  -H "Content-Type: application/json" \
  -d '{
    "events": [{
      "timestamp": "2026-01-19T10:00:00Z",
      "provider": "ANTHROPIC",
      "model": "claude-opus-4.5",
      "inputType": "TEXT",
      "inputHash": "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
      "output": {
        "type": "text",
        "content": "Test"
      }
    }]
  }'
```

**Expected Result:**
- HTTP 200 or 201 (batch endpoint)
- Response contains `rejected: 1`
- Error message mentions missing "id"

### 2.3 Reject Invalid Hash Format
- [ ] Send event with non-hex hash
- [ ] Send event with hash too short
- [ ] Send event with hash too long
- [ ] Verify validation error for each

**Test Command:**
```bash
curl -X POST http://localhost:8080/telemetry/ingest \
  -H "Content-Type: application/json" \
  -d '{
    "events": [{
      "id": "550e8400-e29b-41d4-a716-446655440004",
      "timestamp": "2026-01-19T10:00:00Z",
      "provider": "ANTHROPIC",
      "model": "claude-opus-4.5",
      "inputType": "TEXT",
      "inputHash": "invalid-hash-format",
      "output": {
        "type": "text",
        "content": "Test"
      }
    }]
  }'
```

**Expected Result:**
- Response contains `rejected: 1`
- Error mentions "SHA-256" or "hash format"
- Event not persisted

### 2.4 Reject Invalid Provider
- [ ] Send event with unknown provider
- [ ] Verify rejection with error message
- [ ] Check that OTHER provider is not accepted for unknown

**Test Command:**
```bash
curl -X POST http://localhost:8080/telemetry/ingest \
  -H "Content-Type: application/json" \
  -d '{
    "events": [{
      "id": "550e8400-e29b-41d4-a716-446655440005",
      "timestamp": "2026-01-19T10:00:00Z",
      "provider": "INVALID_PROVIDER",
      "model": "some-model",
      "inputType": "TEXT",
      "inputHash": "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
      "output": {
        "type": "text",
        "content": "Test"
      }
    }]
  }'
```

**Expected Result:**
- Response contains `rejected: 1`
- Error message indicates invalid provider

### 2.5 Reject Invalid InputType
- [ ] Send event with unknown inputType
- [ ] Verify validation error
- [ ] Check supported types are enforced

**Test Command:**
```bash
curl -X POST http://localhost:8080/telemetry/ingest \
  -H "Content-Type: application/json" \
  -d '{
    "events": [{
      "id": "550e8400-e29b-41d4-a716-446655440006",
      "timestamp": "2026-01-19T10:00:00Z",
      "provider": "ANTHROPIC",
      "model": "claude-opus-4.5",
      "inputType": "INVALID_TYPE",
      "inputHash": "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
      "output": {
        "type": "text",
        "content": "Test"
      }
    }]
  }'
```

**Expected Result:**
- Response contains `rejected: 1`
- Error mentions invalid inputType

### 2.6 Reject Future Timestamps
- [ ] Send event with timestamp 5 minutes in future
- [ ] Verify rejection with timestamp error
- [ ] Confirm tolerance for clock skew (should allow ~1 minute)

**Test Command:**
```bash
# Set timestamp 5 minutes in future
FUTURE_TIME=$(date -u -d '+5 minutes' +'%Y-%m-%dT%H:%M:%SZ')
curl -X POST http://localhost:8080/telemetry/ingest \
  -H "Content-Type: application/json" \
  -d "{
    \"events\": [{
      \"id\": \"550e8400-e29b-41d4-a716-446655440007\",
      \"timestamp\": \"$FUTURE_TIME\",
      \"provider\": \"ANTHROPIC\",
      \"model\": \"claude-opus-4.5\",
      \"inputType\": \"TEXT\",
      \"inputHash\": \"ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff\",
      \"output\": {
        \"type\": \"text\",
        \"content\": \"Test\"
      }
    }]
  }"
```

**Expected Result:**
- Response contains `rejected: 1`
- Error message mentions "future timestamp"

---

## 3. Decision Event Persistence

### 3.1 DecisionEvents Persisted to Ruvector
- [ ] Verify DecisionEvent created for valid events
- [ ] Confirm DecisionEvent contains correct fields
- [ ] Check ruvector-service receives persisted events

**Manual Verification:**
1. Send valid telemetry event (see section 1.1)
2. Query ruvector for DecisionEvents:
   ```bash
   curl http://localhost:8081/api/decisions?agentId=agent-telemetry-collector-v1
   ```
3. Verify DecisionEvent in response with:
   - `agentId: "agent-telemetry-collector-v1"`
   - `agentVersion: "1.0.0"`
   - `decision: "ACCEPT_VALID_EVENT"`
   - `confidence: 0.95`
   - `telemetryEventIds` array contains sent event IDs

### 3.2 Decision Metadata Preservation
- [ ] Confirm event metadata preserved in DecisionEvent
- [ ] Check agent decision rationale stored
- [ ] Verify timestamp recorded

**Expected DecisionEvent Structure:**
```json
{
  "id": "UUID",
  "timestamp": "2026-01-19T10:00:00Z",
  "agentId": "agent-telemetry-collector-v1",
  "agentVersion": "1.0.0",
  "decision": "ACCEPT_VALID_EVENT",
  "reasoning": "Event passed all validation checks",
  "confidence": 0.95,
  "telemetryEventIds": ["550e8400-e29b-41d4-a716-446655440000"],
  "metadata": {
    "eventProvider": "ANTHROPIC",
    "eventModel": "claude-opus-4.5",
    "inputType": "TEXT"
  }
}
```

### 3.3 Ruvector Connection Failures
- [ ] Simulate ruvector service unavailable
- [ ] Verify event rejected with appropriate error
- [ ] Check error message is meaningful
- [ ] Confirm proper HTTP status returned

**Expected Behavior:**
- Events rejected when ruvector unavailable
- Error message indicates persistence failure
- No partial or inconsistent state

---

## 4. Self-Observation Telemetry

### 4.1 Self-Observation Events Recorded
- [ ] Process valid telemetry event
- [ ] Query ruvector for self-observation events from agent
- [ ] Verify self-observation telemetry is visible

**Query Command:**
```bash
curl "http://localhost:8081/api/observations?agentId=agent-telemetry-collector-v1"
```

**Expected Result:**
- Self-observation event recorded
- Contains event ingestion details
- Timestamp matches or is close to event ingestion

### 4.2 Self-Observation Accuracy
- [ ] Verify self-observation captures correct event metadata
- [ ] Check processing time metrics recorded
- [ ] Confirm validation status captured

**Expected Self-Observation Fields:**
- `type: "event_ingested"`
- `eventId: <processed event ID>`
- `provider: <ANTHROPIC|OPENAI|GOOGLE|OTHER>`
- `inputType: <TEXT|CHAT|MULTIMODAL>`
- `timestamp: <ISO 8601>`

### 4.3 Observable in Observatory UI
- [ ] Verify agent can be viewed in Observatory dashboard
- [ ] Check agent shows recent events processed
- [ ] Confirm metrics/statistics displayed correctly

**Manual Verification:**
1. Open Observatory dashboard (typically http://localhost:3000)
2. Navigate to Agents or Telemetry section
3. Find "agent-telemetry-collector-v1"
4. Verify recent events visible
5. Check processing statistics match ingested events

---

## 5. CLI Commands

### 5.1 Inspect Command Works
- [ ] Run `npx @llm-observatory/telemetry-collector-agent inspect <event-id>`
- [ ] Verify event details displayed
- [ ] Check formatting and readability

**Test Command:**
```bash
npx @llm-observatory/telemetry-collector-agent inspect \
  550e8400-e29b-41d4-a716-446655440000
```

**Expected Output:**
- Event ID, timestamp, provider displayed
- Full event details shown
- Exit code 0 on success

### 5.2 List Command Works
- [ ] Run list command to show recent events
- [ ] Verify pagination if supported
- [ ] Check filtering options work

**Test Command:**
```bash
npx @llm-observatory/telemetry-collector-agent list --limit 10
```

**Expected Output:**
- List of recent events
- Event summaries displayed
- Proper formatting

### 5.3 Health Command Works
- [ ] Run health command
- [ ] Verify connection status shown
- [ ] Check ruvector connection status

**Test Command:**
```bash
npx @llm-observatory/telemetry-collector-agent health
```

**Expected Output:**
- Agent status (healthy/degraded/unhealthy)
- Ruvector connection status
- Uptime and metrics

---

## 6. No SQL Queries Executed Directly

### 6.1 Verify SQL Abstraction
- [ ] Review logs for direct SQL queries (should be none)
- [ ] Check that all queries go through ruvector-service
- [ ] Verify no database connection from agent

**Log Check:**
```bash
grep -i "SELECT\|INSERT\|UPDATE\|DELETE" agent.log
```

**Expected Result:**
- No SQL statements in agent logs
- All database operations go through ruvector API

### 6.2 Verify Stateless Execution
- [ ] Agent has no persistent database connection
- [ ] All state stored externally (ruvector)
- [ ] Multiple agent instances don't conflict

**Verification Steps:**
1. Check agent source code for database imports
2. Verify no `.env` or configuration for database connection
3. Check that all persistence uses ruvector client library

---

## 7. No Orchestration Triggered

### 7.1 Agent Does Not Trigger Workflows
- [ ] Process events and verify no workflows triggered
- [ ] Check orchestration logs for agent activity (should be none)
- [ ] Confirm agent is purely observational

**Log Check:**
```bash
grep -i "orchestr\|workflow\|spawn\|agent.*trigger" agent.log
```

**Expected Result:**
- No orchestration/workflow triggers in logs
- Agent only records observations

### 7.2 Agent Does Not Spawn Other Agents
- [ ] Run ingestion and check agent count remains same
- [ ] Verify no dynamic agent spawning
- [ ] Confirm agent stays focused on telemetry collection

**Expected Behavior:**
- Agent count unchanged
- No sub-agent spawning
- Focused data collection only

---

## 8. Stateless Execution Verified

### 8.1 Restartability
- [ ] Restart agent mid-operation
- [ ] Verify no data loss or corruption
- [ ] Process same event again, verify idempotent

**Test Steps:**
1. Start processing batch of events
2. Kill agent process
3. Restart agent
4. Process same events again
5. Verify no duplicates or conflicts

**Expected Result:**
- No pending state
- Events processed consistently
- Idempotent behavior

### 8.2 Multiple Instance Support
- [ ] Run multiple agent instances
- [ ] Send events to different instances
- [ ] Verify all record events correctly
- [ ] Check no race conditions

**Test Steps:**
1. Start 2-3 agent instances on different ports
2. Send events to each instance
3. Query ruvector for all events
4. Verify all recorded correctly
5. Check no duplication or conflicts

**Expected Result:**
- Each instance processes independently
- All events recorded
- No conflicts or race conditions
- State consistent across instances

### 8.3 No Local State Persistence
- [ ] Check agent working directory
- [ ] Verify no state files created (.sqlite, .db, etc.)
- [ ] Confirm no local cache that survives restarts

**File Check:**
```bash
find /path/to/agent -type f -name "*.db" -o -name "*.sqlite" -o -name ".cache"
```

**Expected Result:**
- No local state files
- Only configuration files
- No persistence between restarts

---

## 9. Performance and Reliability

### 9.1 Event Processing Latency
- [ ] Send event and measure response time
- [ ] Verify response time < 1000ms for typical event
- [ ] Check latency under load (10+ concurrent requests)

**Load Test:**
```bash
for i in {1..10}; do
  curl -X POST http://localhost:8080/telemetry/ingest \
    -H "Content-Type: application/json" \
    -d '{"events": [...]}' &
done
wait
```

**Expected Result:**
- Response time consistently < 1000ms
- No timeouts under concurrent load
- Proper error handling on overload

### 9.2 Batch Processing
- [ ] Send batch of 50+ events
- [ ] Verify all processed correctly
- [ ] Check response contains accurate counts

**Batch Test:**
```bash
# Send 100-event batch
curl -X POST http://localhost:8080/telemetry/ingest \
  -H "Content-Type: application/json" \
  -d '{"events": [... 100 events ...]}'
```

**Expected Result:**
- All 100 events processed
- Response shows `processed: 100`
- Accurate acceptance/rejection breakdown

### 9.3 Error Recovery
- [ ] Send malformed event in batch with valid events
- [ ] Verify valid events processed despite error
- [ ] Check error doesn't cascade

**Mixed Batch Test:**
```bash
curl -X POST http://localhost:8080/telemetry/ingest \
  -H "Content-Type: application/json" \
  -d '{
    "events": [
      { valid event ... },
      { invalid event ... },
      { valid event ... }
    ]
  }'
```

**Expected Result:**
- `accepted: 2`, `rejected: 1`
- Valid events persisted
- Invalid event detailed error message

---

## 10. Summary and Sign-Off

### Verification Results

| Category | Passed | Failed | Notes |
|----------|--------|--------|-------|
| Valid Event Ingestion | [ ] | [ ] | |
| Invalid Event Rejection | [ ] | [ ] | |
| DecisionEvent Persistence | [ ] | [ ] | |
| Self-Observation Telemetry | [ ] | [ ] | |
| CLI Commands | [ ] | [ ] | |
| No SQL Queries | [ ] | [ ] | |
| No Orchestration | [ ] | [ ] | |
| Stateless Execution | [ ] | [ ] | |
| Performance | [ ] | [ ] | |

### Issues Found

**Critical Issues:**
- [ ] (None found)

**Major Issues:**
- [ ] (None found)

**Minor Issues:**
- [ ] (None found)

### Verified By

- **Date:** _______________
- **Verifier:** _______________
- **Status:** ☐ PASS  ☐ FAIL  ☐ CONDITIONAL PASS

### Sign-Off

I certify that the Telemetry Collector Agent has been verified against the specification and meets all requirements for production deployment.

**Signature:** _______________
**Date:** _______________
**Notes:** _______________

---

## Appendix: Useful Commands

### Debugging
```bash
# View agent logs
tail -f agent.log

# Check agent health
curl http://localhost:8080/telemetry/health

# Query ruvector directly
curl http://localhost:8081/api/decisions

# Inspect specific event
npx @llm-observatory/telemetry-collector-agent inspect <event-id>
```

### Testing
```bash
# Run unit tests
npm test

# Run integration tests
npm run test:integration

# Run smoke tests
./tests/smoke-test.sh

# Test with verbose output
./tests/smoke-test.sh --verbose
```

### Monitoring
```bash
# Monitor events in real-time
watch -n 1 'curl http://localhost:8081/api/decisions | jq ".length"'

# Check agent metrics
curl http://localhost:8080/telemetry/metrics
```
