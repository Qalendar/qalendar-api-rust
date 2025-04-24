// src/handlers/auth_handler.rs
use axum::{extract::{State, Json}, http::StatusCode}; // Added StatusCode
use validator::Validate;
use crate::{
    errors::AppError,
    models::user::{
        RegisterUserPayload, LoginUserPayload, AuthResponse, UserData, User,
        VerifyEmailPayload, ResendVerificationEmailPayload, ForgotPasswordPayload, ResetPasswordPayload, // Import new payloads
    },
    utils::security::{hash_password, verify_password, generate_secure_code, hash_code, verify_code}, // Import code helpers
    auth::jwt::create_token,
    state::AppState,
    email::EmailService, // Import EmailService
};
use chrono::{NaiveDate, Utc, Duration, DateTime}; // Added DateTime
use sqlx::PgPool; // For type hints

// Re-use parse_timestamp helper or ensure it's imported from utils
// fn parse_timestamp(s: &str) -> Result<DateTime<Utc>, AppError> { ... }


// --- Helper to find user by email ---
async fn find_user_by_email(pool: &PgPool, email: &str) -> Result<Option<User>, AppError> {
    sqlx::query_as!(
        User,
        r#"
        SELECT user_id, display_name, email, password_hash, date_of_birth,
               email_verified as "email_verified!",
               verification_code, verification_code_expires_at,
               reset_code, reset_code_expires_at,
               created_at as "created_at!",
               updated_at as "updated_at!",
               deleted_at as "deleted_at!: _"
        FROM users WHERE email = $1
        "#,
        email
    )
    .fetch_optional(pool)
    .await
    .map_err(AppError::from) // sqlx::Error -> AppError
}


// --- Registration Handler (Modified) ---
pub async fn register_user_handler(
    State(state): State<AppState>,
    Json(payload): Json<RegisterUserPayload>,
) -> Result<Json<AuthResponse>, AppError> {
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

    // Check if email already exists (including soft-deleted users?)
    // For registration, let's check only non-deleted emails for uniqueness
    let email_exists: bool = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM users WHERE email = $1 AND deleted_at IS NULL)",
        &email
    )
    .fetch_one(&state.pool)
    .await?
    .unwrap_or(false);

     if email_exists {
        return Err(AppError::EmailInUse);
    }

    let password_hash = hash_password(&password).await?;

    // --- Email Verification Code Generation ---
    let verification_code = generate_secure_code(32); // Generate a random code
    let verification_code_hash = hash_code(&verification_code).await?; // Hash the code
    let verification_code_expires_at = Utc::now() + Duration::minutes(state.config.verification_code_expires_minutes);


    // Insert User (including verification details)
    let insert_result = sqlx::query!(
        r#"
        INSERT INTO users (display_name, email, password_hash, date_of_birth, email_verified, verification_code, verification_code_expires_at)
        VALUES ($1, $2, $3, $4, FALSE, $5, $6) -- email_verified is FALSE by default, but explicit is clear
        RETURNING user_id, display_name, email, email_verified, created_at, date_of_birth
        "#,
        display_name,
        email,
        password_hash,
        date_of_birth,
        verification_code_hash,
        verification_code_expires_at,
    )
    .fetch_one(&state.pool)
    .await?;

    // --- Send Verification Email (Run in background or await?) ---
    // Awaiting is simpler and safer for critical flows like registration.
    // If email sending fails, the user should know.
    let send_email_result = state.email_service.send_verification_email(&email, &verification_code).await;

    // Handle email sending failure - User created, but email failed.
    // We can still return success for the API call, but log the error.
    // Or, decide email sending MUST succeed for registration to complete (stricter).
    // Let's allow registration but report email failure if it happens.
    if let Err(e) = send_email_result {
        tracing::error!("Failed to send verification email for user {}: {:?}", insert_result.user_id, e);
        // Depending on policy, you might want to return a 500 or partial success with warning
        // For now, we'll just log and proceed to return the user/token assuming registration succeeded
        // even if email failed. The user can use /resend later.
        // If you want to return an error: return Err(e);
    }


    let user_data = UserData {
        user_id: insert_result.user_id,
        display_name: insert_result.display_name,
        email: insert_result.email,
        email_verified: insert_result.email_verified.unwrap_or(false), // Should be false
        created_at: insert_result.created_at.unwrap(), // Should not be null
        date_of_birth: insert_result.date_of_birth,
    };

    let token = create_token(user_data.user_id, &state.config)?;

    let response = AuthResponse { token, user: user_data };
    Ok(Json(response))
}

