use axum::{
    extract::{State, Path, Json},
    http::StatusCode,
};
use sqlx::{PgPool, Transaction, Postgres, types::chrono::Utc};
use validator::Validate;
use crate::{
    AppState,
    errors::AppError,
    models::{
        calendar_share::{
            CalendarShare, CreateSharePayload, UpdateSharePayload,
            ShareDetailsResponse, ListSharesResponseItem, SharedWithUserDetail // Import response structs
        },
        enums::SharePrivacyLevel,
        user::{User, BasicUserInfo}, // Need to look up shared_with user by email
    },
    middleware::auth::AuthenticatedUser,
};
use chrono::DateTime; // For parsing date strings

// Re-use or create a shared helper for timestamp parsing
fn parse_timestamp(s: &str) -> Result<DateTime<Utc>, AppError> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| {
            tracing::warn!("Failed to parse timestamp '{}': {}", s, e);
            AppError::ValidationFailed(validator::ValidationErrors::new())
        })
}

// --- Helper: Check if share exists and is owned by the user ---
async fn check_share_ownership(pool: &PgPool, share_id: i32, owner_user_id: i32) -> Result<bool, AppError> {
    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM calendar_shares WHERE share_id = $1 AND owner_user_id = $2)",
        share_id,
        owner_user_id
    )
    .fetch_one(pool)
    .await?;
    Ok(exists.unwrap_or(false))
}

// --- Helper: Validate category IDs exist and belong to the owner ---
async fn validate_category_ids(pool: &PgPool, owner_user_id: i32, category_ids: &[i32]) -> Result<(), AppError> {
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


// --- Create Share (POST /api/me/shares) ---
pub async fn create_share(
    State(state): State<AppState>,
    AuthenticatedUser { user_id: owner_user_id }: AuthenticatedUser,
    Json(payload): Json<CreateSharePayload>,
) -> Result<(StatusCode, Json<ShareDetailsResponse>), AppError> { // Return ShareDetailsResponse
    payload.validate()?;

    let shared_with_user_email = payload.shared_with_user_email.unwrap(); // Required
    let category_ids = payload.category_ids.unwrap(); // Required, validated min_length=1
    let message = payload.message; // Optional
    // Privacy level defaults in DB if not provided, use payload value if present
    let privacy_level = payload.privacy_level.unwrap_or_default(); // Requires Default on ENUM
    let expires_at_str = payload.expires_at; // Optional expiry string

    // Parse expires_at date if provided
    let expires_at = match expires_at_str {
        Some(s) if !s.is_empty() => Some(parse_timestamp(&s)?),
        _ => None,
    };

    // 1. Find the user to share with by email
    let shared_with_user = sqlx::query_as!(
        BasicUserInfo,
        r#"
        SELECT
            user_id, display_name, email, email_verified as "email_verified!: _",
            created_at as "created_at!", updated_at as "updated_at!", deleted_at as "deleted_at!: _"
        FROM users
        WHERE email = $1
        "#,
        shared_with_user_email
    )
    .fetch_optional(&state.pool)
    .await?;

    let shared_with_user = match shared_with_user {
        Some(user) => user,
        None => {
            // Consider a specific error like AppError::SharedWithUserNotFound
            return Err(AppError::UserNotFound); // Re-using UserNotFound
        }
    };

    // Prevent sharing with oneself
    if shared_with_user.user_id == owner_user_id {
        // Consider a specific error like AppError::CannotShareWithSelf
        return Err(AppError::InternalServerError("Cannot share calendar with yourself".to_string()));
    }

    // 2. Check if a share already exists between these two users (owner -> shared_with)
    let share_exists: bool = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM calendar_shares WHERE owner_user_id = $1 AND shared_with_user_id = $2)",
        owner_user_id,
        shared_with_user.user_id
    )
    .fetch_one(&state.pool)
    .await?
    .unwrap_or(false);

    if share_exists {
        // Consider a specific error like AppError::ShareAlreadyExists
        return Err(AppError::InternalServerError("A share already exists with this user".to_string()));
    }

    // 3. Validate that the provided category IDs exist and belong to the owner
    validate_category_ids(&state.pool, owner_user_id, &category_ids).await?;


    // 4. Start a transaction
    let mut tx = state.pool.begin().await?;

    // 5. Insert into calendar_shares
    let created_share = sqlx::query_as!(
        CalendarShare,
        r#"
        INSERT INTO calendar_shares (owner_user_id, shared_with_user_id, message, privacy_level, expires_at)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING share_id, owner_user_id, shared_with_user_id, message as "message!: _",
        privacy_level as "privacy_level!: _", expires_at as "expires_at!: _",
        created_at as "created_at!", updated_at as "updated_at!", deleted_at as "deleted_at!: _"
        "#,
        owner_user_id,
        shared_with_user.user_id,
        message,
        privacy_level as SharePrivacyLevel,
        expires_at,
    )
    .fetch_one(&mut *tx) // Use the transaction with proper dereferencing
    .await?;

    let share_id = created_share.share_id;

    // 6. Insert into calendar_share_categories for each category ID
    // Use QueryBuilder for batch insert if possible, or loop
    for cat_id in &category_ids {
        sqlx::query!(
            r#"
            INSERT INTO calendar_share_categories (share_id, category_id)
            VALUES ($1, $2)
            "#,
            share_id,
            cat_id,
        )
        .execute(&mut *tx) // Use the transaction with proper dereferencing
        .await?;
    }

    // 7. Commit the transaction
    tx.commit().await?;

    // 8. Prepare Response (Fetch the created share with joined data)
    // This is similar to the GET by ID query
     let response_share = sqlx::query_as!(
        ShareDetailsResponse,
        r#"
        SELECT
            cs.share_id,
            cs.owner_user_id,
            cs.shared_with_user_id,
            cs.message as "message!: _", -- Explicit cast for Option<String>
            cs.privacy_level as "privacy_level!: _", -- Explicit cast for ENUM
            cs.expires_at as "expires_at!: _", -- Explicit cast for Option<DateTime<Utc>>
            cs.created_at as "created_at!", -- Explicit cast for DateTime<Utc>
            cs.updated_at as "updated_at!", -- Explicit cast for DateTime<Utc>
            cs.deleted_at as "deleted_at!: _", -- Explicit cast for Option<DateTime<Utc>>
            -- Shared With User Details (aliased)
            u.user_id AS user_id_alias, -- Alias matches struct field name
            u.display_name,
            u.email,
            -- Aggregated Category IDs
            ARRAY_AGG(csc.category_id) FILTER (WHERE csc.category_id IS NOT NULL) AS "shared_category_ids!: Vec<i32>" -- Explicit cast for Vec
        FROM calendar_shares cs
        JOIN users u ON cs.shared_with_user_id = u.user_id
        LEFT JOIN calendar_share_categories csc ON cs.share_id = csc.share_id
        WHERE cs.share_id = $1 -- Fetch the specific created share
        GROUP BY cs.share_id, u.user_id -- Group required for array_agg
        "#,
        share_id
    )
    .fetch_one(&state.pool) // Use the pool AFTER commit
    .await?;


    Ok((StatusCode::CREATED, Json(response_share)))
}


