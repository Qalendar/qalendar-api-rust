use crate::errors::AppError;
use bcrypt::{hash, verify, DEFAULT_COST};
use rand::Rng;
use rand::distr::Alphanumeric;

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

// --- New: Generate a secure random code string ---
pub fn generate_secure_code(length: usize) -> String {
    let mut rng = rand::rng();
    let s: String = (&mut rng).sample_iter(Alphanumeric)
    .take(length) // Take the specified number of characters
    .map(char::from) // Convert to char
    .collect();

    s
}

// --- New: Hash a verification or reset code ---
// Can reuse hash_password as bcrypt works fine for this
pub async fn hash_code(code: &str) -> Result<String, AppError> {
    hash_password(code).await // Bcrypt cost is reasonable for codes too
}

// --- New: Verify a code against a stored hash ---
// Can reuse verify_password
pub async fn verify_code(code: &str, hash: &str) -> Result<bool, AppError> {
    verify_password(code, hash).await
}