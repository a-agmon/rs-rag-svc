use async_trait::async_trait;
use task_graph::{ExecutionContext, TaskError, TaskNode};

use crate::agent_workflow::get_llm_agent;

use super::RAGWorkflow;
use rig::completion::Prompt;

pub struct EnhanceQueryTask;

#[async_trait]
impl TaskNode for EnhanceQueryTask {
    async fn run(&self, ctx: ExecutionContext) -> Result<(), TaskError> {
        let workflow: RAGWorkflow = ctx.get_typed::<RAGWorkflow>().await.ok_or_else(|| {
            TaskError::node_error(self.name(), anyhow::anyhow!("No workflow provided"))
        })?;
        let enhanced_query = enhance_query(workflow.query).await?;
        ctx.update_typed(|workflow: &mut RAGWorkflow| {
            workflow.enhanced_query = enhanced_query;
        })
        .await;

        Ok(())
    }

    fn name(&self) -> &'static str {
        std::any::type_name::<Self>()
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
