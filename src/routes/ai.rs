use axum::{
    routing::post, // Need POST for the AI endpoint
    Router,
};
use crate::AppState; // Import AppState
use crate::handlers::ai_handler; // Import AI handler

// Function to create the AI sub-router
pub fn ai_routes(app_state: AppState) -> Router<AppState> { // Explicitly type state
     Router::new()
        // Route: /api/me/ai-assistant (Handles AI prompt with optional files)
        // Nest this under /api/me or /api directly. Let's add to /me routes.
        // If adding directly under /api: Router::new().route("/ai/prompt", post(ai_handler::handle_ai_prompt))
        // If adding under /api/me: Router::new().route("/ai-assistant", post(ai_handler::handle_ai_prompt))
        // Let's follow the initial thought and add it under /api/me/ai-assistant
        .route("/ai-assistant", post(ai_handler::handle_ai_prompt)) // Mounted under /me, becomes /api/me/ai-assistant

        // Make AppState available
        .with_state(app_state)
}