use axum::{
    routing::{get, post, put, delete},
    Router,
};
use crate::AppState; // Import AppState
use crate::handlers::open_share_handler; // Import open share handlers
use uuid::Uuid; // Import Uuid

// Function to create the open shares sub-router (Owner actions - /api/me/open-shares)
pub fn open_share_routes(app_state: AppState) -> Router<AppState> {
    Router::new()
        // Base route: /api/me/open-shares
        .route(
            "/",
            post(open_share_handler::create_open_share) // POST to create
                .get(open_share_handler::list_open_shares)  // GET to list all owner's open shares
        )
        // Routes with UUID parameter: /api/me/open-shares/:uuid
        .route(
            "/{uuid}", // Use :uuid for path parameter
            get(open_share_handler::get_open_share_by_uuid) // GET by UUID (Owner view)
                .put(open_share_handler::update_open_share)   // PUT to update by UUID
                .delete(open_share_handler::delete_open_share) // DELETE by UUID
        )
        // Make AppState available
        .with_state(app_state)
}