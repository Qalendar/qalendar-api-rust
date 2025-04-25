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

// Function to create the main API router, combining all sub-routers
pub fn create_api_router(app_state: AppState) -> Router {
    let auth_router = auth::auth_routes(app_state.clone()); // Pass state
    let me_router = me::me_routes(app_state.clone()); // Pass state
    let calendar_routes = calendar::calendar_routes(app_state.clone()); // Pass state
    let sync_routes = sync::sync_routes(app_state.clone()); // Pass state

    Router::new() // Group auth routes under /api
        .nest("/auth", auth_router)
        .nest("/me", me_router)
        .nest("/calendar", calendar_routes) // Group calendar routes under /api/calendar
        .nest("/sync", sync_routes) // Group sync routes under /api/sync
        // .nest("/other_feature", other_routes) // Add more features later
}