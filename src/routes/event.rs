use axum::{
    routing::{get, post, put, delete},
    Router,
};
use crate::AppState;
use crate::handlers::event_handler; // Import event handlers

// Function to create the events sub-router
pub fn events_routes(app_state: AppState) -> Router<AppState> {
     Router::new()
        // Base event routes: /api/me/events
        .route(
            "/",
            post(event_handler::create_event) // POST to create base event
            // Removed GET /api/me/events here, as /api/me/calendar will handle range queries
        )
        // Base event routes with ID parameter: /api/me/events/{event_id}
        // These operate on the *base* event record, not occurrences
        .route(
            "/:event_id",
            get(event_handler::get_base_event_by_id) // GET single base event by ID
            .put(event_handler::update_event) // PUT to update base event by ID
            .delete(event_handler::delete_event) // DELETE base event by ID
        )
        // --- Event Exception Routes (Nested under event_id) ---
        // Example: /api/me/events/{event_id}/exceptions
        // .route(
        //     "/:event_id/exceptions",
        //     post(event_handler::create_event_exception) // Create an exception for this event
        //     // GET /api/me/events/{event_id}/exceptions (optional: list exceptions for an event)
        // )
        // Example: /api/me/events/{event_id}/exceptions/{exception_id}
        // .route(
        //     "/:event_id/exceptions/:exception_id",
        //     get(event_handler::get_event_exception_by_id) // Get a specific exception
        //     .put(event_handler::update_event_exception) // Update a specific exception
        //     .delete(event_handler::delete_event_exception) // Delete a specific exception
        // )
        // Make AppState available to all handlers within this router
        .with_state(app_state)
}