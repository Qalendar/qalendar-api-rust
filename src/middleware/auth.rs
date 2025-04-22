use axum::{
    // extract::State,
    // http::{Request, StatusCode},
    // middleware::Next,
    // response::Response,
    RequestPartsExt, // Needed for .extract() on parts
};
use axum_extra::{
    headers::{Authorization, authorization::Bearer},
    TypedHeader,
};
// use axum::body::Body;
// use axum::http::header::AUTHORIZATION;
use crate::{AppState, errors::AppError, auth::jwt};

// Struct that will be injected into handlers upon successful authentication
#[derive(Debug)]
pub struct AuthenticatedUser {
    pub user_id: i32,
    // You could add more user details here if fetched from the DB,
    // but user_id is often sufficient for authorization checks
}

// Implement the FromRequestParts trait to make AuthenticatedUser an extractor
// #[async_trait::async_trait] // Use async_trait for async trait methods
impl axum::extract::FromRequestParts<AppState> for AuthenticatedUser // Use AppState directly
where
    // Ensure AppState meets the requirements for use as Axum state
    AppState: Clone + Send + Sync + 'static,
{
    // Define the error type returned by this extractor
    type Rejection = AppError; // Our custom AppError can be the rejection type
    
    async fn from_request_parts(
        parts: & mut http::request::Parts, state: & AppState
    ) -> Result<Self, Self::Rejection> {
        // Extract the Bearer token from the Authorization header
        let TypedHeader(Authorization(bearer)) = parts.extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|_| AppError::JwtError(jsonwebtoken::errors::Error::from(jsonwebtoken::errors::ErrorKind::InvalidToken)))?; // Return 401 if header is missing/invalid
        
        // Get the token string from the Bearer struct
        let token = bearer.token();

        // Get the JWT secret from the application state
        // state is now directly &AppState
        let config = &state.config;

        // Validate the token using our helper function
        let claims = jwt::validate_token(&token, config)?; // Handles expiration and signature validation

        // If validation succeeds, construct AuthenticatedUser
        Ok(AuthenticatedUser { user_id: claims.sub })
    }
}

// --- Optional: A handler-based middleware approach ---
// You can also write middleware as a standard async function that takes `Request` and `Next`.
// This is useful for things like logging, CORS, or transforming the request/response body.
// For AUTHENTICATION (checking and stopping the request early), the Extractor pattern (above) is often preferred
// because it integrates directly into the handler signature and automatically handles the 401 rejection.
//
// Example (for general middleware):
// pub async fn my_middleware(req: Request<Body>, next: Next) -> Response {
//     // Do something before the handler
//     println!("Processing request: {:?}", req.uri());
//
//     let response = next.run(req).await;
//
//     // Do something after the handler
//     println!("Finished request");
//
//     response
// }