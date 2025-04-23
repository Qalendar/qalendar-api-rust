use axum::Router;

use crate::state::AppState;

pub mod auth; // Declare the auth submodule
pub mod me; // Declare the me submodule
pub mod category; // Declare the category submodule
pub mod deadline;

// Function to create the main API router, combining all sub-routers
pub fn create_api_router(app_state: AppState) -> Router {
    let auth_router = auth::auth_routes(app_state.clone()); // Pass state
    let me_router = me::me_routes(app_state.clone()); // Pass state

    Router::new()
        .nest("/auth", auth_router) // Group auth routes under /api/auth
        .nest("/me", me_router)
        // .nest("/other_feature", other_routes) // Add more features later
}