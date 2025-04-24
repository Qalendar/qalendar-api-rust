use axum::{
    extract::{State, Query, Path, Json},
    http::StatusCode,
};
use sqlx::{PgPool, types::chrono::Utc};
use validator::Validate;
use crate::{
    AppState,
    errors::AppError,
    models::{
        calendar_share::{
            ReceivedShareResponseItem, ShareOwnerDetail // Import new models
        },
        enums::SharePrivacyLevel,
        // Will need models for events, deadlines, event_exceptions, etc. later
        event::Event,
        deadline::Deadline,
        event_invitation::EventInvitation,
        user::User, // Needed for shared calendar view handler
    },
    middleware::auth::AuthenticatedUser,
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