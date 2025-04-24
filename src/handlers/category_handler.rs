use axum::{
    extract::{State, Path, Json},
    http::StatusCode,
};
use sqlx;
use validator::Validate;
use crate::{
    AppState,
    errors::AppError,
    models::category::{Category, CreateCategoryPayload, UpdateCategoryPayload},
    middleware::auth::AuthenticatedUser, // Import the AuthenticatedUser extractor
};

// --- Create Category ---
pub async fn create_category(
    State(state): State<AppState>,
    AuthenticatedUser { user_id }: AuthenticatedUser, // Extract authenticated user ID
    Json(payload): Json<CreateCategoryPayload>,
) -> Result<(StatusCode, Json<Category>), AppError> {
    payload.validate()?; // Validate the input payload

    let name = payload.name.unwrap(); // Safe unwrap after validation
    let color = payload.color.unwrap(); // Safe unwrap after validation

    // is_visible defaults to true in the database schema, so we don't need it in payload for create

    let created_category = sqlx::query_as!(
        Category,
        r#"
        INSERT INTO categories (user_id, name, color)
        VALUES ($1, $2, $3)
        RETURNING category_id, user_id, name, color, is_visible as "is_visible!",
        created_at as "created_at!", updated_at as "updated_at!", deleted_at as "deleted_at!: _"
        "#,
        user_id, // Use the authenticated user_id
        name,
        color,
    )
    .fetch_one(&state.pool) // Use the pool from state
    .await?; // sqlx::Error is automatically mapped to AppError

    // Return 201 Created status code with the created category
    Ok((StatusCode::CREATED, Json(created_category)))
}

// --- Get All Categories for User ---
pub async fn get_categories(
    State(state): State<AppState>,
    AuthenticatedUser { user_id }: AuthenticatedUser, // Extract authenticated user ID
) -> Result<Json<Vec<Category>>, AppError> {
    let categories = sqlx::query_as!(
        Category,
        r#"
        SELECT category_id, user_id, name, color, is_visible as "is_visible!",
        created_at as "created_at!", updated_at as "updated_at!", deleted_at as "deleted_at!: _"
        FROM categories
        WHERE user_id = $1
        ORDER BY name -- Optional: order alphabetically
        "#,
        user_id // Fetch categories for the authenticated user
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(categories))
}

// --- Get Single Category by ID for User ---
pub async fn get_category_by_id(
    State(state): State<AppState>,
    AuthenticatedUser { user_id }: AuthenticatedUser, // Extract authenticated user ID
    Path(category_id): Path<i32>, // Extract category_id from the path
) -> Result<Json<Category>, AppError> {
    let category = sqlx::query_as!(
        Category,
        r#"
        SELECT category_id, user_id, name, color, is_visible as "is_visible!",
        created_at as "created_at!", updated_at as "updated_at!", deleted_at as "deleted_at!: _"
        FROM categories
        WHERE category_id = $1 AND user_id = $2 -- IMPORTANT: Check user_id!
        "#,
        category_id,
        user_id // Ensure the category belongs to the authenticated user
    )
    .fetch_optional(&state.pool) // Use fetch_optional because it might not exist or belong to user
    .await?;

    match category {
        Some(cat) => Ok(Json(cat)),
        None => Err(AppError::CategoryNotFound), // Return CategoryNotFound error
    }
}

// --- Update Category ---
pub async fn update_category(
    State(state): State<AppState>,
    AuthenticatedUser { user_id }: AuthenticatedUser, // Extract authenticated user ID
    Path(category_id): Path<i32>, // Extract category_id from the path
    Json(payload): Json<UpdateCategoryPayload>,
) -> Result<Json<Category>, AppError> {
    payload.validate()?; // Validate the input payload

    // We need to check if the category exists AND belongs to the user first
    let existing_category = sqlx::query_as!(
        Category,
        r#"
        SELECT category_id, user_id, name, color, is_visible as "is_visible!",
        created_at as "created_at!", updated_at as "updated_at!", deleted_at as "deleted_at!: _"
        FROM categories
        WHERE category_id = $1 AND user_id = $2
        "#,
        category_id,
        user_id
    )
    .fetch_optional(&state.pool)
    .await?;

    let mut category_to_update = match existing_category {
        Some(cat) => cat,
        None => return Err(AppError::CategoryNotFound),
    };

    // Apply updates only if the field is provided in the payload
    if let Some(name) = payload.name {
        category_to_update.name = name;
    }
    if let Some(color) = payload.color {
        category_to_update.color = color;
    }
    // if let Some(is_visible) = payload.is_visible {
    //     category_to_update.is_visible = is_visible;
    // }

    // Perform the update query
    let updated_category = sqlx::query_as!(
        Category,
        r#"
        UPDATE categories
        SET name = $1, color = $2, is_visible = $3 -- updated_at trigger handles timestamp
        WHERE category_id = $4 AND user_id = $5 -- Double-check user_id here again for safety
        RETURNING category_id, user_id, name, color, is_visible as "is_visible!",
        created_at as "created_at!", updated_at as "updated_at!", deleted_at as "deleted_at!: _"
        "#,
        category_to_update.name,
        category_to_update.color,
        category_to_update.is_visible,
        category_id,
        user_id // Crucial check
    )
    .fetch_one(&state.pool)
    .await?; // Propagates sqlx errors (including unique constraint for name)

    Ok(Json(updated_category))
}

// --- Delete Category ---
pub async fn delete_category(
    State(state): State<AppState>,
    AuthenticatedUser { user_id }: AuthenticatedUser, // Extract authenticated user ID
    Path(category_id): Path<i32>, // Extract category_id from the path
) -> Result<StatusCode, AppError> {
    // Perform the delete query. Check for user_id!
    let delete_result = sqlx::query!(
        r#"
        UPDATE categories
        SET deleted_at = NOW() -- Soft delete
        WHERE category_id = $1 AND user_id = $2
        "#,
        category_id,
        user_id // Ensure the category belongs to the authenticated user
    )
    .execute(&state.pool) // Use execute for DELETE
    .await?;

    // Check how many rows were affected
    if delete_result.rows_affected() == 0 {
        // No rows deleted means the category didn't exist or didn't belong to the user
        Err(AppError::CategoryNotFound)
    } else {
        // Return 204 No Content on successful deletion
        Ok(StatusCode::NO_CONTENT)
    }
}