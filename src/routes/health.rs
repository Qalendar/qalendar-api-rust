use axum::{response::Json, routing::get, Router};
use serde_json::json;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub version: Arc<String>,
}

async fn health_handler(state: Arc<AppState>) -> Json<serde_json::Value> {
    let version = state.version.clone();
    Json(json!({ "status": "ok", "version": *version }))
}

pub fn health_routes(state: AppState) -> Router {
    Router::new().route("/health", get(health_handler)).with_state(state)
}
