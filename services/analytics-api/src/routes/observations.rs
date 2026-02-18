use axum::{http::StatusCode, routing::post, Json, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tracing::info;

use crate::models::AppState;

#[derive(Debug, Deserialize)]
pub struct ObservationEvent {
    pub source: String,
    pub event_type: String,
    pub execution_id: String,
    pub timestamp: DateTime<Utc>,
    #[serde(default)]
    pub payload: Value,
}

#[derive(Debug, Serialize)]
pub struct ObservationResponse {
    pub status: &'static str,
    pub execution_id: String,
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/api/v1/observations", post(receive_observation))
}

async fn receive_observation(
    Json(event): Json<ObservationEvent>,
) -> (StatusCode, Json<ObservationResponse>) {
    info!(
        source = %event.source,
        event_type = %event.event_type,
        execution_id = %event.execution_id,
        timestamp = %event.timestamp,
        "Observation received"
    );

    (
        StatusCode::ACCEPTED,
        Json(ObservationResponse {
            status: "accepted",
            execution_id: event.execution_id,
        }),
    )
}