// --- Login Handler (Modified) ---
pub async fn login_user_handler(
    State(state): State<AppState>,
    Json(payload): Json<LoginUserPayload>,
) -> Result<Json<AuthResponse>, AppError> {
    payload.validate()?;
    let email = payload.email.unwrap();
    let password = payload.password.unwrap();

    // Find user by email (fetch all fields needed for verification and soft delete check)
    let user = find_user_by_email(&state.pool, &email)
        .await?
        .ok_or(AppError::InvalidCredentials)?; // Use generic invalid credentials for login

    // Check if user is soft-deleted
    if user.deleted_at.is_some() {
         tracing::warn!("Attempted login for soft-deleted user: {}", user.user_id);
         return Err(AppError::InvalidCredentials); // Treat soft-deleted as non-existent for login
    }

    let is_valid_password = verify_password(&password, &user.password_hash).await?;
    if !is_valid_password {
        return Err(AppError::InvalidCredentials);
    }

    // Optional: Enforce email verification on login
    // if !user.email_verified {
    //     tracing::warn!("Login attempt by unverified email: {}", user.email);
    //     return Err(AppError::UserNotVerified); // Return specific error if verification is mandatory for login
    // }


    let token = create_token(user.user_id, &state.config)?;

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


// --- NEW: Verify Email Handler (POST /api/auth/verify-email) ---
pub async fn verify_email_handler(
    State(state): State<AppState>,
    Json(payload): Json<VerifyEmailPayload>,
) -> Result<StatusCode, AppError> { // Return 204 No Content on success

    payload.validate()?;
    let email = payload.email.unwrap();
    let code = payload.code.unwrap();

    // 1. Find user by email (fetch verification fields)
    let user = find_user_by_email(&state.pool, &email).await?;

    let mut user = match user {
        Some(u) => u,
        None => return Err(AppError::UserNotFound),
    };

    // 2. Check if already verified
    if user.email_verified {
        return Err(AppError::UserAlreadyVerified);
    }

    // 3. Check if verification code and expiry exist
    let stored_code_hash = match user.verification_code {
        Some(hash) => hash,
        None => {
            tracing::warn!("Verification attempt with no code stored for user: {}", user.user_id);
             return Err(AppError::VerificationCodeInvalid); // No code was ever generated or already used
        }
    };

    let expires_at = match user.verification_code_expires_at {
        Some(ts) => ts,
        None => {
            tracing::error!("Verification code expiry missing for user: {}", user.user_id);
             return Err(AppError::InternalServerError("Verification code expiry missing".to_string())); // Should not happen if code is stored
        }
    };

    // 4. Check code expiry
    if Utc::now() > expires_at {
        tracing::warn!("Verification code expired for user: {}", user.user_id);
        // Optional: Clear the code/expiry on expiry check failure
        // let _ = sqlx::query!("UPDATE users SET verification_code = NULL, verification_code_expires_at = NULL WHERE user_id = $1", user.user_id)
        //     .execute(&state.pool).await;
        return Err(AppError::VerificationCodeExpired);
    }

    // 5. Verify code against hash
    let is_valid_code = verify_code(&code, &stored_code_hash).await?;

    if !is_valid_code {
        tracing::warn!("Invalid verification code attempt for user: {}", user.user_id);
        // Optional: Increment failed attempts or clear code after N failures
        return Err(AppError::VerificationCodeInvalid);
    }

    // 6. Mark email as verified and clear verification fields
    sqlx::query!(
        r#"
        UPDATE users
        SET email_verified = TRUE, verification_code = NULL, verification_code_expires_at = NULL
        WHERE user_id = $1
        "#,
        user.user_id
    )
    .execute(&state.pool)
    .await?;

    tracing::info!("Email verified for user: {}", user.user_id);

    Ok(StatusCode::NO_CONTENT) // 204 No Content indicates success with no body
}

// --- NEW: Resend Verification Email Handler (POST /api/auth/resend-verification-email) ---
pub async fn resend_verification_email_handler(
    State(state): State<AppState>,
    Json(payload): Json<ResendVerificationEmailPayload>,
) -> Result<StatusCode, AppError> { // Return 204 No Content on success

    payload.validate()?;
    let email = payload.email.unwrap();

    // 1. Find user by email
    let user = find_user_by_email(&state.pool, &email).await?;

    let mut user = match user {
        Some(u) => u,
        None => return Err(AppError::UserNotFound), // Don't confirm user existence for security? Or just return 204 always? Let's return 404.
    };

    // 2. Check if already verified
    if user.email_verified {
        return Err(AppError::UserAlreadyVerified);
    }

    // 3. Generate a *new* verification code and expiry
    let new_verification_code = generate_secure_code(32);
    let new_verification_code_hash = hash_code(&new_verification_code).await?;
    let new_verification_code_expires_at = Utc::now() + Duration::minutes(state.config.verification_code_expires_minutes);


    // 4. Update the user record with the new code and expiry
     sqlx::query!(
        r#"
        UPDATE users
        SET verification_code = $1, verification_code_expires_at = $2, updated_at = NOW() -- Explicitly update updated_at
        WHERE user_id = $3
        "#,
        new_verification_code_hash,
        new_verification_code_expires_at,
        user.user_id
    )
    .execute(&state.pool)
    .await?;


    // 5. Send the new verification email
    state.email_service.send_verification_email(&email, &new_verification_code).await?;

    tracing::info!("Resent verification email for user: {}", user.user_id);

    Ok(StatusCode::NO_CONTENT)
}

// --- NEW: Forgot Password Handler (POST /api/auth/forgot-password) ---
pub async fn forgot_password_handler(
    State(state): State<AppState>,
    Json(payload): Json<ForgotPasswordPayload>,
) -> Result<StatusCode, AppError> { // Return 204 No Content (usually, even if email not found, for security)

    payload.validate()?;
    let email = payload.email.unwrap();

    // 1. Find user by email (fetch reset fields and email_verified)
    let user = find_user_by_email(&state.pool, &email).await?;

    let user = match user {
        Some(u) => u,
        // For security, always return 204 or generic message, don't confirm email existence
        // If you want to be strict and tell the user email wasn't found: return Err(AppError::UserNotFound);
        // Let's return 204 always for production-like behaviour.
        None => {
             tracing::warn!("Forgot password requested for non-existent email: {}", email);
             // Still return 204 OK to avoid leaking info
             return Ok(StatusCode::NO_CONTENT);
        }
    };

    // Optional: Require email to be verified before allowing password reset
    // if !user.email_verified {
    //     tracing::warn!("Forgot password requested for unverified email: {}", user.email);
    //     // Again, consider returning 204 OK or a specific error
    //     return Err(AppError::UserNotVerified);
    // }


    // 2. Generate a new reset code and expiry
    let new_reset_code = generate_secure_code(32); // Use sufficient length
    let new_reset_code_hash = hash_code(&new_reset_code).await?;
    let new_reset_code_expires_at = Utc::now() + Duration::minutes(state.config.reset_code_expires_minutes);


    // 3. Update the user record with the new code and expiry
    sqlx::query!(
        r#"
        UPDATE users
        SET reset_code = $1, reset_code_expires_at = $2, updated_at = NOW() -- Explicitly update updated_at
        WHERE user_id = $3
        "#,
        new_reset_code_hash,
        new_reset_code_expires_at,
        user.user_id
    )
    .execute(&state.pool)
    .await?;


    // 4. Send the password reset email
    // We should send the *original*, non-hashed code here
    state.email_service.send_password_reset_email(&email, &new_reset_code).await?;

    tracing::info!("Password reset email sent for user: {}", user.user_id);

    // Always return 204 for security, even if email didn't exist or sending failed (log the failure)
    Ok(StatusCode::NO_CONTENT)
}

// --- NEW: Reset Password Handler (POST /api/auth/reset-password) ---
pub async fn reset_password_handler(
    State(state): State<AppState>,
    Json(payload): Json<ResetPasswordPayload>,
) -> Result<StatusCode, AppError> { // Return 204 No Content on success

    payload.validate()?;
    let email = payload.email.unwrap();
    let code = payload.code.unwrap(); // The plain text code from the user
    let new_password = payload.new_password.unwrap();

    // 1. Find user by email (fetch reset fields)
    let user = find_user_by_email(&state.pool, &email).await?;

    let mut user = match user {
        Some(u) => u,
        None => {
             tracing::warn!("Password reset attempt with non-existent email: {}", email);
             return Err(AppError::ResetCodeInvalid); // Generic invalid code/email error
        }
    };

    // 2. Check if reset code and expiry exist
    let stored_code_hash = match user.reset_code {
        Some(hash) => hash,
        None => {
            tracing::warn!("Password reset attempt with no code stored for user: {}", user.user_id);
             return Err(AppError::ResetCodeInvalid); // No code was ever generated or already used
        }
    };

    let expires_at = match user.reset_code_expires_at {
        Some(ts) => ts,
        None => {
            tracing::error!("Password reset code expiry missing for user: {}", user.user_id);
             return Err(AppError::InternalServerError("Reset code expiry missing".to_string())); // Should not happen if code is stored
        }
    };

    // 3. Check code expiry
    if Utc::now() > expires_at {
        tracing::warn!("Reset code expired for user: {}", user.user_id);
        // Optional: Clear code/expiry on expiry check failure
        // let _ = sqlx::query!("UPDATE users SET reset_code = NULL, reset_code_expires_at = NULL WHERE user_id = $1", user.user_id)
        //     .execute(&state.pool).await;
        return Err(AppError::ResetCodeExpired);
    }

    // 4. Verify code against hash
    let is_valid_code = verify_code(&code, &stored_code_hash).await?;

    if !is_valid_code {
         tracing::warn!("Invalid reset code attempt for user: {}", user.user_id);
         // Optional: Increment failed attempts or clear code after N failures
        return Err(AppError::ResetCodeInvalid);
    }

    // 5. Hash the new password
    let new_password_hash = hash_password(&new_password).await?;

    // 6. Update password hash and clear reset fields
    sqlx::query!(
        r#"
        UPDATE users
        SET password_hash = $1, reset_code = NULL, reset_code_expires_at = NULL, updated_at = NOW() -- Explicitly update updated_at
        WHERE user_id = $2
        "#,
        new_password_hash,
        user.user_id
    )
    .execute(&state.pool)
    .await?;

    tracing::info!("Password reset successful for user: {}", user.user_id);

    Ok(StatusCode::NO_CONTENT) // 204 No Content indicates success
}