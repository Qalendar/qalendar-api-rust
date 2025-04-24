use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError};
use chrono::{DateTime, Utc};
use sqlx::FromRow; // Needed for sqlx to map database rows
use regex::Regex;

// --- Database Model ---

#[derive(Debug, FromRow, Serialize)] // Derive Serialize for API responses
#[serde(rename_all = "camelCase")] // Automatically convert snake_case DB fields to camelCase for JSON
pub struct Category {
    pub category_id: i32,
    pub user_id: i32, // The owner of the category
    pub name: String,
    pub color: String, // Consider adding validation for color format later
    pub is_visible: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

// --- API Payloads ---

#[derive(Deserialize, Validate, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CreateCategoryPayload {
    #[validate(required, length(min = 1, max = 255))]
    pub name: Option<String>,
    #[validate(required, length(min = 1, max = 50), custom(function = "validate_hex_color"))]
    pub color: Option<String>,

    // is_visible will likely default on creation, or can be optional
    // #[validate(skip)] // Don't validate if not present, or handle in handler
    // pub is_visible: Option<bool>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Deserialize, Validate, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCategoryPayload {
    #[validate(length(min = 1, max = 255))] // Allow updating name
    pub name: Option<String>,

    #[validate(length(min = 1, max = 50), custom(function = "validate_hex_color"))] // Allow updating color
    pub color: Option<String>,

    // // Allow updating visibility
    // pub is_visible: Option<bool>,
}

// Custom validator for hex color format
fn validate_hex_color(color: &str) -> Result<(), ValidationError> {
    let re = Regex::new(r"^#([A-Fa-f0-9]{6}|[A-Fa-f0-9]{3})$").unwrap();
    if re.is_match(color) {
        Ok(())
    } else {
        let mut err = ValidationError::new("invalid_hex_color");
        err.message = Some("Color must be in hex format (#RGB or #RRGGBB)".into());
        Err(err)
    }
}