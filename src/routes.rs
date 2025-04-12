use axum::Router;

use crate::state::AppState;

pub mod auth; // Declare the auth submodule

// Function to create the main API router, combining all sub-routers
pub fn create_api_router(app_state: AppState) -> Router {
    let auth_routes = auth::auth_routes(app_state.clone()); // Pass state

    Router::new()
        .nest("/auth", auth_routes) // Group auth routes under /api/auth
        // .nest("/other_feature", other_routes) // Add more features later
}