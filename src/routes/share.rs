use axum::{
    routing::{get, post, put, delete},
    Router,
};
use crate::AppState; // Import AppState
use crate::handlers::share_handler; // Import share handlers

// Function to create the shares sub-router (Owner actions)
pub fn share_routes(app_state: AppState) -> Router<AppState> {
     Router::new()
        // Base route: /api/me/shares
        .route(
            "/",
            post(share_handler::create_share) // POST to create
            .get(share_handler::list_shares)  // GET to list all
        )
        // Routes with ID parameter: /api/me/shares/{share_id}
        .route(
            "/{share_id}",
            get(share_handler::get_share_by_id) // GET by ID
            .put(share_handler::update_share)   // PUT to update by ID
            .delete(share_handler::delete_share) // DELETE by ID
        )
        // Make AppState available
        .with_state(app_state)
}