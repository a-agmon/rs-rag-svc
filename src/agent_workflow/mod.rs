pub mod data_retriever;
pub mod generate;
pub mod query_enhancer;
use crate::{agent_workflow::data_retriever::DataRetrieverTask, scraper::WebScraper};
use generate::GenerateAnswerTask;
use once_cell::sync::OnceCell;
use query_enhancer::QueryEnhanceTask;
use rig::{agent::Agent, providers::openrouter};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use task_graph::TaskGraph;

// Singleton WebScraper instance that can be shared across tasks
static SCRAPER_INSTANCE: OnceCell<Arc<WebScraper>> = OnceCell::new();

pub struct ScraperSingleton;

impl ScraperSingleton {
    /// Initialize the singleton scraper instance (should be called once at startup)
    pub async fn init() -> anyhow::Result<()> {
        let scraper = WebScraper::new().await?;
        SCRAPER_INSTANCE
            .set(Arc::new(scraper))
            .map_err(|_| anyhow::anyhow!("Scraper singleton already initialized"))?;
        Ok(())
    }

    /// Get a reference to the singleton scraper instance
    pub fn get() -> anyhow::Result<Arc<WebScraper>> {
        SCRAPER_INSTANCE.get().cloned().ok_or_else(|| {
            anyhow::anyhow!(
                "Scraper singleton not initialized. Call ScraperSingleton::init() first."
            )
        })
    }

    /// Check if the singleton is initialized
    pub fn is_initialized() -> bool {
        SCRAPER_INSTANCE.get().is_some()
    }
}

pub mod context_vars {
    pub const QUERY: &str = "query";
    pub const ENHANCED_QUERY: &str = "enhanced_query";
    pub const ANSWER: &str = "answer";
    pub const SEARCH_RESULTS: &str = "search_results";
}

pub fn create_agent_workflow(query: String) -> anyhow::Result<TaskGraph> {
    let mut graph = TaskGraph::new();
    let enhance_task = QueryEnhanceTask::new(query);
    let generate_task = GenerateAnswerTask;
    let retriever_task = DataRetrieverTask;
    graph.add_edge(enhance_task, retriever_task.clone())?;
    graph.add_edge(retriever_task, generate_task)?;
    Ok(graph)
}

pub fn get_llm_agent(prompt: &str) -> anyhow::Result<Agent<openrouter::CompletionModel>> {
    let api_key = std::env::var("OPENROUTER_API_KEY")
        .map_err(|_| anyhow::anyhow!("OPENROUTER_API_KEY not set"))?;
    let client = openrouter::Client::new(&api_key);
    let agent = client.agent("openai/gpt-4o-mini").preamble(prompt).build();
    Ok(agent)
}

// -- data structures  that capture the search results

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SearchResponse {
    #[serde(rename = "searchParameters")]
    pub search_parameters: SearchParameters,
    pub organic: Vec<OrganicResult>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SearchParameters {
    pub q: String,
    #[serde(rename = "type")]
    pub search_type: String,
    pub engine: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OrganicResult {
    pub title: String,
    pub link: String,
    pub snippet: String,
    pub position: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
}
