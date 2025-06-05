use rs_rag_svc::app::{create_app, init_tracing};
use rs_rag_svc::config::Config;
use tracing::{error, info};

#[tokio::main]
async fn main() {
    // Initialize tracing/logging
    init_tracing();

    info!("Starting RS RAG Service...");

    // Load configuration
    let config = Config::from_env();
    info!("Configuration loaded: {:?}", config);

    // Create the application
    let app = match create_app().await {
        Ok(app) => app,
        Err(e) => {
            error!("Failed to create app: {}", e);
            std::process::exit(1);
        }
    };

    // Create TCP listener
    let listener = match tokio::net::TcpListener::bind(&config.bind_address()).await {
        Ok(listener) => {
            info!("Server running on {}", config.server_url());
            info!("Health check: GET /health");
            info!("Agent endpoint: POST /api/agent1");
            listener
        }
        Err(e) => {
            error!("Failed to bind to {}: {}", config.bind_address(), e);
            std::process::exit(1);
        }
    };

    // Start the server
    info!("Server starting...");
    if let Err(e) = axum::serve(listener, app).await {
        error!("Server error: {}", e);
    } else {
        info!("Server shutdown gracefully");
    }
}
