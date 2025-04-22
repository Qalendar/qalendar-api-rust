use axum::{routing::get, Router};
use crate::errors::AppError;
use crate::AppState; // Import AppState
use crate::middleware::auth::AuthenticatedUser; // Import the extractor
use axum::Json; // For returning JSON responses
use serde_json::json; // For simple JSON responses

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
     Router::new()
        // Define a protected route
        .route("/", get(get_authenticated_user_info)) // GET /api/me
        // Add more protected routes under /me here (e.g., /api/me/categories, /api/me/events)
        // Note: Since this entire router is nested under /api/me, all routes defined here
        // implicitly require authentication *if* the AuthenticatedUser extractor is used in the handler.
        // If you had routes within /me that should NOT be protected, you'd need a different approach,
        // but for a /me path, protecting everything is standard.
        .with_state(app_state) // Make AppState available to handlers in this router
}