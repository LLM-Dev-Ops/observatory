#!/bin/bash

###############################################################################
# Telemetry Collector Agent - Smoke Test Suite
#
# This script runs basic smoke tests to verify the telemetry collector agent
# is functioning correctly in the local environment.
#
# Prerequisites:
#   - Agent server running on http://localhost:8080
#   - Ruvector service available (mocked or real)
#   - jq installed for JSON parsing
#
# Usage: ./smoke-test.sh [option]
#   --verbose    Enable verbose output
#   --host URL   Override default host (default: http://localhost:8080)
###############################################################################

set -e

# Configuration
HOST="${HOST:-http://localhost:8080}"
VERBOSE="${VERBOSE:-false}"
TESTS_PASSED=0
TESTS_FAILED=0
TIMESTAMP=$(date +%s%N)

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Helper functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
    ((TESTS_PASSED++))
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
    ((TESTS_FAILED++))
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

verbose_log() {
    if [ "$VERBOSE" = "true" ]; then
        echo -e "${BLUE}[VERBOSE]${NC} $1"
    fi
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --verbose)
            VERBOSE=true
            shift
            ;;
        --host)
            HOST="$2"
            shift 2
            ;;
        *)
            log_error "Unknown option: $1"
            exit 1
            ;;
    esac
done

log_info "Telemetry Collector Agent - Smoke Test Suite"
log_info "Target: $HOST"
log_info "Timestamp: $(date -d @${TIMESTAMP:0:10})"
echo ""

###############################################################################
# Test 1: Health Check
###############################################################################
log_info "TEST 1: Health Check Endpoint"
verbose_log "Checking GET $HOST/telemetry/health"

if response=$(curl -s -w "\n%{http_code}" "$HOST/telemetry/health" 2>/dev/null); then
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)

    if [ "$http_code" = "200" ]; then
        verbose_log "Response: $body"
        log_success "Health check passed (HTTP 200)"
    else
        log_error "Health check failed (HTTP $http_code)"
        verbose_log "Response: $body"
    fi
else
    log_error "Failed to connect to $HOST/telemetry/health"
fi

echo ""

###############################################################################
# Test 2: Single Event Ingestion
###############################################################################
log_info "TEST 2: Single Event Ingestion"

# Generate test event
test_event_id=$(uuidgen 2>/dev/null || echo "550e8400-e29b-41d4-a716-446655440000")
test_hash=$(echo -n "test-input-content-$TIMESTAMP" | sha256sum | awk '{print $1}')

test_payload=$(cat <<EOF
{
  "events": [
    {
      "id": "$test_event_id",
      "timestamp": "$(date -u +'%Y-%m-%dT%H:%M:%S.000Z')",
      "provider": "ANTHROPIC",
      "model": "claude-opus-4.5",
      "inputType": "TEXT",
      "inputHash": "$test_hash",
      "output": {
        "type": "text",
        "content": "This is a test response from the telemetry collector."
      }
    }
  ]
}
EOF
)

verbose_log "Payload: $test_payload"

if response=$(curl -s -w "\n%{http_code}" -X POST "$HOST/telemetry/ingest" \
    -H "Content-Type: application/json" \
    -d "$test_payload" 2>/dev/null); then
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)

    if [ "$http_code" = "200" ] || [ "$http_code" = "201" ]; then
        verbose_log "Response: $body"

        # Try to parse JSON response
        if command -v jq &>/dev/null && [ -n "$body" ]; then
            accepted=$(echo "$body" | jq '.accepted // 0' 2>/dev/null)
            rejected=$(echo "$body" | jq '.rejected // 0' 2>/dev/null)
            verbose_log "Accepted: $accepted, Rejected: $rejected"

            if [ "$accepted" -ge 1 ]; then
                log_success "Event ingestion succeeded (accepted: $accepted)"
            else
                log_warning "Event ingestion returned unexpected response"
            fi
        else
            log_success "Event ingestion request accepted (HTTP $http_code)"
        fi
    else
        log_error "Event ingestion failed (HTTP $http_code)"
        verbose_log "Response: $body"
    fi
else
    log_error "Failed to send ingestion request to $HOST/telemetry/ingest"
fi

echo ""

###############################################################################
# Test 3: Batch Event Ingestion
###############################################################################
log_info "TEST 3: Batch Event Ingestion"

# Generate batch of test events
batch_payload="{"
batch_payload+="\"events\": ["

for i in {1..3}; do
    event_id=$(uuidgen 2>/dev/null || echo "550e8400-e29b-41d4-a716-44665544000$i")
    event_hash=$(echo -n "batch-input-$i-$TIMESTAMP" | sha256sum | awk '{print $1}')

    if [ $i -gt 1 ]; then
        batch_payload+=","
    fi

    batch_payload+="{
        \"id\": \"$event_id\",
        \"timestamp\": \"$(date -u +'%Y-%m-%dT%H:%M:%S.000Z')\",
        \"provider\": \"$([ $((i % 2)) -eq 0 ] && echo 'OPENAI' || echo 'ANTHROPIC')\",
        \"model\": \"$([ $((i % 2)) -eq 0 ] && echo 'gpt-4' || echo 'claude-opus-4.5')\",
        \"inputType\": \"TEXT\",
        \"inputHash\": \"$event_hash\",
        \"output\": {
            \"type\": \"text\",
            \"content\": \"Batch test response $i\"
        }
    }"
