use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError};
use chrono::{DateTime, Utc, NaiveDateTime}; // Use NaiveDateTime for TIMESTAMP without time zone if necessary, otherwise Utc
use sqlx::FromRow;
use sqlx::types::chrono::Utc as ChronoUtc; // Import Utc specifically for DateTime<Utc>

// --- Database Model ---

// Note: sqlx maps Postgres ENUMs automatically if the feature is enabled
// and you use a Rust enum that derives sqlx::Type and #[sqlx(type_name = "...")].
// Let's define Rust enums matching our SQL ones.

#[derive(Debug, Serialize, Deserialize, Clone, Copy, sqlx::Type, Default)]
#[sqlx(type_name = "deadline_priority_level", rename_all = "lowercase")] // Match SQL ENUM name and values
#[serde(rename_all = "lowercase")] // Match SQL values for JSON
pub enum DeadlinePriority {
    #[default] // Default value for the enum
    Normal,
    Important,
    Urgent,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, sqlx::Type)]
#[sqlx(type_name = "workload_unit_type", rename_all = "lowercase")] // Match SQL ENUM name and values
#[serde(rename_all = "lowercase")] // Match SQL values for JSON
pub enum WorkloadUnit {
    Minutes,
    Hours,
    Days,
}


#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Deadline {
    pub deadline_id: i32,
    pub user_id: i32, // The owner of the deadline
    pub category_id: Option<i32>, // Optional category
    pub title: String,
    pub description: Option<String>, // Assuming description is optional in DB
    pub due_date: DateTime<Utc>, // Matches TIMESTAMP WITH TIME ZONE
    pub priority: DeadlinePriority, // Use Rust ENUM
    pub workload_magnitude: Option<i32>, // Optional integer
    pub workload_unit: Option<WorkloadUnit>, // Optional ENUM
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// --- API Payloads ---

#[derive(Deserialize, Validate, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CreateDeadlinePayload {
    // Category ID is optional, but validate if provided (validator runs only if Some)
    #[validate(custom(function = "validate_positive_i32"))]
    pub category_id: Option<i32>,

    #[validate(required, length(min = 1, max = 255))]
    pub title: Option<String>,

    #[validate(length(max = 1000))] // Optional description, max length
    pub description: Option<String>,

    #[validate(required)] // Due date is required
    // Use String for validation, parse to DateTime<Utc> in handler
    pub due_date: Option<String>,

    // Priority is optional in payload, will default in DB if not provided
    #[validate(custom(function = "validate_optional_deadline_priority"))]
    pub priority: Option<DeadlinePriority>,

    // Workload fields are optional together
    #[validate(custom(function = "validate_optional_workload_magnitude"))]
    pub workload_magnitude: Option<i32>,
    #[validate(custom(function = "validate_optional_workload_unit"))]
    pub workload_unit: Option<WorkloadUnit>,
}

#[derive(Deserialize, Validate, Debug)]
#[validate(schema(function = "validate_workload_pair"))] // Apply validation at the struct level
#[serde(rename_all = "camelCase")]
pub struct UpdateDeadlinePayload {
     // Category ID can be updated, still optional (validator runs only if Some)
    #[validate(custom(function = "validate_positive_i32"))]
    pub category_id: Option<i32>,

    #[validate(length(min = 1, max = 255))] // Allow updating title
    pub title: Option<String>,

    #[validate(length(max = 1000))] // Allow updating description
    pub description: Option<String>,

    // Allow updating due date
    pub due_date: Option<String>,

    // Allow updating priority
    #[validate(custom(function = "validate_optional_deadline_priority"))]
    pub priority: Option<DeadlinePriority>,

    // Allow updating workload fields
    #[validate(custom(function = "validate_optional_workload_magnitude"))]
    pub workload_magnitude: Option<i32>,
    #[validate(custom(function = "validate_optional_workload_unit"))]
    pub workload_unit: Option<WorkloadUnit>,

    // Dummy field is no longer needed for this validation
    // #[serde(skip)]
    // _workload_pair_validation: (),
}

// --- Custom Validator Functions ---

// Helper to validate a positive integer (used for optional category_id)
// Assumes validator crate calls this only when category_id is Some(i32).
fn validate_positive_i32(value: i32) -> Result<(), ValidationError> {
    if value <= 0 {
         let mut err = ValidationError::new("invalid_value");
         err.message = Some("ID must be positive".into());
         return Err(err);
    }
    Ok(())
}

// Helper to validate optional DeadlinePriority (enum parsing handles values)
// Note: If DeadlinePriority needed custom validation logic, a similar pattern might apply.
// The validator passes a reference to the inner value of Option<T>
fn validate_optional_deadline_priority(_value: &DeadlinePriority) -> Result<(), ValidationError> {
    // Validation handled by serde deserialization + sqlx::Type
    Ok(())
}

// Helper to validate optional WorkloadUnit (enum parsing handles values)
// The validator passes a reference to the inner value of Option<T>
fn validate_optional_workload_unit(_value: &WorkloadUnit) -> Result<(), ValidationError> {
     // Validation handled by serde deserialization + sqlx::Type
    Ok(())
}

// Helper to validate optional WorkloadMagnitude (check for negative)
// The validator passes the inner value of Option<T> if Some
fn validate_optional_workload_magnitude(value: i32) -> Result<(), ValidationError> {
    // value is the inner i32
    if value < 0 {
        let mut err = ValidationError::new("invalid_value");
        err.message = Some("Workload magnitude cannot be negative".into());
        return Err(err);
    }
    Ok(())
}


// Custom validator for UpdateDeadlinePayload to check workload pair
// The validator passes a reference to the struct being validated
fn validate_workload_pair(payload: &UpdateDeadlinePayload) -> Result<(), ValidationError> {
    // If one is Some, the other must also be Some
    if payload.workload_magnitude.is_some() != payload.workload_unit.is_some() {
         let mut err = ValidationError::new("workload_pair_required");
         err.message = Some("Workload magnitude and unit must be provided together".into());
         Err(err)
    } else {
        Ok(())
    }
}