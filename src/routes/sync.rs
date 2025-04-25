use axum::{
    routing::{get, post}, // Add post later for POST /api/sync
    Router,
};
use crate::AppState; // Import AppState
use crate::handlers::sync_handler; // Import sync handlers

// Function to create the sync sub-router
pub fn sync_routes(app_state: AppState) -> Router { // Explicitly type state
     Router::new()
        // Route: /api/me/sync (Fetch updates for owned data)
        // Note: This could also be /api/me/sync for consistency
        .route(
            "/me", // Mounted under /api, so this becomes /api/me/sync
            get(sync_handler::sync_owned_data)
        )
        // Route: GET /api/sync/calendar/shares/:share_id (Fetch updates for a specific shared calendar view)
        .route(
            "/calendar/shares/{share_id}", // New path
            get(sync_handler::sync_shared_calendar_data)
        )
        // Route: POST /api/sync (Process client changes - implement later) MAYBE
        // .route(
        //     "/",
        //     post(sync_handler::process_client_sync)
        // )

        // Make AppState available to all handlers within this router
        .with_state(app_state)
}