// --- List Shares (GET /api/me/shares) ---
// Returns a list of shares created by the authenticated user, including shared_with user and categories
pub async fn list_shares(
    State(state): State<AppState>,
    AuthenticatedUser { user_id: owner_user_id }: AuthenticatedUser,
) -> Result<Json<Vec<ListSharesResponseItem>>, AppError> { // Return ListSharesResponseItem

     let shares = sqlx::query_as!(
        ListSharesResponseItem, // Use the response struct
        r#"
        SELECT
            cs.share_id,
            cs.owner_user_id,
            cs.shared_with_user_id,
            cs.message as "message!: _", -- Explicit cast for Option<String>
            cs.privacy_level as "privacy_level!: _", -- Explicit cast for ENUM
            cs.expires_at as "expires_at!: _", -- Explicit cast for Option<DateTime<Utc>>
            cs.created_at as "created_at!", -- Explicit cast for DateTime<Utc>
            cs.updated_at as "updated_at!", -- Explicit cast for DateTime<Utc>
            cs.deleted_at as "deleted_at!: _", -- Explicit cast for Option<DateTime<Utc>>
            -- Shared With User Details (aliased)
            u.user_id AS user_id_alias, -- Alias matches struct field name
            u.display_name,
            u.email,
            -- Aggregated Category IDs
            ARRAY_AGG(csc.category_id) FILTER (WHERE csc.category_id IS NOT NULL) AS "shared_category_ids!: Vec<i32>" -- Explicit cast for Vec
        FROM calendar_shares cs
        JOIN users u ON cs.shared_with_user_id = u.user_id
        LEFT JOIN calendar_share_categories csc ON cs.share_id = csc.share_id
        WHERE cs.owner_user_id = $1 -- Filter by the owner user
        GROUP BY cs.share_id, u.user_id -- Group required for array_agg
        ORDER BY cs.created_at DESC -- Optional: order by creation date
        "#,
        owner_user_id
    )
    .fetch_all(&state.pool)
    .await?; // sqlx::Error -> AppError::DatabaseError

    Ok(Json(shares))
}

