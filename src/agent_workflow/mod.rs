pub mod enhance;
pub mod generate;
use enhance::EnhanceQueryTask;
use generate::GenerateAnswerTask;
use rig::{agent::Agent, providers::openrouter};
use task_graph::TaskGraph;

pub mod context_vars {
    pub const QUERY: &str = "query";
    pub const ENHANCED_QUERY: &str = "enhanced_query";
    pub const ANSWER: &str = "answer";
}

pub fn create_agent_workflow(query: String) -> anyhow::Result<TaskGraph> {
    let mut graph = TaskGraph::new();
    let enhance_task = EnhanceQueryTask::new(query);
    let generate_task = GenerateAnswerTask;
    graph.add_edge(enhance_task, generate_task)?;
    Ok(graph)
}

pub fn get_llm_agent(prompt: &str) -> anyhow::Result<Agent<openrouter::CompletionModel>> {
    let api_key = std::env::var("OPENROUTER_API_KEY")
        .map_err(|_| anyhow::anyhow!("OPENROUTER_API_KEY not set"))?;
    let client = openrouter::Client::new(&api_key);
    let agent = client.agent("openai/gpt-4o-mini").preamble(prompt).build();
    Ok(agent)
}
