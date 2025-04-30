// src/handlers/me_handler.rs
use axum::{
    extract::{State, Path, Json}, // Add Path for future parameterized endpoints under /me if needed
    http::StatusCode,
};
use sqlx::{Execute, PgPool};
use validator::Validate;
use crate::{
    AppState,
    errors::AppError,
    models::user::{
        User, UserData, // Import User and UserData
        UpdateUserPayload, DeleteUserPayload, // Import new payloads
    },
    utils::security::verify_password, // Need to verify password for deletion
    middleware::auth::AuthenticatedUser, // Import AuthenticatedUser
};
use chrono::{NaiveDate, Utc, DateTime}; // Import DateTime
use sqlx::Row; // Needed for query_scalar exists check

// Re-use find_user_by_email helper or ensure it's imported
// It's better to keep core user fetching logic like this centralized.
// Let's make find_user_by_email public in auth_handler.rs and import it.
// For now, copy it locally for illustration:
async fn find_user_by_id(pool: &PgPool, user_id: i32) -> Result<Option<User>, AppError> {
    sqlx::query_as!(
        User,
        r#"
        SELECT user_id, display_name, email, password_hash, date_of_birth as "date_of_birth!: _",
               email_verified as "email_verified!",
               verification_code, verification_code_expires_at as "verification_code_expires_at!: _",
               reset_code, reset_code_expires_at as "reset_code_expires_at!: _",
               created_at as "created_at!",
               updated_at as "updated_at!",
               deleted_at as "deleted_at!: _",
               tfa_enabled as "tfa_enabled!: _",
               tfa_secret as "tfa_secret!: _"
        FROM users WHERE user_id = $1
        "#,
        user_id
    )
    .fetch_optional(pool)
    .await
    .map_err(AppError::from) // sqlx::Error -> AppError
}


// --- GET /api/me handler (Modified) ---
// Returns details of the authenticated user
pub async fn get_authenticated_user_info(
    State(state): State<AppState>,
    AuthenticatedUser { user_id }: AuthenticatedUser,
) -> Result<Json<UserData>, AppError> { // Return UserData struct

    // Fetch user details from the DB
    let user = find_user_by_id(&state.pool, user_id).await?
        .ok_or(AppError::UserNotFound)?; // Should ideally never happen if auth succeeded

    // Map User DB model to UserData response struct
    let user_data = UserData {
        user_id: user.user_id,
        display_name: user.display_name,
        email: user.email,
        email_verified: user.email_verified,
        created_at: user.created_at,
        date_of_birth: user.date_of_birth,
        tfa_enabled: Some(user.tfa_enabled), // Include TFA status
    };

    tracing::info!("Authenticated user {} accessed /me.", user_id);

    Ok(Json(user_data))
}

