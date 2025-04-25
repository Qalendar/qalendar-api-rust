use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use validator::ValidationErrors;
use async_openai::error::OpenAIError as AsyncOpenAIError;

#[derive(Debug)] // Allow printing the error during development
pub enum AppError {
    DatabaseConnectionError(sqlx::Error),
    DatabaseError(sqlx::Error),
    ValidationFailed(ValidationErrors),
    HashingError(bcrypt::BcryptError),
    JwtError(jsonwebtoken::errors::Error),
    InvalidCredentials,
    EmailInUse,
    UserNotFound, // More specific than InvalidCredentials sometimes
    ConfigurationError(String), // For config loading errors
    InternalServerError(String), // Catch-all for unexpected errors
    DeadlineNotFound,
    EventNotFound,
    CategoryNotFound,
    CategoryNameAlreadyExists, // For unique constraint violation
    ShareNotFound,         // For calendar_shares
    InvitationNotFound,    // For event_invitations
    CannotModifySharedItem, // Trying to edit/delete an item you don't own via a share
    CannotInviteToNonOwnedEvent, // Trying to invite to an event you don't own
    CannotRespondToNonInvitedEvent, // Trying to respond to an invitation you didn't receive
    UserAlreadyVerified,
    UserNotVerified, // User needs verification before action (e.g. reset)
    VerificationCodeInvalid,
    VerificationCodeExpired,
    ResetCodeInvalid, // Includes cases where code is missing, incorrect, or user email doesn't match code
    ResetCodeExpired,
    EmailSendingError(String), // Error specifically from the email service
    TfaCodeInvalid, // Invalid TFA code
    TfaAlreadyEnabled, // TFA is already enabled for the user
    TfaNotEnabled, // For cases where TFA is not enabled but required
    // Consider UserNotFound for when an email address isn't found for password reset/resend\
    OpenAIError(String), // <-- Add this
    FileUploadError(String), // For issues reading/processing uploaded files
    InvalidMultipartData(String), // For malformed multipart requests
}

