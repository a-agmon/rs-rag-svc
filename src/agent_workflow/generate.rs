use async_trait::async_trait;
use task_graph::{ExecutionContext, TaskError, TaskNode};

use super::RAGWorkflow;

pub struct GenerateAnswerTask;

#[async_trait]
impl TaskNode for GenerateAnswerTask {
    async fn run(&self, ctx: ExecutionContext) -> Result<(), TaskError> {
        let workflow: RAGWorkflow = ctx.get_typed::<RAGWorkflow>().await.ok_or_else(|| {
            TaskError::node_error(self.name(), anyhow::anyhow!("No workflow provided"))
        })?;
        let answer = format!("[answer] {} [answer]", workflow.enhanced_query);
        ctx.update_typed(|workflow: &mut RAGWorkflow| {
            workflow.answer = answer;
        })
        .await;
        Ok(())
    }

    fn name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    #[doc = " Optional: Get a description of what this task does"]
    fn description(&self) -> Option<&'static str> {
        None
    }
}
