use axum::{
    routing::{get, put}, // Only GET and PUT for invitee actions here
    Router,
};
use crate::AppState; // Import AppState
use crate::handlers::invitation_handler; // Import invitation handlers

// Function to create the /api/me/invitations sub-router (Invitee actions)
pub fn invitation_routes(app_state: AppState) -> Router {
     Router::new()
        // Base route: /api/me/invitations (List my invitations)
        .route("/", get(invitation_handler::list_my_invitations))
        // Route for responding to a specific invitation: /api/me/invitations/{invitation_id}/status
        .route("/:invitation_id/status", put(invitation_handler::respond_to_invitation))
        // Make AppState available
        .with_state(app_state)
}