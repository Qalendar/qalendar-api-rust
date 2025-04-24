use axum::{
    routing::get, // Only GET for recipient view
    Router,
};
use crate::AppState; // Import AppState
use crate::handlers::calendar_handler; // Import handlers from calendar_handler

// Function to create the /api/shared-calendars sub-router (Recipient actions)
pub fn shared_calendar_routes(app_state: AppState) -> Router<AppState> {
     Router::new()
        // Base route: /api/calendar/shares (List calendars shared WITH me)
        .route("/", get(calendar_handler::list_received_shares))
        // Route for getting content of a specific shared calendar: /api/shared-calendars/:share_id
        // Route: /api/calendar/shares/:share_id (View a specific shared calendar)
        .route("/:share_id", get(calendar_handler::get_shared_calendar))
        // Make AppState available
        .with_state(app_state)
}