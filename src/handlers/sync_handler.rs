// src/handlers/sync_handler.rs
use axum::{
    extract::{Path, Query, State},
    Json,
};
use sqlx::PgPool;
use crate::{
    errors::AppError, middleware::auth::AuthenticatedUser, models::{
        calendar::{SharedCalendarDeadline, SharedCalendarEvent}, calendar_share::{CalendarShare, ListSharesResponseItem}, category::Category, deadline::Deadline, enums::*, event::Event, event_invitation::{EventInvitation, MyInvitationResponseItem}, sync::{SyncResponse, SyncSharedCalendarResponse, SyncSinceParams} // Import all enums
    }, AppState
};
use chrono::{DateTime, Utc, TimeZone}; // Import Utc, TimeZone

use crate::utils::calendar::parse_optional_timestamp; // Utility function for parsing timestamps


// --- GET /api/me/sync handler ---
pub async fn sync_owned_data(
    State(state): State<AppState>,
    AuthenticatedUser { user_id: authenticated_user_id }: AuthenticatedUser,
    Query(params): Query<SyncSinceParams>, // Extract 'since' query param
) -> Result<Json<SyncResponse>, AppError> {

    let since_timestamp = parse_optional_timestamp(params.since)?;
    let now = Utc::now(); // Timestamp for this sync operation

    // --- Fetch Categories ---
    let categories = sqlx::query_as!(
        Category,
        r#"
        SELECT category_id, user_id, name, color, is_visible as "is_visible!: _",
        created_at as "created_at!", updated_at as "updated_at!", deleted_at as "deleted_at!: _"
        FROM categories
        WHERE user_id = $1 AND ( ($2::TIMESTAMPTZ IS NULL) OR (updated_at > $2) )
        "#,
        authenticated_user_id,
        since_timestamp, // Bind Option<DateTime<Utc>>
    )
    .fetch_all(&state.pool)
    .await?;

    // --- Fetch Deadlines ---
    let deadlines = sqlx::query_as!(
        Deadline,
        r#"
        SELECT
           deadline_id, user_id, category_id, title, description as "description!: _",
           due_date as "due_date!", virtual_due_date as "virtual_due_date!: _",
           priority as "priority!: _",
           workload_magnitude as "workload_magnitude!: _", workload_unit as "workload_unit!: _",
           created_at as "created_at!", updated_at as "updated_at!", deleted_at as "deleted_at!: _"
        FROM deadlines
        WHERE user_id = $1 AND ( ($2::TIMESTAMPTZ IS NULL) OR (updated_at > $2) )
        "#,
        authenticated_user_id,
        since_timestamp,
    )
    .fetch_all(&state.pool)
    .await?;

    // --- Fetch Events (Owned OR Accepted Invite, Updated Since) ---
    // This needs to fetch the base event record if it or its relevant invitation status changed.
    let events = sqlx::query_as!(
        Event,
        r#"
        WITH RelevantEvents AS (
            -- Owned Events Updated Since
            SELECT event_id FROM events
            WHERE user_id = $1 AND ( ($2::TIMESTAMPTZ IS NULL) OR (updated_at > $2) )
            UNION
            -- Events Where I am Accepted Invitee AND Invitation Status Changed Since
            SELECT event_id FROM event_invitations
            WHERE invited_user_id = $1 AND status = $3 AND ( ($2::TIMESTAMPTZ IS NULL) OR (updated_at > $2) )
            UNION
            -- Events Where I am Accepted Invitee AND Event Itself Changed Since
            SELECT ei.event_id FROM event_invitations ei
            JOIN events e ON ei.event_id = e.event_id
            WHERE ei.invited_user_id = $1 AND ei.status = $3 AND ( ($2::TIMESTAMPTZ IS NULL) OR (e.updated_at > $2) )
        )
        SELECT
           e.event_id, e.user_id, e.category_id, e.title, e.description as "description!: _",
           e.start_time as "start_time!", e.end_time as "end_time!",
           e.location as "location!: _", e.rrule as "rrule!: _",
           e.created_at as "created_at!", e.updated_at as "updated_at!", e.deleted_at as "deleted_at!: _"
        FROM events e
        JOIN RelevantEvents re ON e.event_id = re.event_id
        ORDER BY e.start_time
        "#,
        authenticated_user_id, // $1
        since_timestamp,       // $2
        EventInvitationStatus::Accepted as EventInvitationStatus // $3
    )
    .fetch_all(&state.pool)
    .await?;


    // --- Fetch Received Invitations (Updates since 'since') ---
    let received_invitations = sqlx::query_as!(
        EventInvitation,
        r#"
        SELECT invitation_id, event_id, owner_user_id, invited_user_id, status as "status!: _",
        created_at as "created_at!", updated_at as "updated_at!", deleted_at as "deleted_at!: _"
        FROM event_invitations
        WHERE invited_user_id = $1 AND ( ($2::TIMESTAMPTZ IS NULL) OR (updated_at > $2) )
        "#,
        authenticated_user_id,
        since_timestamp,
    )
    .fetch_all(&state.pool)
    .await?;


    // --- Fetch Shares Created By Me (Updates since 'since') ---
     let shares_created = sqlx::query_as!(
        ListSharesResponseItem, // Use the detailed response struct
        r#"
        SELECT
            cs.share_id, cs.owner_user_id, cs.shared_with_user_id, cs.message,
            cs.privacy_level as "privacy_level!: _", cs.expires_at as "expires_at!: _",
            cs.created_at as "created_at!", cs.updated_at as "updated_at!", cs.deleted_at as "deleted_at!: _",
            u.user_id AS user_id_alias, u.display_name, u.email,
            ARRAY_AGG(csc.category_id) FILTER (WHERE csc.category_id IS NOT NULL) AS "shared_category_ids!: Vec<i32>"
        FROM calendar_shares cs
        JOIN users u ON cs.shared_with_user_id = u.user_id
        LEFT JOIN calendar_share_categories csc ON cs.share_id = csc.share_id
        WHERE cs.owner_user_id = $1 AND ( ($2::TIMESTAMPTZ IS NULL) OR (cs.updated_at > $2) )
        GROUP BY cs.share_id, u.user_id
        ORDER BY cs.created_at DESC
        "#,
        authenticated_user_id, // $1
        since_timestamp // $2
    )
    .fetch_all(&state.pool)
    .await?;


    // --- Fetch Shares Received By Me (Updates since 'since') ---
     let shares_received = sqlx::query_as!(
        ListSharesResponseItem, // Use the detailed response struct
        r#"
        SELECT
            cs.share_id, cs.owner_user_id, cs.shared_with_user_id, cs.message,
            cs.privacy_level as "privacy_level!: _", cs.expires_at as "expires_at!: _",
            cs.created_at as "created_at!", cs.updated_at as "updated_at!", cs.deleted_at as "deleted_at!: _",
            u.user_id AS user_id_alias, u.display_name, u.email,
            ARRAY_AGG(csc.category_id) FILTER (WHERE csc.category_id IS NOT NULL) AS "shared_category_ids!: Vec<i32>"
        FROM calendar_shares cs
        JOIN users u ON cs.owner_user_id = u.user_id -- Join with OWNER this time
        LEFT JOIN calendar_share_categories csc ON cs.share_id = csc.share_id
        WHERE cs.shared_with_user_id = $1 AND ( ($2::TIMESTAMPTZ IS NULL) OR (cs.updated_at > $2) )
        GROUP BY cs.share_id, u.user_id
        ORDER BY cs.created_at DESC
        "#,
        authenticated_user_id, // $1
        since_timestamp // $2
    )
    .fetch_all(&state.pool)
    .await?;

    // --- Combine into Response ---
    let response = SyncResponse {
        categories,
        deadlines,
        events,
        received_invitations,
        shares_created,
        shares_received,
        sync_timestamp: now,
    };

    Ok(Json(response))
}