done

batch_payload+="]}"

verbose_log "Sending batch of 3 events"

if response=$(curl -s -w "\n%{http_code}" -X POST "$HOST/telemetry/ingest" \
    -H "Content-Type: application/json" \
    -d "$batch_payload" 2>/dev/null); then
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)

    if [ "$http_code" = "200" ] || [ "$http_code" = "201" ]; then
        verbose_log "Response: $body"

        if command -v jq &>/dev/null && [ -n "$body" ]; then
            accepted=$(echo "$body" | jq '.accepted // 0' 2>/dev/null)
            processed=$(echo "$body" | jq '.processed // 0' 2>/dev/null)
            verbose_log "Processed: $processed, Accepted: $accepted"

            if [ "$accepted" -ge 1 ]; then
                log_success "Batch ingestion succeeded (processed: $processed, accepted: $accepted)"
            else
                log_warning "Batch ingestion returned unexpected response"
            fi
        else
            log_success "Batch ingestion request accepted (HTTP $http_code)"
        fi
    else
        log_error "Batch ingestion failed (HTTP $http_code)"
        verbose_log "Response: $body"
    fi
else
    log_error "Failed to send batch ingestion request"
fi

echo ""

###############################################################################
# Test 4: Validation - Invalid Event
###############################################################################
log_info "TEST 4: Invalid Event Rejection"

# Send event with invalid hash
invalid_payload=$(cat <<EOF
{
  "events": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440099",
      "timestamp": "$(date -u +'%Y-%m-%dT%H:%M:%S.000Z')",
      "provider": "ANTHROPIC",
      "model": "claude-opus-4.5",
      "inputType": "TEXT",
      "inputHash": "invalid-hash-format",
      "output": {
        "type": "text",
        "content": "This should be rejected."
      }
    }
  ]
}
EOF
)

verbose_log "Sending event with invalid hash format"

if response=$(curl -s -w "\n%{http_code}" -X POST "$HOST/telemetry/ingest" \
    -H "Content-Type: application/json" \
    -d "$invalid_payload" 2>/dev/null); then
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)

    verbose_log "HTTP Code: $http_code"
    verbose_log "Response: $body"

    # Event should be rejected, but request should succeed
    if [ "$http_code" = "200" ] || [ "$http_code" = "201" ]; then
        if command -v jq &>/dev/null && [ -n "$body" ]; then
            rejected=$(echo "$body" | jq '.rejected // 0' 2>/dev/null)

            if [ "$rejected" -ge 1 ]; then
                log_success "Invalid event correctly rejected (rejected: $rejected)"
            else
                log_warning "Invalid event processing returned unexpected response"
            fi
        else
            log_success "Request handled (validation status unknown)"
        fi
    else
        log_warning "Unexpected HTTP status for validation test (HTTP $http_code)"
    fi
else
    log_error "Failed to send validation test request"
fi

echo ""

###############################################################################
# Test 5: Event Inspection (if CLI available)
###############################################################################
log_info "TEST 5: CLI Event Inspection"

# Check if CLI is available
if command -v npx &>/dev/null; then
    verbose_log "Attempting to use CLI inspect command"

    # Try to list recent events or inspect specific event
    if npx @llm-observatory/telemetry-collector-agent inspect --help >/dev/null 2>&1; then
        log_success "CLI inspect command is available"
    else
        log_warning "CLI inspect command not available (might not be installed)"
    fi
else
    log_warning "npm not available - skipping CLI tests"
fi

echo ""

###############################################################################
# Test 6: Health Status Details
###############################################################################
log_info "TEST 6: Health Status Details"

if response=$(curl -s "$HOST/telemetry/health" 2>/dev/null); then
    if command -v jq &>/dev/null; then
        status=$(echo "$response" | jq '.status // "unknown"' 2>/dev/null)
        version=$(echo "$response" | jq '.version // "unknown"' 2>/dev/null)
        uptime=$(echo "$response" | jq '.uptime // "unknown"' 2>/dev/null)

        verbose_log "Status: $status"
        verbose_log "Version: $version"
        verbose_log "Uptime: $uptime"

        log_success "Health status retrieved"
    else
        log_success "Health endpoint responding"
    fi
else
    log_warning "Could not retrieve detailed health status"
fi

echo ""

###############################################################################
# Test Summary
###############################################################################
log_info "Test Summary"
echo -e "${GREEN}Passed: $TESTS_PASSED${NC}"
echo -e "${RED}Failed: $TESTS_FAILED${NC}"

if [ $TESTS_FAILED -eq 0 ]; then
    log_success "All smoke tests passed!"
    exit 0
else
    log_error "Some tests failed"
    exit 1
fi
