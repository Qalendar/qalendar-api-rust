use sqlx::postgres::PgPool;
use std::sync::Arc;
use crate::config::Config;
use crate::email::EmailService;

#[derive(Clone)]
pub struct AppState {
   pub pool: PgPool,
   pub config: Arc<Config>,
   pub email_service: EmailService,
}