use serde::{Deserialize, Serialize};

/// Request payload for the agent endpoint
#[derive(Debug, Deserialize)]
pub struct AgentRequest {
    pub query: String,
}

/// Response payload for the agent endpoint
#[derive(Debug, Serialize)]
pub struct AgentResponse {
    pub answer: String,
}

/// Response payload for the health check endpoint
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub message: String,
}

impl HealthResponse {
    pub fn ok() -> Self {
        Self {
            status: "ok".to_string(),
            message: "Service is healthy".to_string(),
        }
    }
}

impl AgentRequest {
    /// Validates if the query is not empty or just whitespace
    pub fn is_valid(&self) -> bool {
        !self.query.trim().is_empty()
    }
}

impl AgentResponse {
    pub fn new(answer: String) -> Self {
        Self { answer }
    }
}