// --- Get Single Share (GET /api/me/shares/:share_id) ---
// Returns details for a specific share owned by the user
pub async fn get_share_by_id(
    State(state): State<AppState>,
    AuthenticatedUser { user_id: owner_user_id }: AuthenticatedUser,
    Path(share_id): Path<i32>,
) -> Result<Json<ShareDetailsResponse>, AppError> { // Return ShareDetailsResponse

    // Fetch the share with joined data and categories, filtering by owner_user_id
     let share = sqlx::query_as!(
        ShareDetailsResponse, // Use the response struct
        r#"
        SELECT
            cs.share_id,
            cs.owner_user_id,
            cs.shared_with_user_id,
            cs.message as "message!: _", -- Explicit cast for Option<String>
            cs.privacy_level as "privacy_level!: _", -- Explicit cast for ENUM
            cs.expires_at as "expires_at!: _", -- Explicit cast for Option<DateTime<Utc>>
            cs.created_at as "created_at!", -- Explicit cast for DateTime<Utc>
            cs.updated_at as "updated_at!", -- Explicit cast for DateTime<Utc>
            cs.deleted_at as "deleted_at!: _", -- Explicit cast for Option<DateTime<Utc>>
            -- Shared With User Details (aliased)
            u.user_id AS user_id_alias, -- Alias matches struct field name
            u.display_name,
            u.email,
            -- Aggregated Category IDs
            ARRAY_AGG(csc.category_id) FILTER (WHERE csc.category_id IS NOT NULL) AS "shared_category_ids!: Vec<i32>" -- Explicit cast for Vec
        FROM calendar_shares cs
        JOIN users u ON cs.shared_with_user_id = u.user_id
        LEFT JOIN calendar_share_categories csc ON cs.share_id = csc.share_id
        WHERE cs.share_id = $1 AND cs.owner_user_id = $2 -- IMPORTANT: Filter by ID AND owner
        GROUP BY cs.share_id, u.user_id -- Group required for array_agg
        "#,
        share_id,
        owner_user_id
    )
    .fetch_optional(&state.pool) // Use fetch_optional as it might not exist or belong to user
    .await?;

    match share {
        Some(s) => Ok(Json(s)),
        None => Err(AppError::ShareNotFound), // Return ShareNotFound error
    }
}


