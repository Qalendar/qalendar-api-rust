use validator::ValidationError;
use chrono::{DateTime, Utc};

// Helper to validate that if a string is present, it's a valid RFC3339 timestamp
pub fn validate_optional_rfc3339_string(value: &str) -> Result<(), ValidationError> {
    if DateTime::parse_from_rfc3339(value).is_err() {
        let mut err = ValidationError::new("invalid_rfc3339");
        err.message = Some("Must be a valid RFC3339 timestamp string".into());
        return Err(err);
    }
    Ok(())
}