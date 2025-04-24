use axum::Router;

use crate::state::AppState;

pub mod auth; // Declare the auth submodule
pub mod me; // Declare the me submodule
pub mod category; // Declare the category submodule
pub mod deadline; // Declare the deadline submodule
pub mod event; // Declare the event submodule
pub mod invitation; // Declare the invitation submodule
pub mod share; // Declare the calendar_share submodule
pub mod shared_calendar;

// Function to create the main API router, combining all sub-routers
pub fn create_api_router(app_state: AppState) -> Router {
    let auth_router = auth::auth_routes(app_state.clone()); // Pass state
    let me_router = me::me_routes(app_state.clone()); // Pass state
    let shared_calendar_routes = shared_calendar::shared_calendar_routes(app_state.clone()); // Pass state

    Router::new() // Group auth routes under /api
        .nest("/auth", auth_router)
        .nest("/me", me_router)
        .nest("/shared-calendars", shared_calendar_routes)
        // .nest("/other_feature", other_routes) // Add more features later
}