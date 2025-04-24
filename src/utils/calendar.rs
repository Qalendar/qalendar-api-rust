use chrono::{DateTime, Utc};
use crate::errors::AppError;

// Helper to parse RFC3339 timestamp strings (like "2023-10-27T10:00:00Z")
pub fn parse_timestamp(s: &str) -> Result<DateTime<Utc>, AppError> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc)) // Ensure it's Utc
        .map_err(|e| {
            tracing::warn!("Failed to parse timestamp '{}': {}", s, e);
            // Return a validation-like error for bad date format
            // Could make a dedicated AppError::InvalidTimestamp
            AppError::ValidationFailed(validator::ValidationErrors::new()) // Simple error for now
        })
}

// Helper to parse optional 'since' timestamp
pub fn parse_optional_timestamp(since_str: Option<String>) -> Result<Option<DateTime<Utc>>, AppError> {
    match since_str {
        Some(s) => DateTime::parse_from_rfc3339(&s)
            .map(|dt| Some(dt.with_timezone(&Utc)))
            .map_err(|e| {
                tracing::warn!("Failed to parse 'since' timestamp '{}': {}", s, e);
                AppError::ValidationFailed(validator::ValidationErrors::new()) // Simple error
            }),
        None => Ok(None), // No timestamp provided
    }
}