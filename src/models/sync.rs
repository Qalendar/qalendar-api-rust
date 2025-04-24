use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use crate::models::{
    category::Category,
    deadline::Deadline,
    event::Event,
    event_invitation::{EventInvitation, MyInvitationResponseItem}, // Use MyInvitationResponseItem for detail
    calendar_share::{CalendarShare, ShareDetailsResponse, ListSharesResponseItem}, // Use ListSharesResponseItem
    // Add other models if needed later
};

use super::calendar::{SharedCalendarDeadline, SharedCalendarEvent};

// Payload for GET /api/me/sync?since=...
#[derive(Deserialize, Debug)]
pub struct SyncSinceParams {
    // Optional: Use String and parse, or directly deserialize if format is consistent
    pub since: Option<String>, // ISO 8601 timestamp string
}

// Response for GET /api/me/sync
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncResponse {
    pub categories: Vec<Category>,
    pub deadlines: Vec<Deadline>,
    pub events: Vec<Event>, // Includes owned and accepted invites (updated since `since`)
    // Split invitations for clarity: those received by me, and those I created for my events?
    // Let's just return updates to invitations I received for now.
    // Frontend can use this + event list to figure out status for events I own.
    pub received_invitations: Vec<EventInvitation>, // Just the base invitation updates
    pub shares_created: Vec<ListSharesResponseItem>, // Shares created by me
    pub shares_received: Vec<ListSharesResponseItem>, // Shares received by me (uses same response item struct)

    // Optional: Explicit list of deleted item IDs?
    // pub deleted_items: DeletedItems,

    // Timestamp of this sync operation on the server
    pub sync_timestamp: DateTime<Utc>,
}

// Response for GET /api/sync/calendar/shares/:share_id
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncSharedCalendarResponse {
    // Share details (updated since 'since', or current if share is new since 'since')
    // Include deleted_at here so client knows if share was revoked
    pub share_info: Option<CalendarShare>, // Option in case the share was deleted since last sync

    // Events relevant to the share (updated since 'since', filtered by categories/privacy)
    pub events: Vec<SharedCalendarEvent>,

    // Deadlines relevant to the share (updated since 'since', filtered by categories/privacy)
    pub deadlines: Vec<SharedCalendarDeadline>,

    // Timestamp of this sync operation on the server
    pub sync_timestamp: DateTime<Utc>,
}

// Optional structure for deleted items if not relying solely on deleted_at
// #[derive(Debug, Serialize, Default)]
// #[serde(rename_all = "camelCase")]
// pub struct DeletedItems {
//     pub category_ids: Vec<i32>,
//     pub deadline_ids: Vec<i32>,
//     pub event_ids: Vec<i32>,
//     pub invitation_ids: Vec<i32>,
//     pub share_ids: Vec<i32>,
// }