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

// Optional: Payload for listing invitations for an event (owner)
// #[derive(Deserialize, Validate, Debug)]
// #[serde(rename_all = "camelCase")]
// pub struct ListEventInvitationsParams {
//    // Could add query parameters here if needed, like filtering by status
// }

// Optional: Payload for listing *my* invitations (invitee)
#[derive(Deserialize, Validate, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ListMyInvitationsParams {
   // Allow filtering by status, e.g., GET /api/me/invitations?status=pending
   pub status: Option<EventInvitationStatus>,
}