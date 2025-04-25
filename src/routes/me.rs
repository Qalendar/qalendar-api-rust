use axum::{routing::get, Router};
use crate::errors::AppError;
use crate::AppState; // Import AppState
use crate::middleware::auth::AuthenticatedUser; // Import the extractor
use axum::Json; // For returning JSON responses
use serde_json::json; // For simple JSON responses

use super::{category, deadline, event, invitation, share, tfa}; // Import submodules

use crate::handlers::auth_handler::{
    initiate_tfa_setup_handler, complete_tfa_setup_handler, disable_tfa_handler, // Import new 2FA handlers
};

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
    let events_router = event::events_routes(app_state.clone());
    let invitations_router = invitation::invitations_routes(app_state.clone());
    let shares_router = share::share_routes(app_state.clone());
    let tfa_router = tfa::tfa_routes(app_state.clone());

    Router::new()
       // Define the base protected route /api/me
       .route("/", get(get_authenticated_user_info)) // /api/me
       .nest("/categories", categories_router) // /api/me/categories
       .nest("/deadlines", deadlines_router)   // /api/me/deadlines
       .nest("/events", events_router) // /api/me/events
       .nest("/invitations", invitations_router) // /api/me/invitations
       .nest("/shares", shares_router) // /api/me/shares
       // Make AppState available to direct /me handlers (like get_authenticated_user_info)
       // --- NEW 2FA Routes (Protected under /me/tfa) ---
       .nest("/tfa", tfa_router) // /api/me/tfa
       .with_state(app_state)
}