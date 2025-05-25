use crate::agent_workflow::{context_vars, create_agent_workflow};
use crate::error::{AppError, AppResult};
use crate::models::{AgentRequest, AgentResponse, HealthResponse};
use axum::{extract::Json, response::Json as ResponseJson};
use task_graph::ContextExt;
use tracing::{debug, info};

/// Health check handler
/// Returns the service status and health information
pub async fn health_check() -> AppResult<ResponseJson<HealthResponse>> {
    debug!("Health check endpoint called");

    let response = HealthResponse::ok();

    info!("Health check successful");
    Ok(ResponseJson(response))
}

/// Agent handler for processing user queries
/// Accepts a JSON payload with a query and returns a processed response
pub async fn agent_handler(
    Json(payload): Json<AgentRequest>,
) -> AppResult<ResponseJson<AgentResponse>> {
    info!("Agent endpoint called with query: {}", payload.query);

    // Validate the request
    if !payload.is_valid() {
        return Err(AppError::ValidationError(
            "Query cannot be empty or only whitespace".to_string(),
        ));
    }

    let graph = create_agent_workflow(payload.query);
    if let Err(e) = graph {
        return Err(AppError::InternalServerError(e.to_string()));
    }
    let graph = graph.unwrap();

    // run the workflow
    graph
        .execute()
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    let answer: String = graph
        .context()
        .get(context_vars::ANSWER)
        .await
        .ok_or_else(|| {
            AppError::InternalServerError("Failed to retrieve answer from context".to_string())
        })?;

    let response = AgentResponse::new(answer);
    info!("Successfully processed query, returning response");
    Ok(ResponseJson(response))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_check() {
        let result = health_check().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_agent_handler_valid_query() {
        // Skip test if OPENROUTER_API_KEY is not set
        if std::env::var("OPENROUTER_API_KEY").is_err() {
            println!("Skipping test: OPENROUTER_API_KEY not set");
            return;
        }

        let request = AgentRequest {
            query: "test query".to_string(),
        };

        let result = agent_handler(Json(request)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_agent_handler_empty_query() {
        // Skip test if OPENROUTER_API_KEY is not set
        if std::env::var("OPENROUTER_API_KEY").is_err() {
            println!("Skipping test: OPENROUTER_API_KEY not set");
            return;
        }

        let request = AgentRequest {
            query: "".to_string(),
        };

        let result = agent_handler(Json(request)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_agent_handler_whitespace_query() {
        // Skip test if OPENROUTER_API_KEY is not set
        if std::env::var("OPENROUTER_API_KEY").is_err() {
            println!("Skipping test: OPENROUTER_API_KEY not set");
            return;
        }

        let request = AgentRequest {
            query: "   ".to_string(),
        };

        let result = agent_handler(Json(request)).await;
        assert!(result.is_err());
    }
}
