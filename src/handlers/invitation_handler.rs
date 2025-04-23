use axum::{
    extract::{State, Path, Json, Query}, // Added Query for GET filtering
    http::StatusCode,
};
use sqlx::{PgPool, Row, types::chrono::Utc}; // Added Row for exists check
use validator::Validate;
use crate::{
    AppState,
    errors::AppError,
    models::{
        event_invitation::{EventInvitation, InviteUserPayload, InvitationResponsePayload, ListMyInvitationsParams},
        enums::EventInvitationStatus, // Import the ENUM
        user::User, // Need User model to look up invitee by email
    },
    middleware::auth::AuthenticatedUser,
};

// --- Helper: Check if event exists and is owned by the user ---
async fn check_event_ownership(pool: &PgPool, event_id: i32, user_id: i32) -> Result<bool, AppError> {
    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM events WHERE event_id = $1 AND user_id = $2)",
        event_id,
        user_id
    )
    .fetch_one(pool)
    .await?;
    Ok(exists.unwrap_or(false)) // query_scalar!(EXISTS(...)) returns Option<bool>
}

// --- Helper: Check if invitation exists and is for the invited user ---
async fn check_invitation_invitee(pool: &PgPool, invitation_id: i32, invited_user_id: i32) -> Result<bool, AppError> {
    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM event_invitations WHERE invitation_id = $1 AND invited_user_id = $2)",
        invitation_id,
        invited_user_id
    )
    .fetch_one(pool)
    .await?;
    Ok(exists.unwrap_or(false))
}


// --- OWNER ACTION: Create Invitation (POST /api/me/events/:event_id/invitations) ---
pub async fn create_invitation(
    State(state): State<AppState>,
    AuthenticatedUser { user_id: owner_user_id }: AuthenticatedUser, // Renamed for clarity
    Path(event_id): Path<i32>,
    Json(payload): Json<InviteUserPayload>,
) -> Result<(StatusCode, Json<EventInvitation>), AppError> {
    payload.validate()?;
    let invited_user_email = payload.invited_user_email.unwrap();

    // 1. Check if the event exists and is owned by the authenticated user
    if !check_event_ownership(&state.pool, event_id, owner_user_id).await? {
        return Err(AppError::EventNotFound); // Or AppError::CannotInviteToNonOwnedEvent
    }

    // 2. Find the user to invite by email
    let invited_user = sqlx::query_as!(
        User, // Need to import User model
        r#"
        SELECT
            user_id, display_name, email, password_hash, date_of_birth as "date_of_birth!: _",
            email_verified as "email_verified!", created_at as "created_at!: _", updated_at as "updated_at!: _"
        FROM users
        WHERE email = $1
        "#,
        invited_user_email
    )
    .fetch_optional(&state.pool)
    .await?;

    let invited_user = match invited_user {
        Some(user) => user,
        None => {
             // User not found means they cannot be invited
             // Consider a specific error like AppError::InvitedUserNotFound
            return Err(AppError::UserNotFound); // Re-using UserNotFound for now
        }
    };

    // Prevent inviting oneself
    if invited_user.user_id == owner_user_id {
        // Consider a specific error like AppError::CannotInviteSelf
        return Err(AppError::InternalServerError("Cannot invite yourself".to_string())); // Simple error for now
    }


    // 3. Check if an invitation already exists for this user and event
    let invitation_exists: bool = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM event_invitations WHERE event_id = $1 AND invited_user_id = $2)",
        event_id,
        invited_user.user_id
    )
    .fetch_one(&state.pool)
    .await?
    .unwrap_or(false);

    if invitation_exists {
        // Consider a specific error like AppError::InvitationAlreadyExists
        return Err(AppError::InternalServerError("Invitation already exists for this user and event".to_string())); // Simple error for now
    }

    // 4. Create the invitation
    let created_invitation = sqlx::query_as!(
        EventInvitation,
        r#"
        INSERT INTO event_invitations (event_id, owner_user_id, invited_user_id)
        VALUES ($1, $2, $3)
        RETURNING
            invitation_id, event_id, owner_user_id, invited_user_id, status as "status!: _",
            created_at as "created_at!", updated_at as "updated_at!"
        "#,
        event_id,
        owner_user_id,
        invited_user.user_id,
    )
    .fetch_one(&state.pool)
    .await?;

    Ok((StatusCode::CREATED, Json(created_invitation)))
}

// --- OWNER ACTION: List Invitations for an Event (GET /api/me/events/:event_id/invitations) ---
// Might want to return invited user's display_name/email here too, requires JOIN or separate query
pub async fn list_invitations_for_event(
    State(state): State<AppState>,
    AuthenticatedUser { user_id: owner_user_id }: AuthenticatedUser,
    Path(event_id): Path<i32>,
) -> Result<Json<Vec<EventInvitation>>, AppError> {
     // 1. Check if the event exists and is owned by the authenticated user
    if !check_event_ownership(&state.pool, event_id, owner_user_id).await? {
        return Err(AppError::EventNotFound); // Or AppError::CannotViewInvitationsForNonOwnedEvent
    }

    // 2. Fetch invitations for this event
    let invitations = sqlx::query_as!(
        EventInvitation,
        r#"
        SELECT
            invitation_id, event_id, owner_user_id, invited_user_id, status as "status!: _",
            created_at as "created_at!", updated_at as "updated_at!"
        FROM event_invitations
        WHERE event_id = $1 AND owner_user_id = $2 -- Ensure only fetch invites for YOUR event
        ORDER BY created_at -- Optional: order
        "#,
        event_id,
        owner_user_id
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(invitations))
}


