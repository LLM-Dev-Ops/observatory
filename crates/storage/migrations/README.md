# Database Migrations

This directory contains TimescaleDB migrations for the LLM Observatory storage layer.

Only numbered production migrations live at this directory's top level so SQLx can
discover them safely. Operational queries, verification scripts, and migration tests
live in `support/` and must be run explicitly.

## Migration Files

The migrations are numbered sequentially and build upon each other:

1. **001_initial_schema.sql** - Core tables (llm_traces, llm_metrics, llm_logs)
2. **002_add_hypertables.sql** - Convert tables to TimescaleDB hypertables
3. **003_create_indexes.sql** - Add performance indexes
4. **004_continuous_aggregates.sql** - ✅ **FIXED** - Create continuous aggregates for analytics
5. **005_retention_policies.sql** - Configure data retention
6. **006_supporting_tables.sql** - Model pricing, API keys, users, projects

## Migration 004 Fix (November 2025)

**Status:** ✅ FIXED for TimescaleDB 2.14+

Migration 004 has been updated to fix compatibility issues with TimescaleDB continuous aggregates. See:
- **[004_FIX_SUMMARY.md](004_FIX_SUMMARY.md)** - Quick start and executive summary
- **[004_MIGRATION_NOTES.md](004_MIGRATION_NOTES.md)** - Detailed technical documentation

### Quick Deploy

```bash
# Automated deployment with tests
./deploy_004.sh --test

# Or manual deployment
psql -U postgres -d llm_observatory -f 004_continuous_aggregates.sql
```

## Migration Naming Convention

Migrations follow the format: `NNN_description.sql` where NNN is a sequential number.

Example:
- `001_initial_schema.sql`
- `002_add_hypertables.sql`
- `003_create_indexes.sql`

## Creating Migrations

Use SQLx CLI to create migrations:

```bash
# Install SQLx CLI if not already installed
cargo install sqlx-cli --no-default-features --features postgres

# Create a new migration
sqlx migrate add -r create_traces_table
```

This will create two files:
- `{timestamp}_create_traces_table.up.sql` - Forward migration
- `{timestamp}_create_traces_table.down.sql` - Rollback migration

## Running Migrations

```bash
# Run all pending migrations
sqlx migrate run --database-url postgres://user:pass@localhost/dbname

# Revert the last migration
sqlx migrate revert --database-url postgres://user:pass@localhost/dbname
```

## Expected Schema

### Traces Tables

```sql
-- traces table
CREATE TABLE traces (
    id UUID PRIMARY KEY,
    trace_id VARCHAR(32) NOT NULL,
    service_name VARCHAR(255) NOT NULL,
    start_time TIMESTAMPTZ NOT NULL,
    end_time TIMESTAMPTZ,
    duration_us BIGINT,
    status VARCHAR(50) NOT NULL,
    status_message TEXT,
    root_span_name VARCHAR(255),
    attributes JSONB NOT NULL DEFAULT '{}',
    resource_attributes JSONB NOT NULL DEFAULT '{}',
    span_count INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- trace_spans table
CREATE TABLE trace_spans (
    id UUID PRIMARY KEY,
    trace_id UUID NOT NULL REFERENCES traces(id) ON DELETE CASCADE,
    span_id VARCHAR(16) NOT NULL,
    parent_span_id VARCHAR(16),
    name VARCHAR(255) NOT NULL,
    kind VARCHAR(50) NOT NULL,
    service_name VARCHAR(255) NOT NULL,
    start_time TIMESTAMPTZ NOT NULL,
    end_time TIMESTAMPTZ,
    duration_us BIGINT,
    status VARCHAR(50) NOT NULL,
    status_message TEXT,
    attributes JSONB NOT NULL DEFAULT '{}',
    events JSONB,
    links JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- trace_events table
CREATE TABLE trace_events (
    id UUID PRIMARY KEY,
    span_id UUID NOT NULL REFERENCES trace_spans(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL,
    attributes JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

### Metrics Tables

```sql
-- metrics table
CREATE TABLE metrics (
    id UUID PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    unit VARCHAR(50),
    metric_type VARCHAR(50) NOT NULL,
    service_name VARCHAR(255) NOT NULL,
    attributes JSONB NOT NULL DEFAULT '{}',
    resource_attributes JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(name, service_name)
);

