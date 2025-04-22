use axum::{
    routing::{get, post, put, delete},
    Router,
};
use crate::AppState; // Import AppState
use crate::handlers::category_handler; // Import category handlers

// Function to create the categories sub-router
pub fn categories_routes(app_state: AppState) -> Router<AppState> {
     Router::new()
        // Base route: /api/me/categories
        .route(
            "/",
            post(category_handler::create_category) // POST to create
            .get(category_handler::get_categories) // GET to list all
        )
        // Routes with ID parameter: /api/me/categories/{category_id}
        .route(
            "/{category_id}",
            get(category_handler::get_category_by_id) // GET by ID
            .put(category_handler::update_category) // PUT to update by ID
            .delete(category_handler::delete_category) // DELETE by ID
        )
        // Make AppState available to all handlers within this router
        .with_state(app_state)
}