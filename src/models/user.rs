use serde::{Deserialize, Serialize};
use validator::Validate; // Make sure validator is imported
use chrono::{DateTime, Utc, NaiveDate};
use sqlx::FromRow; // Needed for sqlx to map database rows

// --- API Payloads ---

#[derive(Deserialize, Validate, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RegisterUserPayload {
    #[validate(required, length(min = 1, max = 100))]
    pub display_name: Option<String>, // Use Option for required validation message

    #[validate(required, email)]
    pub email: Option<String>,

    #[validate(required, length(min = 8))]
    pub password: Option<String>,

    // Optional: Add custom date validation if needed
    // Format will be checked during parsing later
    pub dob: Option<String>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Deserialize, Validate, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LoginUserPayload {
    #[validate(required, email)]
    pub email: Option<String>,
    #[validate(required)]
    pub password: Option<String>,
}

// --- API Responses ---

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UserData {
    #[serde(rename = "userId")] // Match frontend expectation
    pub user_id: i32,
    #[serde(rename = "displayName")]
    pub display_name: String,
    pub email: String,
    #[serde(rename = "emailVerified")]
    pub email_verified: bool,
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
    // Optionally include dob
    #[serde(rename = "dateOfBirth", skip_serializing_if = "Option::is_none")]
    pub date_of_birth: Option<NaiveDate>,
}

#[derive(Serialize, Debug)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserData,
}


// --- Database Model ---

// Represents the structure in the 'users' table
#[derive(FromRow, Debug, Serialize)] // Automatically map rows to this struct
#[serde(rename_all = "camelCase")]
pub struct User {
    pub user_id: i32,
    pub display_name: String,
    pub email: String,
    pub password_hash: String,
    pub date_of_birth: Option<NaiveDate>, // Matches DATE type in Postgres
    pub email_verified: bool,
    // Ignore verification/reset codes for now, add later if needed for Phase 3
    // pub verification_code: Option<String>,
    // pub verification_code_expires_at: Option<DateTime<Utc>>,
    // pub reset_code: Option<String>,
    // pub reset_code_expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>, // Matches TIMESTAMP WITH TIME ZONE
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}