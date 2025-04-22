use axum::{
    extract::{State, Path, Json},
    http::StatusCode,
};
use sqlx;
use validator::Validate;
use crate::{
    AppState,
    errors::AppError,
    models::deadline::{Deadline, CreateDeadlinePayload, UpdateDeadlinePayload},
    middleware::auth::AuthenticatedUser,
};
use chrono::{DateTime, Utc}; // Import for parsing dates

// --- Create Deadline ---
pub async fn create_deadline(
    State(state): State<AppState>,
    AuthenticatedUser { user_id }: AuthenticatedUser,
    Json(payload): Json<CreateDeadlinePayload>,
) -> Result<(StatusCode, Json<Deadline>), AppError> {
    payload.validate()?; // Validate the input payload

    let title = payload.title.unwrap();
    let description = payload.description; // Optional
    let due_date_str = payload.due_date.unwrap(); // Required field
    let category_id = payload.category_id; // Optional
    let priority = payload.priority.unwrap_or_default(); // Use default if not provided
    let workload_magnitude = payload.workload_magnitude; // Optional
    let workload_unit = payload.workload_unit; // Optional

    // Parse due_date string to DateTime<Utc>
    let due_date: DateTime<Utc> = DateTime::parse_from_rfc3339(&due_date_str)
        .map(|dt| dt.with_timezone(&Utc)) // Ensure it's Utc if timezone info is present
        .map_err(|_| AppError::ValidationFailed(validator::ValidationErrors::new()))?; // Simple error for parse failure


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
            return Err(AppError::CategoryNotFound); // Return CategoryNotFound if it doesn't exist or belong to user
        }
    }


    let created_deadline = sqlx::query_as!(
        Deadline,
        r#"
        INSERT INTO deadlines (user_id, category_id, title, description, due_date, priority, workload_magnitude, workload_unit)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING deadline_id, user_id, category_id as "category_id!: _", title, description as "description!: _", due_date, priority as "priority!: _", workload_magnitude as "workload_magnitude!: _", workload_unit as "workload_unit!: _", created_at as "created_at!", updated_at as "updated_at!"
        "#,
        user_id,
        category_id,
        title,
        description,
        due_date,
        priority as _, // Cast to the enum type
        workload_magnitude,
        workload_unit.as_ref() as _, // Use as_ref() for the optional enum
    )
    .fetch_one(&state.pool)
    .await?;

    Ok((StatusCode::CREATED, Json(created_deadline)))
}

// --- Get All Deadlines for User ---
pub async fn get_deadlines(
    State(state): State<AppState>,
    AuthenticatedUser { user_id }: AuthenticatedUser,
) -> Result<Json<Vec<Deadline>>, AppError> {
    let deadlines = sqlx::query_as!(
        Deadline,
        r#"
        SELECT deadline_id, user_id, category_id as "category_id!: _", title, description as "description!: _", due_date, priority as "priority!: _", workload_magnitude as "workload_magnitude!: _", workload_unit as "workload_unit!: _", created_at as "created_at!", updated_at as "updated_at!"
        FROM deadlines
        WHERE user_id = $1
        ORDER BY due_date, updated_at -- Order by due date, then last updated
        "#,
        user_id
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(deadlines))
}

// --- Get Single Deadline by ID for User ---
pub async fn get_deadline_by_id(
    State(state): State<AppState>,
    AuthenticatedUser { user_id }: AuthenticatedUser,
    Path(deadline_id): Path<i32>,
) -> Result<Json<Deadline>, AppError> {
    let deadline = sqlx::query_as!(
        Deadline,
         r#"
        SELECT deadline_id, user_id, category_id as "category_id!: _", title, description as "description!: _", due_date, priority as "priority!: _", workload_magnitude as "workload_magnitude!: _", workload_unit as "workload_unit!: _", created_at as "created_at!", updated_at as "updated_at!"
        FROM deadlines
        WHERE deadline_id = $1 AND user_id = $2 -- IMPORTANT: Check user_id!
        "#,
        deadline_id,
        user_id
    )
    .fetch_optional(&state.pool)
    .await?;

    match deadline {
        Some(dl) => Ok(Json(dl)),
        None => Err(AppError::DeadlineNotFound), // Return DeadlineNotFound error
    }
}

