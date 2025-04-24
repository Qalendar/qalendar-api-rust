use axum::{
    extract::{State, Path, Json},
    http::StatusCode,
};
use sqlx::{PgPool, types::chrono::Utc};
use validator::Validate;
use crate::{
    AppState,
    errors::AppError,
    models::deadline::{Deadline, CreateDeadlinePayload, UpdateDeadlinePayload}, // Import deadline models
    middleware::auth::AuthenticatedUser,
};
use chrono::DateTime; // For parsing date strings
use crate::utils::calendar::parse_timestamp; // Import the helper function for parsing timestamps

use crate::models::enums::{DeadlinePriorityLevel, WorkloadUnitType}; // Import enums

// --- Create Deadline ---
pub async fn create_deadline(
    State(state): State<AppState>,
    AuthenticatedUser { user_id }: AuthenticatedUser,
    Json(mut payload): Json<CreateDeadlinePayload>, // Use 'mut' because we modify it slightly
) -> Result<(StatusCode, Json<Deadline>), AppError> {
    payload.validate()?;

    let title = payload.title.unwrap();
    let category_id = payload.category_id; // Option<i32> is fine
    let description = payload.description; // Option<String> is fine

    // Parse the due_date string into DateTime<Utc>
    let due_date_str = payload.due_date.unwrap(); // Required by validation
    let due_date = parse_timestamp(&due_date_str)?;

    // Priority defaults in the DB, use payload value if provided
    let priority = payload.priority.unwrap_or_default(); // Requires Default trait on ENUM

    let workload_magnitude = payload.workload_magnitude; // Option<i32>
    let workload_unit = payload.workload_unit; // Option<WorkloadUnitType>

    // Optional: Validate category_id exists and belongs to the user if provided
    if let Some(cat_id) = category_id {
       let category_exists: Option<bool> = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM categories WHERE category_id = $1 AND user_id = $2)",
            cat_id,
            user_id
        )
        .fetch_one(&state.pool)
        .await?;

        if category_exists != Some(true) {
             // Category ID is invalid or doesn't belong to the user
             // Consider a more specific error like AppError::InvalidCategoryId
             return Err(AppError::CategoryNotFound); // Re-using CategoryNotFound for now
        }
    }


    let virtual_due_date = match &payload.virtual_due_date {
        Some(date_str) => Some(parse_timestamp(date_str)?),
        None => None,
    };

    let created_deadline = sqlx::query_as!(
        Deadline,
        r#"
        INSERT INTO deadlines (user_id, category_id, title, description, due_date, virtual_due_date, priority,
        workload_magnitude, workload_unit)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING
           deadline_id, user_id, category_id, title, description, due_date, virtual_due_date, priority as "priority!: _",
           workload_magnitude as "workload_magnitude!: _", workload_unit as "workload_unit!: _",
           created_at as "created_at!", updated_at as "updated_at!", deleted_at as "deleted_at!: _"
        "#,
        user_id,
        category_id,
        title,
        description,
        due_date,
        virtual_due_date,
        priority as DeadlinePriorityLevel,
        workload_magnitude,
        workload_unit as Option<WorkloadUnitType>,
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
        SELECT
           deadline_id, user_id, category_id, title, description, due_date, virtual_due_date, priority as "priority!: _",
           workload_magnitude as "workload_magnitude!: _", workload_unit as "workload_unit!: _",
           created_at as "created_at!", updated_at as "updated_at!", deleted_at as "deleted_at!: _"
        FROM deadlines
        WHERE user_id = $1
        ORDER BY due_date -- Optional: order by due date
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
        SELECT
           deadline_id, user_id, category_id, title, description, due_date, virtual_due_date, priority as "priority!: _",
           workload_magnitude as "workload_magnitude!: _", workload_unit as "workload_unit!: _",
           created_at as "created_at!", updated_at as "updated_at!", deleted_at as "deleted_at!: _"
        FROM deadlines
        WHERE deadline_id = $1 AND user_id = $2 -- IMPORTANT: Check user_id!
        "#,
        deadline_id,
        user_id
    )
    .fetch_optional(&state.pool)
    .await?;

    match deadline {
        Some(d) => Ok(Json(d)),
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
    payload.validate()?;

    // Fetch existing deadline to check ownership and get current values
    let existing_deadline = sqlx::query_as!(
        Deadline,
        r#"
        SELECT
           deadline_id, user_id, category_id, title, description, due_date, virtual_due_date as "virtual_due_date!: _",
           priority as "priority!: _",
           workload_magnitude as "workload_magnitude!: _", workload_unit as "workload_unit!: _",
           created_at as "created_at!", updated_at as "updated_at!", deleted_at as "deleted_at!: _"
        FROM deadlines
        WHERE deadline_id = $1 AND user_id = $2
        "#,
        deadline_id,
        user_id
    )
    .fetch_optional(&state.pool)
    .await?;

    let mut deadline_to_update = match existing_deadline {
        Some(d) => d,
        None => return Err(AppError::DeadlineNotFound),
    };

    // Apply updates only if the field is provided in the payload
    if let Some(title) = payload.title {
        deadline_to_update.title = title;
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
    } else {
        // If category_id is not provided, keep the existing one
        deadline_to_update.category_id = deadline_to_update.category_id;
    }
    // If description is explicitly set to null in JSON, it should become None
    if payload.description.is_some() || (payload.description.is_none() && payload.description.as_ref().is_some()) {
        deadline_to_update.description = payload.description;
    }

    if let Some(due_date_str) = payload.due_date {
        deadline_to_update.due_date = parse_timestamp(&due_date_str)?;
    }
    if let Some(priority) = payload.priority {
        deadline_to_update.priority = priority;
    }
    // Handle workload updates carefully: they must be updated together
    if payload.workload_magnitude.is_some() || payload.workload_unit.is_some() {
        // Validation chk_workload_update already ensures both are Some or both None if either is provided
        // So, if we reach here, either both are Some or both are None.
        // If both are Some, use them. If both are None, set both to None.
        deadline_to_update.workload_magnitude = payload.workload_magnitude;
        deadline_to_update.workload_unit = payload.workload_unit;
    }
     // If neither workload_magnitude nor workload_unit was in the payload JSON at all,
     // their Options will be None, and we don't overwrite deadline_to_update.workload_magnitude/unit.
     // If one was in the payload but the other wasn't, payload.validate() already caught it.
     // If both were in the payload and were nulls, they become Option::None, and we set deadline_to_update.workload_magnitude/unit to None.


    // Perform the update query
    let updated_deadline = sqlx::query_as!(
        Deadline,
        r#"
        UPDATE deadlines
        SET
            category_id = $1,
            title = $2,
            description = $3,
            due_date = $4,
            virtual_due_date = $5,
            priority = $6,
            workload_magnitude = $7,
            workload_unit = $8
            -- updated_at trigger handles timestamp
        WHERE deadline_id = $9 AND user_id = $10 -- Double-check user_id here again for safety
        RETURNING
           deadline_id, user_id, category_id, title, description, due_date, virtual_due_date as "virtual_due_date!: _",
           priority as "priority!: _",
           workload_magnitude as "workload_magnitude!: _", workload_unit as "workload_unit!: _",
           created_at as "created_at!", updated_at as "updated_at!", deleted_at as "deleted_at!: _"
        "#,
        deadline_to_update.category_id,
        deadline_to_update.title,
        deadline_to_update.description,
        deadline_to_update.due_date,
        deadline_to_update.virtual_due_date,
        deadline_to_update.priority as DeadlinePriorityLevel,
        deadline_to_update.workload_magnitude,
        deadline_to_update.workload_unit as Option<WorkloadUnitType>,
        deadline_id,
        user_id // Crucial check
    )
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(updated_deadline))
}

// --- Delete Deadline ---
pub async fn delete_deadline(
    State(state): State<AppState>,
    AuthenticatedUser { user_id }: AuthenticatedUser,
    Path(deadline_id): Path<i32>,
) -> Result<StatusCode, AppError> {
    let delete_result = sqlx::query!(
        r#"
        UPDATE deadlines
        SET deleted_at = NOW() -- Soft delete
        WHERE deadline_id = $1 AND user_id = $2
        "#,
        deadline_id,
        user_id
    )
    .execute(&state.pool)
    .await?;

    if delete_result.rows_affected() == 0 {
        Err(AppError::DeadlineNotFound)
    } else {
        Ok(StatusCode::NO_CONTENT)
    }
}