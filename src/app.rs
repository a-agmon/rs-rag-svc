use std::sync::Arc;

use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::agent_workflow::create_agent_workflow;
use crate::routes::create_routes;

/// Initialize tracing and logging for the application
pub fn init_tracing() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                "rs_rag_svc=debug,tower_http=debug,axum::rejection=trace".into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}

/// Create and configure the Axum application with all routes and middleware
pub async fn create_app() -> Result<Router, anyhow::Error> {
    info!("Initializing application router");
    let graph = Arc::new(create_agent_workflow()?);

    Ok(Router::new()
        .merge(create_routes())
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(graph))
}
