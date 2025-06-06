use crate::agent_workflow::OrganicResult;
use crate::agent_workflow::context_vars;
use crate::agent_workflow::get_llm_agent;
use async_trait::async_trait;
use rig::completion::Prompt;
use task_graph::{Context, ContextExt, GraphError, Task};
use tracing::info;

#[derive(Debug, Clone)]
pub struct GenerateAnswerTask;

#[async_trait]
impl Task for GenerateAnswerTask {
    async fn run(&self, context: Context) -> Result<(), GraphError> {
        info!("Generating answer from search results");
        let search_results: Vec<String> = context
            .get(context_vars::SEARCH_RESULTS)
            .await
            .ok_or_else(|| GraphError::TaskExecutionFailed("Missing search results".to_string()))?;

        let question: String = context
            .get(context_vars::QUERY)
            .await
            .ok_or_else(|| GraphError::TaskExecutionFailed("Missing question".to_string()))?;

        let answer = generate_answer(question, search_results)
            .await
            .map_err(|e| {
                GraphError::TaskExecutionFailed(format!("Failed to generate answer: {}", e))
            })?;
        //info!("Answer: {}", answer);
        context.set(context_vars::ANSWER, answer).await;
        Ok(())
    }
}

const PROMPT: &str = r#"
Question: {question}

Relevant texts:
{search_results}

Answer:
"#;

const SYSTEM_PROMPT: &str = r#"
You are a helpful research assistant. You are given a question and a list of relevant texts.
Your task is to answer the question based on the texts only. Do not make up any information.
note the dates in the texts to make sure you are using the most recent information.
"#;

async fn generate_answer(question: String, search_results: Vec<String>) -> anyhow::Result<String> {
    let agent = get_llm_agent(SYSTEM_PROMPT)?;
    let prompt = PROMPT
        .replace("{question}", &question)
        .replace("{search_results}", &search_results.join("\n"));
    let response = agent.prompt(prompt).await?;
    Ok(response)
}
