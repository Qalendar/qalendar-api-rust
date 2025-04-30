use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError};
use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

// Import enums and other models
use crate::models::enums::SharePrivacyLevel;
use crate::models::user::User; // To include owner user details


// --- Database Model (matches open_calendar_shares table) ---
#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenCalendarShare {
    pub open_share_id: Uuid, // Use Uuid type
    pub owner_user_id: i32,
    pub privacy_level: SharePrivacyLevel,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}


// --- API Payloads ---

#[derive(Deserialize, Validate, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CreateOpenSharePayload {
    // List of category IDs to share
    #[validate(required, length(min = 1))] // Must provide at least one category
    pub category_ids: Option<Vec<i32>>,

    // Privacy level is optional, defaults in DB or handler
    pub privacy_level: Option<SharePrivacyLevel>,

    // Expiry date is optional
    // String in payload, parse in handler
    pub expires_at: Option<String>,
}

#[derive(Deserialize, Validate, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UpdateOpenSharePayload {
    // Allow updating the list of category IDs
    pub category_ids: Option<Vec<i32>>, // Can be an empty vector to unshare all categories

    // Allow updating privacy level
    pub privacy_level: Option<SharePrivacyLevel>,

    // Allow updating or removing expiry date (set to null in JSON)
    pub expires_at: Option<String>,
}


// --- API Response Structures (for GET requests) ---

// Helper struct for details about the owner user
#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareOwnerDetail {
    #[serde(rename = "userId")] // Match frontend expectation
    pub user_id_alias: i32, // Alias from SQL query
    #[serde(rename = "displayName")]
    pub display_name: String,
    pub email: String, // Maybe don't include email for public shares? Or include only for owner list? Let's include for owner list.
    pub deleted_at: Option<DateTime<Utc>>, // Is the owner user soft-deleted?
}


// Response struct for GET /api/me/open-shares/{uuid} or GET /api/me/open-shares
#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenShareDetailsResponse {
    pub open_share_id: Uuid, // Use Uuid
    pub owner_user_id: i32,

    pub privacy_level: SharePrivacyLevel,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>, // Is the share itself soft-deleted?

    // // Direct owner user fields from query
    // #[serde(flatten)] // Embed these fields directly
    // pub owner_user: ShareOwnerDetail,

    // Direct user fields from query
    #[serde(rename = "userId")]
    pub user_id_alias: i32,
    pub display_name: String,
    pub email: String,

    // List of category IDs included in the share (aggregated in query)
    // Use Option<Vec<i32>> to gracefully handle potential NULL from ARRAY_AGG
    // #[sqlx(json)] // Tell sqlx how to handle the array_agg result (as JSON array string)
    pub shared_category_ids: Option<Vec<i32>>,
}

// List response item reuses the detail struct
pub type ListOpenSharesResponseItem = OpenShareDetailsResponse;