// --- Update Deadline ---
pub async fn update_deadline(
    State(state): State<AppState>,
    AuthenticatedUser { user_id }: AuthenticatedUser,
    Path(deadline_id): Path<i32>,
    Json(payload): Json<UpdateDeadlinePayload>,
) -> Result<Json<Deadline>, AppError> {
    payload.validate()?; // Validate the input payload

    // We need to check if the deadline exists AND belongs to the user first
    let existing_deadline = sqlx::query_as!(
         Deadline,
         r#"
        SELECT deadline_id, user_id, category_id as "category_id!: _", title, description as "description!: _", due_date, priority as "priority!: _", workload_magnitude as "workload_magnitude!: _", workload_unit as "workload_unit!: _", created_at as "created_at!", updated_at as "updated_at!"
        FROM deadlines
        WHERE deadline_id = $1 AND user_id = $2
        "#,
        deadline_id,
        user_id
    )
    .fetch_optional(&state.pool)
    .await?;

    let mut deadline_to_update = match existing_deadline {
        Some(dl) => dl,
        None => return Err(AppError::DeadlineNotFound),
    };

    // Check if the provided category_id exists and belongs to the user if it's being updated
     if payload.category_id.is_some() {
         let cat_id = payload.category_id.unwrap(); // Safe unwrap because we checked is_some()
         // Handle None category explicitly: if payload sends null/None category_id, set it to NULL
         if cat_id <= 0 { // Using <= 0 as convention for "unset" or "null" category ID
             deadline_to_update.category_id = None;
         } else {
            let category_exists_for_user: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM categories WHERE category_id = $1 AND user_id = $2)"
            )
            .bind(cat_id)
            .bind(user_id)
            .fetch_one(&state.pool)
            .await?;

            if !category_exists_for_user {
                return Err(AppError::CategoryNotFound); // Return CategoryNotFound if it doesn't exist or belong to user
            }
            deadline_to_update.category_id = Some(cat_id); // Update the category_id
         }
     }


    // Apply updates only if the field is provided in the payload
    if let Some(title) = payload.title {
        deadline_to_update.title = title;
    }
    if payload.description.is_some() { // Handle explicit null for description
        deadline_to_update.description = payload.description;
    }
    if let Some(due_date_str) = payload.due_date {
        let due_date = DateTime::parse_from_rfc3339(&due_date_str)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|_| AppError::ValidationFailed(validator::ValidationErrors::new()))?;
        deadline_to_update.due_date = due_date;
    }
    if let Some(priority) = payload.priority {
        deadline_to_update.priority = priority;
    }

    // Handle workload pair update: if either is Some, both must be Some (validated by validate_workload_pair)
    if payload.workload_magnitude.is_some() || payload.workload_unit.is_some() {
        deadline_to_update.workload_magnitude = payload.workload_magnitude;
        deadline_to_update.workload_unit = payload.workload_unit;
    } // If both are None, we don't update the existing values


    // Perform the update query
    let updated_deadline = sqlx::query_as!(
        Deadline,
        r#"
        UPDATE deadlines
        SET category_id = $1, title = $2, description = $3, due_date = $4, priority = $5, workload_magnitude = $6, workload_unit = $7
        WHERE deadline_id = $8 AND user_id = $9 -- Double-check user_id here again for safety
        RETURNING deadline_id, user_id, category_id as "category_id!: _", title, description as "description!: _", due_date, priority as "priority!: _", workload_magnitude as "workload_magnitude!: _", workload_unit as "workload_unit!: _", created_at as "created_at!", updated_at as "updated_at!"
        "#,
        deadline_to_update.category_id,
        deadline_to_update.title,
        deadline_to_update.description,
        deadline_to_update.due_date,
        deadline_to_update.priority as _,
        deadline_to_update.workload_magnitude,
        deadline_to_update.workload_unit as _,
        deadline_id,
        user_id // Crucial check
    )
    .fetch_one(&state.pool)
    .await?; // Propagates sqlx errors

    Ok(Json(updated_deadline))
}

// --- Delete Deadline ---
pub async fn delete_deadline(
    State(state): State<AppState>,
    AuthenticatedUser { user_id }: AuthenticatedUser,
    Path(deadline_id): Path<i32>,
) -> Result<StatusCode, AppError> {
    // Perform the delete query. Check for user_id!
    let delete_result = sqlx::query!(
        r#"
        DELETE FROM deadlines
        WHERE deadline_id = $1 AND user_id = $2
        "#,
        deadline_id,
        user_id // Ensure the deadline belongs to the authenticated user
    )
    .execute(&state.pool) // Use execute for DELETE
    .await?;

    // Check how many rows were affected
    if delete_result.rows_affected() == 0 {
        // No rows deleted means the deadline didn't exist or didn't belong to the user
        Err(AppError::DeadlineNotFound)
    } else {
        // Return 204 No Content on successful deletion
        Ok(StatusCode::NO_CONTENT)
    }
}