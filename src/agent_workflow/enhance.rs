use async_trait::async_trait;

use task_graph::{Context, ContextExt, GraphError, Task};
use tracing::info;

use crate::agent_workflow::context_vars;
use crate::agent_workflow::get_llm_agent;

use rig::completion::Prompt;

#[derive(Debug, Clone)]
pub struct EnhanceQueryTask {
    query: String,
}
impl EnhanceQueryTask {
    pub fn new(query: String) -> Self {
        Self { query }
    }
}

#[async_trait]
impl Task for EnhanceQueryTask {
    async fn run(&self, context: Context) -> Result<(), GraphError> {
        let enhanced_query = enhance_query(self.query.clone())
            .await
            .map_err(|e| GraphError::TaskExecutionFailed(e.to_string()))?;
        info!("Enhanced query: {}", enhanced_query);

        context
            .set(context_vars::ENHANCED_QUERY, enhanced_query)
            .await;
        info!("Enhanced query set in context");

        Ok(())
    }
}

const ENHANCE_QUERY_PROMPT: &str = r#"You are a search assistant.
Improve the user query for retrieval.
Rewrite it and add keywords so that a similarity search will find more relevant documents.
Keep it short (one sentence).
"#;

async fn enhance_query(query: String) -> anyhow::Result<String> {
    let agent = get_llm_agent(ENHANCE_QUERY_PROMPT)?;
    let response = agent.prompt(query).await?;
    Ok(response)
}
