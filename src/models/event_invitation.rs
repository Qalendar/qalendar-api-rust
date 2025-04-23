use serde::{Deserialize, Serialize};
use validator::Validate;
use chrono::{DateTime, Utc};
use sqlx::FromRow;

use crate::models::enums::EventInvitationStatus;

// --- Database Model ---

#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventInvitation {
    pub invitation_id: i32,
    pub event_id: i32,
    pub owner_user_id: i32, // Owner of the event
    pub invited_user_id: i32, // The user who received the invitation
    pub status: EventInvitationStatus, // Use imported ENUM
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // Optional: Could include details of the event itself here if needed often
    // but joining tables in queries is usually better.
}


// --- API Payloads ---

// For inviting a user to an event (owner action)
#[derive(Deserialize, Validate, Debug)]
#[serde(rename_all = "camelCase")]
pub struct InviteUserPayload {
    #[validate(required, email)]
    pub invited_user_email: Option<String>,
}

// For responding to an invitation (invitee action)
#[derive(Deserialize, Validate, Debug)]
#[serde(rename_all = "camelCase")]
pub struct InvitationResponsePayload {
    #[validate(required)]
    pub status: Option<EventInvitationStatus>, // Expect the new status
}

// Payload for listing *my* invitations (invitee) - now includes filtering
#[derive(Deserialize, Validate, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ListMyInvitationsParams {
   // Allow filtering by status using query parameters (e.g., ?status=pending)
   // validator allows deserialization of Option<ENUM> from query string if FromStr is implemented on the ENUM
   pub status: Option<EventInvitationStatus>,
}

// Payload for listing invitations for an event (owner) - now includes filtering
#[derive(Deserialize, Validate, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ListEventInvitationsParams {
   // Allow filtering by status, e.g., ?status=accepted
   pub status: Option<EventInvitationStatus>,
}


// --- API Response Structures (NEW) ---
// These structs combine data from event_invitations and joined tables

// Response item for GET /api/me/invitations (list my invitations)
// Includes invitation details AND relevant event details
#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MyInvitationResponseItem {
    // Fields from EventInvitation
    pub invitation_id: i32,
    pub owner_user_id: i32, // Who owns the event
    pub invited_user_id: i32, // Should be the authenticated user's ID
    pub status: EventInvitationStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,

    // Fields from the joined Event directly included in this struct
    pub event_id: i32,
    pub user_id: i32, // Owner user_id (redundant with owner_user_id above, but matches Event struct)
    pub category_id: Option<i32>,
    pub title: String,
    pub description: Option<String>,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub location: Option<String>,
    pub rrule: Option<String>,
    // Use aliases for timestamps to avoid clash with invitation timestamps
    pub event_created_at: DateTime<Utc>,
    pub event_updated_at: DateTime<Utc>,
}

// Keep this as a conversion target for API responses, serialization, or documentation if needed
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventDetailsForInvitation {
    pub event_id: i32,
    pub user_id: i32,
    pub category_id: Option<i32>,
    pub title: String,
    pub description: Option<String>,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub location: Option<String>,
    pub rrule: Option<String>,
    pub event_created_at: DateTime<Utc>,
    pub event_updated_at: DateTime<Utc>,
}


// Response item for GET /api/me/events/:event_id/invitations (list invitations for my event)
// Includes invitation details AND relevant invited user details
#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventInvitationResponseItem {
    // Fields from EventInvitation
    pub invitation_id: i32,
    pub event_id: i32,
    pub owner_user_id: i32, // Should be the authenticated user's ID
    pub status: EventInvitationStatus,
    pub created_at: DateTime<Utc>, // Invitation created_at
    pub updated_at: DateTime<Utc>, // Invitation updated_at

    // Fields from the joined User (the invited user) - flattened
    pub invited_user_id: i32, // The invited user's ID
    pub invited_user_display_name: String,
    pub invited_user_email: String,
}

// Keep this as a conversion target for API responses, serialization, or documentation if needed
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InvitedUserDetails {
    pub user_id: i32, // The invited user's ID
    pub display_name: String,
    pub email: String,
    // Don't include password_hash or sensitive info
    // Maybe include date_of_birth if relevant?
    // pub date_of_birth: Option<NaiveDate>,
    // Maybe include their email_verified status?
    // pub email_verified: bool,
    // Use aliases for timestamps to avoid clash with invitation timestamps
    // pub user_created_at: DateTime<Utc>,
    // pub user_updated_at: DateTime<Utc>,
}