#!/bin/bash
# ============================================================================
# Migration 004 Deployment Script
# ============================================================================
# Purpose: Deploy continuous aggregates migration with validation
# Usage: ./deploy_004.sh [options]
# Options:
#   --db-host HOST     Database host (default: localhost)
#   --db-port PORT     Database port (default: 5432)
#   --db-name NAME     Database name (default: llm_observatory)
#   --db-user USER     Database user (default: postgres)
#   --test             Run test suite after deployment
#   --rollback         Rollback the migration
#   --dry-run          Show what would be executed without running

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Default configuration
DB_HOST="${DB_HOST:-localhost}"
DB_PORT="${DB_PORT:-5432}"
DB_NAME="${DB_NAME:-llm_observatory}"
DB_USER="${DB_USER:-postgres}"
RUN_TESTS=false
DO_ROLLBACK=false
DRY_RUN=false

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --db-host)
            DB_HOST="$2"
            shift 2
            ;;
        --db-port)
            DB_PORT="$2"
            shift 2
            ;;
        --db-name)
            DB_NAME="$2"
            shift 2
            ;;
        --db-user)
            DB_USER="$2"
            shift 2
            ;;
        --test)
            RUN_TESTS=true
            shift
            ;;
        --rollback)
            DO_ROLLBACK=true
            shift
            ;;
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        -h|--help)
            echo "Usage: $0 [options]"
            echo ""
            echo "Options:"
            echo "  --db-host HOST     Database host (default: localhost)"
            echo "  --db-port PORT     Database port (default: 5432)"
            echo "  --db-name NAME     Database name (default: llm_observatory)"
            echo "  --db-user USER     Database user (default: postgres)"
            echo "  --test             Run test suite after deployment"
            echo "  --rollback         Rollback the migration"
            echo "  --dry-run          Show what would be executed"
            echo "  -h, --help         Show this help message"
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            exit 1
            ;;
    esac
done

# Function to execute SQL
execute_sql() {
    local sql_file=$1
    local description=$2

    if [ "$DRY_RUN" = true ]; then
        echo -e "${YELLOW}[DRY RUN]${NC} Would execute: $sql_file ($description)"
        return 0
    fi

    echo -e "${GREEN}==> Executing: $description${NC}"
    PGPASSWORD="${PGPASSWORD:-}" psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -f "$sql_file"

    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✓ Success: $description${NC}"
        return 0
    else
        echo -e "${RED}✗ Failed: $description${NC}"
        return 1
    fi
}

# Function to check prerequisites
check_prerequisites() {
    echo -e "${GREEN}==> Checking prerequisites${NC}"

    # Check psql is installed
    if ! command -v psql &> /dev/null; then
        echo -e "${RED}✗ psql not found. Please install PostgreSQL client.${NC}"
        exit 1
    fi

    # Check database connection
    if [ "$DRY_RUN" = false ]; then
        echo "Testing database connection..."
        if ! PGPASSWORD="${PGPASSWORD:-}" psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -c "SELECT version();" &> /dev/null; then
            echo -e "${RED}✗ Cannot connect to database${NC}"
            echo "Connection details:"
            echo "  Host: $DB_HOST"
            echo "  Port: $DB_PORT"
            echo "  Database: $DB_NAME"
            echo "  User: $DB_USER"
            exit 1
        fi
        echo -e "${GREEN}✓ Database connection successful${NC}"

        # Check TimescaleDB is installed
        TIMESCALE_VERSION=$(PGPASSWORD="${PGPASSWORD:-}" psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -t -c "SELECT extversion FROM pg_extension WHERE extname = 'timescaledb';" | xargs)
        if [ -z "$TIMESCALE_VERSION" ]; then
            echo -e "${RED}✗ TimescaleDB extension not found${NC}"
            exit 1
        fi
        echo -e "${GREEN}✓ TimescaleDB version: $TIMESCALE_VERSION${NC}"

        # Check if llm_traces hypertable exists
        HYPERTABLE_EXISTS=$(PGPASSWORD="${PGPASSWORD:-}" psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -t -c "SELECT COUNT(*) FROM timescaledb_information.hypertables WHERE hypertable_name = 'llm_traces';" | xargs)
        if [ "$HYPERTABLE_EXISTS" -eq 0 ]; then
            echo -e "${YELLOW}⚠ Warning: llm_traces hypertable not found. Run migrations 001-003 first.${NC}"
        else
            echo -e "${GREEN}✓ llm_traces hypertable exists${NC}"
        fi
    fi

    echo ""
}

