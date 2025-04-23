use axum::{
    extract::{State, Path, Json, Query}, // Added Query for range fetching later
    http::StatusCode,
};
use sqlx;
use validator::Validate;
use crate::{
    AppState,
    errors::AppError,
    models::event::{Event, CreateEventPayload, UpdateEventPayload, CalendarEventOccurrence, EventException}, // Include EventException
    middleware::auth::AuthenticatedUser,
    calendar::recurrence::{expand_rrule, apply_exceptions}, // Import recurrence logic
};
use chrono::{DateTime, Utc, Duration}; // Keep Duration for potential date calculations
use serde::Deserialize; // For query parameters
use rrule::{RRule, Unvalidated}; // For RRULE parsing

use crate::utils::recurrence::validate_optional_rfc3339_string; // Import the custom validator

// --- Query Parameters for Date Range Fetching ---
#[derive(Deserialize, Validate, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DateRangeParams {
    #[validate(required, custom(function = "validate_optional_rfc3339_string"))] // Validate if present (required means needs to be in query string)
    pub start_time: Option<String>, // Required for range queries
    #[validate(required, custom(function = "validate_optional_rfc3339_string"))] // Validate if present
    pub end_time: Option<String>, // Required for range queries
}

// --- Create Event ---
pub async fn create_event(
    State(state): State<AppState>,
    AuthenticatedUser { user_id }: AuthenticatedUser,
    Json(payload): Json<CreateEventPayload>,
) -> Result<(StatusCode, Json<Event>), AppError> {
    payload.validate()?;

    let title = payload.title.unwrap();
    let description = payload.description;
    let start_time_str = payload.start_time.unwrap();
    let end_time_str = payload.end_time.unwrap();
    let location = payload.location;
    let rrule = payload.rrule;
    let category_id = payload.category_id;

    // Parse date/time strings
    let start_time = DateTime::parse_from_rfc3339(&start_time_str)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|_| AppError::ValidationFailed(validator::ValidationErrors::new()))?; // Basic parse error

    let end_time = DateTime::parse_from_rfc3339(&end_time_str)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|_| AppError::ValidationFailed(validator::ValidationErrors::new()))?; // Basic parse error

    // Basic validation: end_time must be after start_time
    if end_time <= start_time {
         let mut errors = validator::ValidationErrors::new();
         errors.add("end_time", validator::ValidationError::new("end_time_after_start"));
         return Err(AppError::ValidationFailed(errors));
    }


    // Optional: Check if category_id exists and belongs to the user if provided
    if let Some(cat_id) = category_id {
        let category_exists_for_user: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM categories WHERE category_id = $1 AND user_id = $2)"
        )
        .bind(cat_id)
        .bind(user_id)
        .fetch_one(&state.pool)
        .await?;

        if !category_exists_for_user {
            return Err(AppError::CategoryNotFound);
        }
    }

    // Insert event into the database
    let created_event = sqlx::query_as!(
        Event,
        r#"
        INSERT INTO events (user_id, category_id, title, description, start_time, end_time, location, rrule)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING event_id, user_id, category_id as "category_id: _", title, description as "description!: _", start_time, end_time, location as "location!: _", rrule as "rrule!: _", created_at as "created_at!", updated_at as "updated_at!"
        "#,
        user_id,
        category_id,
        title,
        description,
        start_time,
        end_time,
        location,
        rrule, // Insert the RRULE string directly
    )
    .fetch_one(&state.pool)
    .await?;

    Ok((StatusCode::CREATED, Json(created_event)))
}

// --- Get Single Base Event by ID for User ---
// This fetches the *base* event record, not occurrences. Useful for editing the event settings.
pub async fn get_base_event_by_id(
    State(state): State<AppState>,
    AuthenticatedUser { user_id }: AuthenticatedUser,
    Path(event_id): Path<i32>,
) -> Result<Json<Event>, AppError> {
    let event = sqlx::query_as!(
        Event,
        r#"
        SELECT event_id, user_id, category_id as "category_id: _", title, description as "description!: _", start_time, end_time, location as "location!: _", rrule as "rrule!: _", created_at as "created_at!", updated_at as "updated_at!"
        FROM events
        WHERE event_id = $1 AND user_id = $2 -- IMPORTANT: Check user_id!
        "#,
        event_id,
        user_id
    )
    .fetch_optional(&state.pool)
    .await?;

    match event {
        Some(evt) => Ok(Json(evt)),
        None => Err(AppError::EventNotFound),
    }
}

