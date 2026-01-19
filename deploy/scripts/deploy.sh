#!/bin/bash
# LLM Observatory - Production Deployment Script
# Usage: ./deploy.sh [dev|staging|prod]

set -euo pipefail

# ============================================================================
# CONFIGURATION
# ============================================================================

ENV="${1:-dev}"
PROJECT_ID="${PROJECT_ID:-agentics-dev}"
REGION="${REGION:-us-central1}"
SERVICE_NAME="llm-observatory"

# Validate environment
if [[ ! "$ENV" =~ ^(dev|staging|prod)$ ]]; then
  echo "Error: Environment must be dev, staging, or prod"
  exit 1
fi

echo "=========================================="
echo "LLM Observatory Deployment"
echo "=========================================="
echo "Environment: $ENV"
echo "Project: $PROJECT_ID"
echo "Region: $REGION"
echo "Service: $SERVICE_NAME"
echo "=========================================="

# ============================================================================
# PRE-FLIGHT CHECKS
# ============================================================================

echo ""
echo "[1/7] Pre-flight checks..."

# Check gcloud authentication
if ! gcloud auth print-access-token &> /dev/null; then
  echo "Error: Not authenticated with gcloud. Run 'gcloud auth login'"
  exit 1
fi

# Check project
gcloud config set project "$PROJECT_ID"

# Check required APIs
REQUIRED_APIS=(
  "run.googleapis.com"
  "cloudbuild.googleapis.com"
  "secretmanager.googleapis.com"
  "containerregistry.googleapis.com"
)

for api in "${REQUIRED_APIS[@]}"; do
  if ! gcloud services list --enabled --filter="name:$api" --format="value(name)" | grep -q "$api"; then
    echo "Enabling $api..."
    gcloud services enable "$api"
  fi
done

echo "Pre-flight checks passed"

# ============================================================================
# CREATE SERVICE ACCOUNT (if not exists)
# ============================================================================

echo ""
echo "[2/7] Setting up service account..."

SA_NAME="llm-observatory-sa"
SA_EMAIL="${SA_NAME}@${PROJECT_ID}.iam.gserviceaccount.com"

if ! gcloud iam service-accounts describe "$SA_EMAIL" &> /dev/null; then
  echo "Creating service account..."
  gcloud iam service-accounts create "$SA_NAME" \
    --display-name="LLM Observatory Service Account" \
    --description="Service account for LLM Observatory Cloud Run service"

  # Grant required roles
  ROLES=(
    "roles/run.invoker"
    "roles/secretmanager.secretAccessor"
    "roles/logging.logWriter"
    "roles/monitoring.metricWriter"
    "roles/cloudtrace.agent"
  )

  for role in "${ROLES[@]}"; do
    gcloud projects add-iam-policy-binding "$PROJECT_ID" \
      --member="serviceAccount:$SA_EMAIL" \
      --role="$role" \
      --quiet
  done
else
  echo "Service account already exists"
fi

# ============================================================================
# CREATE SECRET (if not exists)
# ============================================================================

echo ""
echo "[3/7] Setting up secrets..."

SECRET_NAME="ruvector-api-key"

if ! gcloud secrets describe "$SECRET_NAME" &> /dev/null; then
  echo "Creating secret $SECRET_NAME..."
  echo "Enter RuVector API key (will be stored in Secret Manager):"
  read -rs RUVECTOR_API_KEY

  echo -n "$RUVECTOR_API_KEY" | gcloud secrets create "$SECRET_NAME" \
    --replication-policy="automatic" \
    --data-file=-

  echo "Secret created"
else
  echo "Secret already exists"
fi

# ============================================================================
# BUILD DOCKER IMAGE
# ============================================================================

echo ""
echo "[4/7] Building Docker image..."

cd "$(dirname "$0")/../.."

IMAGE_TAG="gcr.io/${PROJECT_ID}/${SERVICE_NAME}:$(git rev-parse --short HEAD)"
IMAGE_LATEST="gcr.io/${PROJECT_ID}/${SERVICE_NAME}:latest"

