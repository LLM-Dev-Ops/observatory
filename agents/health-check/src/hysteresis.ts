/**
 * Health Check Agent - Hysteresis Logic
 *
 * Prevents state flapping by requiring consecutive samples
 * before transitioning between health states.
 *
 * Design:
 * - Quick degradation: 1 sample to worsen state (fail fast)
 * - Slow improvement: 3 samples to improve state (ensure recovery is real)
 */

import type { HealthState, StateTransition } from '../contracts/schemas.js';
import type { HysteresisConfig } from './config.js';
import { isBetter, isWorse } from './indicators.js';

// ============================================================================
// HYSTERESIS STATE
// ============================================================================

export interface HysteresisState {
  current_state: HealthState;
  pending_state: HealthState | null;
  consecutive_samples: number;
  last_transition_time: string | null;
  time_in_current_state_seconds: number;
}

// ============================================================================
// HYSTERESIS RESULT
// ============================================================================

export interface HysteresisResult {
  new_state: HealthState;
  transition_occurred: boolean;
  consecutive_samples: number;
  hysteresis_applied: boolean;
}

// ============================================================================
// INITIAL STATE
// ============================================================================

/**
 * Create initial hysteresis state for a new target.
 */
export function createInitialHysteresisState(
  initial_state: HealthState = 'healthy'
): HysteresisState {
  return {
    current_state: initial_state,
    pending_state: null,
    consecutive_samples: 1,
    last_transition_time: null,
    time_in_current_state_seconds: 0,
  };
}

// ============================================================================
// APPLY HYSTERESIS
// ============================================================================

/**
 * Apply hysteresis to prevent state flapping.
 *
 * Rules:
 * - If computed state is worse than current: check threshold_to_degrade (default: 1)
 * - If computed state is better than current: check threshold_to_improve (default: 3)
 * - If computed state equals current: reset consecutive counter
 */
export function applyHysteresis(
  previous: HysteresisState,
  computed_state: HealthState,
  config: HysteresisConfig,
  current_timestamp: string
): HysteresisResult {
  // Same state - reset counter and stay
  if (computed_state === previous.current_state) {
    return {
      new_state: previous.current_state,
      transition_occurred: false,
      consecutive_samples: 1,
      hysteresis_applied: false,
    };
  }

  // Determine if this is improvement or degradation
  const isDegrading = isWorse(computed_state, previous.current_state);
  const isImproving = isBetter(computed_state, previous.current_state);

  // Check if we're continuing in the same direction
  const sameDirection = previous.pending_state === computed_state;
  const consecutiveSamples = sameDirection
    ? previous.consecutive_samples + 1
    : 1;

  // Determine threshold based on direction
  const threshold = isDegrading
    ? config.threshold_to_degrade
    : config.threshold_to_improve;

  // Check if we've met the threshold
  if (consecutiveSamples >= threshold) {
    return {
      new_state: computed_state,
      transition_occurred: true,
      consecutive_samples: 1, // Reset after transition
      hysteresis_applied: threshold > 1,
    };
  }

  // Not enough consecutive samples - stay in current state
  return {
    new_state: previous.current_state,
    transition_occurred: false,
    consecutive_samples: consecutiveSamples,
    hysteresis_applied: true,
  };
}

// ============================================================================
// UPDATE HYSTERESIS STATE
// ============================================================================

/**
 * Update hysteresis state based on the result.
 */
export function updateHysteresisState(
  previous: HysteresisState,
  computed_state: HealthState,
  result: HysteresisResult,
  current_timestamp: string,
  evaluation_interval_seconds: number
): HysteresisState {
  if (result.transition_occurred) {
    return {
      current_state: result.new_state,
      pending_state: null,
      consecutive_samples: 1,
      last_transition_time: current_timestamp,
      time_in_current_state_seconds: 0,
    };
  }

  // No transition - update time in current state and track pending
  return {
    current_state: previous.current_state,
    pending_state: computed_state !== previous.current_state ? computed_state : null,
    consecutive_samples: result.consecutive_samples,
    last_transition_time: previous.last_transition_time,
    time_in_current_state_seconds: previous.time_in_current_state_seconds + evaluation_interval_seconds,
  };
}

// ============================================================================
// BUILD STATE TRANSITION
// ============================================================================

/**
 * Build a StateTransition object from hysteresis state and result.
 */
export function buildStateTransition(
  previous: HysteresisState | null,
  result: HysteresisResult,
  config: HysteresisConfig,
  current_timestamp: string
): StateTransition {
  return {
    previous_state: previous?.current_state,
    current_state: result.new_state,
    transition_time: result.transition_occurred ? current_timestamp : undefined,
    time_in_current_state_seconds: previous?.time_in_current_state_seconds ?? 0,
    consecutive_samples_in_state: result.consecutive_samples,
    hysteresis_threshold: Math.max(config.threshold_to_improve, config.threshold_to_degrade),
  };
}

// ============================================================================
// LOAD PREVIOUS HYSTERESIS STATE
// ============================================================================

/**
 * Load previous hysteresis state from a StateTransition object.
 * Used when loading state from previous evaluation.
 */
export function loadFromStateTransition(
  transition: StateTransition
): HysteresisState {
  return {
    current_state: transition.current_state,
    pending_state: null,
    consecutive_samples: transition.consecutive_samples_in_state,
    last_transition_time: transition.transition_time ?? null,
    time_in_current_state_seconds: transition.time_in_current_state_seconds,
  };
}

// ============================================================================
// EVALUATE WITH HYSTERESIS
// ============================================================================

export interface HysteresisEvaluationInput {
  computed_state: HealthState;
  previous_state: HysteresisState | null;
  config: HysteresisConfig;
  current_timestamp: string;
  evaluation_interval_seconds: number;
}

export interface HysteresisEvaluationResult {
  final_state: HealthState;
  state_transition: StateTransition;
  new_hysteresis_state: HysteresisState;
  transition_occurred: boolean;
}

/**
 * Complete hysteresis evaluation - processes computed state and returns
 * the final state after applying hysteresis rules.
 */
export function evaluateWithHysteresis(
  input: HysteresisEvaluationInput
): HysteresisEvaluationResult {
  // Initialize previous state if not provided
  const previous = input.previous_state ?? createInitialHysteresisState(input.computed_state);

  // Apply hysteresis
  const result = applyHysteresis(
    previous,
    input.computed_state,
    input.config,
    input.current_timestamp
  );

  // Update hysteresis state
  const newHysteresisState = updateHysteresisState(
    previous,
    input.computed_state,
    result,
    input.current_timestamp,
    input.evaluation_interval_seconds
  );

  // Build state transition
  const stateTransition = buildStateTransition(
    previous,
    result,
    input.config,
    input.current_timestamp
  );

  return {
    final_state: result.new_state,
    state_transition: stateTransition,
    new_hysteresis_state: newHysteresisState,
    transition_occurred: result.transition_occurred,
  };
}
