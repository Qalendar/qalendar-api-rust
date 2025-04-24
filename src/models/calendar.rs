use serde::{Serialize, Deserialize}; // Need Deserialize for testing potentially
use chrono::{DateTime, Utc};
use crate::models::enums::{DeadlinePriorityLevel, WorkloadUnitType, SharePrivacyLevel}; // Import enums
use crate::models::event::Event; // Import base Event structure
use crate::models::deadline::Deadline; // Import base Deadline structure


// --- Response struct for GET /api/calendar ---
// (This remains as is)
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserCalendarResponse {
    pub events: Vec<Event>,
    pub deadlines: Vec<Deadline>,
}


// --- Response structs for GET /api/calendar/shares/{share_id} ---

// Represents an event in a shared calendar, applying privacy
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SharedCalendarEvent {
    pub event_id: i32,
    pub owner_user_id: i32, // The ID of the event owner (sharer)

    // These fields are optional or modified based on privacy
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category_id: Option<i32>,
    pub title: String, // Can be "Busy" in busy_only mode
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>, // Will be None in busy_only mode

    pub start_time: DateTime<Utc>, // Always included
    pub end_time: DateTime<Utc>,   // Always included

    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>, // Will be None in busy_only mode

    #[serde(skip_serializing_if = "Option::is_none")]
    pub rrule: Option<String>, // Will be None in busy_only mode

    // Timestamps - maybe exclude in busy_only or keep? Let's keep for sync purposes
    // pub created_at: DateTime<Utc>, // Might omit
    // pub updated_at: DateTime<Utc>, // Might omit
}

// Represents a deadline in a shared calendar, applying privacy
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SharedCalendarDeadline {
    pub deadline_id: i32,
    pub owner_user_id: i32, // The ID of the deadline owner (sharer)

    // These fields are optional or modified based on privacy
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category_id: Option<i32>,
    pub title: String, // Can be "Busy" in busy_only mode
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>, // Will be None in busy_only mode

    pub due_date: DateTime<Utc>, // Always included

    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<DeadlinePriorityLevel>, // Will be None in busy_only mode
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workload_magnitude: Option<i32>, // Will be None in busy_only mode
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workload_unit: Option<WorkloadUnitType>, // Will be None in busy_only mode

    // Timestamps
    // pub created_at: DateTime<Utc>, // Might omit
    // pub updated_at: DateTime<Utc>, // Might omit
}

// Overall response struct for GET /api/calendar/shares/{share_id}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SharedCalendarResponse {
    pub share_id: i32,
    pub owner_user_id: i32, // The user who owns this calendar
    pub message: Option<String>, // Message from the share
    pub privacy_level: SharePrivacyLevel, // Show the sharee what level they have

    pub events: Vec<SharedCalendarEvent>,
    pub deadlines: Vec<SharedCalendarDeadline>,
    // Could also include shared categories list here if useful
    // pub shared_category_ids: Vec<i32>,
}