use axum::{
    routing::get,
    Router,
};
use crate::AppState; // Import AppState
use crate::handlers::calendar_handler; // Import the calendar handler
use super::shared_calendar; // Import the shared_calendar module

// Function to create the calendar sub-router
pub fn calendar_routes(app_state: AppState) -> Router { // Explicitly type state
    let shared_calendar_routes = shared_calendar::shared_calendar_routes(app_state.clone()); // Pass state
     Router::new()
        // Base route: /api/calendar
        .route(
            "/",
            get(calendar_handler::get_user_calendar) // GET to fetch all items
        )
        // Route for seeing calendars shared WITH me: /api/calendar/shares
        .nest("/shares", shared_calendar_routes)
        // Add the shared calendar route here later: /api/calendar/shares/{share_id}

        // Make AppState available to all handlers within this router
        .with_state(app_state)
}