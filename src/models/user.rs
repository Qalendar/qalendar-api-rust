use serde::{Deserialize, Serialize};
use validator::Validate; // Make sure validator is imported
use chrono::{DateTime, Utc, NaiveDate};
use sqlx::FromRow; // Needed for sqlx to map database rows

// --- API Payloads ---

#[derive(Deserialize, Validate, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RegisterUserPayload {
    #[validate(required, length(min = 1, max = 100))]
    pub display_name: Option<String>, // Use Option for required validation message

    #[validate(required, email)]
    pub email: Option<String>,

    #[validate(required, length(min = 8))]
    pub password: Option<String>,

    // Optional: Add custom date validation if needed
    // Format will be checked during parsing later
    pub dob: Option<String>,
}

#[derive(Deserialize, Validate, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LoginUserPayload {
    #[validate(required, email)]
    pub email: Option<String>,
    #[validate(required)]
    pub password: Option<String>,
}

#[derive(Deserialize, Validate, Debug)]
#[serde(rename_all = "camelCase")]
pub struct VerifyEmailPayload {
    #[validate(required, email)]
    pub email: Option<String>,
    #[validate(required, length(min = 32))] // Or match the exact expected code length
    pub code: Option<String>, // The verification code sent via email
}

#[derive(Deserialize, Validate, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ResendVerificationEmailPayload {
    #[validate(required, email)]
    pub email: Option<String>,
}

#[derive(Deserialize, Validate, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ForgotPasswordPayload {
    #[validate(required, email)]
    pub email: Option<String>,
}

#[derive(Deserialize, Validate, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ResetPasswordPayload {
    #[validate(required, email)]
    pub email: Option<String>, // Need email to find the user and match the code
    #[validate(required, length(min = 32))] // Or match expected code length
    pub code: Option<String>, // The reset code from the email link
    #[validate(required, length(min = 8))] // New password validation
    pub new_password: Option<String>,
}

#[derive(FromRow, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BasicUserInfo {
    pub user_id: i32,
    pub display_name: String,
    pub email: String,
    pub email_verified: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

// #[derive(FromRow, Debug, Serialize)]
// #[serde(rename_all = "camelCase")]
// pub struct UserInfo {
//     pub user_id: i32,
//     pub display_name: String,
//     pub email: String,
//     pub email_verified: bool,
//     pub password_hash: String,
//     pub date_of_birth: Option<NaiveDate>,
//     pub created_at: DateTime<Utc>,
//     pub updated_at: DateTime<Utc>,
//     pub deleted_at: Option<DateTime<Utc>>,
//     pub tfa_enabled: bool,
//     pub verification_code: Option<String>,
// }

#[derive(FromRow, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TfaUserInfo {
    pub user_id: i32,
    pub password_hash: String,
    pub tfa_enabled: bool,
    pub tfa_secret: Option<String>,
    pub deleted_at: Option<DateTime<Utc>>,
}

// --- New Payloads for 2FA Flow ---

// Payload for initiating 2FA setup (no body needed)
#[derive(Deserialize, Validate, Debug)]
pub struct InitiateTfaSetupPayload {
    #[validate(required)]
    pub password: Option<String>, // Require password to initiate 2FA setup
}

// Payload for completing 2FA setup
#[derive(Deserialize, Validate, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CompleteTfaSetupPayload {
    #[validate(required)]
    pub tfa_code: Option<String>, // Code from authenticator app
    // Include the secret returned by initiate, or re-generate it server-side based on user state?
    // Sending it back is simpler for the client flow, but less secure.
    // Let's assume the server stores the temporary secret in the user's record upon initiate.
    // So only the code is needed here.
    // pub tfa_secret: Option<String>, // The temporary secret from initiate
}

// Payload for disabling 2FA
#[derive(Deserialize, Validate, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DisableTfaPayload {
    #[validate(required)]
    pub password: Option<String>, // Require password to disable 2FA
    // Optional: require current 2FA code too?
    #[validate(required)]
    pub tfa_code: Option<String>,
}

// Payload for verifying 2FA code during login (Implicit Two-Step)
#[derive(Deserialize, Validate, Debug)]
#[serde(rename_all = "camelCase")]
pub struct VerifyTfaLoginPayload {
    #[validate(required)]
    pub user_id: Option<i32>, // Identify the user who passed step 1 login
    #[validate(required)]
    pub tfa_code: Option<String>, // The TOTP code
}

// --- API Responses ---

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UserData {
    #[serde(rename = "userId")] // Match frontend expectation
    pub user_id: i32,
    #[serde(rename = "displayName")]
    pub display_name: String,
    pub email: String,
    #[serde(rename = "emailVerified")]
    pub email_verified: bool,
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
    // Optionally include dob
    #[serde(rename = "dateOfBirth", skip_serializing_if = "Option::is_none")]
    pub date_of_birth: Option<NaiveDate>,
}

#[derive(Serialize, Debug)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserData,
    pub code_prefix: Option<String>, // Optional prefix for the verificatiion and reset codes
}

// Separate response for sending prefix of verification and reset codes
#[derive(Serialize, Debug)]
pub struct CodeResponse {
    pub code_prefix: Option<String>, // The prefix part of the verification/reset code
}

// New Response type for login when 2FA is required
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TfaRequiredResponse {
    pub user_id: i32, // Client needs this to call the verify-tfa endpoint
    // Add other user details client might need *before* full authentication?
    // e.g., display_name, email - BE CAREFUL not to send sensitive data
    // For security, maybe only send userId and a flag.
}

// Unified Login Response Enum
#[derive(Serialize, Debug)]
#[serde(untagged)] // Axum/serde will try each variant until one matches
pub enum LoginResponse {
    Auth(AuthResponse),             // Successful login, no 2FA or 2FA verified
    TfaRequired(TfaRequiredResponse), // 2FA is required after password step
}

// --- Response struct for Initiate TFA Setup ---
// Needs to include the secret (base32 encoded) and the OTP URI
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct InitiateTfaResponse {
    pub tfa_secret_base32: String,
    pub otp_auth_uri: String, // The otpauth:// URI string
    // Frontend uses these to generate QR code and show the secret text
}

// --- Database Model ---

// Represents the structure in the 'users' table
#[derive(FromRow, Debug, Serialize)] // Automatically map rows to this struct
#[serde(rename_all = "camelCase")]
pub struct User {
    pub user_id: i32,
    pub display_name: String,
    pub email: String,
    pub password_hash: String,
    pub date_of_birth: Option<NaiveDate>, // Matches DATE type in Postgres
    pub email_verified: bool,
    pub verification_code: Option<String>,
    pub verification_code_expires_at: Option<DateTime<Utc>>,
    pub reset_code: Option<String>,
    pub reset_code_expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>, // Matches TIMESTAMP WITH TIME ZONE
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub tfa_enabled: bool,
    pub tfa_secret: Option<String>,
}