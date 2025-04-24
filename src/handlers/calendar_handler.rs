use axum::{
    extract::{State, Query, Path, Json},
    http::StatusCode,
};
use sqlx::{PgPool, types::chrono::Utc};
use validator::Validate;
use crate::{
    errors::AppError, middleware::auth::AuthenticatedUser, models::{
        calendar::UserCalendarResponse, calendar_share::{
            ReceivedShareResponseItem, ShareOwnerDetail // Import new models
        }, deadline::Deadline, enums::{EventInvitationStatus, SharePrivacyLevel}, event::Event, event_invitation::EventInvitation, user::User // Needed for shared calendar view handler
    }, AppState
};
use chrono::DateTime; // For parsing date strings

// Re-use or create a shared helper for timestamp parsing
// Ideally in src/utils/datetime.rs
// For now, keeping it local:
fn parse_timestamp(s: &str) -> Result<DateTime<Utc>, AppError> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| {
            tracing::warn!("Failed to parse timestamp '{}': {}", s, e);
            AppError::ValidationFailed(validator::ValidationErrors::new())
        })
}

// Need payload for range filtering for the *content* handlers later
// #[derive(Deserialize, Validate, Debug)]
// #[serde(rename_all = "camelCase")]
// pub struct CalendarRangeParams {
//    // Could add validators for format if needed
//    pub start: Option<String>,
//    pub end: Option<String>,
// }


// --- RECIPENT ACTION: List Calendars Shared With Me (GET /api/shared-calendars) ---
pub async fn list_received_shares(
    State(state): State<AppState>,
    AuthenticatedUser { user_id: shared_with_user_id }: AuthenticatedUser, // The user receiving the share
) -> Result<Json<Vec<ReceivedShareResponseItem>>, AppError> {
    let shares = sqlx::query_as!(
        ReceivedShareResponseItem, // Use the new response struct
        r#"
        SELECT
            cs.share_id,
            cs.owner_user_id,
            cs.shared_with_user_id,
            cs.message as "message!: _", -- Optional message
            cs.privacy_level as "privacy_level!: _", -- Explicit cast for ENUM
            cs.expires_at as "expires_at!: _", -- Optional expiry date
            cs.created_at as "created_at!", -- Explicit cast for DateTime
            cs.updated_at as "updated_at!", -- Explicit cast for DateTime
            -- Owner User Details (aliased)
            u.user_id AS user_id_alias, -- Alias matches struct field name
            u.display_name,
            u.email,
            -- Aggregated Category IDs
            ARRAY_AGG(csc.category_id) FILTER (WHERE csc.category_id IS NOT NULL) AS "shared_category_ids!: Vec<i32>" -- Explicit cast for Vec
        FROM calendar_shares cs
        JOIN users u ON cs.owner_user_id = u.user_id -- Join with owner
        LEFT JOIN calendar_share_categories csc ON cs.share_id = csc.share_id
        WHERE cs.shared_with_user_id = $1 -- Filter by the user who *received* the share
        GROUP BY cs.share_id, u.user_id -- Group required for array_agg
        ORDER BY cs.created_at DESC -- Optional: order
        "#,
        shared_with_user_id
    )
    .fetch_all(&state.pool)
    .await?;

    // Optional: Filter out expired shares on the backend if desired,
    // or let the frontend handle it based on expires_at date.

    Ok(Json(shares))
}

// --- Get User Calendar Items (GET /api/calendar) ---
// Returns all owned events, owned deadlines, and accepted invited events
pub async fn get_user_calendar(
    State(state): State<AppState>,
    AuthenticatedUser { user_id: authenticated_user_id }: AuthenticatedUser,
    // No Query parameters needed for ranges in this simplified version
) -> Result<Json<UserCalendarResponse>, AppError> {

    // Query 1: Fetch all owned events AND events where the user is an accepted invitee
    let events = sqlx::query_as!(
        Event,
        r#"
        SELECT
           event_id, user_id, category_id, title, description as "description!: _",
           start_time, end_time, location as "location!: _", rrule as "rrule!: _",
           created_at as "created_at!", updated_at as "updated_at!"
        FROM events
        WHERE user_id = $1 -- Owned events
           OR event_id IN (
               SELECT event_id
               FROM event_invitations
               WHERE invited_user_id = $1 AND status = $2
           ) -- Accepted invited events
        ORDER BY start_time
        "#,
        authenticated_user_id,
        EventInvitationStatus::Accepted as EventInvitationStatus // Bind the ENUM value for filtering accepted invites
    )
    .fetch_all(&state.pool)
    .await?; // Propagates sqlx::Error -> AppError::DatabaseError


    // Query 2: Fetch all owned deadlines
    let deadlines = sqlx::query_as!(
        Deadline,
        r#"
        SELECT
           deadline_id, user_id, category_id, title, description as "description!: _",
           due_date, priority as "priority!: _",
           workload_magnitude as "workload_magnitude!: _", workload_unit as "workload_unit!: _",
           created_at as "created_at!", updated_at as "updated_at!"
        FROM deadlines
        WHERE user_id = $1 -- Owned deadlines
        ORDER BY due_date -- Order by due date
        "#,
        authenticated_user_id,
        // No second parameter needed for deadlines query
    )
    .fetch_all(&state.pool)
    .await?; // Propagates sqlx::Error -> AppError::DatabaseError


    // Combine results into the response struct
    let response = UserCalendarResponse {
        events,
        deadlines,
    };

    Ok(Json(response))
}