// --- NEW: PUT /api/me handler ---
// Updates details of the authenticated user
pub async fn update_user_handler(
    State(state): State<AppState>,
    AuthenticatedUser { user_id }: AuthenticatedUser,
    Json(payload): Json<UpdateUserPayload>,
) -> Result<Json<UserData>, AppError> { // Return updated UserData
    payload.validate()?;

    // Fetch user to ensure they exist and are not deleted (if you disallow updates to deleted users)
    let user = find_user_by_id(&state.pool, user_id).await?
        .ok_or(AppError::UserNotFound)?; // Should not happen if auth succeeded

    if user.deleted_at.is_some() {
        tracing::warn!("Attempted update on soft-deleted user: {}", user_id);
        return Err(AppError::UserNotFound); // Treat deleted user as not found for updates
    }

    // Prepare update query parts
    let mut query_builder: sqlx::QueryBuilder<'_, sqlx::Postgres> = sqlx::QueryBuilder::new("UPDATE users SET ");
    let mut sep = ""; // Separator for adding fields

    // Apply updates only if the field is provided in the payload
    if let Some(display_name) = payload.display_name {
        query_builder.push(sep);
        query_builder.push("display_name = ");
        query_builder.push_bind(display_name);
        sep = ", ";
    }

    let mut updated_dob: Option<NaiveDate> = user.date_of_birth; // Start with current DOB
    // Handle DOB update: parse if string provided, or set to NULL if explicit null in payload
     if payload.dob.is_some() || (payload.dob.is_none() && payload.dob.as_ref().is_some()) {
         updated_dob = match payload.dob {
             Some(ref s) if !s.is_empty() => {
                 Some(NaiveDate::parse_from_str(&s, "%Y-%m-%d").map_err(|e| {
                     tracing::warn!("Invalid date format for DOB update: {}", e);
                      AppError::ValidationFailed(validator::ValidationErrors::new()) // Simple error
                 })?)
             },
             _ => None, // Set to NULL if payload is None or empty string
         };
     }
    // Only add DOB to query if it changed or was provided (either non-null or explicit null)
    // This logic is slightly tricky with Option. A simpler way might be to always set the field
    // if it was *present* in the payload JSON, even if its value is null.
    // Let's use the `is_some()` check to see if the field was present in the payload.
    if payload.dob.is_some() || (payload.dob.is_none() && payload.dob.as_ref().is_some()) {
         query_builder.push(sep);
         query_builder.push("date_of_birth = ");
         query_builder.push_bind(updated_dob);
         sep = ", ";
    }


    // If no fields were provided for update (e.g., empty JSON {}), return the current user data
    if sep.is_empty() {
        tracing::info!("No user fields provided for update for user {}", user_id);
        // Refetch the user to ensure returned data is up-to-date if needed, or just return current
         let updated_user = find_user_by_id(&state.pool, user_id).await?
            .ok_or(AppError::UserNotFound)?; // Re-fetch to get updated_at timestamp if trigger changes it

         let user_data = UserData {
            user_id: updated_user.user_id,
            display_name: updated_user.display_name,
            email: updated_user.email,
            email_verified: updated_user.email_verified,
            created_at: updated_user.created_at,
            date_of_birth: updated_user.date_of_birth,
            tfa_enabled: Some(updated_user.tfa_enabled),
        };
        return Ok(Json(user_data));
    }

    // Add explicit updated_at = NOW() if you don't rely solely on the trigger
    // query_builder.push(sep);
    // query_builder.push("updated_at = NOW()");

    // Add WHERE clause
    query_builder.push(" WHERE user_id = ");
    query_builder.push_bind(user_id);

    // Add RETURNING clause to get the updated user data
    query_builder.push(r#"
        RETURNING user_id, display_name, email, date_of_birth,
               email_verified as "email_verified!", created_at as "created_at!", updated_at as "updated_at!",
               deleted_at as "deleted_at!: _", tfa_enabled as "tfa_enabled!", tfa_secret
    "#);

    // Execute the update query
    // let updated_user = sqlx::query_as::<_, User>(&query_builder.build().sql())
    //     .fetch_one(&state.pool)
    //     .await?; // Propagates sqlx::Error

    let updated_user = query_builder
    .build_query_as::<User>()
    .fetch_one(&state.pool)
    .await?;

    // Map updated User DB model to UserData response struct
    let user_data = UserData {
        user_id: updated_user.user_id,
        display_name: updated_user.display_name,
        email: updated_user.email,
        email_verified: updated_user.email_verified,
        created_at: updated_user.created_at,
        date_of_birth: updated_user.date_of_birth,
        tfa_enabled: Some(updated_user.tfa_enabled),
    };

    tracing::info!("User {} updated profile.", user_id);

    Ok(Json(user_data))
}

// --- NEW: DELETE /api/me handler (Soft Delete User) ---
// Soft-deletes the authenticated user's account after password confirmation
pub async fn delete_user_handler(
    State(state): State<AppState>,
    AuthenticatedUser { user_id }: AuthenticatedUser,
    Json(payload): Json<DeleteUserPayload>,
) -> Result<StatusCode, AppError> { // Return 204 No Content on success
    payload.validate()?;
    let password = payload.password.unwrap(); // Required by validation

    // 1. Fetch user to verify password and check if already deleted
    let user = find_user_by_id(&state.pool, user_id).await?
        .ok_or(AppError::UserNotFound)?; // Should not happen

    // 2. Check if already soft-deleted
    if user.deleted_at.is_some() {
        tracing::warn!("Attempted to delete already soft-deleted user: {}", user_id);
        return Err(AppError::UserNotFound); // Treat as already gone/not found
    }

    // 3. Verify the provided password
    let is_valid_password = verify_password(&password, &user.password_hash).await?;
    if !is_valid_password {
        return Err(AppError::InvalidCredentials); // Use generic error for security
    }

    // 4. Perform the soft delete (UPDATE users SET deleted_at = NOW())
    // This will also set updated_at via the trigger.
    let update_result = sqlx::query!(
        r#"
        UPDATE users
        SET deleted_at = NOW()
        WHERE user_id = $1 AND deleted_at IS NULL -- Ensure only delete if not already deleted
        "#,
        user_id
    )
    .execute(&state.pool)
    .await?; // Propagates sqlx::Error

    // Check how many rows were affected (should be 1 if not already deleted)
    if update_result.rows_affected() == 0 {
         // Should not happen given the check above, but safety
         tracing::error!("Soft delete failed for user {} unexpectedly (0 rows affected).", user_id);
         return Err(AppError::InternalServerError("Failed to delete user account".to_string()));
    }

    tracing::info!("User {} soft-deleted their account.", user_id);

    // Consider invalidating the user's current JWT here if possible/desired
    // (Requires a JWT revocation mechanism, which we haven't implemented)
    // For now, the user will still have a valid token until expiry, but subsequent
    // requests will fail if handlers check for deleted_at IS NULL.

    Ok(StatusCode::NO_CONTENT) // 204 No Content
}