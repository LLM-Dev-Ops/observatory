//! # WebSocket Models
//!
//! This module contains data models for WebSocket real-time functionality including:
//! - Event types for real-time updates
//! - Subscription management
//! - WebSocket message formats

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// WebSocket Message Types
// ============================================================================

/// Client-to-server WebSocket messages
#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    /// Subscribe to events
    Subscribe {
        /// Event types to subscribe to
        events: Vec<EventType>,
        /// Optional filters
        #[serde(default)]
        filters: EventFilters,
    },
    /// Unsubscribe from events
    Unsubscribe {
        /// Event types to unsubscribe from
        events: Vec<EventType>,
    },
    /// Ping to keep connection alive
    Ping,
}

/// Server-to-client WebSocket messages
#[derive(Debug, Serialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    /// Subscription confirmed
    Subscribed {
        events: Vec<EventType>,
        subscription_id: String,
    },
    /// Unsubscription confirmed
    Unsubscribed { events: Vec<EventType> },
    /// Event notification
    Event {
        event_type: EventType,
        data: serde_json::Value,
        timestamp: DateTime<Utc>,
    },
    /// Error message
    Error { code: String, message: String },
    /// Pong response
    Pong,
}

// ============================================================================
// Event Types
// ============================================================================

/// Types of real-time events
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    /// New trace created
    TraceCreated,
    /// Trace updated
    TraceUpdated,
    /// Metric threshold exceeded
    MetricThreshold,
    /// Cost threshold exceeded
    CostThreshold,
    /// Export job status changed
    ExportJobStatus,
    /// System alert triggered
    SystemAlert,
}

impl EventType {
    /// Check if an event type requires elevated permissions
    pub fn requires_permission(&self) -> &'static str {
        match self {
            EventType::TraceCreated => "traces:read",
            EventType::TraceUpdated => "traces:read",
            EventType::MetricThreshold => "metrics:read",
            EventType::CostThreshold => "costs:read",
            EventType::ExportJobStatus => "exports:read",
            EventType::SystemAlert => "alerts:read",
        }
    }
}

// ============================================================================
// Event Filters
// ============================================================================

/// Filters for event subscriptions
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EventFilters {
    /// Filter by provider
    pub provider: Option<String>,
    /// Filter by model
    pub model: Option<String>,
    /// Filter by environment
    pub environment: Option<String>,
    /// Filter by user ID
    pub user_id: Option<String>,
    /// Filter by minimum cost
    pub min_cost: Option<f64>,
    /// Filter by minimum duration
    pub min_duration_ms: Option<i32>,
}

