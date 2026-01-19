#!/bin/bash
# Deployment script for Telemetry Collector Agent
# Copyright 2025 LLM Observatory Contributors
# SPDX-License-Identifier: Apache-2.0

set -e

# Configuration
FUNCTION_NAME="telemetry-collector"
RUNTIME="nodejs20"
ENTRY_POINT="telemetryCollector"
REGION="${REGION:-us-central1}"
MEMORY="${MEMORY:-256MB}"
TIMEOUT="${TIMEOUT:-60s}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}Telemetry Collector Agent - Deployment${NC}"
echo "========================================"

# Check if gcloud is installed
if ! command -v gcloud &> /dev/null; then
    echo -e "${RED}Error: gcloud CLI is not installed${NC}"
    exit 1
fi

# Build TypeScript
echo -e "${YELLOW}Building TypeScript...${NC}"
npm run build

if [ $? -ne 0 ]; then
    echo -e "${RED}Build failed${NC}"
    exit 1
fi

echo -e "${GREEN}Build successful${NC}"

# Deploy to Google Cloud Functions
echo -e "${YELLOW}Deploying to Google Cloud Functions...${NC}"

gcloud functions deploy ${FUNCTION_NAME} \
  --gen2 \
  --runtime=${RUNTIME} \
  --region=${REGION} \
  --source=. \
  --entry-point=${ENTRY_POINT} \
  --trigger-http \
  --allow-unauthenticated \
  --memory=${MEMORY} \
  --timeout=${TIMEOUT} \
  --set-env-vars "AGENT_ID=telemetry-collector-agent,AGENT_VERSION=1.0.0,RUVECTOR_ENDPOINT=${RUVECTOR_ENDPOINT:-http://localhost:3001}" \
  --max-instances=100 \
  --min-instances=0

if [ $? -eq 0 ]; then
    echo -e "${GREEN}Deployment successful!${NC}"

    # Get function URL
    FUNCTION_URL=$(gcloud functions describe ${FUNCTION_NAME} \
      --region=${REGION} \
      --gen2 \
      --format='value(serviceConfig.uri)')

    echo ""
    echo -e "${GREEN}Function URL: ${FUNCTION_URL}${NC}"
    echo ""
    echo "Test endpoints:"
    echo "  Health: ${FUNCTION_URL}/health"
    echo "  Ingest: ${FUNCTION_URL}/ingest"
else
    echo -e "${RED}Deployment failed${NC}"
    exit 1
fi