// --- GET /api/sync/calendar/shares/:share_id handler ---
// Fetches UPDATES for a specific shared calendar view since a given timestamp
pub async fn sync_shared_calendar_data(
    State(state): State<AppState>,
    AuthenticatedUser { user_id: authenticated_user_id }: AuthenticatedUser, // The sharee
    Path(share_id): Path<i32>, // The ID of the specific share instance
    Query(params): Query<SyncSinceParams>, // Extract 'since' query param
) -> Result<Json<SyncSharedCalendarResponse>, AppError> {

    let since_timestamp = parse_optional_timestamp(params.since)?;
    let now = Utc::now();

    // 1. Fetch the CURRENT share details (even if deleted) to check access and get info
    // We fetch regardless of deleted_at here to know if access *was* possible.
    let share_info = sqlx::query_as!(
        CalendarShare,
        r#"
        SELECT share_id, owner_user_id, shared_with_user_id, message as "message!: _",
        privacy_level as "privacy_level!: _",
        expires_at as "expires_at!: _", created_at as "created_at!", updated_at as "updated_at!", deleted_at as "deleted_at!: _"
        FROM calendar_shares
        WHERE share_id = $1 AND shared_with_user_id = $2
        "#,
        share_id,
        authenticated_user_id
    )
    .fetch_optional(&state.pool)
    .await?;

    // If share doesn't exist or isn't for this user, return empty sync result (or 404?)
    // Let's return an empty result, indicating no updates or access removed.
    let share = match share_info {
        Some(s) => s,
        None => {
            return Ok(Json(SyncSharedCalendarResponse {
                share_info: None, // Indicate share not found/accessible
                events: vec![],
                deadlines: vec![],
                sync_timestamp: now,
            }));
        }
    };

    // Check if expired BEFORE the 'since' timestamp (if since exists). If so, no updates needed.
    // If it expired AFTER 'since', we still need to fetch the share_info update (deleted_at).
    if let Some(expires_at) = share.expires_at {
         if let Some(since) = since_timestamp {
              if expires_at <= since {
                   // Expired before last sync, check if share itself was updated (e.g., deleted)
                    return sync_just_share_update(state, share_id, authenticated_user_id, since_timestamp, now).await;
              }
         } else {
              // No 'since' timestamp, check if expired now
              if Utc::now() > expires_at {
                   // Expired now, check if share itself was updated recently
                   return sync_just_share_update(state, share_id, authenticated_user_id, since_timestamp, now).await;
              }
         }
    }

     // If the share was deleted AFTER the 'since' timestamp, we only need to return the share_info
     if let Some(deleted_at) = share.deleted_at {
          if let Some(since) = since_timestamp {
               if deleted_at > since {
                    return Ok(Json(SyncSharedCalendarResponse {
                         share_info: Some(share), // Return share with deleted_at set
                         events: vec![],
                         deadlines: vec![],
                         sync_timestamp: now,
                    }));
               } else {
                    // Deleted before last sync, return empty
                     return Ok(Json(SyncSharedCalendarResponse {
                         share_info: None, // Indicate no longer accessible
                         events: vec![],
                         deadlines: vec![],
                         sync_timestamp: now,
                     }));
               }
          } else {
               // No 'since', but share is deleted, return empty
                return Ok(Json(SyncSharedCalendarResponse {
                     share_info: None, // Indicate no longer accessible
                     events: vec![],
                     deadlines: vec![],
                     sync_timestamp: now,
                 }));
          }
     }

    // --- If share is active and accessible ---

    let owner_user_id = share.owner_user_id;
    let privacy_level = share.privacy_level;

    // 2. Get the list of categories currently included in this share (needed for filtering)
    // No need to check 'since' for categories, just get the current list for filtering items.
    let shared_category_ids: Vec<i32> = sqlx::query_scalar!(
        "SELECT category_id FROM calendar_share_categories WHERE share_id = $1",
        share_id
    )
    .fetch_all(&state.pool)
    .await?;


    // 3. Fetch Events (owned by sharer in shared categories OR sharer is accepted invitee)
    //    AND updated since 'since'
    let events_query = sqlx::query_as!(
        Event,
        r#"
        SELECT
           e.event_id, e.user_id, e.category_id, e.title, e.description as "description!: _",
           e.start_time as "start_time!", e.end_time as "end_time!",
           e.location as "location!: _", e.rrule as "rrule!: _",
           e.created_at as "created_at!", e.updated_at as "updated_at!", e.deleted_at as "deleted_at!: _"
        FROM events e
        WHERE
           ( ($3::TIMESTAMPTZ IS NULL) OR (e.updated_at > $3) ) -- Filter by event update time
           AND
           (
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
                       WHERE invited_user_id = $1 AND status = $4 -- Sharer is accepted invitee
                       -- No need to check invitation updated_at here, only event updated_at matters for showing the event
                   )
               )
           )
        ORDER BY e.start_time
        "#,
        owner_user_id, // $1
        &shared_category_ids, // $2
        since_timestamp, // $3 - Bind Option<DateTime<Utc>>
        EventInvitationStatus::Accepted as EventInvitationStatus // $4
    );

    let mut events = events_query.fetch_all(&state.pool).await?;


    // 4. Fetch Deadlines (owned by sharer, in shared categories, updated since 'since')
    let mut deadlines = sqlx::query_as!(
        Deadline,
        r#"
        SELECT
           deadline_id, user_id, category_id, title, description as "description!: _",
           due_date as "due_date!", virtual_due_date as "virtual_due_date!: _",
           priority as "priority!: _",
           workload_magnitude as "workload_magnitude!: _", workload_unit as "workload_unit!: _",
           created_at as "created_at!", updated_at as "updated_at!", deleted_at as "deleted_at!: _"
        FROM deadlines
        WHERE user_id = $1 -- Only deadlines owned by the sharer
          AND category_id = ANY($2) -- Filter by shared categories
          AND ( ($3::TIMESTAMPTZ IS NULL) OR (updated_at > $3) ) -- Filter by deadline update time
        ORDER BY due_date
        "#,
        owner_user_id, // $1
        &shared_category_ids, // $2
        since_timestamp, // $3
    )
    .fetch_all(&state.pool)
    .await?;


    // 5. Apply Privacy Level (apply BEFORE returning)
    let events: Vec<SharedCalendarEvent> = if privacy_level == SharePrivacyLevel::Limited {
        // Apply Limited transformation to Events
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
    let deadlines: Vec<SharedCalendarDeadline> = if privacy_level == SharePrivacyLevel::Limited {
        // Apply Limited transformation to Deadlines
        deadlines.into_iter().map(|deadline| {
            SharedCalendarDeadline {
                deadline_id: deadline.deadline_id,
                owner_user_id: deadline.user_id, // Map user_id to owner_user_id
                category_id: None, // Clear for privacy
                title: "Deadline".to_string(),
                description: None, // Clear for privacy
                due_date: deadline.due_date,
                priority: None, // Clear for privacy
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

    // 6. Check if the share configuration ITSELF was updated since 'since'
    // If items haven't updated but the share config (e.g., privacy) did, we still need to include share_info
    let share_updated_since = if let Some(since) = since_timestamp {
        share.updated_at > since
    } else {
        true // No 'since', always include current share info
    };

    // Only include share_info if it was updated or if there are updated items
     let final_share_info = if share_updated_since || !events.is_empty() || !deadlines.is_empty() {
         Some(share)
     } else {
         None
     };


    // 7. Combine results into the response struct
    let response = SyncSharedCalendarResponse {
        share_info: final_share_info, // Send current share info only if relevant update occurred
        events,
        deadlines,
        sync_timestamp: now,
    };

    Ok(Json(response))
}


// --- Helper function for sync_shared_calendar_data edge cases ---
async fn sync_just_share_update(
    state: AppState,
    share_id: i32,
    authenticated_user_id: i32,
    since_timestamp: Option<DateTime<Utc>>,
    now: DateTime<Utc>
) -> Result<Json<SyncSharedCalendarResponse>, AppError> {
     // Check if the share record itself was updated (e.g., deleted) since 'since'
     let share_info = sqlx::query_as!(
        CalendarShare,
        r#"
        SELECT share_id, owner_user_id, shared_with_user_id, message as "message!: _", privacy_level as "privacy_level!: _",
        expires_at as "expires_at!: _", created_at as "created_at!", updated_at as "updated_at!", deleted_at as "deleted_at!: _"
        FROM calendar_shares
        WHERE share_id = $1 AND shared_with_user_id = $2
          AND ( ($3::TIMESTAMPTZ IS NULL) OR (updated_at > $3) ) -- Check if share record updated
        "#,
        share_id,
        authenticated_user_id,
        since_timestamp
    )
    .fetch_optional(&state.pool)
    .await?;

     Ok(Json(SyncSharedCalendarResponse {
         share_info, // Send share info only if it was updated since last sync
         events: vec![],
         deadlines: vec![],
         sync_timestamp: now,
    }))
}