// --- Update Share (PUT /api/me/shares/:share_id) ---
pub async fn update_share(
    State(state): State<AppState>,
    AuthenticatedUser { user_id: owner_user_id }: AuthenticatedUser,
    Path(share_id): Path<i32>,
    Json(payload): Json<UpdateSharePayload>,
) -> Result<Json<ShareDetailsResponse>, AppError> {
    payload.validate()?;

    // 1. Start a transaction
    let mut tx = state.pool.begin().await?;

    // 2. Fetch existing share within the transaction to lock it (or just for data)
    // Using fetch_one within transaction is fine for getting initial data.
    // The UPDATE query below will actually acquire the row lock.
    let existing_share = sqlx::query_as!(
        CalendarShare,
        r#"
        SELECT
            share_id, owner_user_id, shared_with_user_id, message,
            privacy_level as "privacy_level!: _", expires_at as "expires_at!: _",
            created_at as "created_at!", updated_at as "updated_at!", deleted_at as "deleted_at!: _"
        FROM calendar_shares
        WHERE share_id = $1 AND owner_user_id = $2
        FOR UPDATE -- Add FOR UPDATE to explicitly lock the row for this transaction
        "#,
        share_id,
        owner_user_id
    )
    .fetch_optional(&mut *tx) // Use the transaction with proper dereferencing
    .await?;

    let mut share_to_update = match existing_share {
        Some(s) => s,
        None => {
            tx.rollback().await?; // Rollback the transaction if share not found
            return Err(AppError::ShareNotFound);
        }
    };

    // 3. Parse expires_at date string if provided in payload
    let mut updated_expires_at = share_to_update.expires_at;
    if payload.expires_at.is_some() || (payload.expires_at.is_none() && payload.expires_at.as_ref().is_some()) {
        updated_expires_at = match payload.expires_at {
            Some(s) if !s.is_empty() => Some(parse_timestamp(&s)?),
            _ => None, // Set to NULL if payload is None or empty string
        };
    }

    // Apply non-category/expiry updates only if the field is provided in the payload
    if payload.message.is_some() || (payload.message.is_none() && payload.message.as_ref().is_some()) {
        share_to_update.message = payload.message;
    }
    if let Some(privacy_level) = payload.privacy_level {
        share_to_update.privacy_level = privacy_level;
    }
    share_to_update.expires_at = updated_expires_at; // Apply updated expiry


    // 4. Handle Category Updates if provided in payload
    if let Some(category_ids) = payload.category_ids {
        // Validate the provided category IDs using the *main pool* (validation doesn't modify data,
        // doesn't need to be in the transaction, and using the pool avoids potential deadlocks
        // if validation queries needed to acquire locks)
        validate_category_ids(&state.pool, owner_user_id, &category_ids).await?; // Use &state.pool

        // Delete existing categories for this share *within the transaction*
        sqlx::query!("UPDATE calendar_share_categories SET deleted_at = NOW() WHERE share_id = $1", share_id)
            .execute(&mut *tx)
            .await?;

        // Insert the new set of category IDs *within the transaction*
        for cat_id in &category_ids {
            sqlx::query!(
                r#"
                INSERT INTO calendar_share_categories (share_id, category_id)
                VALUES ($1, $2)
                "#,
                share_id,
                cat_id,
            )
            .execute(&mut *tx)
            .await?;
        }
        // Note: If the category_ids vector was empty, we just deleted existing categories
        // and inserted none, correctly unsharing all.
    }
    // If payload.category_ids was None, we skip this block and leave categories unchanged.

    // 5. Perform the update query for the calendar_shares table *within the transaction*
    let updated_share_db = sqlx::query_as!(
        CalendarShare,
        r#"
        UPDATE calendar_shares
        SET
            message = $1,
            privacy_level = $2,
            expires_at = $3
            -- updated_at trigger handles timestamp
        WHERE share_id = $4 AND owner_user_id = $5 -- Double-check user_id here again for safety
        RETURNING share_id, owner_user_id, shared_with_user_id, message as "message!: _",
        privacy_level as "privacy_level!: _", expires_at as "expires_at!: _",
        created_at as "created_at!", updated_at as "updated_at!", deleted_at as "deleted_at!: _"
        "#,
        share_to_update.message,
        share_to_update.privacy_level as SharePrivacyLevel,
        share_to_update.expires_at,
        share_id,
        owner_user_id
    )
    .fetch_one(&mut *tx) // Use the transaction!
    .await?;


    // 6. Commit the transaction if all operations succeeded
    tx.commit().await?;

    // 7. Prepare Response (Fetch the *final* updated share with joined data *outside* the transaction)
    // This ensures we get the committed state, including updated category links.
    // This is the same query as GET by ID
    let response_share = sqlx::query_as!(
        ShareDetailsResponse,
        r#"
        SELECT
            cs.share_id, cs.owner_user_id, cs.shared_with_user_id, cs.message as "message!: _",
            cs.privacy_level as "privacy_level!: _", cs.expires_at as "expires_at!: _",
            cs.created_at as "created_at!", cs.updated_at as "updated_at!", cs.deleted_at as "deleted_at!: _",
            u.user_id AS user_id_alias, u.display_name, u.email,
            ARRAY_AGG(csc.category_id) FILTER (WHERE csc.category_id IS NOT NULL) AS "shared_category_ids!: Vec<i32>" -- Use FILTER for empty array
        FROM calendar_shares cs
        JOIN users u ON cs.shared_with_user_id = u.user_id
        LEFT JOIN calendar_share_categories csc ON cs.share_id = csc.share_id
        WHERE cs.share_id = $1 AND cs.owner_user_id = $2 -- Fetch the specific updated share
        GROUP BY cs.share_id, u.user_id
        "#,
        share_id,
        owner_user_id
    )
    .fetch_one(&state.pool) // Use the main pool AFTER commit
    .await?;


    Ok(Json(response_share))
}


// --- Delete Share (DELETE /api/me/shares/:share_id) ---
pub async fn delete_share(
    State(state): State<AppState>,
    AuthenticatedUser { user_id: owner_user_id }: AuthenticatedUser,
    Path(share_id): Path<i32>,
) -> Result<StatusCode, AppError> {
    // Perform the delete query. Check for user_id!
    // ON DELETE CASCADE on calendar_share_categories handles deleting those rows automatically
    let delete_result = sqlx::query!(
        r#"
        UPDATE calendar_shares
        SET deleted_at = NOW() -- Soft delete
        WHERE share_id = $1 AND owner_user_id = $2
        "#,
        share_id,
        owner_user_id
    )
    .execute(&state.pool)
    .await?;

    if delete_result.rows_affected() == 0 {
        // No rows deleted means the share didn't exist or didn't belong to the user
        Err(AppError::ShareNotFound) // Use ShareNotFound error
    } else {
        // Return 204 No Content on successful deletion
        Ok(StatusCode::NO_CONTENT)
    }
}