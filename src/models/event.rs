use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError};
use chrono::{DateTime, Utc, TimeZone};
use sqlx::FromRow;
use sqlx::types::chrono::Utc as ChronoUtc; // Import Utc specifically for DateTime<Utc>
use std::fmt;

use crate::utils::recurrence::validate_optional_rfc3339_string; // Import the custom validator for RFC3339 strings

// --- Database Models ---

// Base Event (from 'events' table)
#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub event_id: i32,
    pub user_id: i32,
    pub category_id: Option<i32>,
    pub title: String,
    pub description: Option<String>,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>, // Duration is implied by end_time - start_time
    pub location: Option<String>,
    pub rrule: Option<String>, // Stores the RRULE string (null for non-recurring)
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Event Exception (from 'event_exceptions' table)
#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventException {
    pub exception_id: i32,
    pub event_id: i32, // Parent recurring event ID
    pub original_occurrence_time: DateTime<Utc>, // The start time of the occurrence this exception applies to
    pub is_deleted: bool, // If true, this occurrence is cancelled

    // Override fields (apply if not deleted)
    pub title: Option<String>,
    pub description: Option<String>,
    pub start_time: Option<DateTime<Utc>>, // New start time if moved/modified
    pub end_time: Option<DateTime<Utc>>,   // New end time if moved/modified
    pub location: Option<String>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// --- Representation for Calendar Display ---
// This struct represents a single occurrence of an event, potentially overridden by an exception.
// This is what the API will return for date range queries.
#[derive(Debug, Serialize, Deserialize, Clone)] // Derive Deserialize/Clone for frontend usage
#[serde(rename_all = "camelCase")]
pub struct CalendarEventOccurrence {
    // Fields from the base event (or overridden)
    pub event_id: i32, // Original event ID
    // No user_id here, as this is for display, source is implied or handled by sync metadata
    pub category_id: Option<i32>, // From base event
    pub title: String, // Overridden by exception if present
    pub description: Option<String>, // Overridden by exception if present
    pub start_time: DateTime<Utc>, // Original occurrence start or exception override
    pub end_time: DateTime<Utc>, // Original occurrence end or exception override
    pub location: Option<String>, // Overridden by exception if present

    // Fields specific to recurrence/exceptions
    pub original_occurrence_time: Option<DateTime<Utc>>, // None for non-recurring, Some for recurring
    pub exception_id: Option<i32>, // ID of the exception if this occurrence is an exception

    // Might need more fields later (e.g., is_recurring, is_exception, source_calendar_id/user_id for shared)
}


// --- API Payloads ---

#[derive(Deserialize, Validate, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CreateEventPayload {
    #[validate(custom(function = "validate_optional_positive_i32"))]
    pub category_id: Option<i32>,

    #[validate(required, length(min = 1, max = 255))]
    pub title: Option<String>,

    #[validate(length(max = 1000))]
    pub description: Option<String>,

    #[validate(required)] // Start time is required
    pub start_time: Option<String>, // RFC3339 string

    #[validate(required)] // End time is required
    pub end_time: Option<String>, // RFC3339 string

    #[validate(length(max = 255))]
    pub location: Option<String>,

    // RRULE is optional. If provided, validate format (basic check for now)
    #[validate(custom(function = "validate_rrule_format"))]
    pub rrule: Option<String>,
}

#[derive(Deserialize, Validate, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UpdateEventPayload {
    #[validate(custom(function = "validate_optional_positive_i32"))]
    pub category_id: Option<i32>,

    #[validate(length(min = 1, max = 255))]
    pub title: Option<String>,

    #[validate(length(max = 1000))]
    pub description: Option<String>,

    // Allow updating times (required if present)
    #[validate(custom(function = "validate_optional_rfc3339_string"))]
    pub start_time: Option<String>,
    #[validate(custom(function = "validate_optional_rfc3339_string"))]
    pub end_time: Option<String>,

    #[validate(length(max = 255))]
    pub location: Option<String>,

    // Allow updating rrule (can be set to null to make non-recurring)
    #[validate(custom(function = "validate_optional_rrule_format"))]
    pub rrule: Option<String>,
}

