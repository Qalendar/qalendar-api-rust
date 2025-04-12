use crate::config::Config;
use crate::errors::AppError;
use jsonwebtoken::{encode, EncodingKey, Header };
// use jsonwebtoken::{DecodingKey, Validation, Algorithm, decode, TokenData};
use serde::{Deserialize, Serialize};
use chrono::Utc;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: i32, // Subject (user_id)
    pub exp: i64, // Expiration timestamp (seconds since epoch) - Use i64 to match Duration::seconds
}

pub fn create_token(user_id: i32, config: &Config) -> Result<String, AppError> {

    let now_ts = Utc::now().timestamp(); // Get current timestamp as i64
    let expiration = now_ts.checked_add(config.jwt_expiration_seconds) // Directly add seconds
        .expect("Failed to calculate expiration time (overflow?)");

    let claims = Claims {
        sub: user_id,
        exp: expiration, // Assign the directly calculated i64 timestamp
    };

    // ... rest of the function (header, encoding_key, encode) ...
    let header = Header::default();
    let encoding_key = EncodingKey::from_secret(config.jwt_secret.as_ref());

    encode(&header, &claims, &encoding_key).map_err(AppError::from)
}

// pub fn validate_token(token: &str, config: &Config) -> Result<Claims, AppError> {
//     let decoding_key = DecodingKey::from_secret(config.jwt_secret.as_ref());

//     // Create validation parameters *requiring* expiration check
//     let mut validation = Validation::new(Algorithm::HS256);

//     // Ensure expiration validation is explicitly enabled (it's the default, but let's be sure)
//     validation.validate_exp = true;

//     // Set leeway to 0 for strict checking in tests (and often production)
//     validation.leeway = 0;

//     // Perform the decoding and validation
//     let token_data: Result<TokenData<Claims>, jsonwebtoken::errors::Error> =
//         decode::<Claims>(token, &decoding_key, &validation);

//     token_data
//         .map(|data| data.claims) // Extract claims on success
//         .map_err(|e| { // Map JWT errors to AppError
//             tracing::debug!("JWT validation failed: {}", e);
//             // Check the specific error kind
//             match e.kind() {
//                 jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
//                     println!("Validation failed specifically due to: ExpiredSignature"); // Debug log
//                     AppError::JwtError(e) // Return the original error wrapped in AppError
//                 }
//                 _ => {
//                      println!("Validation failed due to other reason: {:?}", e.kind()); // Debug log
//                      AppError::JwtError(e) // Return the original error wrapped in AppError
//                 }
//             }
//         })
// }