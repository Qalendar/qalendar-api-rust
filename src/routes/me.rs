use axum::{routing::get, Router};
use crate::errors::AppError;
use crate::AppState; // Import AppState
use crate::middleware::auth::AuthenticatedUser; // Import the extractor
use axum::Json; // For returning JSON responses
use serde_json::json; // For simple JSON responses

use super::{category, deadline, event, invitation, share, tfa, ai, open_share}; // Import submodules

// Import me_handler for the /me routes
use crate::handlers::me_handler::{
    get_authenticated_user_info, // Import the modified GET handler
    update_user_handler, // Import the new PUT handler
    delete_user_handler, // Import the new DELETE handler
};

// Handler that requires authentication
// Axum automatically runs the AuthenticatedUser extractor before this handler
// If the extractor fails (invalid/missing token), this handler is NEVER reached.
// Instead, the AppError::JwtError (mapped to 401 Unauthorized) is returned.
// pub async fn get_authenticated_user_info(
//     // This extractor runs FIRST. If it succeeds, `user` is the AuthenticatedUser struct.
//     AuthenticatedUser { user_id }: AuthenticatedUser,
//     // Access state if needed
//     // State(state): State<AppState>,
// ) -> Result<Json<serde_json::Value>, AppError> { // Use Result<Json<...>, AppError>
//     // If we reach here, the user is authenticated, and `user_id` is available.
//     tracing::info!("Authenticated user accessed /me: {}", user_id);

//     // Now you can use user_id to fetch user details from the DB if needed,
//     // or just return the ID as proof of authentication.

//     Ok(Json(json!({
//         "message": "You are authenticated!",
//         "userId": user_id,
//         // You could fetch more details here:
//         // "userDetails": fetch_user_from_db(user_id, &state.pool).await?
//     })))
// }


// Function to create the /me sub-router
pub fn me_routes(app_state: AppState) -> Router {
    // Create the categories router, passing AppState
    let categories_router = category::categories_routes(app_state.clone());
    let deadlines_router = deadline::deadlines_routes(app_state.clone());
    let events_router = event::events_routes(app_state.clone());
    let invitations_router = invitation::invitations_routes(app_state.clone());
    let shares_router = share::share_routes(app_state.clone());
    let tfa_router = tfa::tfa_routes(app_state.clone());
    let ai_router = ai::ai_routes(app_state.clone());
    let open_share_router = open_share::open_share_routes(app_state.clone());

    Router::new()
       // --- Base /api/me routes (GET, PUT, DELETE for the user themselves) ---
       .route(
           "/",
           get(get_authenticated_user_info) // GET /api/me (get user details)
           .put(update_user_handler) // PUT /api/me (update user details)
           .delete(delete_user_handler) // DELETE /api/me (soft delete user)
       )
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
        // --- NEW AI Assistant Route ---
       .nest("/ai-assistant", ai_router) // Nest the AI router here. Its route is "/ai-assistant",
        // so combined path is /me/ai-assistant
        .nest("/open-shares", open_share_router)
       .with_state(app_state)
}