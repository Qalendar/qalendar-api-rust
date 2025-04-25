use crate::errors::AppError;
use crate::config::Config;
use totp_rs::{Secret, TOTP, Algorithm}; // Import necessary items from totp-rs
use base32::{encode, Alphabet}; // Use base32 for encoding/decoding secrets
use std::time::SystemTime; // Needed for TOTP timestamp


// Constants for TOTP (default values from RFC 6238)
const TIME_STEP: u64 = 30; // 30 seconds
const DIGITS: usize = 6;   // 6 digits
// const SHA1: &str = "SHA1"; // Hashing algorithm

// Helper to generate a new random TOTP secret (as base32 string)
pub fn generate_tfa_secret_base32() -> String {
    let secret = Secret::generate_secret(); // Generate a new secret
    match secret.to_encoded() {
        Secret::Encoded(encoded_string) => encoded_string,
        _ => panic!("Failed to encode secret as base32"), // This shouldn't happen
    }
}

// Helper to generate the otpauth:// URI for an email and secret
pub fn generate_otp_auth_uri(email: &str, secret_base32: &str, issuer: &str) -> Result<String, AppError> {
    // Recreate Secret from base32 string
    let secret = Secret::Encoded(secret_base32.to_string())
        .to_bytes()
        .map_err(|e| AppError::InternalServerError(format!("Failed to decode base32 secret: {}", e)))?; // Handle decoding error

    // Build the TOTP configuration
    let totp = TOTP::new(
        Algorithm::SHA1,
        DIGITS,
        1, // Window size - allow codes from one time step before and after
        TIME_STEP,
        secret,
        Some(issuer.to_string()), // Issuer (app name)
        email.to_string(), // Label (user email)
    )
        .map_err(|e| AppError::InternalServerError(format!("Failed to build OTP URI: {}", e)))?;

    Ok(totp.to_string()) // Convert the TOTPUrl struct to its string representation
}

// Helper to validate a TOTP code against a secret
pub fn verify_tfa_code(secret_base32: &str, code: &str) -> Result<bool, AppError> {
    // Recreate Secret from base32 string
    let secret = Secret::Encoded(secret_base32.to_string())
        .to_bytes()
        .map_err(|e| AppError::InternalServerError(format!("Failed to decode base32 secret for verification: {}", e)))?;

    // Build the TOTP validator
     let totp = TOTP::new(
        Algorithm::SHA1,
        DIGITS,
        1, // Window size - allow codes from one time step before and after
        TIME_STEP,
        secret,
        None, // No issuer needed for verification
        String::new(), // Empty label for verification
     )
     .map_err(|e| AppError::InternalServerError(format!("Failed to build TOTP validator: {}", e)))?;

    // // Get the current timestamp for logging or debugging purposes
    // let current_timestamp = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)
    //     .map_err(|e| AppError::InternalServerError(format!("Failed to get system time: {}", e)))?
    //     .as_secs();

    let code_is_valid = totp.check_current(code)
        .map_err(|e| AppError::InternalServerError(format!("Failed to check TOTP code: {}", e)))?;

    Ok(code_is_valid)
}