// --- OWNER ACTION: Revoke Invitation (DELETE /api/me/events/:event_id/invitations/:invitation_id) ---
pub async fn revoke_invitation(
    State(state): State<AppState>,
    AuthenticatedUser { user_id: owner_user_id }: AuthenticatedUser,
    Path((event_id, invitation_id)): Path<(i32, i32)>, // Extract both IDs from the path
) -> Result<StatusCode, AppError> {
     // 1. Check if the event exists and is owned by the authenticated user
    if !check_event_ownership(&state.pool, event_id, owner_user_id).await? {
        return Err(AppError::EventNotFound); // Or AppError::CannotRevokeInvitationForNonOwnedEvent
    }

    // 2. Perform the delete query. Check event_id, invitation_id, AND owner_user_id!
    let delete_result = sqlx::query!(
        r#"
        DELETE FROM event_invitations
        WHERE invitation_id = $1 AND event_id = $2 AND owner_user_id = $3
        "#,
        invitation_id,
        event_id,
        owner_user_id
    )
    .execute(&state.pool)
    .await?;

    if delete_result.rows_affected() == 0 {
        // No rows deleted means the invitation didn't exist, didn't belong to this event,
        // or the event didn't belong to the owner.
        Err(AppError::InvitationNotFound) // Use InvitationNotFound for clarity
    } else {
        Ok(StatusCode::NO_CONTENT)
    }
}

// --- INVITEE ACTION: List My Invitations (GET /api/me/invitations) ---
// Returns invitations received by the authenticated user.
// Can filter by status using query parameters (e.g., ?status=pending)
pub async fn list_my_invitations(
    State(state): State<AppState>,
    AuthenticatedUser { user_id: invited_user_id }: AuthenticatedUser, // Renamed
    Query(params): Query<ListMyInvitationsParams>, // Extract query parameters
) -> Result<Json<Vec<EventInvitation>>, AppError> {

    let invitations = if let Some(status_filter) = params.status {
        // Query with status filter
        sqlx::query_as!(
            EventInvitation,
            r#"
            SELECT
                invitation_id, event_id, owner_user_id, invited_user_id, status as "status!: _",
                created_at as "created_at!", updated_at as "updated_at!"
            FROM event_invitations
            WHERE invited_user_id = $1 AND status = $2
            ORDER BY created_at
            "#,
            invited_user_id,
            status_filter as EventInvitationStatus // Cast the ENUM
        )
        .fetch_all(&state.pool)
        .await?
    } else {
        // Query without status filter
        sqlx::query_as!(
            EventInvitation,
            r#"
            SELECT
                invitation_id, event_id, owner_user_id, invited_user_id, status as "status!: _",
                created_at as "created_at!", updated_at as "updated_at!"
            FROM event_invitations
            WHERE invited_user_id = $1
            ORDER BY created_at
            "#,
            invited_user_id
        )
        .fetch_all(&state.pool)
        .await?
    };

    // Note: Frontend will likely need event details here (title, time etc).
    // You'll probably need to perform a JOIN with the 'events' table in the query
    // or fetch event details separately if needed on this endpoint.
    // For sync, you will JOIN with events in the sync handler.

    Ok(Json(invitations))
}

// --- INVITEE ACTION: Respond to Invitation (PUT /api/me/invitations/:invitation_id/status) ---
pub async fn respond_to_invitation(
    State(state): State<AppState>,
    AuthenticatedUser { user_id: invited_user_id }: AuthenticatedUser, // Renamed
    Path(invitation_id): Path<i32>,
    Json(payload): Json<InvitationResponsePayload>,
) -> Result<Json<EventInvitation>, AppError> { // Return the updated invitation

    payload.validate()?;
    let new_status = payload.status.unwrap(); // Required by validation

    // 1. Check if the invitation exists and is for the authenticated user (the invitee)
    if !check_invitation_invitee(&state.pool, invitation_id, invited_user_id).await? {
        return Err(AppError::InvitationNotFound); // Use InvitationNotFound
    }

    // 2. Update the invitation status
    let updated_invitation = sqlx::query_as!(
        EventInvitation,
        r#"
        UPDATE event_invitations
        SET status = $1 -- updated_at trigger handles timestamp
        WHERE invitation_id = $2 AND invited_user_id = $3 -- IMPORTANT: Check invited_user_id!
        RETURNING invitation_id, event_id, owner_user_id, invited_user_id, status as "status!: _",
        created_at as "created_at!", updated_at as "updated_at!"
        "#,
        new_status as EventInvitationStatus, // Cast the ENUM
        invitation_id,
        invited_user_id // Crucial check
    )
    .fetch_one(&state.pool)
    .await?; // Propagates sqlx errors

    Ok(Json(updated_invitation)) // Return the updated invitation object
}
