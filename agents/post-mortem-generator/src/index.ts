/**
 * Post-Mortem Generator Agent - Main Exports
 *
 * CONSTITUTIONAL CLASSIFICATION: READ-ONLY
 *
 * This agent generates structured, reproducible post-mortem reports from
 * historical telemetry, failure classifications, and health evaluations.
 */

// Handler exports
export { handlePostMortemGeneration } from './handler.js';

// Generator exports
export { generatePostMortem } from './generator.js';
export type { GeneratorInput, GeneratorResult } from './generator.js';

// Emitter exports
export {
  createDecisionEvent,
  validateDecisionEventCompliance,
  verifyConstitutionalCompliance,
  ConstitutionalViolationError,
} from './emitter.js';

// Client exports
export { RuvectorClient, initializeClient, getClient } from './ruvector-client.js';

// CLI exports
export { cliCommands, runCLI, CLIError } from './cli.js';
export type { CLICommands } from './cli.js';

// Config exports
export { loadConfig, resetConfig } from './config.js';
export type { AgentConfig } from './config.js';

// Re-export contracts
export * from '../contracts/index.js';
