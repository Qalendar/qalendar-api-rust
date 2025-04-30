use axum::{
    extract::{State, Path, Json},
    http::StatusCode,
};
use sqlx::{PgPool, types::chrono::Utc};
use validator::Validate;
use crate::{
    AppState,
    errors::AppError,
    models::event::{Event, CreateEventPayload, UpdateEventPayload}, // Import event models
    middleware::auth::AuthenticatedUser,
};
use chrono::DateTime; // For parsing date strings

use crate::utils::calendar::parse_timestamp; // Utility function for parsing timestamps


// --- Create Event ---
pub async fn create_event(
    State(state): State<AppState>,
    AuthenticatedUser { user_id }: AuthenticatedUser,
    Json(payload): Json<CreateEventPayload>,
) -> Result<(StatusCode, Json<Event>), AppError> {
    payload.validate()?;

    let title = payload.title.unwrap(); // Required by validation
    let category_id = payload.category_id; // Required by validation
    let description = payload.description; // Option<String>
    let location = payload.location;     // Option<String>
    let rrule = payload.rrule;         // Option<String>

    // Parse required timestamps
    let start_time_str = payload.start_time.unwrap();
    let start_time = parse_timestamp(&start_time_str)?;
    let end_time_str = payload.end_time.unwrap();
    let end_time = parse_timestamp(&end_time_str)?;

    // Optional: Validate category_id existence and ownership if provided
    if let Some(cat_id) = category_id {
       let category_exists: Option<bool> = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM categories WHERE category_id = $1 AND user_id = $2)",
            cat_id,
            user_id
        )
        .fetch_one(&state.pool)
        .await?;

        if category_exists != Some(true) {
             return Err(AppError::CategoryNotFound); // Re-using error for now
        }
    }

    let created_event = sqlx::query_as!(
        Event,
        r#"
        INSERT INTO events (user_id, category_id, title, description, start_time, end_time, location, rrule)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING
           event_id, user_id, category_id, title, description as "description!: _", start_time, end_time,
           location as "location!: _", rrule as "rrule!: _",
           created_at as "created_at!", updated_at as "updated_at!", deleted_at as "deleted_at!: _"
        "#,
        user_id,
        category_id,
        title,
        description,
        start_time,
        end_time,
        location,
        rrule,
    )
    .fetch_one(&state.pool)
    .await?; // sqlx::Error -> AppError::DatabaseError

    Ok((StatusCode::CREATED, Json(created_event)))
}

// --- Get All Events for User ---
pub async fn get_events(
    State(state): State<AppState>,
    AuthenticatedUser { user_id }: AuthenticatedUser,
) -> Result<Json<Vec<Event>>, AppError> {
    let events = sqlx::query_as!(
        Event,
        r#"
        SELECT
           event_id, user_id, category_id, title, description as "description!: _", start_time, end_time,
           location as "location!: _", rrule as "rrule!: _",
           created_at as "created_at!", updated_at as "updated_at!", deleted_at as "deleted_at!: _"
        FROM events
        WHERE user_id = $1 AND deleted_at IS NULL
        ORDER BY start_time -- Optional: order by start time
        "#,
        user_id
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(events))
}

// --- Get Single Event by ID for User ---
pub async fn get_event_by_id(
    State(state): State<AppState>,
    AuthenticatedUser { user_id }: AuthenticatedUser,
    Path(event_id): Path<i32>,
) -> Result<Json<Event>, AppError> {
    let event = sqlx::query_as!(
        Event,
        r#"
        SELECT
           event_id, user_id, category_id, title, description as "description!: _", start_time, end_time,
           location as "location!: _", rrule as "rrule!: _",
           created_at as "created_at!", updated_at as "updated_at!", deleted_at as "deleted_at!: _"
        FROM events
        WHERE event_id = $1 AND user_id = $2 -- IMPORTANT: Check user_id!
        "#,
        event_id,
        user_id
    )
    .fetch_optional(&state.pool)
    .await?;

    match event {
        Some(e) => Ok(Json(e)),
        None => Err(AppError::EventNotFound), // Return EventNotFound error
    }
}

