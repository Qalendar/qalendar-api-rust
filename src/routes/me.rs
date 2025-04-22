use axum::{routing::get, Router};
use crate::errors::AppError;
use crate::AppState; // Import AppState
use crate::middleware::auth::AuthenticatedUser; // Import the extractor
use axum::Json; // For returning JSON responses
use serde_json::json; // For simple JSON responses

use crate::routes::category;
use crate::routes::deadline;

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

    Router::new()
       // Define the base protected route /api/me
       .route("/", get(get_authenticated_user_info))
       // Nest the categories router under /api/me/categories
       .nest("/categories", categories_router)
              // Nest the deadlines router under /api/me/deadlines

       .nest("/deadlines", deadlines_router)
       // Make AppState available to direct /me handlers (like get_authenticated_user_info)
       .with_state(app_state)
}