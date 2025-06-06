use axum::{Extension, Router};
use tower_http::cors::CorsLayer;

use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::agent_workflow::ScraperSingleton;
use crate::routes::create_routes;
use crate::scraper::WebScraper;

/// Initialize tracing and logging for the application
pub fn init_tracing() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rs_rag_svc=info,tower_http=debug,axum::rejection=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}

/// Create and configure the Axum application with all routes and middleware
pub async fn create_app() -> Result<Router, anyhow::Error> {
    info!("Initializing application router");

    // Initialize shared scraper instanceז
    info!("Initializing web scraper...");
    ScraperSingleton::init().await?;
    let scraper = WebScraper::new().await?;
    info!("Web scraper initialized successfully");

    Ok(Router::new()
        .merge(create_routes())
        .layer(Extension(scraper)) // Add scraper as shared state
        //.layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive()))
}
