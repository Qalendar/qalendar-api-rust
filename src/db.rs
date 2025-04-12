use sqlx::postgres::{PgPool, PgPoolOptions};
use std::time::Duration;
use crate::config::Config;
use crate::errors::AppError;

pub async fn create_pool(config: &Config) -> Result<PgPool, AppError> {
    PgPoolOptions::new()
        .max_connections(10) // Adjust pool size as needed
        .acquire_timeout(Duration::from_secs(5))
        .connect(&config.database_url)
        .await
        .map_err(|e| {
            tracing::error!("Failed to connect to database: {}", e);
            AppError::DatabaseConnectionError(e)
        })
}