// --- Update Base Event ---
pub async fn update_event(
    State(state): State<AppState>,
    AuthenticatedUser { user_id }: AuthenticatedUser,
    Path(event_id): Path<i32>,
    Json(payload): Json<UpdateEventPayload>,
) -> Result<Json<Event>, AppError> {
    payload.validate()?;

    // Check if the event exists AND belongs to the user first
    let existing_event = sqlx::query_as!(
        Event,
        r#"
        SELECT event_id, user_id, category_id as "category_id: _", title, description as "description!: _", start_time, end_time, location as "location!: _", rrule as "rrule!: _", created_at as "created_at!", updated_at as "updated_at!"
        FROM events
        WHERE event_id = $1 AND user_id = $2
        "#,
        event_id,
        user_id
    )
    .fetch_optional(&state.pool)
    .await?;

    let mut event_to_update = match existing_event {
        Some(evt) => evt,
        None => return Err(AppError::EventNotFound),
    };

    // Apply updates only if the field is provided in the payload
    if payload.category_id.is_some() {
        let cat_id = payload.category_id.unwrap();
        if cat_id <= 0 { // Convention for unsetting category
             event_to_update.category_id = None;
        } else {
            let category_exists_for_user: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM categories WHERE category_id = $1 AND user_id = $2)"
            )
            .bind(cat_id)
            .bind(user_id)
            .fetch_one(&state.pool)
            .await?;

            if !category_exists_for_user {
                return Err(AppError::CategoryNotFound);
            }
            event_to_update.category_id = Some(cat_id);
        }
    }
    if let Some(title) = payload.title {
        event_to_update.title = title;
    }
    if payload.description.is_some() { // Allow explicit null
        event_to_update.description = payload.description;
    }
    if let Some(start_time_str) = payload.start_time.as_ref() {
         let start_time = DateTime::parse_from_rfc3339(&start_time_str)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|_| AppError::ValidationFailed(validator::ValidationErrors::new()))?;
        event_to_update.start_time = start_time;
    }
     if let Some(end_time_str) = payload.end_time.as_ref() {
         let end_time = DateTime::parse_from_rfc3339(&end_time_str)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|_| AppError::ValidationFailed(validator::ValidationErrors::new()))?;
        event_to_update.end_time = end_time;
    }
    if payload.location.is_some() { // Allow explicit null
        event_to_update.location = payload.location;
    }
    // Note: If start/end times are updated, validate end_time > start_time again
    if let Some(start_time) = payload.start_time.as_ref().map(|s| DateTime::parse_from_rfc3339(s).unwrap_or_else(|_| Utc::now().into())) { // Temporary parse for validation
         if let Some(end_time) = payload.end_time.as_ref().map(|s| DateTime::parse_from_rfc3339(s).unwrap_or_else(|_| Utc::now().into())) {
              if end_time <= start_time {
                let mut errors = validator::ValidationErrors::new();
                errors.add("end_time", validator::ValidationError::new("end_time_after_start"));
                return Err(AppError::ValidationFailed(errors));
             }
         } else if event_to_update.end_time <= start_time { // Compare new start with old end
             let mut errors = validator::ValidationErrors::new();
             errors.add("end_time", validator::ValidationError::new("end_time_after_start"));
             return Err(AppError::ValidationFailed(errors));
         }
    } else if let Some(end_time) = payload.end_time.as_ref().map(|s| DateTime::parse_from_rfc3339(s).unwrap_or_else(|_| Utc::now().into())) { // Compare new end with old start
        if end_time <= event_to_update.start_time {
             let mut errors = validator::ValidationErrors::new();
             errors.add("end_time", validator::ValidationError::new("end_time_after_start"));
             return Err(AppError::ValidationFailed(errors));
        }
    }


    if payload.rrule.is_some() { // Allow setting rrule to null
        // If setting a new RRULE, try parsing it to catch basic format errors early
        if let Some(rrule_str) = &payload.rrule {
            if !rrule_str.is_empty() {
                rrule_str.parse::<RRule<Unvalidated>>()
                    .map_err(|e| AppError::InvalidRecurrenceRule(format!("Failed to parse new RRULE: {}", e)))?;
            }
        }
        event_to_update.rrule = payload.rrule;
    }


    // Perform the update query
    let updated_event = sqlx::query_as!(
        Event,
        r#"
        UPDATE events
        SET category_id = $1, title = $2, description = $3, start_time = $4, end_time = $5, location = $6, rrule = $7
        WHERE event_id = $8 AND user_id = $9
        RETURNING event_id, user_id, category_id as "category_id!: _", title, description as "description!: _", start_time, end_time, location as "location!: _", rrule as "rrule!: _", created_at as "created_at!", updated_at as "updated_at!"
        "#,
        event_to_update.category_id,
        event_to_update.title,
        event_to_update.description,
        event_to_update.start_time,
        event_to_update.end_time,
        event_to_update.location,
        event_to_update.rrule,
        event_id,
        user_id
    )
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(updated_event))
}