docker build -t "$IMAGE_TAG" -t "$IMAGE_LATEST" -f deploy/Dockerfile .

# ============================================================================
# PUSH TO CONTAINER REGISTRY
# ============================================================================

echo ""
echo "[5/7] Pushing to Container Registry..."

docker push "$IMAGE_TAG"
docker push "$IMAGE_LATEST"

# ============================================================================
# DEPLOY TO CLOUD RUN
# ============================================================================

echo ""
echo "[6/7] Deploying to Cloud Run..."

# Get ruvector service URL based on environment
case "$ENV" in
  prod)
    RUVECTOR_URL="https://ruvector-service-${REGION}.a.run.app"
    MIN_INSTANCES=1
    MAX_INSTANCES=50
    ;;
  staging)
    RUVECTOR_URL="https://ruvector-service-staging-${REGION}.a.run.app"
    MIN_INSTANCES=0
    MAX_INSTANCES=10
    ;;
  *)
    RUVECTOR_URL="https://ruvector-service-dev-${REGION}.a.run.app"
    MIN_INSTANCES=0
    MAX_INSTANCES=5
    ;;
esac

gcloud run deploy "$SERVICE_NAME" \
  --image="$IMAGE_TAG" \
  --region="$REGION" \
  --platform=managed \
  --allow-unauthenticated \
  --memory=512Mi \
  --cpu=1 \
  --min-instances="$MIN_INSTANCES" \
  --max-instances="$MAX_INSTANCES" \
  --concurrency=80 \
  --timeout=60s \
  --set-env-vars="SERVICE_NAME=$SERVICE_NAME,SERVICE_VERSION=$(git rev-parse --short HEAD),PLATFORM_ENV=$ENV,RUVECTOR_SERVICE_URL=$RUVECTOR_URL" \
  --set-secrets="RUVECTOR_API_KEY=${SECRET_NAME}:latest" \
  --service-account="$SA_EMAIL"

# ============================================================================
# VERIFY DEPLOYMENT
# ============================================================================

echo ""
echo "[7/7] Verifying deployment..."

SERVICE_URL=$(gcloud run services describe "$SERVICE_NAME" --region="$REGION" --format='value(status.url)')

echo "Service URL: $SERVICE_URL"
echo ""

# Wait for service to be ready
sleep 5

# Health check
echo "Running health check..."
HTTP_STATUS=$(curl -s -o /dev/null -w "%{http_code}" "$SERVICE_URL/health")

if [ "$HTTP_STATUS" = "200" ]; then
  echo "Health check: PASSED"
elif [ "$HTTP_STATUS" = "503" ]; then
  echo "Health check: DEGRADED (ruvector-service may be unavailable)"
else
  echo "Health check: FAILED (status: $HTTP_STATUS)"
  exit 1
fi

# Check agent endpoints
echo ""
echo "Checking agent endpoints..."
curl -s "$SERVICE_URL/" | jq .endpoints

echo ""
echo "=========================================="
echo "DEPLOYMENT COMPLETE"
echo "=========================================="
echo "Service URL: $SERVICE_URL"
echo "Environment: $ENV"
echo "Version: $(git rev-parse --short HEAD)"
echo ""
echo "Agent Endpoints:"
echo "  - Telemetry: $SERVICE_URL/api/v1/telemetry/ingest"
echo "  - Usage:     $SERVICE_URL/api/v1/usage/analyze"
echo "  - Failure:   $SERVICE_URL/api/v1/failure/classify"
echo "  - Health:    $SERVICE_URL/api/v1/health-check/evaluate"
echo "  - SLO:       $SERVICE_URL/api/v1/slo/enforce"
echo "  - PostMortem:$SERVICE_URL/api/v1/postmortem/generate"
echo "  - Viz:       $SERVICE_URL/api/v1/visualization/generate"
echo "=========================================="