impl EventFilters {
    /// Check if an event passes the filters
    pub fn matches(&self, event_data: &EventData) -> bool {
        // Provider filter
        if let Some(provider) = &self.provider {
            if let Some(event_provider) = &event_data.provider {
                if provider != event_provider {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Model filter
        if let Some(model) = &self.model {
            if let Some(event_model) = &event_data.model {
                if model != event_model {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Environment filter
        if let Some(environment) = &self.environment {
            if let Some(event_env) = &event_data.environment {
                if environment != event_env {
                    return false;
                }
            } else {
                return false;
            }
        }

        // User ID filter
        if let Some(user_id) = &self.user_id {
            if let Some(event_user) = &event_data.user_id {
                if user_id != event_user {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Cost filter
        if let Some(min_cost) = self.min_cost {
            if let Some(event_cost) = event_data.cost_usd {
                if event_cost < min_cost {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Duration filter
        if let Some(min_duration) = self.min_duration_ms {
            if let Some(event_duration) = event_data.duration_ms {
                if event_duration < min_duration {
                    return false;
                }
            } else {
                return false;
            }
        }

        true
    }
}

// ============================================================================
// Event Data
// ============================================================================

/// Common event data fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventData {
    /// Trace ID
    pub trace_id: Option<String>,
    /// Organization ID
    pub org_id: String,
    /// Provider
    pub provider: Option<String>,
    /// Model
    pub model: Option<String>,
    /// Environment
    pub environment: Option<String>,
    /// User ID
    pub user_id: Option<String>,
    /// Cost in USD
    pub cost_usd: Option<f64>,
    /// Duration in milliseconds
    pub duration_ms: Option<i32>,
    /// Additional metadata
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

// ============================================================================
// Specific Event Payloads
// ============================================================================

/// Trace created event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceCreatedEvent {
    pub trace_id: String,
    pub org_id: String,
    pub provider: String,
    pub model: String,
    pub environment: String,
    pub user_id: Option<String>,
    pub cost_usd: f64,
    pub duration_ms: i32,
    pub status_code: String,
    pub timestamp: DateTime<Utc>,
}

impl TraceCreatedEvent {
    pub fn to_event_data(&self) -> EventData {
        EventData {
            trace_id: Some(self.trace_id.clone()),
            org_id: self.org_id.clone(),
            provider: Some(self.provider.clone()),
            model: Some(self.model.clone()),
            environment: Some(self.environment.clone()),
            user_id: self.user_id.clone(),
            cost_usd: Some(self.cost_usd),
            duration_ms: Some(self.duration_ms),
            metadata: HashMap::new(),
        }
    }
}

/// Metric threshold event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricThresholdEvent {
    pub metric_name: String,
    pub threshold_value: f64,
    pub current_value: f64,
    pub org_id: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub environment: Option<String>,
    pub timestamp: DateTime<Utc>,
}

impl MetricThresholdEvent {
    pub fn to_event_data(&self) -> EventData {
        let mut metadata = HashMap::new();
        metadata.insert(
            "metric_name".to_string(),
            serde_json::json!(self.metric_name),
        );
        metadata.insert(
            "threshold_value".to_string(),
            serde_json::json!(self.threshold_value),
        );
        metadata.insert(
            "current_value".to_string(),
            serde_json::json!(self.current_value),
        );

        EventData {
            trace_id: None,
            org_id: self.org_id.clone(),
            provider: self.provider.clone(),
            model: self.model.clone(),
            environment: self.environment.clone(),
            user_id: None,
            cost_usd: None,
            duration_ms: None,
            metadata,
        }
    }
}

/// Cost threshold event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostThresholdEvent {
    pub threshold_value: f64,
    pub current_value: f64,
    pub period: String,
    pub org_id: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub timestamp: DateTime<Utc>,
}

impl CostThresholdEvent {
    pub fn to_event_data(&self) -> EventData {
        let mut metadata = HashMap::new();
        metadata.insert(
            "threshold_value".to_string(),
            serde_json::json!(self.threshold_value),
        );
        metadata.insert(
            "current_value".to_string(),
            serde_json::json!(self.current_value),
        );
        metadata.insert("period".to_string(), serde_json::json!(self.period));

        EventData {
            trace_id: None,
            org_id: self.org_id.clone(),
            provider: self.provider.clone(),
            model: self.model.clone(),
            environment: None,
            user_id: None,
            cost_usd: Some(self.current_value),
            duration_ms: None,
            metadata,
        }
    }
}

/// Export job status event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportJobStatusEvent {
    pub job_id: String,
    pub org_id: String,
    pub status: String,
    pub progress_percent: Option<i32>,
    pub trace_count: Option<i64>,
    pub timestamp: DateTime<Utc>,
}

impl ExportJobStatusEvent {
    pub fn to_event_data(&self) -> EventData {
        let mut metadata = HashMap::new();
        metadata.insert("job_id".to_string(), serde_json::json!(self.job_id));
        metadata.insert("status".to_string(), serde_json::json!(self.status));
        if let Some(progress) = self.progress_percent {
            metadata.insert("progress_percent".to_string(), serde_json::json!(progress));
        }
        if let Some(count) = self.trace_count {
            metadata.insert("trace_count".to_string(), serde_json::json!(count));
        }

        EventData {
            trace_id: None,
            org_id: self.org_id.clone(),
            provider: None,
            model: None,
            environment: None,
            user_id: None,
            cost_usd: None,
            duration_ms: None,
            metadata,
        }
    }
}

// ============================================================================
// Subscription Management
// ============================================================================

/// Subscription details
#[derive(Debug, Clone)]
pub struct Subscription {
    pub subscription_id: String,
    pub org_id: String,
    pub event_types: Vec<EventType>,
    pub filters: EventFilters,
    pub created_at: DateTime<Utc>,
}

impl Subscription {
    /// Check if an event matches this subscription
    pub fn matches_event(&self, event_type: &EventType, event_data: &EventData) -> bool {
        // Check if subscribed to this event type
        if !self.event_types.contains(event_type) {
            return false;
        }

        // Check if organization matches
        if self.org_id != event_data.org_id {
            return false;
        }

        // Apply filters
        self.filters.matches(event_data)
    }
}

// ============================================================================
// Connection State
// ============================================================================

/// WebSocket connection state
#[derive(Debug, Clone)]
pub struct ConnectionState {
    pub connection_id: String,
    pub org_id: String,
    pub user_id: String,
    pub permissions: Vec<String>,
    pub subscriptions: Vec<Subscription>,
    pub connected_at: DateTime<Utc>,
    pub last_ping: DateTime<Utc>,
}

impl ConnectionState {
    /// Check if connection has permission for an event type
    pub fn has_permission_for(&self, event_type: &EventType) -> bool {
        let required = event_type.requires_permission();
        self.permissions.iter().any(|p| p == required)
    }

    /// Add a subscription
    pub fn add_subscription(&mut self, subscription: Subscription) {
        self.subscriptions.push(subscription);
    }

    /// Remove subscriptions for specific event types
    pub fn remove_subscriptions(&mut self, event_types: &[EventType]) {
        self.subscriptions
            .retain(|sub| !sub.event_types.iter().any(|et| event_types.contains(et)));
    }

    /// Get all subscriptions matching an event
    pub fn matching_subscriptions(
        &self,
        event_type: &EventType,
        event_data: &EventData,
    ) -> Vec<&Subscription> {
        self.subscriptions
            .iter()
            .filter(|sub| sub.matches_event(event_type, event_data))
            .collect()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_type_permissions() {
        assert_eq!(EventType::TraceCreated.requires_permission(), "traces:read");
        assert_eq!(EventType::CostThreshold.requires_permission(), "costs:read");
        assert_eq!(EventType::SystemAlert.requires_permission(), "alerts:read");
    }

    #[test]
    fn test_event_filters_matches() {
        let filters = EventFilters {
            provider: Some("openai".to_string()),
            min_cost: Some(1.0),
            ..Default::default()
        };

        let mut event_data = EventData {
            trace_id: Some("trace1".to_string()),
            org_id: "org1".to_string(),
            provider: Some("openai".to_string()),
            cost_usd: Some(2.0),
            ..Default::default()
        };

        // Should match
        assert!(filters.matches(&event_data));

        // Should not match - wrong provider
        event_data.provider = Some("anthropic".to_string());
        assert!(!filters.matches(&event_data));

        // Should not match - cost too low
        event_data.provider = Some("openai".to_string());
        event_data.cost_usd = Some(0.5);
        assert!(!filters.matches(&event_data));
    }

    #[test]
    fn test_subscription_matches_event() {
        let subscription = Subscription {
            subscription_id: "sub1".to_string(),
            org_id: "org1".to_string(),
            event_types: vec![EventType::TraceCreated],
            filters: EventFilters {
                provider: Some("openai".to_string()),
                ..Default::default()
            },
            created_at: Utc::now(),
        };

        let event_data = EventData {
            trace_id: Some("trace1".to_string()),
            org_id: "org1".to_string(),
            provider: Some("openai".to_string()),
            ..Default::default()
        };

        // Should match
        assert!(subscription.matches_event(&EventType::TraceCreated, &event_data));

        // Should not match - different event type
        assert!(!subscription.matches_event(&EventType::MetricThreshold, &event_data));

        // Should not match - different org
        let wrong_org_data = EventData {
            org_id: "org2".to_string(),
            ..event_data
        };
        assert!(!subscription.matches_event(&EventType::TraceCreated, &wrong_org_data));
    }

    #[test]
    fn test_connection_state_permissions() {
        let mut state = ConnectionState {
            connection_id: "conn1".to_string(),
            org_id: "org1".to_string(),
            user_id: "user1".to_string(),
            permissions: vec!["traces:read".to_string(), "metrics:read".to_string()],
            subscriptions: vec![],
            connected_at: Utc::now(),
            last_ping: Utc::now(),
        };

        // Should have permission
        assert!(state.has_permission_for(&EventType::TraceCreated));
        assert!(state.has_permission_for(&EventType::MetricThreshold));

        // Should not have permission
        assert!(!state.has_permission_for(&EventType::CostThreshold));
    }
}
