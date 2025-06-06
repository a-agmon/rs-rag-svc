use async_trait::async_trait;

use task_graph::{Context, ContextExt, GraphError, Task};
use tracing::info;

use crate::agent_workflow::context_vars;
use crate::agent_workflow::get_llm_agent;

use rig::completion::Prompt;

#[derive(Debug, Clone)]
pub struct QueryEnhanceTask {
    query: String,
}
impl QueryEnhanceTask {
    pub fn new(query: String) -> Self {
        Self { query }
    }
}

#[async_trait]
impl Task for QueryEnhanceTask {
    async fn run(&self, context: Context) -> Result<(), GraphError> {
        context.set(context_vars::QUERY, self.query.clone()).await;

        let enhanced_query = enhance_query(self.query.clone())
            .await
            .map_err(|e| GraphError::TaskExecutionFailed(e.to_string()))?;
        info!("Enhanced query: {}", enhanced_query);

        context
            .set(context_vars::ENHANCED_QUERY, enhanced_query)
            .await;

        Ok(())
    }
}

const ENHANCE_QUERY_PROMPT: &str = r#"
You are a search assistant, helping users refine their web site search queries.
You are given a user query and you need to rewrite it in a way that will maximize the number of relevant documents found in a google search.
Output only the list of words and terms, no other text, no commas or other punctuation.
"#;

async fn enhance_query(query: String) -> anyhow::Result<String> {
    let agent = get_llm_agent(ENHANCE_QUERY_PROMPT)?;
    let q = format!("\nUser query:\n{}", query);
    let response = agent.prompt(q).await?;
    Ok(response)
}
