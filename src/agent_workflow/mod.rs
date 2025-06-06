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

// -- data structures that capture the Google Custom Search API results

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SearchResponse {
    pub kind: String,
    pub items: Option<Vec<OrganicResult>>,
    #[serde(rename = "searchInformation")]
    pub search_information: Option<SearchInformation>,
    pub queries: Option<Queries>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SearchInformation {
    #[serde(rename = "totalResults")]
    pub total_results: String,
    #[serde(rename = "searchTime")]
    pub search_time: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Queries {
    pub request: Option<Vec<RequestQuery>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RequestQuery {
    pub title: String,
    #[serde(rename = "searchTerms")]
    pub search_terms: String,
    pub count: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OrganicResult {
    pub title: String,
    pub link: String,
    #[serde(rename = "htmlSnippet")]
    pub snippet: String, // Maps htmlSnippet to snippet for consistency
    #[serde(rename = "displayLink")]
    pub display_link: Option<String>,
}