// --- Delete Base Event ---
pub async fn delete_event(
    State(state): State<AppState>,
    AuthenticatedUser { user_id }: AuthenticatedUser,
    Path(event_id): Path<i32>,
) -> Result<StatusCode, AppError> {
    // Perform the delete query. Check for user_id!
    // ON DELETE CASCADE in the schema handles deleting associated exceptions and invitations
    let delete_result = sqlx::query!(
        r#"
        DELETE FROM events
        WHERE event_id = $1 AND user_id = $2
        "#,
        event_id,
        user_id
    )
    .execute(&state.pool)
    .await?;

    if delete_result.rows_affected() == 0 {
        Err(AppError::EventNotFound)
    } else {
        Ok(StatusCode::NO_CONTENT)
    }
}

// --- Placeholder for Get Events in Range (Implemented later) ---
// This is the complex one used for calendar view.
// It will query base events and exceptions, expand RRULES, apply exceptions.
pub async fn get_events_in_range(
     State(state): State<AppState>,
     AuthenticatedUser { user_id }: AuthenticatedUser,
     Query(params): Query<DateRangeParams>, // Extract query parameters
) -> Result<Json<Vec<CalendarEventOccurrence>>, AppError> {
    // Basic validation for date range params
    params.validate()?;

    let start_time_str = params.start_time.unwrap();
    let end_time_str = params.end_time.unwrap();

     let range_start = DateTime::parse_from_rfc3339(&start_time_str)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|_| AppError::ValidationFailed(validator::ValidationErrors::new()))?;

    let range_end = DateTime::parse_from_rfc3339(&end_time_str)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|_| AppError::ValidationFailed(validator::ValidationErrors::new()))?;

    // --- Implementation will go here ---
    // 1. Fetch relevant base events (non-recurring in range, recurring before range_end)
    // 2. Fetch relevant exceptions for these recurring events
    // 3. Iterate through fetched events:
    //    - If non-recurring, add as CalendarEventOccurrence if within range
    //    - If recurring, call expand_rrule, then filter/collect related exceptions, call apply_exceptions
    // 4. Collect all CalendarEventOccurrence into a single Vec
    // 5. Return the Vec<CalendarEventOccurrence>

    // For now, return empty list or a placeholder
    tracing::warn!("get_events_in_range handler not fully implemented yet, returning empty list.");
    Ok(Json(vec![])) // Placeholder
}


// --- Placeholder for Event Exception CRUD (Implemented later) ---
// Handlers for creating, getting, updating, and deleting exceptions
// pub async fn create_event_exception(...) -> Result<...>
// pub async fn get_event_exception_by_id(...) -> Result<...>
// pub async fn update_event_exception(...) -> Result<...>
// pub async fn delete_event_exception(...) -> Result<...>