#[derive(Deserialize, Validate, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CreateEventExceptionPayload {
    // event_id is from the path, not payload
    #[validate(required)] // Original occurrence time is required
    pub original_occurrence_time: Option<String>, // RFC3339 string

    #[validate(required)] // is_deleted is required
    pub is_deleted: Option<bool>,

    // Override fields (optional if is_deleted is true, required if false)
    #[validate(length(min = 1, max = 255))]
    pub title: Option<String>,

    #[validate(length(max = 1000))]
    pub description: Option<String>,

    #[validate(custom(function = "validate_optional_rfc3339_string"))]
    pub start_time: Option<String>, // New start time
    #[validate(custom(function = "validate_optional_rfc3339_string"))]
    pub end_time: Option<String>,   // New end time

    #[validate(length(max = 255))]
    pub location: Option<String>,

    // // Cross-field validation: if is_deleted is false, start_time and end_time must be Some
    // #[validate(custom(function = "validate_exception_modification_fields"))]
    // #[serde(skip)]
    // _modification_fields_validation: (),
}

#[derive(Deserialize, Validate, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UpdateEventExceptionPayload {
     // original_occurrence_time is the identifier, shouldn't be updated via PUT
    #[validate(required)] // is_deleted update is required
    pub is_deleted: Option<bool>,

    // Override fields (optional if is_deleted is true, required if false)
    #[validate(length(min = 1, max = 255))]
    pub title: Option<String>,

    #[validate(length(max = 1000))]
    pub description: Option<String>,

    #[validate(custom(function = "validate_optional_rfc3339_string"))]
    pub start_time: Option<String>,
    #[validate(custom(function = "validate_optional_rfc3339_string"))]
    pub end_time: Option<String>,

    #[validate(length(max = 255))]
    pub location: Option<String>,

    //  // Cross-field validation: if is_deleted is false, start_time and end_time must be Some
    // #[validate(custom(function = "validate_exception_modification_fields"))]
    // #[serde(skip)]
    // _modification_fields_validation: (),
}


// --- Custom Validator Functions (Add these to models/category.rs or a common utils file if preferred) ---

// Helper to validate optional positive integer (like category_id) - Re-use from category.rs
fn validate_optional_positive_i32(value: i32) -> Result<(), ValidationError> {
        if value <= 0 {
             let mut err = ValidationError::new("invalid_value");
             err.message = Some("ID must be positive".into());
             return Err(err);
        }
    Ok(())
}

// Basic validation for RRULE format (can be enhanced using a proper iCal parsing library)
fn validate_rrule_format(rrule: &str) -> Result<(), ValidationError> {
    // A very basic check: ensure it starts with "FREQ=" and contains at least one key-value pair
    if !rrule.starts_with("FREQ=") || !rrule.contains('=') {
         let mut err = ValidationError::new("invalid_rrule");
         err.message = Some("RRULE must start with FREQ= and contain key-value pairs".into());
         return Err(err);
    }
    // TODO: More robust validation using an iCal parsing library
    Ok(())
}

// Helper for validating optional rrule format
fn validate_optional_rrule_format(rrule: &str) -> Result<(), ValidationError> {
    validate_rrule_format(rrule)?; // Use the non-optional validator
     Ok(())
}

// // Custom validator for event exceptions payload
// fn validate_exception_modification_fields(payload: &CreateEventExceptionPayload) -> Result<(), ValidationError> {
//     // If is_deleted is explicitly false, start_time and end_time must be provided
//     if let Some(false) = payload.is_deleted {
//         if payload.start_time.is_none() || payload.end_time.is_none() {
//              let mut err = ValidationError::new("modification_requires_times");
//              err.message = Some("New start and end times are required for event modifications".into());
//              return Err(err);
//         }
//     }
//     Ok(())
// }

// // Custom validator for event exceptions payload (Update version)
// fn validate_exception_modification_fields(payload: &UpdateEventExceptionPayload) -> Result<(), ValidationError> {
//     // If is_deleted is explicitly false, start_time and end_time must be provided if THEY ARE PRESENT in the payload
//     // This is slightly different validation for update: you CAN update to is_deleted=false without providing times if the *existing* times were valid.
//     // However, the DB constraint `chk_exception_modification` handles the final state.
//     // For payload validation, let's enforce that if you provide is_deleted: false, you MUST provide start_time and end_time *in this payload*.
//      if let Some(false) = payload.is_deleted {
//         if payload.start_time.is_none() || payload.end_time.is_none() {
//              let mut err = ValidationError::new("modification_requires_times_in_payload");
//              err.message = Some("When setting is_deleted to false, new start and end times must be provided in the payload".into());
//              return Err(err);
//         }
//     }
//     Ok(())
// }