use axum::Router;

use crate::state::AppState;

pub mod auth; // Declare the auth submodule
pub mod me; // Declare the me submodule
pub mod category; // Declare the category submodule
pub mod deadline; // Declare the deadline submodule
pub mod event; // Declare the event submodule
pub mod invitation; // Declare the invitation submodule
pub mod share; // Declare the calendar_share submodule
pub mod shared_calendar; // Declare the shared_calendar submodule
pub mod calendar; // Declare the calendar submodule
pub mod sync; // Declare the sync submodule
pub mod tfa; // Declare the tfa submodule
pub mod ai; // Declare the ai submodule
pub mod open_share; // Declare the open_share submodule
pub mod health; // Declare the health submodule
pub mod teapot; // Declare the teapot submodule
pub mod mirror; // Declare the mirror submodule

// Function to create the main API router, combining all sub-routers
pub fn create_api_router(app_state: AppState) -> Router {
    let auth_router = auth::auth_routes(app_state.clone()); // Pass state
    let me_router = me::me_routes(app_state.clone()); // Pass state
    let calendar_routes = calendar::calendar_routes(app_state.clone()); // Pass state
    let sync_routes = sync::sync_routes(app_state.clone()); // Pass state
    let health_routes = health::health_routes(app_state.clone()); // Pass state
    let teapot_routes = teapot::teapot_routes(app_state.clone()); // Pass state
    let mirror_routes = mirror::mirror_routes(app_state.clone()); // Pass state

    Router::new() // Group auth routes under /api
        .nest("/auth", auth_router)
        .nest("/me", me_router)
        .nest("/calendar", calendar_routes) // Group calendar routes under /api/calendar
        .nest("/sync", sync_routes) // Group sync routes under /api/sync
        .nest("/health", health_routes) // Group health routes under /api/health
        .nest("/teapot", teapot_routes) // Group teapot routes under /api/teapot
        .nest("/mirror", mirror_routes) // Group mirror routes under /api/mirror
        // .nest("/other_feature", other_routes) // Add more features later
}
