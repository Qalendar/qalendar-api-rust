use chrono::{DateTime, Utc};
use sqlx::PgPool;
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

// --- Helper: Validate category IDs exist and belong to the owner ---
pub async fn validate_category_ids(pool: &PgPool, owner_user_id: i32, category_ids: &[i32]) -> Result<(), AppError> {
    if category_ids.is_empty() {
        // If the list is empty, it's valid (meaning unshare all categories)
        return Ok(());
    }

    // Query to count how many of the provided category_ids exist and belong to the user
    let count: i64 = sqlx::query_scalar!(
        r#"
        SELECT COUNT(*)
        FROM categories
        WHERE category_id = ANY($1) AND user_id = $2
        "#,
        &category_ids, // Pass as slice/array
        owner_user_id
    )
        .fetch_one(pool)
        .await?
        .unwrap_or(0); // Unwrap the Option<i64> to i64, defaulting to 0 if NULL

    // If the count doesn't match the number of provided IDs, some are invalid or don't belong to user
    if count as usize != category_ids.len() {
        // Could make a more specific error finding which IDs are invalid
        let mut err = validator::ValidationErrors::new();
        err.add("categoryIds", validator::ValidationError::new("invalid_category_id_or_ownership"));
        return Err(AppError::ValidationFailed(err));
    }

    Ok(())
}