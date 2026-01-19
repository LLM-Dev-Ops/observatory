# Telemetry Collector Agent - Contract Layer

This directory contains the **contract layer** for the Telemetry Collector Agent in LLM-Observatory.

## Overview

The contract layer defines the formal interfaces between:
- **Raw telemetry input** (LlmSpan from `crates/core`)
- **Normalized canonical output** format
- **Decision events** for ruvector-service persistence

## Constitutional Constraints

**CRITICAL**: This agent is **READ-ONLY, NON-ENFORCING, NON-ANALYTICAL**.

The contract layer enforces these constitutional rules:

| Rule | Enforcement |
|------|-------------|
| **No SQL execution** | N/A (agent doesn't have database access) |
| **No orchestration** | Validated in `validateConstitutionalOperation()` |
| **No state modification** | Validated in `validateConstitutionalOperation()` |
| **No constraint application** | `DecisionEventSchema.constraints_applied` = `[]` (literal) |
| **Perfect confidence** | `DecisionEventSchema.confidence` = `1.0` (literal) |
| **Fixed decision type** | `DecisionEventSchema.decision_type` = `"telemetry_ingestion"` (literal) |

## Files

### `schemas.ts`
Zod schemas for runtime validation:

- **`TelemetryEventSchema`** - Raw input (matches Rust `LlmSpan` from `crates/core/src/span.rs`)
- **`NormalizedTelemetrySchema`** - Canonical output format
- **`DecisionEventSchema`** - For ruvector-service persistence (enforces constitutional constraints)

All schemas are **strict** (`z.strict()`) to prevent unknown fields.

### `types.ts`
TypeScript types exported from schemas:

- Schema-derived types (`TelemetryEvent`, `NormalizedTelemetry`, `DecisionEvent`)
- Agent metadata types
- Validation result types
- Processing result types
- Error types and codes

### `validation.ts`
Validation utilities:

- **`validateTelemetryEvent()`** - Validate raw input
- **`validateDecisionEvent()`** - Validate decision event + constitutional constraints
- **`hashInput()`** - SHA256 hash for input tracking
- **`hashInputs()`** - Hash multiple inputs (batch processing)
- **`validateConstitutionalOperation()`** - Ensure operation is read-only

### `index.ts`
Barrel export of all contracts.

## Usage

### Validating Telemetry Events

```typescript
import { validateTelemetryEvent } from './contracts';

const result = validateTelemetryEvent(rawInput);

if (result.success) {
  // Use typed data
  console.log('Span ID:', result.data.span_id);
  console.log('Provider:', result.data.provider);
  console.log('Input hash:', result.metadata?.inputHash);
} else {
  // Handle validation errors
  for (const error of result.errors) {
    console.error(`${error.path.join('.')}: ${error.message}`);
  }
}
```

### Creating Decision Events

```typescript
import {
  type DecisionEvent,
  type NormalizedTelemetry,
  validateDecisionEvent,
  hashInput,
} from './contracts';

// Normalize telemetry events
const normalized: NormalizedTelemetry[] = [...];

// Create decision event
const decisionEvent: DecisionEvent = {
  agent_id: 'telemetry-collector',
  agent_version: '1.0.0',
  decision_type: 'telemetry_ingestion', // MUST be this literal
  confidence: 1.0, // ALWAYS 1.0 for read-only
  constraints_applied: [], // ALWAYS empty for read-only
  inputs_hash: hashInput(originalEvent),
  outputs: normalized,
  execution_ref: `exec_${Date.now()}_${randomUUID()}`,
  timestamp: new Date().toISOString(),
};

// Validate constitutional constraints
const validationResult = validateDecisionEvent(decisionEvent);

if (!validationResult.success) {
  throw new Error('Constitutional violation detected');
}
```

### Hashing Inputs

```typescript
import { hashInput, hashInputs } from './contracts';

// Hash a single event
const hash = hashInput(telemetryEvent);
// => "a3c5d7e9f1b2d4c6e8f0a2b4c6d8e0f2a4b6c8d0e2f4a6b8c0d2e4f6a8b0c2d4"

// Hash multiple events (batch)
const batchHash = hashInputs([event1, event2, event3]);

// Hash with options
const hashNoMetadata = hashInput(event, {
  includeMetadata: false,
  excludeFields: ['attributes', 'events'],
});
```

## Schema Alignment

### Rust ↔ TypeScript Mappings

| Rust (`crates/core`) | TypeScript (contracts) | Notes |
|---------------------|------------------------|-------|
| `LlmSpan` | `TelemetryEvent` | 1:1 mapping |
| `Provider` | `ProviderSchema` | Enum + custom string |
| `TokenUsage` | `TokenUsageSchema` | snake_case fields |
| `Cost` | `CostSchema` | snake_case fields |
| `Latency` | `LatencySchema` | Timestamps as ISO 8601 strings |
| `Metadata` | `MetadataSchema` | snake_case fields |
| `LlmInput` | `LlmInputSchema` | Discriminated union |
| `LlmOutput` | `LlmOutputSchema` | Simple object |
| `SpanStatus` | `SpanStatusSchema` | Uppercase enum |

### Field Name Conventions

- **Rust**: `snake_case` (e.g., `prompt_tokens`)
- **TypeScript**: `snake_case` (matching Rust for consistency)
- **Zod schemas**: Use object keys matching Rust exactly

## Validation Performance

Typical validation times:

- **TelemetryEvent validation**: <1ms per event
- **DecisionEvent validation**: <2ms (includes constitutional checks)
- **Input hashing (SHA256)**: <0.5ms per event

Batch processing of 1000 events: ~1 second (1ms/event average).

## Error Handling

All validation functions return `ValidationResult<T>`:

```typescript
interface ValidationResult<T> {
  success: boolean;
  data?: T;           // Only if success=true
  errors?: ValidationError[];  // Only if success=false
  metadata?: {
    validationTimeMs: number;
    schemaVersion: string;
    inputHash?: string;
  };
}
```

Error codes (from `TelemetryCollectorErrorCode`):

- `INVALID_SCHEMA` - Schema mismatch
- `MISSING_REQUIRED_FIELD` - Required field missing
- `INVALID_FIELD_TYPE` - Wrong field type
- `INVALID_FIELD_VALUE` - Invalid value (e.g., negative number)
- `CONSTITUTIONAL_VIOLATION` - Violates agent constitution
- `NORMALIZATION_FAILED` - Failed to normalize
- `HASHING_FAILED` - Failed to hash input

## Testing

Run contract tests:

```bash
# From repository root
npm test agents/telemetry-collector/contracts

# Watch mode
npm test -- --watch agents/telemetry-collector/contracts
```

Test coverage target: **100%** (contracts are critical).

## Schema Evolution

When updating schemas:

1. **Increment schema version** in `NormalizedTelemetrySchema.schema_version`
2. **Add migration logic** if breaking changes
3. **Update tests** to cover new fields
4. **Document changes** in changelog

## Dependencies

- **`zod`**: Runtime schema validation
- **`crypto`**: SHA256 hashing (Node.js built-in)

No external dependencies beyond these.

## License

Apache-2.0 (see file headers)

## See Also

- **Rust schemas**: `crates/core/src/span.rs`, `crates/core/src/types.rs`
- **Node.js SDK types**: `sdk/nodejs/src/types.ts`
- **Agent constitution**: `agents/telemetry-collector/CONSTITUTION.md`
- **ruvector-service client**: `agents/telemetry-collector/adapters/ruvector-client.ts`
