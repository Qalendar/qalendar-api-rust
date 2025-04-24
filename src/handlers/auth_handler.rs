use axum::{extract::State, Json};
use validator::Validate; // Import the Validate trait
use crate::{
    errors::AppError,
    models::user::{RegisterUserPayload, LoginUserPayload, AuthResponse, UserData, User},
    utils::security::{hash_password, verify_password},
    auth::jwt::create_token,
    state::AppState,
};
use chrono::NaiveDate;

// --- Registration Handler ---
pub async fn register_user_handler(
    State(state): State<AppState>, // Extract combined state
    Json(payload): Json<RegisterUserPayload>,
) -> Result<Json<AuthResponse>, AppError> {
    let pool = &state.pool; // Access pool from state
    let config = &state.config; // Access config from state
    // ... rest of the handler logic ...
    payload.validate()?;
    let display_name = payload.display_name.unwrap();
    let email = payload.email.unwrap();
    let password = payload.password.unwrap();
    let dob_str = payload.dob;

    let date_of_birth = match dob_str {
        Some(s) if !s.is_empty() => {
            Some(NaiveDate::parse_from_str(&s, "%Y-%m-%d").map_err(|e| {
                tracing::warn!("Invalid date format for DOB: {}", e);
                 AppError::ValidationFailed(validator::ValidationErrors::new())
            })?)
        }
        _ => None,
    };

    let email_exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE email = $1)")
        .bind(&email)
        .fetch_one(pool) // Use pool from state
        .await?;

     if email_exists {
        return Err(AppError::EmailInUse);
    }

    let password_hash = hash_password(&password).await?;

    let insert_result = sqlx::query!(
        r#"
        INSERT INTO users (display_name, email, password_hash, date_of_birth)
        VALUES ($1, $2, $3, $4)
        RETURNING user_id, display_name, email, email_verified, created_at, date_of_birth
        "#,
        display_name,
        email,
        password_hash,
        date_of_birth,
    )
    .fetch_one(pool) // Use pool from state
    .await?;

    let token = create_token(insert_result.user_id, config)?; // Use config from state

    let user_data = UserData {
        user_id: insert_result.user_id,
        display_name: insert_result.display_name,
        email: insert_result.email,
        email_verified: insert_result.email_verified.unwrap_or(false),
        created_at: insert_result.created_at.unwrap(),
        date_of_birth: insert_result.date_of_birth,
    };

    let response = AuthResponse { token, user: user_data };
    Ok(Json(response))
}

// --- Login Handler ---
pub async fn login_user_handler(
    State(state): State<AppState>,
    Json(payload): Json<LoginUserPayload>,
) -> Result<Json<AuthResponse>, AppError> {
    let pool = &state.pool;
    let config = &state.config;
    // ... rest of handler logic ...
    payload.validate()?;
    let email = payload.email.unwrap();
    let password = payload.password.unwrap();

    let user = sqlx::query_as!(
        User,
        r#"
        SELECT user_id, display_name, email, password_hash, date_of_birth, 
               email_verified as "email_verified!", 
               created_at as "created_at!", 
               updated_at as "updated_at!",
               deleted_at as "deleted_at!: _"
        FROM users WHERE email = $1
        "#,
        email
    )
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::UserNotFound)?;

    // Explicitly "use" the field to satisfy the lint
    let _ = user.updated_at; // Read the value into a dead variable `_`

    let is_valid_password = verify_password(&password, &user.password_hash).await?;
    if !is_valid_password {
        return Err(AppError::InvalidCredentials);
    }

    let token = create_token(user.user_id, config)?; // Use config from state

    let user_data = UserData {
        user_id: user.user_id,
        display_name: user.display_name,
        email: user.email,
        email_verified: user.email_verified,
        created_at: user.created_at,
        date_of_birth: user.date_of_birth,
    };
    let response = AuthResponse { token, user: user_data };
    Ok(Json(response))
}