use axum::{
    routing::{get, post, put, delete},
    Router,
};
use crate::AppState; // Import AppState
use crate::handlers::event_handler; // Import event handlers

// Function to create the events sub-router
pub fn events_routes(app_state: AppState) -> Router<AppState> {
     Router::new()
        // Base route: /api/me/events
        .route(
            "/",
            post(event_handler::create_event) // POST to create
            .get(event_handler::get_events)  // GET to list all
        )
        // Routes with ID parameter: /api/me/events/{event_id}
        .route(
            "/{event_id}",
            get(event_handler::get_event_by_id) // GET by ID
            .put(event_handler::update_event)   // PUT to update by ID
            .delete(event_handler::delete_event) // DELETE by ID
        )
        // Make AppState available to all handlers within this router
        .with_state(app_state)
}