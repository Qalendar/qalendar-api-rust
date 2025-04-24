use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError};
use chrono::{DateTime, Utc};
use sqlx::FromRow;

// Import enums from the centralized module
use super::enums::SharePrivacyLevel;
use super::category::Category; // Might be useful for response types
use super::user::User; // To include shared_with user details


// --- Database Model (matches calendar_shares table) ---
#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarShare {
    pub share_id: i32,
    pub owner_user_id: i32,
    pub shared_with_user_id: i32,
    pub message: Option<String>, // Can be NULL
    pub privacy_level: SharePrivacyLevel, // Use imported ENUM
    pub expires_at: Option<DateTime<Utc>>, // Can be NULL
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}


// --- API Payloads ---

#[derive(Deserialize, Validate, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CreateSharePayload {
    #[validate(required, email)]
    pub shared_with_user_email: Option<String>, // Invite by email

    // List of category IDs to share
    #[validate(required, length(min = 1))] // Must provide at least one category
    pub category_ids: Option<Vec<i32>>,

    #[validate(length(max = 1000))] // Optional message
    pub message: Option<String>,

    // Privacy level is optional, defaults in DB or handler
    pub privacy_level: Option<SharePrivacyLevel>,

    // Expiry date is optional
    // String in payload, parse in handler
    pub expires_at: Option<String>,
}

#[derive(Deserialize, Validate, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSharePayload {
    // shared_with_user_email cannot be changed after creation

    // Allow updating the list of category IDs
    pub category_ids: Option<Vec<i32>>, // Can be an empty vector to unshare all categories

    #[validate(length(max = 1000))] // Allow updating message
    pub message: Option<String>,

    // Allow updating privacy level
    pub privacy_level: Option<SharePrivacyLevel>,

    // Allow updating or removing expiry date (set to null in JSON)
    pub expires_at: Option<String>,
}


// --- API Response Structures (for GET requests) ---

// Keep this as a conversion target for API responses, serialization, or documentation if needed
#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SharedWithUserDetail {
    #[serde(rename = "userId")] // Match frontend expectation
    pub user_id_alias: i32, // Alias from SQL query
    #[serde(rename = "displayName")]
    pub display_name: String,
    pub email: String,
}


// Response struct for GET /api/me/shares/{share_id}
#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareDetailsResponse {
    // Fields from CalendarShare
    pub share_id: i32,
    pub owner_user_id: i32,
    pub shared_with_user_id: i32, // Direct field from query
    pub message: Option<String>,
    pub privacy_level: SharePrivacyLevel,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,

    // Direct user fields from query
    #[serde(rename = "userId")] 
    pub user_id_alias: i32,
    pub display_name: String,
    pub email: String,
    
    // Category IDs
    pub shared_category_ids: Vec<i32>,
}

// Response struct for GET /api/me/shares (list all shares)
// Similar to ShareDetailsResponse, but maybe slightly less detail or just use the same struct
// Let's re-use ShareDetailsResponse for simplicity, assuming the query returns the same structure.
pub type ListSharesResponseItem = ShareDetailsResponse;

// Keep this as a conversion target for API responses, serialization, or documentation if needed
#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareOwnerDetail {
    #[serde(rename = "userId")]
    pub user_id_alias: i32, // Alias from SQL query
    #[serde(rename = "displayName")]
    pub display_name: String,
    pub email: String,
}


// Response struct for GET /api/shared-calendars (list calendars shared WITH me)
#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceivedShareResponseItem {
    // Fields from CalendarShare
    pub share_id: i32,
    pub owner_user_id: i32, // The ID of the user who shared it
    pub shared_with_user_id: i32, // Should match the authenticated user's ID
    pub message: Option<String>,
    pub privacy_level: SharePrivacyLevel,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>, // Update time for the share *settings*

    // // Joined ShareOwner details (aliased in query)
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