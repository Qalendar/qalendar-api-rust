use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use validator::ValidationErrors;

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
    // Add other specific errors as needed
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
            }
        };

        let body = Json(json!({ "error": error_message }));
        (status, body).into_response()
    }
}

// Convenience conversions using `?` operator
impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        // Basic check: could refine to differentiate connection vs query errors better if needed
        AppError::DatabaseError(e)
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