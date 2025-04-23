use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError};
use chrono::{DateTime, Utc};
use sqlx::FromRow;

// --- Database Model ---

#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub event_id: i32,
    pub user_id: i32, // The owner of the event
    pub category_id: i32, // Optional link to category
    pub title: String,
    pub description: Option<String>, // Allow NULL in DB
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub location: Option<String>, // Allow NULL in DB
    pub rrule: Option<String>, // Store RRULE string, nullable
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// --- API Payloads ---

#[derive(Deserialize, Validate, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CreateEventPayload {
    #[validate(required, length(min = 1, max = 255))]
    pub title: Option<String>,

    #[validate(required)] // Category ID is required for creating an event
    pub category_id: Option<i32>,

    #[validate(length(max = 1000))] // Optional max length validation
    pub description: Option<String>,

    #[validate(required)] // Start time is required for any event
    pub start_time: Option<String>, // String in payload, parse in handler

    #[validate(required)] // End time is required for any event
    // Note: For recurring, this defines duration from start_time
    pub end_time: Option<String>, // String in payload, parse in handler

    #[validate(length(max = 255))] // Optional location
    pub location: Option<String>,

    // RRULE is optional (makes it a recurring event if present)
    // Could add custom validation for RRULE format if desired (complex!)
    pub rrule: Option<String>,
}

#[derive(Deserialize, Validate, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UpdateEventPayload {
    #[validate(length(min = 1, max = 255))] // Allow updating title
    pub title: Option<String>,

    pub category_id: Option<i32>, // Allow updating category

    #[validate(length(max = 1000))]
    pub description: Option<String>,

    // Allow updating start/end time - if one is updated, the other often should be too?
    // Or do we allow just changing duration? Let's make both optional but recommend providing both if changing timing.
    // No specific validation rule here beyond basic parsing for now.
    pub start_time: Option<String>,
    pub end_time: Option<String>,

    #[validate(length(max = 255))]
    pub location: Option<String>,

    // Allow updating or removing RRULE
    pub rrule: Option<String>, // Allow setting to null/empty string to make non-recurring
}
