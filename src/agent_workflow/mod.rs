pub mod enhance;
pub mod generate;
use bon::Builder;
use enhance::EnhanceQueryTask;
use generate::GenerateAnswerTask;
use rig::{agent::Agent, providers::openrouter};
use serde::{Deserialize, Serialize};
use task_graph::TaskGraph;

#[derive(Debug, Clone, Builder, Serialize, Deserialize)]
pub struct RAGWorkflow {
    #[builder(default)]
    pub query: String,
    #[builder(default)]
    pub enhanced_query: String,
    #[builder(default)]
    pub answer: String,
}

pub fn create_agent_workflow() -> anyhow::Result<TaskGraph> {
    let mut graph = TaskGraph::new();
    let enhance_task = graph.add_node(Box::new(EnhanceQueryTask));
    let generate_task = graph.add_node(Box::new(GenerateAnswerTask));
    graph.add_always_edge(enhance_task, generate_task)?;
    Ok(graph)
}

pub fn get_llm_agent(prompt: &str) -> anyhow::Result<Agent<openrouter::CompletionModel>> {
    let api_key = std::env::var("OPENROUTER_API_KEY")
        .map_err(|_| anyhow::anyhow!("OPENROUTER_API_KEY not set"))?;
    let client = openrouter::Client::new(&api_key);
    let agent = client.agent("openai/gpt-4o-mini").preamble(prompt).build();
    Ok(agent)
}
