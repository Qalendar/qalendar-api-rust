use axum::{routing::get, Router};
use crate::errors::AppError;
use crate::AppState; // Import AppState
use crate::middleware::auth::AuthenticatedUser; // Import the extractor
use axum::Json; // For returning JSON responses
use serde_json::json; // For simple JSON responses

use crate::routes::category;
use crate::routes::deadline;
use crate::routes::event;

// Handler that requires authentication
// Axum automatically runs the AuthenticatedUser extractor before this handler
// If the extractor fails (invalid/missing token), this handler is NEVER reached.
// Instead, the AppError::JwtError (mapped to 401 Unauthorized) is returned.
pub async fn get_authenticated_user_info(
    // This extractor runs FIRST. If it succeeds, `user` is the AuthenticatedUser struct.
    AuthenticatedUser { user_id }: AuthenticatedUser,
    // Access state if needed
    // State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> { // Use Result<Json<...>, AppError>
    // If we reach here, the user is authenticated, and `user_id` is available.
    tracing::info!("Authenticated user accessed /me: {}", user_id);

    // Now you can use user_id to fetch user details from the DB if needed,
    // or just return the ID as proof of authentication.

    Ok(Json(json!({
        "message": "You are authenticated!",
        "userId": user_id,
        // You could fetch more details here:
        // "userDetails": fetch_user_from_db(user_id, &state.pool).await?
    })))
}


// Function to create the /me sub-router
pub fn me_routes(app_state: AppState) -> Router {
    // Create the categories router, passing AppState
    let categories_router = category::categories_routes(app_state.clone());
    let deadlines_router = deadline::deadlines_routes(app_state.clone());
    let events_router = event::events_routes(app_state.clone()); // <-- Create the events router

    Router::new()
       // Define the base protected route /api/me
       .route("/", get(get_authenticated_user_info))
       // Nest the categories router under /api/me/categories
       .nest("/categories", categories_router)
              // Nest the deadlines router under /api/me/deadlines

       .nest("/deadlines", deadlines_router)
       .nest("/events", events_router) // <-- Nested event CRUD routes here

       // --- Dedicated route for fetching calendar items for a date range ---
       // This will return CalendarEventOccurrence and Deadlines within the range
    //    .route("/calendar", get(event_handler::get_events_in_range)) // Example route for fetching events+deadlines in range
                                                                     // Note: This specific handler only fetches events for now.
                                                                     // We'll need to update this handler or add another one
                                                                     // that fetches BOTH events and deadlines for the calendar view.
                                                                     // The sync handler might serve a similar purpose but for *changes*.

       // Add more feature routes here later (sync, shares, invitations)
       // .route("/sync", get(me_handler::sync_owned_data)) // Sync handler will be complex
       // .nest("/shares", shares::shares_routes(app_state.clone())) // Sharing routes
       // .nest("/invitations", invitations::invitations_routes(app_state.clone())) // Invitation routes

       // Make AppState available to direct /me handlers
       .with_state(app_state)
}