# Function to rollback migration
rollback_migration() {
    echo -e "${YELLOW}==> Rolling back Migration 004${NC}"

    if [ "$DRY_RUN" = true ]; then
        echo -e "${YELLOW}[DRY RUN]${NC} Would rollback continuous aggregates"
        return 0
    fi

    # Drop continuous aggregates and their policies
    ROLLBACK_SQL=$(cat <<'EOF'
BEGIN;

-- Drop continuous aggregate policies
DO $$
DECLARE
    agg_name TEXT;
BEGIN
    FOR agg_name IN
        SELECT view_name
        FROM timescaledb_information.continuous_aggregates
        WHERE view_name IN ('llm_metrics_1min', 'llm_metrics_1hour', 'llm_metrics_1day', 'llm_error_summary')
    LOOP
        EXECUTE 'DROP MATERIALIZED VIEW IF EXISTS ' || agg_name || ' CASCADE';
        RAISE NOTICE 'Dropped continuous aggregate: %', agg_name;
    END LOOP;
END $$;

-- Drop helper views and functions (from percentile_queries.sql if installed)
DROP VIEW IF EXISTS llm_percentiles_realtime CASCADE;
DROP VIEW IF EXISTS llm_percentiles_hourly CASCADE;
DROP VIEW IF EXISTS llm_percentiles_approximate CASCADE;
DROP VIEW IF EXISTS llm_sla_monitoring CASCADE;
DROP FUNCTION IF EXISTS approximate_percentile(DOUBLE PRECISION, DOUBLE PRECISION, DOUBLE PRECISION) CASCADE;
DROP FUNCTION IF EXISTS compare_model_percentiles(INTERVAL) CASCADE;
DROP FUNCTION IF EXISTS user_percentiles(TEXT, INTERVAL) CASCADE;

COMMIT;
EOF
)

    echo "$ROLLBACK_SQL" | PGPASSWORD="${PGPASSWORD:-}" psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME"

    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✓ Rollback completed successfully${NC}"
    else
        echo -e "${RED}✗ Rollback failed${NC}"
        exit 1
    fi
}

# Function to deploy migration
deploy_migration() {
    echo -e "${GREEN}==> Deploying Migration 004: Continuous Aggregates${NC}"

    # Get the script directory
    SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

    # Deploy the migration
    execute_sql "$SCRIPT_DIR/004_continuous_aggregates.sql" "Creating continuous aggregates"

    # Optionally install percentile helper queries
    if [ -f "$SCRIPT_DIR/support/percentile_queries.sql" ]; then
        echo ""
        read -p "Install percentile helper views/functions? (y/N) " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            execute_sql "$SCRIPT_DIR/support/percentile_queries.sql" "Installing percentile helpers"
        fi
    fi

    echo ""
    echo -e "${GREEN}==> Verifying deployment${NC}"

    if [ "$DRY_RUN" = false ]; then
        # Check that all 4 aggregates were created
        AGG_COUNT=$(PGPASSWORD="${PGPASSWORD:-}" psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -t -c "SELECT COUNT(*) FROM timescaledb_information.continuous_aggregates WHERE view_name IN ('llm_metrics_1min', 'llm_metrics_1hour', 'llm_metrics_1day', 'llm_error_summary');" | xargs)

        if [ "$AGG_COUNT" -eq 4 ]; then
            echo -e "${GREEN}✓ All 4 continuous aggregates created successfully${NC}"
        else
            echo -e "${RED}✗ Expected 4 aggregates, found $AGG_COUNT${NC}"
            exit 1
        fi

        # List created aggregates
        echo ""
        echo "Created continuous aggregates:"
        PGPASSWORD="${PGPASSWORD:-}" psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -c "SELECT view_name, format_interval(refresh_interval) AS refresh_interval FROM timescaledb_information.continuous_aggregates WHERE view_name IN ('llm_metrics_1min', 'llm_metrics_1hour', 'llm_metrics_1day', 'llm_error_summary') ORDER BY view_name;"
    fi
}

# Function to run tests
run_tests() {
    echo ""
    echo -e "${GREEN}==> Running test suite${NC}"

    SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

    if [ -f "$SCRIPT_DIR/support/test_004_continuous_aggregates.sql" ]; then
        execute_sql "$SCRIPT_DIR/support/test_004_continuous_aggregates.sql" "Running test suite"
    else
        echo -e "${YELLOW}⚠ Test file not found: support/test_004_continuous_aggregates.sql${NC}"
    fi
}

# Main execution
main() {
    echo "============================================================================"
    echo "  Migration 004: Continuous Aggregates Deployment"
    echo "============================================================================"
    echo ""
    echo "Configuration:"
    echo "  Database Host: $DB_HOST"
    echo "  Database Port: $DB_PORT"
    echo "  Database Name: $DB_NAME"
    echo "  Database User: $DB_USER"
    echo "  Dry Run: $DRY_RUN"
    echo ""

    # Check prerequisites
    check_prerequisites

    # Execute rollback or deployment
    if [ "$DO_ROLLBACK" = true ]; then
        rollback_migration
    else
        deploy_migration

        # Run tests if requested
        if [ "$RUN_TESTS" = true ]; then
            run_tests
        fi
    fi

    echo ""
    echo "============================================================================"
    echo -e "${GREEN}  Deployment Complete${NC}"
    echo "============================================================================"
    echo ""
    echo "Next steps:"
    echo "  1. Review the deployment logs above"
    echo "  2. Check continuous aggregate status:"
    echo "     psql -h $DB_HOST -d $DB_NAME -c \"SELECT * FROM timescaledb_information.continuous_aggregates;\""
    echo "  3. Monitor refresh jobs:"
    echo "     psql -h $DB_HOST -d $DB_NAME -c \"SELECT * FROM timescaledb_information.jobs WHERE proc_name = 'policy_refresh_continuous_aggregate';\""
    echo "  4. See 004_MIGRATION_NOTES.md for usage examples"
    echo ""
}

# Run main function
main
