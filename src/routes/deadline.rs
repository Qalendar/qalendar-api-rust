use axum::{
    routing::{get, post, put, delete},
    Router,
};
use crate::AppState; // Import AppState
use crate::handlers::deadline_handler; // Import deadline handlers

// Function to create the deadlines sub-router
pub fn deadlines_routes(app_state: AppState) -> Router<AppState> {
     Router::new()
        // Base route: /api/me/deadlines
        .route(
            "/",
            post(deadline_handler::create_deadline) // POST to create
            .get(deadline_handler::get_deadlines) // GET to list all
        )
        // Routes with ID parameter: /api/me/deadlines/{deadline_id}
        .route(
            "/{deadline_id}",
            get(deadline_handler::get_deadline_by_id) // GET by ID
            .put(deadline_handler::update_deadline) // PUT to update by ID
            .delete(deadline_handler::delete_deadline) // DELETE by ID
        )
        // Make AppState available to all handlers within this router
        .with_state(app_state)
}