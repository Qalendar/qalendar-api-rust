use axum::{
    routing::get,
    Router,
};
use crate::AppState; // Import AppState
use crate::handlers::calendar_handler; // Import the calendar handler
use super::shared_calendar; // Import the shared_calendar module

// Function to create the calendar sub-router
pub fn calendar_routes(app_state: AppState) -> Router { // Explicitly type state
    Router::new()
        // Route: /api/calendar (User's own consolidated view)
        .route("/", get(calendar_handler::get_user_calendar))

        // --- SHARED CALENDAR ROUTES ---
        // Nest routes related to private shared calendars under /api/calendar/shares
        .nest("/shares",
              Router::new()
                  // Route: /api/calendar/shares (List calendars shared WITH the authenticated user)
                  .route("/", get(calendar_handler::list_received_shares))
                  // Route: /api/calendar/shares/:share_id (View a specific private shared calendar)
                  .route("/:share_id", get(calendar_handler::get_shared_calendar))
                  .with_state(app_state.clone())
        )

        // --- NEW: OPEN SHARED CALENDAR ROUTE (Public) ---
        // Route: /api/calendar/open-shares/:uuid
        .route(
            "/open-shares/:uuid", // Use :uuid for path parameter
            get(calendar_handler::get_open_shared_calendar) // Public handler
        )
        // No .with_state needed on the public route itself, handler accesses it via State extractor

        // Make AppState available to handlers within this MAIN router (calendar_routes)
        // (Used by get_user_calendar directly, and passed down to nested routers)
        .with_state(app_state)
}