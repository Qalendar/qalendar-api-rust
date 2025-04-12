use crate::errors::AppError;
use std::env;

#[derive(Clone)]
pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub server_address: String,
    pub jwt_expiration_seconds: i64,
}

impl Config {
    // Update function signature to return AppError
    pub fn from_env() -> Result<Self, AppError> {
        let database_url = env::var("DATABASE_URL")
            .map_err(|e| AppError::ConfigurationError(format!("Missing DATABASE_URL: {}", e)))?;

        let jwt_secret = env::var("JWT_SECRET")
             .map_err(|e| AppError::ConfigurationError(format!("Missing JWT_SECRET: {}", e)))?;

        let server_address = env::var("SERVER_ADDRESS")
            .unwrap_or_else(|_| "0.0.0.0:8000".to_string()); // Default is fine here

        let jwt_expiration_str = env::var("JWT_EXPIRATION_SECONDS")
            .unwrap_or_else(|_| "86400".to_string()); // Default is fine

        let jwt_expiration_seconds = jwt_expiration_str.parse::<i64>()
            .map_err(|e| AppError::ConfigurationError(format!("Invalid JWT_EXPIRATION_SECONDS format: {}", e)))?;

        Ok(Self {
            database_url,
            jwt_secret,
            server_address,
            jwt_expiration_seconds,
        })
    }
}