-- metric_data_points table
CREATE TABLE metric_data_points (
    id UUID PRIMARY KEY,
    metric_id UUID NOT NULL REFERENCES metrics(id) ON DELETE CASCADE,
    timestamp TIMESTAMPTZ NOT NULL,
    value DOUBLE PRECISION,
    count BIGINT,
    sum DOUBLE PRECISION,
    min DOUBLE PRECISION,
    max DOUBLE PRECISION,
    buckets JSONB,
    quantiles JSONB,
    exemplars JSONB,
    attributes JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

### Logs Table

```sql
-- logs table
CREATE TABLE logs (
    id UUID PRIMARY KEY,
    timestamp TIMESTAMPTZ NOT NULL,
    observed_timestamp TIMESTAMPTZ NOT NULL,
    severity_number INTEGER NOT NULL,
    severity_text VARCHAR(20) NOT NULL,
    body TEXT NOT NULL,
    service_name VARCHAR(255) NOT NULL,
    trace_id VARCHAR(32),
    span_id VARCHAR(16),
    trace_flags INTEGER,
    attributes JSONB NOT NULL DEFAULT '{}',
    resource_attributes JSONB NOT NULL DEFAULT '{}',
    scope_name VARCHAR(255),
    scope_version VARCHAR(50),
    scope_attributes JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

## Indexes

Essential indexes for query performance:

```sql
-- Traces indexes
CREATE INDEX idx_traces_trace_id ON traces(trace_id);
CREATE INDEX idx_traces_service_name ON traces(service_name);
CREATE INDEX idx_traces_start_time ON traces(start_time);
CREATE INDEX idx_traces_status ON traces(status);
CREATE INDEX idx_traces_duration ON traces(duration_us);

-- Spans indexes
CREATE INDEX idx_spans_trace_id ON trace_spans(trace_id);
CREATE INDEX idx_spans_span_id ON trace_spans(span_id);
CREATE INDEX idx_spans_parent_span_id ON trace_spans(parent_span_id);
CREATE INDEX idx_spans_start_time ON trace_spans(start_time);

-- Metrics indexes
CREATE INDEX idx_metrics_name ON metrics(name);
CREATE INDEX idx_metrics_service_name ON metrics(service_name);
CREATE INDEX idx_metric_points_metric_id ON metric_data_points(metric_id);
CREATE INDEX idx_metric_points_timestamp ON metric_data_points(timestamp);

-- Logs indexes
CREATE INDEX idx_logs_timestamp ON logs(timestamp);
CREATE INDEX idx_logs_service_name ON logs(service_name);
CREATE INDEX idx_logs_severity ON logs(severity_number);
CREATE INDEX idx_logs_trace_id ON logs(trace_id);
CREATE INDEX idx_logs_span_id ON logs(span_id);

-- Full-text search on log body
CREATE INDEX idx_logs_body_fts ON logs USING gin(to_tsvector('english', body));
```

## Data Types

- `UUID` - Unique identifiers
- `VARCHAR(n)` - String fields with size limits
- `TEXT` - Unbounded text fields
- `TIMESTAMPTZ` - Timestamps with timezone
- `BIGINT` - Large integers (microsecond durations)
- `DOUBLE PRECISION` - Floating point numbers
- `INTEGER` - Standard integers
- `JSONB` - Binary JSON for attributes (indexed and queryable)

## Best Practices

1. **Always include both up and down migrations**
2. **Test migrations on a copy of production data**
3. **Keep migrations small and focused**
4. **Document breaking changes**
5. **Use transactions for safety** (SQLx does this by default)
6. **Create indexes separately from table creation** for large tables
7. **Use JSONB for flexible schema** on attributes
8. **Include created_at and updated_at** timestamps
