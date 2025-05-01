use axum::{extract::State, response::Json, routing::get, Router};
use serde_json::json;
use std::sync::Arc;

use crate::state::AppState;

async fn health_handler(State(state): State<AppState>) -> Json<serde_json::Value> {
    let version = state.config.version.clone();
    Json(json!({ "status": "ok", "version": version }))
}

pub fn health_routes(state: AppState) -> Router {
    Router::new().route("/", get(health_handler)).with_state(state)
}