// How AppError should be converted into an HTTP response
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::DatabaseConnectionError(e) => {
                tracing::error!("Database connection error: {:?}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Database connection failed".to_string())
            }
            AppError::DatabaseError(e) => {
                tracing::error!("Database query error: {:?}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "An internal database error occurred".to_string())
            }
            AppError::ValidationFailed(e) => {
                let errors = e.field_errors().into_iter()
                    .map(|(field, errors)| {
                        let messages = errors.iter().map(|e| e.message.as_ref().map(|s| s.to_string()).unwrap_or_else(|| "Invalid input".to_string())).collect::<Vec<_>>().join(", ");
                        format!("{}: {}", field, messages)
                    })
                    .collect::<Vec<_>>()
                    .join("; ");
                tracing::warn!("Validation failed: {}", errors);
                (StatusCode::BAD_REQUEST, format!("Validation failed: {}", errors))
            }
            AppError::HashingError(e) => {
                tracing::error!("Password hashing error: {:?}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Could not process password".to_string())
            }
             AppError::JwtError(e) => {
                tracing::error!("JWT error: {:?}", e);
                // Don't expose internal JWT details
                (StatusCode::UNAUTHORIZED, "Invalid or expired token".to_string())
            }
            AppError::InvalidCredentials => (StatusCode::UNAUTHORIZED, "Invalid email or password".to_string()),
            AppError::EmailInUse => (StatusCode::CONFLICT, "Email address is already in use".to_string()),
            AppError::UserNotFound => (StatusCode::NOT_FOUND, "User not found".to_string()), // Or Unauthorized for login
            AppError::ConfigurationError(msg) => {
                tracing::error!("Configuration Error: {}", msg);
                // This error usually happens at startup before serving requests,
                // but if it were to occur later, 500 is appropriate.
                (StatusCode::INTERNAL_SERVER_ERROR, format!("Server configuration error: {}", msg))
            },
            AppError::InternalServerError(msg) => {
                tracing::error!("Internal Server Error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, "An unexpected error occurred".to_string())
            },
            AppError::DeadlineNotFound => (StatusCode::NOT_FOUND, "Deadline not found".to_string()),
            AppError::EventNotFound => (StatusCode::NOT_FOUND, "Event not found".to_string()),
            AppError::CategoryNotFound => (StatusCode::NOT_FOUND, "Category not found".to_string()),
            AppError::ShareNotFound => (StatusCode::NOT_FOUND, "Share not found".to_string()),
            AppError::InvitationNotFound => (StatusCode::NOT_FOUND, "Invitation not found".to_string()),
            AppError::CategoryNameAlreadyExists => (StatusCode::CONFLICT, "A category with this name already exists".to_string()),
            AppError::CannotModifySharedItem => (StatusCode::FORBIDDEN, "Cannot modify item shared with you".to_string()),
            AppError::CannotInviteToNonOwnedEvent => (StatusCode::FORBIDDEN, "Cannot invite to an event you do not own".to_string()),
            AppError::CannotRespondToNonInvitedEvent => (StatusCode::FORBIDDEN, "Cannot respond to an invitation you did not receive".to_string()),
            AppError::UserAlreadyVerified => (StatusCode::CONFLICT, "User is already verified".to_string()),
            AppError::UserNotVerified => (StatusCode::FORBIDDEN, "User email not verified".to_string()),
            AppError::VerificationCodeInvalid => (StatusCode::BAD_REQUEST, "Invalid verification code".to_string()),
            AppError::VerificationCodeExpired => (StatusCode::BAD_REQUEST, "Verification code expired".to_string()),
            AppError::ResetCodeInvalid => (StatusCode::BAD_REQUEST, "Invalid reset code or email".to_string()), // Keep vague for security
            AppError::ResetCodeExpired => (StatusCode::BAD_REQUEST, "Reset code expired".to_string()),
            AppError::EmailSendingError(msg) => {
                 tracing::error!("Email sending failed: {}", msg);
                 (StatusCode::INTERNAL_SERVER_ERROR, "Failed to send email".to_string()) // Don't expose internal error message
            }
            AppError::TfaCodeInvalid => (StatusCode::BAD_REQUEST, "Invalid TFA code".to_string()), // Keep vague for security
            AppError::TfaAlreadyEnabled => (StatusCode::BAD_REQUEST, "TFA is already enabled".to_string()), // Keep vague for security
            AppError::TfaNotEnabled => (StatusCode::BAD_REQUEST, "TFA is not enabled".to_string()), // Keep vague for security
            // Use existing errors for cases like UserNotFound, InvalidCredentials, ValidationFailed
            // e.g., trying to resend verification email to non-existent email -> UserNotFound (404)
            AppError::OpenAIError(msg) => {
                tracing::error!("OpenAI API error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, "Failed to communicate with AI service".to_string()) // Don't expose internal details
           }
            AppError::FileUploadError(msg) => {
                tracing::warn!("File upload processing error: {}", msg);
                (StatusCode::BAD_REQUEST, format!("File processing failed: {}", msg))
           }
            AppError::InvalidMultipartData(msg) => {
                tracing::warn!("Invalid multipart data: {}", msg);
                (StatusCode::BAD_REQUEST, format!("Invalid request data: {}", msg))
           }
        };

        let body = Json(json!({ "error": error_message }));
        (status, body).into_response()
    }
}

// Convenience conversions using `?` operator
impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        // Check if it's a unique constraint violation for categories
        if let sqlx::Error::Database(db_error) = &e {
            // PostgreSQL unique violation error code is often "23505"
            // Check your database driver docs for exact code if needed
            if let Some(code) = db_error.code() {
                if code.as_ref() == "23505" {
                    // You might need more specific checks if other unique constraints exist
                    // For now, assume 23505 on category INSERT implies name conflict
                     tracing::warn!("Database Unique Constraint Error: {:?}", e);
                    return AppError::CategoryNameAlreadyExists;
                }
            }
        }
        tracing::error!("Unmapped Database Error: {:?}", e);
        AppError::DatabaseError(e) // Fallback to generic DB error
    }
}

impl From<ValidationErrors> for AppError {
    fn from(e: ValidationErrors) -> Self {
        AppError::ValidationFailed(e)
    }
}

impl From<bcrypt::BcryptError> for AppError {
    fn from(e: bcrypt::BcryptError) -> Self {
        AppError::HashingError(e)
    }
}

impl From<jsonwebtoken::errors::Error> for AppError {
    fn from(e: jsonwebtoken::errors::Error) -> Self {
        AppError::JwtError(e)
    }
}

impl From<AsyncOpenAIError> for AppError {
    fn from(e: AsyncOpenAIError) -> Self {
        AppError::OpenAIError(e.to_string()) // Convert to string for storing in our error variant
    }
}