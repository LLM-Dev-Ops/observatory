#!/bin/bash
# SLO/SLA Enforcement Agent - Smoke Test Script
#
# This script performs basic verification of the agent deployment.
#
# Usage:
#   ./scripts/smoke-test.sh [endpoint]
#
# Example:
#   ./scripts/smoke-test.sh http://localhost:8080
#   ./scripts/smoke-test.sh https://slo-enforcement-xyz.run.app

set -e

ENDPOINT="${1:-http://localhost:8080}"
PASSED=0
FAILED=0

echo "================================================"
echo "SLO/SLA Enforcement Agent - Smoke Tests"
echo "================================================"
echo "Endpoint: $ENDPOINT"
echo ""

# Helper function
check() {
  local name="$1"
  local expected="$2"
  local actual="$3"

  if [ "$actual" = "$expected" ]; then
    echo "✓ $name"
    ((PASSED++))
  else
    echo "✗ $name (expected: $expected, got: $actual)"
    ((FAILED++))
  fi
}

# Test 1: Health endpoint
echo "Test 1: Health Check"
HEALTH_RESPONSE=$(curl -s -o /dev/null -w "%{http_code}" "$ENDPOINT/health")
check "Health endpoint returns 200" "200" "$HEALTH_RESPONSE"

# Test 2: Root endpoint (agent info)
echo ""
echo "Test 2: Agent Info"
INFO_RESPONSE=$(curl -s "$ENDPOINT/")
AGENT_ID=$(echo "$INFO_RESPONSE" | jq -r '.agent_id')
check "Agent ID is correct" "slo-enforcement-agent" "$AGENT_ID"

CLASSIFICATION=$(echo "$INFO_RESPONSE" | jq -r '.classification')
check "Classification is enforcement-class" "enforcement-class" "$CLASSIFICATION"

ACTUATING=$(echo "$INFO_RESPONSE" | jq -r '.actuating')
check "Actuating is false" "false" "$ACTUATING"

# Test 3: Enforce endpoint with valid input
echo ""
echo "Test 3: Enforce Endpoint"

ENFORCE_PAYLOAD='{
  "slo_definitions": [
    {
      "slo_id": "test-latency",
      "name": "Test Latency SLO",
      "indicator": "latency_p95",
      "operator": "lt",
      "threshold": 500,
      "window": "5m",
      "enabled": true,
      "is_sla": false,
      "warning_threshold_percentage": 80
    }
  ],
  "metrics": [
    {
      "metric_id": "550e8400-e29b-41d4-a716-446655440000",
      "indicator": "latency_p95",
      "value": 750,
      "window": "5m",
      "timestamp": "'$(date -u +"%Y-%m-%dT%H:%M:%SZ")'"
    }
  ],
  "evaluation_time": "'$(date -u +"%Y-%m-%dT%H:%M:%SZ")'"
}'

ENFORCE_RESPONSE=$(curl -s -w "\n%{http_code}" \
  -X POST \
  -H "Content-Type: application/json" \
  -d "$ENFORCE_PAYLOAD" \
  "$ENDPOINT/enforce")

ENFORCE_STATUS=$(echo "$ENFORCE_RESPONSE" | tail -n1)
ENFORCE_BODY=$(echo "$ENFORCE_RESPONSE" | sed '$d')

check "Enforce endpoint returns 200" "200" "$ENFORCE_STATUS"

SUCCESS=$(echo "$ENFORCE_BODY" | jq -r '.success')
check "Response success is true" "true" "$SUCCESS"

VIOLATIONS=$(echo "$ENFORCE_BODY" | jq -r '.data.violations | length')
check "One violation detected" "1" "$VIOLATIONS"

# Test 4: Validation error handling
echo ""
echo "Test 4: Validation Error Handling"

INVALID_PAYLOAD='{"invalid": "payload"}'
INVALID_RESPONSE=$(curl -s -o /dev/null -w "%{http_code}" \
  -X POST \
  -H "Content-Type: application/json" \
  -d "$INVALID_PAYLOAD" \
  "$ENDPOINT/enforce")

check "Invalid payload returns 400" "400" "$INVALID_RESPONSE"

# Test 5: Unknown endpoint handling
echo ""
echo "Test 5: Unknown Endpoint"
UNKNOWN_RESPONSE=$(curl -s -o /dev/null -w "%{http_code}" "$ENDPOINT/unknown")
check "Unknown endpoint returns 404" "404" "$UNKNOWN_RESPONSE"

# Test 6: CORS headers
echo ""
echo "Test 6: CORS Headers"
CORS_RESPONSE=$(curl -s -I -X OPTIONS "$ENDPOINT/enforce" 2>/dev/null | grep -i "access-control-allow-origin" | wc -l)
if [ "$CORS_RESPONSE" -ge 1 ]; then
  echo "✓ CORS headers present"
  ((PASSED++))
else
  echo "✗ CORS headers missing"
  ((FAILED++))
fi

# Test 7: Agent headers
echo ""
echo "Test 7: Agent Headers"
AGENT_HEADER=$(curl -s -I "$ENDPOINT/health" 2>/dev/null | grep -i "x-agent-id" | wc -l)
if [ "$AGENT_HEADER" -ge 1 ]; then
  echo "✓ X-Agent-ID header present"
  ((PASSED++))
else
  echo "✗ X-Agent-ID header missing"
  ((FAILED++))
fi

# Summary
echo ""
echo "================================================"
echo "Results: $PASSED passed, $FAILED failed"
echo "================================================"

if [ "$FAILED" -gt 0 ]; then
  exit 1
fi

echo ""
echo "All smoke tests passed!"