// --- Update Event ---
pub async fn update_event(
    State(state): State<AppState>,
    AuthenticatedUser { user_id }: AuthenticatedUser,
    Path(event_id): Path<i32>,
    Json(payload): Json<UpdateEventPayload>,
) -> Result<Json<Event>, AppError> {
    payload.validate()?;

    // Fetch existing event to check ownership and get current values
    let existing_event = sqlx::query_as!(
        Event,
        r#"
        SELECT
           event_id, user_id, category_id, title, description as "description!: _", start_time, end_time,
           location as "location!: _", rrule as "rrule!: _",
           created_at as "created_at!", updated_at as "updated_at!", deleted_at as "deleted_at!: _"
        FROM events
        WHERE event_id = $1 AND user_id = $2
        "#,
        event_id,
        user_id
    )
    .fetch_optional(&state.pool)
    .await?;

    let mut event_to_update = match existing_event {
        Some(e) => e,
        None => return Err(AppError::EventNotFound),
    };


    // Apply updates only if the field is provided in the payload (Option::is_some())
    if let Some(title) = payload.title {
        event_to_update.title = title;
    }
    // First validate if the new category_id exists and belongs to the user
    if let Some(new_cat_id) = payload.category_id {
        let category_exists: Option<bool> = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM categories WHERE category_id = $1 AND user_id = $2)",
            new_cat_id,
            user_id
        )
        .fetch_one(&state.pool)
        .await?;

        if category_exists != Some(true) {
            return Err(AppError::CategoryNotFound);
        }
        
        // Only update the category_id after validation
        event_to_update.category_id = new_cat_id;
    }
    
    // Handle optional fields carefully: None in JSON should set DB column to NULL
    if payload.description.is_some() || (payload.description.is_none() && payload.description.as_ref().is_some()) {
        event_to_update.description = payload.description;
    }
    if payload.location.is_some() || (payload.location.is_none() && payload.location.as_ref().is_some()) {
        event_to_update.location = payload.location;
    }
    if payload.rrule.is_some() || (payload.rrule.is_none() && payload.rrule.as_ref().is_some()) {
        event_to_update.rrule = payload.rrule;
    }


    // Handle time updates
    let mut updated_start_time = event_to_update.start_time;
    if let Some(start_time_str) = payload.start_time {
        updated_start_time = parse_timestamp(&start_time_str)?;
    }
    let mut updated_end_time = event_to_update.end_time;
    if let Some(end_time_str) = payload.end_time {
        updated_end_time = parse_timestamp(&end_time_str)?;
    }
    // Apply updated times if they were successfully parsed from the payload
    event_to_update.start_time = updated_start_time;
    event_to_update.end_time = updated_end_time;


    // Perform the update query
    let updated_event = sqlx::query_as!(
        Event,
        r#"
        UPDATE events
        SET
            category_id = $1,
            title = $2,
            description = $3,
            start_time = $4,
            end_time = $5,
            location = $6,
            rrule = $7
            -- updated_at trigger handles timestamp
        WHERE event_id = $8 AND user_id = $9 -- Double-check user_id here again for safety
        RETURNING
           event_id, user_id, category_id, title, description as "description!: _", start_time, end_time,
           location as "location!: _", rrule as "rrule!: _",
           created_at as "created_at!", updated_at as "updated_at!", deleted_at as "deleted_at!: _"
        "#,
        event_to_update.category_id,
        event_to_update.title,
        event_to_update.description,
        event_to_update.start_time,
        event_to_update.end_time,
        event_to_update.location,
        event_to_update.rrule,
        event_id,
        user_id // Crucial check
    )
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(updated_event))
}

// --- Delete Event ---
pub async fn delete_event(
    State(state): State<AppState>,
    AuthenticatedUser { user_id }: AuthenticatedUser,
    Path(event_id): Path<i32>,
) -> Result<StatusCode, AppError> {
    let delete_result = sqlx::query!(
        r#"
        UPDATE events
        SET deleted_at = NOW() -- Soft delete
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