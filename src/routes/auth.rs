use axum::{
    routing::post,
    Router,
};
use crate::handlers::auth_handler::{register_user_handler, login_user_handler};
use crate::state::AppState;

pub fn auth_routes(app_state: AppState) -> Router {
     Router::new()
        .route("/register", post(register_user_handler))
        .route("/login", post(login_user_handler))
        .with_state(app_state)
}