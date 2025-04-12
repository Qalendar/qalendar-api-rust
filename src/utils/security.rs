use crate::errors::AppError;
use bcrypt::{hash, verify, DEFAULT_COST};

pub async fn hash_password(password: &str) -> Result<String, AppError> {
    let password_str = password.to_string(); // Clone password for the blocking task

    // Hashing is CPU-intensive, run it in a blocking task
    tokio::task::spawn_blocking(move || {
        hash(&password_str, DEFAULT_COST).map_err(AppError::HashingError)
    })
    .await
    .map_err(|e| AppError::InternalServerError(format!("Hashing task failed: {}", e)))? // Handle join error
}

pub async fn verify_password(password: &str, hash: &str) -> Result<bool, AppError> {
    let password_str = password.to_string();
    let hash_str = hash.to_string();

    // Verification is also CPU-intensive
    tokio::task::spawn_blocking(move || {
        verify(&password_str, &hash_str).map_err(AppError::HashingError)
    })
    .await
    .map_err(|e| AppError::InternalServerError(format!("Verification task failed: {}", e)))?
}