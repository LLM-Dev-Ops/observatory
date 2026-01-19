# LLM Observatory - Unified Cloud Run Service
# Multi-stage build for minimal production image

# ============================================================================
# BUILD STAGE
# ============================================================================
FROM node:20-alpine AS builder

WORKDIR /app

# Copy package files
COPY deploy/service/package*.json ./

# Install dependencies
RUN npm ci --only=production && npm cache clean --force

# Copy source
COPY deploy/service/tsconfig.json ./
COPY deploy/service/src ./src

# Install dev dependencies for build
RUN npm install --save-dev typescript @types/node @types/express

# Build TypeScript
RUN npm run build

# ============================================================================
# PRODUCTION STAGE
# ============================================================================
FROM node:20-alpine AS production

# Security: Run as non-root user
RUN addgroup -g 1001 -S nodejs && \
    adduser -S nodejs -u 1001

WORKDIR /app

# Copy production dependencies
COPY --from=builder /app/node_modules ./node_modules
COPY --from=builder /app/dist ./dist
COPY --from=builder /app/package.json ./

# Set ownership
RUN chown -R nodejs:nodejs /app

USER nodejs

# Cloud Run expects port 8080
ENV PORT=8080
ENV NODE_ENV=production

EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
  CMD wget --no-verbose --tries=1 --spider http://localhost:8080/health || exit 1

# Start the service
CMD ["node", "dist/index.js"]
