use crate::handlers::{agent_handler, health_check};
use axum::{Router, routing::get, routing::post};

/// Creates and configures all application routes
pub fn create_routes() -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/api/agent1", post(agent_handler))
}
