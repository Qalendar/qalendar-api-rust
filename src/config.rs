use crate::errors::AppError;
use std::env;

#[derive(Clone)]
pub struct Config {
    pub version: String, // Version 
    pub database_url: String,
    pub jwt_secret: String,
    pub server_address: String,
    pub jwt_expiration_seconds: i64,

    // Email Service Configuration
    pub smtp_server: String,
    pub smtp_port: u16, // Use u16 for port
    pub smtp_user: String, // Often the same as sender email
    pub smtp_password: String,
    pub sender_email: String,
    pub sender_name: String, // Friendly name for sender

    // Code Expiration Durations (in minutes)
    pub verification_code_expires_minutes: i64, // Use i64 for chrono::Duration
    pub reset_code_expires_minutes: i64,

    // Frontend Configuration
    pub frontend_url: String,

    // AI Configuration
    pub openai_api_key: String,

    pub openai_system_prompt: String, // System prompt for OpenAI API
}

impl Config {
    // Update function signature to return AppError
    pub fn from_env() -> Result<Self, AppError> {
        let version = env!("CARGO_PKG_VERSION").to_string(); // <-- Inject version here

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

        // Load Email Config
        let smtp_server = env::var("SMTP_SERVER")
            .map_err(|e| AppError::ConfigurationError(format!("Missing SMTP_SERVER: {}", e)))?;
        let smtp_port_str = env::var("SMTP_PORT")
            .map_err(|e| AppError::ConfigurationError(format!("Missing SMTP_PORT: {}", e)))?;
        let smtp_port = smtp_port_str.parse::<u16>()
            .map_err(|e| AppError::ConfigurationError(format!("Invalid SMTP_PORT format: {}", e)))?;
        let smtp_user = env::var("SMTP_USER")
            .map_err(|e| AppError::ConfigurationError(format!("Missing SMTP_USER: {}", e)))?;
        let smtp_password = env::var("SMTP_PASSWORD")
            .map_err(|e| AppError::ConfigurationError(format!("Missing SMTP_PASSWORD: {}", e)))?;
        let sender_email = env::var("SENDER_EMAIL")
            .map_err(|e| AppError::ConfigurationError(format!("Missing SENDER_EMAIL: {}", e)))?;
        let sender_name = env::var("SENDER_NAME")
            .unwrap_or_else(|_| "Qalendar App".to_string()); // Default sender name

        // Load Code Expiration Durations
        let verification_minutes_str = env::var("VERIFICATION_CODE_EXPIRES_MINUTES")
            .unwrap_or_else(|_| "30".to_string()); // Default 30 minutes
        let verification_code_expires_minutes = verification_minutes_str.parse::<i64>()
            .map_err(|e| AppError::ConfigurationError(format!("Invalid VERIFICATION_CODE_EXPIRES_MINUTES format: {}", e)))?;

        let reset_minutes_str = env::var("RESET_CODE_EXPIRES_MINUTES")
            .unwrap_or_else(|_| "15".to_string()); // Default 15 minutes
        let reset_code_expires_minutes = reset_minutes_str.parse::<i64>()
            .map_err(|e| AppError::ConfigurationError(format!("Invalid RESET_CODE_EXPIRES_MINUTES format: {}", e)))?;

         // --- Load Frontend URL ---
        let frontend_url = env::var("FRONTEND_URL")
            .map_err(|e| AppError::ConfigurationError(format!("Missing FRONTEND_URL: {}", e)))?;

        // --- Load OpenAI API Key ---
        let openai_api_key = env::var("OPENAI_API_KEY")
            .map_err(|e| AppError::ConfigurationError(format!("Missing OPENAI_API_KEY: {}", e)))?;

        // --- Load OpenAI System Prompt ---
        let openai_system_prompt = env::var("OPENAI_SYSTEM_PROMPT")
            .unwrap_or_else(|_| "You are a helpful assistant.".to_string()); // Default system prompt

        Ok(Self {
            version,
            database_url,
            jwt_secret,
            server_address,
            jwt_expiration_seconds,
            smtp_server,
            smtp_port,
            smtp_user,
            smtp_password,
            sender_email,
            sender_name,
            verification_code_expires_minutes,
            reset_code_expires_minutes,
            frontend_url,
            openai_api_key,
            openai_system_prompt,
        })
    }
}