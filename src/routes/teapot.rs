use axum::{extract::State, response::Json, routing::get, Router};
use serde_json::{json, Value};
use std::sync::Arc;
use http::StatusCode;

use crate::{errors::AppError, state::AppState};

async fn teapot_handler(State(state): State<AppState>) -> (StatusCode, Json<Value>) {
    // Create a JSON response with a 418 status code
    let response = json!({
        "status": "I'm a teapot",
        "message": "This is a teapot response."
    });

    (StatusCode::IM_A_TEAPOT, Json(response))
}

pub fn teapot_routes(state: AppState) -> Router {
    Router::new().route("/", get(teapot_handler))
    .with_state(state)
}
