use axum::{
    extract::{State, Query, Path, Json},
    http::StatusCode,
};
use sqlx::{PgPool, types::chrono::Utc};
use validator::Validate;
use crate::{
    errors::AppError, middleware::auth::AuthenticatedUser, models::{
        calendar::{SharedCalendarDeadline, SharedCalendarEvent, SharedCalendarResponse, UserCalendarResponse}, calendar_share::{
            CalendarShare, ListSharesResponseItem, ReceivedShareResponseItem, ShareOwnerDetail // Import new models
        }, deadline::Deadline, enums::{DeadlinePriorityLevel, EventInvitationStatus, SharePrivacyLevel}, event::Event, event_invitation::EventInvitation, user::User // Needed for shared calendar view handler
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

// --- Handler to list calendars shared WITH the authenticated user (GET /api/calendar/shares) ---
// This handler was previously GET /api/me/shared-calendars or similar.
// It lists the 'calendar_shares' records where the authenticated user is shared_with_user_id.
// This doesn't return the calendar items, just the list of shares they have received.
pub async fn list_received_shares(
    State(state): State<AppState>,
    AuthenticatedUser { user_id: shared_with_user_id }: AuthenticatedUser, // The user receiving shares
) -> Result<Json<Vec<crate::models::calendar_share::ListSharesResponseItem>>, AppError> { // Returns the same item structure as listing owner's shares, but filtered differently

    // Fetch shares where the authenticated user is the shared_with_user
     let shares = sqlx::query_as!(
        ListSharesResponseItem, // Use the response struct defined in calendar_share.rs
        r#"
        SELECT
            cs.share_id,
            cs.owner_user_id,
            cs.shared_with_user_id, -- Should match shared_with_user_id = $1
            cs.message as "message!: _",
            cs.privacy_level as "privacy_level!: _",
            cs.expires_at as "expires_at!: _",
            cs.created_at as "created_at!",
            cs.updated_at as "updated_at!",
            cs.deleted_at as "deleted_at!: _",
            -- Owner User Details (aliased - the sharer)
            u.user_id AS user_id_alias, -- Alias matches struct field name
            u.display_name,
            u.email,
            -- Aggregated Category IDs included in the share
            ARRAY_AGG(csc.category_id) FILTER (WHERE csc.category_id IS NOT NULL) AS "shared_category_ids!: Vec<i32>"
        FROM calendar_shares cs
        JOIN users u ON cs.owner_user_id = u.user_id -- JOIN with the owner user
        LEFT JOIN calendar_share_categories csc ON cs.share_id = csc.share_id
        WHERE cs.shared_with_user_id = $1 -- Filter by the shared_with user (authenticated user)
        GROUP BY cs.share_id, u.user_id -- Group required for array_agg
        ORDER BY cs.created_at DESC -- Optional: order by creation date
        "#,
        shared_with_user_id
    )
    .fetch_all(&state.pool)
    .await?;

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
           created_at as "created_at!", updated_at as "updated_at!", deleted_at as "deleted_at!: _"
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
           due_date, virtual_due_date as "virtual_due_date!: _", priority as "priority!: _",
           workload_magnitude as "workload_magnitude!: _", workload_unit as "workload_unit!: _",
           created_at as "created_at!", updated_at as "updated_at!", deleted_at as "deleted_at!: _"
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

// --- Get Shared Calendar Items (GET /api/calendar/shares/:share_id) ---
// Fetches items from a shared calendar for the invitee
pub async fn get_shared_calendar(
    State(state): State<AppState>,
    AuthenticatedUser { user_id: authenticated_user_id }: AuthenticatedUser, // The sharee
    Path(share_id): Path<i32>, // The ID of the specific share instance
    // No Query parameters for ranges in this simplified version
) -> Result<Json<SharedCalendarResponse>, AppError> {

    // 1. Verify the share exists and is intended for the authenticated user (sharee)
    let share = sqlx::query_as!(
        CalendarShare,
        r#"
        SELECT
            share_id, owner_user_id, shared_with_user_id, message as "message!: _",
            privacy_level as "privacy_level!: _", expires_at as "expires_at!: _",
            created_at as "created_at!", updated_at as "updated_at!", deleted_at as "deleted_at!: _"
        FROM calendar_shares
        WHERE share_id = $1 AND shared_with_user_id = $2
        "#,
        share_id,
        authenticated_user_id // Check if the authenticated user is the shared_with_user
    )
    .fetch_optional(&state.pool)
    .await?;

    let share = match share {
        Some(s) => s,
        None => return Err(AppError::ShareNotFound), // Share does not exist or is not shared with this user
    };

    // Check if the share has expired
    if let Some(expires_at) = share.expires_at {
        if Utc::now() > expires_at {
            return Err(AppError::ShareNotFound); // Treat as not found/accessible if expired
        }
    }

    let owner_user_id = share.owner_user_id; // The sharer's ID
    let privacy_level = share.privacy_level;

    // 2. Get the list of categories included in this share
    let shared_category_ids: Vec<i32> = sqlx::query_scalar!(
        "SELECT category_id FROM calendar_share_categories WHERE share_id = $1",
        share_id
    )
    .fetch_all(&state.pool)
    .await?;


    // 3. Fetch Events (owned by sharer AND in shared categories, OR where sharer is accepted invitee)
    let events_query = sqlx::query_as!(
        Event,
        r#"
        SELECT
           event_id, user_id, category_id, title, description as "description!: _",
           start_time, end_time, location as "location!: _", rrule as "rrule!: _",
           created_at as "created_at!: _", updated_at as "updated_at!: _", deleted_at as "deleted_at!: _"
        FROM events e
        WHERE
           ( -- Case 1: Events owned by the sharer included in the share
               e.user_id = $1 -- Sharer's user_id (owner_user_id)
               AND e.category_id = ANY($2) -- Category is in the list of shared categories
           )
           OR
           ( -- Case 2: Events owned by others where the sharer (owner_user_id) is an accepted invitee
               e.user_id != $1 -- Not owned by the sharer
               AND e.event_id IN (
                   SELECT event_id
                   FROM event_invitations
                   WHERE invited_user_id = $1 AND status = $3
               )
           )
        ORDER BY e.start_time
        "#,
        owner_user_id, // $1
        &shared_category_ids, // $2 - Pass Vec<i32> as array
        EventInvitationStatus::Accepted as EventInvitationStatus // $3 - Bind the ENUM
    );

    let mut events = events_query.fetch_all(&state.pool).await?;


    // 4. Fetch Deadlines (owned by the sharer AND in shared categories - assuming deadlines follow category sharing?)
    // UPDATE: Deadlines are only owned by the sharer according to plan, and not inviteable.
    // It seems the intent is to share deadlines based on shared *categories*, same as events.
    // Let's update the query to filter deadlines by categories too.
     let mut deadlines = sqlx::query_as!(
        Deadline,
        r#"
        SELECT
           deadline_id, user_id, category_id, title, description as "description!: _",
           due_date, virtual_due_date as "virtual_due_date!: _", priority as "priority!: _",
           workload_magnitude as "workload_magnitude!: _", workload_unit as "workload_unit!: _",
           created_at as "created_at!", updated_at as "updated_at!", deleted_at as "deleted_at!: _"
        FROM deadlines
        WHERE user_id = $1 -- Only deadlines owned by the sharer
          AND category_id = ANY($2) -- Filter by shared categories
        ORDER BY due_date
        "#,
        owner_user_id, // $1
        &shared_category_ids // $2
    )
    .fetch_all(&state.pool)
    .await?;


    // 5. Apply Privacy Level and convert to shared calendar formats
    let events = if privacy_level == SharePrivacyLevel::BusyOnly {
        // Apply busyOnly transformation to Events
        events.into_iter().map(|event| {
            SharedCalendarEvent {
                event_id: event.event_id,
                owner_user_id: event.user_id, // Map user_id to owner_user_id
                category_id: None, // Clear for privacy
                title: "Busy".to_string(),
                description: None, // Clear for privacy
                start_time: event.start_time,
                end_time: event.end_time,
                location: None, // Clear for privacy
                rrule: None, // Clear for privacy
            }
        }).collect()
    } else {
        // Full detail mode - keep original values but convert to SharedCalendarEvent
        events.into_iter().map(|event| {
            SharedCalendarEvent {
                event_id: event.event_id,
                owner_user_id: event.user_id, // Map user_id to owner_user_id
                category_id: Some(event.category_id), // Keep but convert to Option
                title: event.title,
                description: event.description,
                start_time: event.start_time,
                end_time: event.end_time,
                location: event.location,
                rrule: event.rrule,
            }
        }).collect()
    };

    // Apply similar transformation to Deadlines
    let deadlines = if privacy_level == SharePrivacyLevel::BusyOnly {
        // Apply busyOnly transformation to Deadlines
        deadlines.into_iter().map(|deadline| {
            SharedCalendarDeadline {
                deadline_id: deadline.deadline_id,
                owner_user_id: deadline.user_id, // Map user_id to owner_user_id
                category_id: None, // Clear for privacy
                title: "Deadline".to_string(),
                description: None, // Clear for privacy
                due_date: deadline.due_date,
                priority: Some(DeadlinePriorityLevel::Normal), // Default but as Option
                workload_magnitude: None, // Clear for privacy
                workload_unit: None, // Clear for privacy
            }
        }).collect()
    } else {
        // Full detail mode
        deadlines.into_iter().map(|deadline| {
            SharedCalendarDeadline {
                deadline_id: deadline.deadline_id,
                owner_user_id: deadline.user_id, // Map user_id to owner_user_id
                category_id: Some(deadline.category_id), // Keep but convert to Option
                title: deadline.title,
                description: deadline.description,
                due_date: deadline.due_date,
                priority: Some(deadline.priority), // Keep but convert to Option
                workload_magnitude: deadline.workload_magnitude,
                workload_unit: deadline.workload_unit,
            }
        }).collect()
    };

    // 6. Combine results into the response struct
    let response = SharedCalendarResponse {
        share_id: share.share_id,
        owner_user_id: share.owner_user_id,
        message: share.message,
        privacy_level: share.privacy_level,
        events,
        deadlines,
    };

    Ok(Json(response))
}