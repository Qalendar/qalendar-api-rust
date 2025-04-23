use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError};
use chrono::{DateTime, Utc};
use sqlx::FromRow;

// --- Database Models ---

// Define the ENUM types as Rust enums
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, Serialize, Deserialize)] // sqlx::Type for database mapping
#[sqlx(type_name = "deadline_priority_level", rename_all = "lowercase")] // Match DB ENUM name and values
#[serde(rename_all = "camelCase")]

#[derive(Default)]
pub enum DeadlinePriorityLevel {
    #[default]
    Normal,
    Important,
    Urgent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, Serialize, Deserialize)]
#[sqlx(type_name = "workload_unit_type", rename_all = "lowercase")]
#[serde(rename_all = "camelCase")]
pub enum WorkloadUnitType {
    Minutes,
    Hours,
    Days,
}


#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Deadline {
    pub deadline_id: i32,
    pub user_id: i32, // Owner
    pub category_id: Option<i32>, // Optional link to category
    pub title: String,
    pub description: Option<String>, // Allow NULL in DB
    pub due_date: DateTime<Utc>, // TIMESTAMP WITH TIME ZONE
    pub priority: DeadlinePriorityLevel, // Use the Rust ENUM
    pub workload_magnitude: Option<i32>, // Corresponds to INTEGER, can be NULL
    pub workload_unit: Option<WorkloadUnitType>, // Corresponds to ENUM, can be NULL
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}


// --- API Payloads ---

// For creating a Deadline
#[derive(Deserialize, Validate, Debug)]
#[serde(rename_all = "camelCase")]
#[validate(schema(function = "validate_workload_pair"))] // Apply validation at struct level
pub struct CreateDeadlinePayload {
    #[validate(required, length(min = 1, max = 255))]
    pub title: Option<String>,
    #[validate(required)]
    pub category_id: Option<i32>,
    #[validate(length(max = 1000))]
    pub description: Option<String>,
    #[validate(required)]
    pub due_date: Option<String>,
    #[validate(required)]
    pub priority: Option<DeadlinePriorityLevel>,
    pub workload_magnitude: Option<i32>,
    pub workload_unit: Option<WorkloadUnitType>,
}

// For updating a Deadline
#[derive(Deserialize, Validate, Debug)]
#[serde(rename_all = "camelCase")]
#[validate(schema(function = "validate_workload_pair_update"))] // Apply specific update validation
pub struct UpdateDeadlinePayload {
    #[validate(length(min = 1, max = 255))]
    pub title: Option<String>,
    pub category_id: Option<i32>,
    #[validate(length(max = 1000))]
    pub description: Option<String>,
    pub due_date: Option<String>,
    pub priority: Option<DeadlinePriorityLevel>,
    pub workload_magnitude: Option<i32>,
    pub workload_unit: Option<WorkloadUnitType>,
}

// Custom validator for workload magnitude/unit pair
fn validate_workload_pair(payload: &CreateDeadlinePayload) -> Result<(), ValidationError> {
    match (payload.workload_magnitude, payload.workload_unit) {
        (Some(_), None) | (None, Some(_)) => {
            // Error: one is present, the other isn't
            let mut err = ValidationError::new("workload_pair");
            err.message = Some("Both workload magnitude and unit must be provided if either is present".into());
            Err(err)
        },
        _ => Ok(()), // Valid: either both are Some, or both are None
    }
}

// Note: UpdateDeadlinePayload needs its own workload validation function
fn validate_workload_pair_update(payload: &UpdateDeadlinePayload) -> Result<(), ValidationError> {
    match (payload.workload_magnitude, payload.workload_unit) {
        (Some(_), None) | (None, Some(_)) => {
            // Error: one is present, the other isn't
            let mut err = ValidationError::new("workload_pair");
            err.message = Some("Both workload magnitude and unit must be provided if either is present for update".into());
            Err(err)
        },
        _ => Ok(()), // Valid: either both are Some, or both are None
    }
}