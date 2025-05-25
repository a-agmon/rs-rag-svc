use async_trait::async_trait;
use task_graph::{Context, ContextExt, GraphError, Task};
use tracing::info;

use crate::agent_workflow::context_vars;

#[derive(Debug, Clone)]
pub struct GenerateAnswerTask;

#[async_trait]
impl Task for GenerateAnswerTask {
    async fn run(&self, context: Context) -> Result<(), GraphError> {
        info!("Generating answer");
        let enhanced_query: String = context
            .get(context_vars::ENHANCED_QUERY)
            .await
            .ok_or_else(|| GraphError::TaskExecutionFailed("Missing enhanced query".to_string()))?;

        let answer = format!("[answer] {} [answer]", enhanced_query);
        info!("Answer: {}", answer);
        context.set(context_vars::ANSWER, answer).await;
        Ok(())
    }
}
