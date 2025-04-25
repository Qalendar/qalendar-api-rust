use axum::{
    routing::post,
    Router,
};
use crate::handlers::auth_handler::{
    register_user_handler, login_user_handler,
    verify_email_handler, resend_verification_email_handler,
    forgot_password_handler, reset_password_handler, verify_tfa_login_handler,
};
use crate::state::AppState;

pub fn auth_routes(app_state: AppState) -> Router {
     Router::new()
        .route("/register", post(register_user_handler))
        .route("/login", post(login_user_handler))
        .route("/verify-email", post(verify_email_handler))
        .route("/resend-verification-email", post(resend_verification_email_handler))
        .route("/forgot-password", post(forgot_password_handler))
        .route("/reset-password", post(reset_password_handler))
        // This endpoint is called *after* a successful password login if 2FA is required
        .route("/verify-tfa", post(verify_tfa_login_handler))
        .with_state(app_state)
}