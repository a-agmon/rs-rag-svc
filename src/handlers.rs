use std::sync::Arc;

use axum::extract::State;
use axum::{extract::Json, response::Json as ResponseJson};
use task_graph::{ExecutionContext, TaskGraph};
use tracing::{debug, info};

use crate::agent_workflow::RAGWorkflow;
use crate::error::{AppError, AppResult};
use crate::models::{AgentRequest, AgentResponse, HealthResponse};

/// Health check handler
/// Returns the service status and health information
pub async fn health_check(
    State(_): State<Arc<TaskGraph>>,
) -> AppResult<ResponseJson<HealthResponse>> {
    debug!("Health check endpoint called");

    let response = HealthResponse::ok();

    info!("Health check successful");
    Ok(ResponseJson(response))
}

/// Agent handler for processing user queries
/// Accepts a JSON payload with a query and returns a processed response
pub async fn agent_handler(
    State(graph): State<Arc<TaskGraph>>,
    Json(payload): Json<AgentRequest>,
) -> AppResult<ResponseJson<AgentResponse>> {
    info!("Agent endpoint called with query: {}", payload.query);

    let ctx = ExecutionContext::new();
    let workflow = RAGWorkflow::builder().query(payload.query.clone()).build();
    ctx.set_typed(workflow).await;

    // run the workflow
    let result_ctx = graph
        .execute(ctx)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    let workflow = result_ctx.get_typed::<RAGWorkflow>().await.ok_or_else(|| {
        AppError::InternalServerError("Failed to retrieve workflow from context".to_string())
    })?;

    info!("Result context: {:?}", workflow.clone());
    let response = AgentResponse::new(workflow.answer);

    info!("Successfully processed query, returning response");
    Ok(ResponseJson(response))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::extract::State;
    use std::sync::Arc;
    use task_graph::TaskGraph;

    fn dummy_state() -> State<Arc<TaskGraph>> {
        State(Arc::new(TaskGraph::new()))
    }

    #[tokio::test]
    async fn test_health_check() {
        let result = health_check(dummy_state()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_agent_handler_valid_query() {
        let request = AgentRequest {
            query: "test query".to_string(),
        };

        let result = agent_handler(dummy_state(), Json(request)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_agent_handler_empty_query() {
        let request = AgentRequest {
            query: "".to_string(),
        };

        let result = agent_handler(dummy_state(), Json(request)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_agent_handler_whitespace_query() {
        let request = AgentRequest {
            query: "   ".to_string(),
        };

        let result = agent_handler(dummy_state(), Json(request)).await;
        assert!(result.is_err());
    }
}
