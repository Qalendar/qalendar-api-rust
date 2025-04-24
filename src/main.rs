use axum::Router;
use std::{net::SocketAddr, sync::Arc};
use tokio;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, fmt};
use tower_http::{ // Import tower_http components
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use dotenvy::dotenv;

// Declare modules using the new style
mod config;
mod db;
mod errors;
mod models; // Declares user module inside
mod state; // Declares AppState module inside
mod routes; // Declares auth module inside + create_api_router
mod handlers; // Declares auth_handler module inside
mod auth; // Declares jwt module inside
mod utils; // Declares security module inside
mod middleware; // Declares auth module inside
mod email; // Declares email module inside

use config::Config; // Use the Config struct
use errors::AppError; // Use our custom error type
use routes::create_api_router; // Use the function from routes module
use state::AppState; // Use the AppState struct
use email::EmailService; // Use the EmailService struct


#[tokio::main]
async fn main() -> Result<(), AppError> { // Return our AppError
    // Initialize tracing (logging)
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "qalendar_api=debug,tower_http=debug,axum::rejection=trace".into())) // Added more specific logging targets
        .with(fmt::layer()) // Use fmt::layer for structured logging output
        .init();

    tracing::info!("Starting Qalendar API Server...");

    // Load configuration from .env file
    dotenv().expect(".env file not found"); // Use dotenvy
    let config = Arc::new(Config::from_env()?); // Use Arc for shared state
    tracing::info!("Configuration loaded successfully.");

    // Create database connection pool
    let pool = db::create_pool(&config).await?; // Use db module function
    tracing::info!("Database pool created successfully.");

    // Create the Email Service
    let email_service = EmailService::new(config.clone())?; // Pass clone of Arc<Config>
    tracing::info!("Email service initialized.");

    // Create the application state - this is the single source of truth for state
    let app_state = AppState {
        pool: pool.clone(), // Clone the pool for the state
        config: config.clone(), // Clone the Arc<Config>
        email_service,
    };

    // Configure CORS
    let cors = CorsLayer::new()
        // Allow requests from any origin - BE CAREFUL IN PRODUCTION!
        .allow_origin(Any)
        // Allow common methods
        .allow_methods([
            http::Method::GET,
            http::Method::POST,
            http::Method::PUT,
            http::Method::PATCH,
            http::Method::DELETE,
            http::Method::OPTIONS, // Needed for preflight requests
        ])
        // Allow common headers
        .allow_headers([
            http::header::AUTHORIZATION,
            http::header::ACCEPT,
            http::header::CONTENT_TYPE,
        ]);
        // For production, replace .allow_origin(Any) with:
        // .allow_origin("http://your-frontend-domain.com".parse::<HeaderValue>().unwrap())
        // Or use a list of allowed origins.

    // Build the main application router by calling the central router function
    // This function will internally handle nesting and passing state to sub-routers
    let app = Router::new()
        .nest("/api", create_api_router(app_state.clone())) // Mount API routes
        // Add logging layer
        .layer(TraceLayer::new_for_http())
        // Add CORS layer
        .layer(cors);

    // Parse the server address
    let addr: SocketAddr = config.server_address
        .parse()
        .expect("Invalid server address format");

    tracing::info!("Server listening on {}", addr);

    // Start the server using axum-server
    axum_server::bind(addr) // Use axum_server::bind
        .serve(app.into_make_service())
        .await
        .map_err(|e| { // Handle potential server binding/runtime errors
            tracing::error!("Server failed: {}", e);
            // Convert the error into your AppError type if necessary,
            // or handle it directly (e.g., panic or exit)
            AppError::InternalServerError(format!("Server runtime error: {}", e))
        })?;

    Ok(())
}