# ============================================================================
# Multi-stage Dockerfile for Analytics API Service
# ============================================================================
# This Dockerfile builds the Analytics API service using a multi-stage build
# to minimize the final image size and improve security.

# ============================================================================
# Stage 1: Chef - Prepare dependencies for caching
# ============================================================================
FROM rust:1.93-slim-bookworm AS chef

# Install cargo-chef for dependency caching
RUN cargo install cargo-chef --locked

WORKDIR /app

# ============================================================================
# Stage 2: Planner - Analyze dependencies
# ============================================================================
FROM chef AS planner

# Copy the entire workspace
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY services ./services

# Generate recipe for dependencies
RUN cargo chef prepare --recipe-path recipe.json

# ============================================================================
# Stage 3: Builder - Build dependencies (cached layer)
# ============================================================================
FROM chef AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy recipe from planner
COPY --from=planner /app/recipe.json recipe.json

# Build dependencies (this layer is cached)
RUN cargo chef cook --release --recipe-path recipe.json

# ============================================================================
# Stage 4: Builder - Build application
# ============================================================================
# Copy source code
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY services ./services

# Build the analytics-api binary
RUN cargo build --release --bin analytics-api

# Verify the binary was built
RUN ls -lh /app/target/release/ && \
    test -f /app/target/release/analytics-api && \
    echo "Binary built successfully"

# ============================================================================
# Stage 5: Runtime - Create minimal runtime image
# ============================================================================
FROM debian:bookworm-slim AS runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 -s /bin/bash analytics && \
    mkdir -p /app && \
    chown -R analytics:analytics /app

# Switch to non-root user
USER analytics
WORKDIR /app

# Copy binary from builder
COPY --from=builder --chown=analytics:analytics /app/target/release/analytics-api /app/analytics-api

# Verify binary permissions
RUN ls -lh /app/analytics-api && \
    test -x /app/analytics-api && \
    echo "Binary is executable"

# Set environment variables
ENV RUST_LOG=info
ENV RUST_BACKTRACE=1

# Expose ports
# 8080 - HTTP API
# 9091 - Metrics
EXPOSE 8080 9091

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD ["/app/analytics-api", "--help"] || exit 1

# Run the binary
CMD ["/app/analytics-api"]

# ============================================================================
# Labels
# ============================================================================
LABEL org.opencontainers.image.title="LLM Observatory Analytics API"
LABEL org.opencontainers.image.description="Analytics API service for LLM Observatory"
LABEL org.opencontainers.image.vendor="LLM Observatory Contributors"
LABEL org.opencontainers.image.licenses="Apache-2.0"
LABEL org.opencontainers.image.source="https://github.com/llm-observatory/llm-observatory"
