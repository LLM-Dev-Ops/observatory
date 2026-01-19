/**
 * Post-Mortem Generator Agent - Configuration
 *
 * Loads and validates agent configuration from environment.
 */

import type { RuvectorConfig } from './types/ruvector.js';
import { AGENT_METADATA } from '../contracts/schemas.js';

export interface AgentConfig {
  agent: {
    id: string;
    version: string;
    classification: {
      type: string;
      subtype: string;
    };
  };
  ruvector: RuvectorConfig;
  processing: {
    maxTimelineEvents: number;
    defaultTimeWindowHours: number;
    timeoutMs: number;
  };
}

const defaultConfig: AgentConfig = {
  agent: {
    id: AGENT_METADATA.id,
    version: AGENT_METADATA.version,
    classification: {
      type: AGENT_METADATA.classification.type,
      subtype: AGENT_METADATA.classification.subtype,
    },
  },
  ruvector: {
    endpoint: process.env.RUVECTOR_ENDPOINT || 'http://localhost:3001',
    apiKey: process.env.RUVECTOR_API_KEY,
    timeout: parseInt(process.env.RUVECTOR_TIMEOUT || '30000', 10),
    retryAttempts: parseInt(process.env.RUVECTOR_RETRY_ATTEMPTS || '3', 10),
    retryDelayMs: parseInt(process.env.RUVECTOR_RETRY_DELAY_MS || '1000', 10),
    maxRetryDelayMs: parseInt(process.env.RUVECTOR_MAX_RETRY_DELAY_MS || '10000', 10),
    connectionPoolSize: parseInt(process.env.RUVECTOR_POOL_SIZE || '5', 10),
  },
  processing: {
    maxTimelineEvents: parseInt(process.env.MAX_TIMELINE_EVENTS || '1000', 10),
    defaultTimeWindowHours: parseInt(process.env.DEFAULT_TIME_WINDOW_HOURS || '24', 10),
    timeoutMs: parseInt(process.env.PROCESSING_TIMEOUT_MS || '60000', 10),
  },
};

let loadedConfig: AgentConfig | null = null;

export function loadConfig(): AgentConfig {
  if (loadedConfig) {
    return loadedConfig;
  }

  loadedConfig = { ...defaultConfig };
  return loadedConfig;
}

export function resetConfig(): void {
  loadedConfig = null;
}
