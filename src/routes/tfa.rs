use axum::{
    routing::{get, post, put, delete},
    Router,
};
use crate::AppState; // Import AppState
use crate::handlers::auth_handler::{
    initiate_tfa_setup_handler, complete_tfa_setup_handler, disable_tfa_handler,
}; // Import TFA handlers

// Function to create the 2FA sub-router
pub fn tfa_routes(app_state: AppState) -> Router<AppState> {
    Router::new()
    // Route: /api/me/tfa/setup/initiate (Initiate setup)
    .route(
        "/setup/initiate",
    post(initiate_tfa_setup_handler)
)
    // Route: /api/me/tfa/setup/complete (Complete setup)
    .route(
        "/setup/complete",
        post(complete_tfa_setup_handler)
)
    // Route: /api/me/tfa/disable (Disable 2FA)
    .route(
        "/disable",
    post(disable_tfa_handler)
)
    // These handlers require authentication via AuthenticatedUser extractor,
    // which runs before the handler, even though the router itself doesn't
    // have an explicit layer here. The handlers are protected by design.
    // Make AppState available to these handlers.
    .with_state(app_state.clone())
}