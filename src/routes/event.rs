use axum::{
    routing::{get, post, put, delete},
    Router,
};
use crate::AppState; // Import AppState
use crate::handlers::{
    event_handler, invitation_handler
}; // Import handlers

// Function to create the events sub-router
pub fn events_routes(app_state: AppState) -> Router<AppState> {
     Router::new()
        // Base route: /api/me/events
        .route(
            "/",
            post(event_handler::create_event) // POST to create
            .get(event_handler::get_events)  // GET to list all
        )
        // Routes with ID parameter: /api/me/events/{event_id}
        .route(
            "/{event_id}",
            get(event_handler::get_event_by_id) // GET by ID
            .put(event_handler::update_event)   // PUT to update by ID
            .delete(event_handler::delete_event) // DELETE by ID
        )
        // --- OWNER-SIDE INVITATION ROUTES ---
        // Nest these under /api/me/events/:event_id/invitations
        // We can define a nested router specific to the event ID path segment
        .nest(
            "/:event_id/invitations", // Path segment capturing event_id
            Router::new()
               // Routes under /api/me/events/:event_id/invitations
               .route("/",
                   post(invitation_handler::create_invitation) // POST to invite
                   .get(invitation_handler::list_invitations_for_event) // GET to list invites
               )
                // Route for a specific invitation: /api/me/events/:event_id/invitations/:invitation_id
               .route("/:invitation_id",
                   delete(invitation_handler::revoke_invitation) // DELETE to revoke
               )
                // The handlers for these nested routes also need AppState.
                // Since this nested router is created within events_routes,
                // which has app_state available, we can pass it down.
                .with_state(app_state.clone()) // Pass AppState down to this nested router
       )
       // Make AppState available to all handlers within this MAIN router (events_routes)
        .with_state(app_state)
}