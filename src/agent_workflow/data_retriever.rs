use async_trait::async_trait;

use task_graph::{Context, ContextExt, GraphError, Task};
use tracing::{debug, info, warn};

use crate::agent_workflow::ScraperSingleton;
use crate::agent_workflow::SearchResponse;
use crate::agent_workflow::context_vars;

#[derive(Debug, Clone)]
pub struct DataRetrieverTask;

/// Check if a URL is likely to be scrapeable (HTML content)
fn is_scrapeable_url(url: &str) -> bool {
    let url_lower = url.to_lowercase();

    // Skip common document file extensions
    let non_scrapeable_extensions = [
        ".pdf", ".doc", ".docx", ".xls", ".xlsx", ".ppt", ".pptx", ".zip", ".rar", ".tar", ".gz",
        ".7z", ".mp3", ".mp4", ".avi", ".mov", ".wav", ".jpg", ".jpeg", ".png", ".gif", ".bmp",
        ".svg", ".exe", ".dmg", ".app", ".deb", ".rpm",
    ];

    // Check if URL ends with any non-scrapeable extension
    for ext in &non_scrapeable_extensions {
        if url_lower.ends_with(ext) {
            return false;
        }
    }

    // Additional check for download URLs
    if url_lower.contains("/download/")
        && (url_lower.contains(".doc")
            || url_lower.contains(".pdf")
            || url_lower.contains(".xls")
            || url_lower.contains(".ppt"))
    {
        return false;
    }

    true
}

#[async_trait]
impl Task for DataRetrieverTask {
    async fn run(&self, context: Context) -> Result<(), GraphError> {
        info!("Retrieving data");
        let query: String = context
            .get(context_vars::ENHANCED_QUERY)
            .await
            .ok_or_else(|| GraphError::TaskExecutionFailed("Missing enhanced query".to_string()))?;

        info!("Data retriever using enhanced query: '[{}]'", query);

        let search_response = retrieve_data(query).await.map_err(|e| {
            GraphError::TaskExecutionFailed(format!("Failed to retrieve data: {}", e))
        })?;

        info!("Retrieved {} search results", search_response.organic.len());

        // Filter URLs to only include scrapeable ones
        let scrapeable_results: Vec<_> = search_response
            .organic
            .iter()
            .filter(|result| {
                let is_scrapeable = is_scrapeable_url(&result.link);
                if !is_scrapeable {
                    warn!(
                        "Skipping non-scrapeable URL: {} ({})",
                        result.link, result.title
                    );
                }
                is_scrapeable
            })
            .collect();

        info!("Filtered to {} scrapeable URLs", scrapeable_results.len());

        if scrapeable_results.is_empty() {
            warn!("No scrapeable URLs found in search results");
            context
                .set(context_vars::SEARCH_RESULTS, Vec::<String>::new())
                .await;
            return Ok(());
        }

        let scraper = ScraperSingleton::get().map_err(|e| {
            GraphError::TaskExecutionFailed(format!("Failed to get scraper: {}", e))
        })?;

        let scrape_futures = scrapeable_results.iter().map(|result| {
            info!("Scraping URL: {}", result.link);
            scraper.scrape_text(result.link.as_str())
        });

        let scraped_results: Vec<String> = futures::future::join_all(scrape_futures)
            .await
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| GraphError::TaskExecutionFailed(format!("Failed to scrape: {}", e)))?;

        // Filter out empty or very short scraped content
        let valid_scraped_txts: Vec<String> = scraped_results
            .into_iter()
            .filter(|text| text.trim().len() > 100) // Only keep substantial content
            .collect();

        info!(
            "Successfully scraped {} URLs with substantial content",
            valid_scraped_txts.len()
        );

        context
            .set(context_vars::SEARCH_RESULTS, valid_scraped_txts)
            .await;

        Ok(())
    }
}

const BASE_URL: &str = "https://google.serper.dev/search";
const SEARCH_TARGET: &str = "site:www.btselem.org";

async fn retrieve_data(query: String) -> anyhow::Result<SearchResponse> {
    let api_key =
        std::env::var("SERPER_API_KEY").map_err(|_| anyhow::anyhow!("SERPER_API_KEY not set"))?;
    let client = reqwest::Client::builder().build()?;
    let query_encoded = query.split_whitespace().collect::<Vec<_>>().join("+");
    let url = format!(
        "{}?q={}+{}&apiKey={}&num=5&tbs=qdr:3y",
        BASE_URL, query_encoded, SEARCH_TARGET, api_key
    );
    info!("Executing search with URL: {}", url);
    let request = client.request(reqwest::Method::GET, &url);
    let response = request.send().await?;
    info!("Received response status: {}", response.status());
    let body = response.text().await?;
    debug!("Response body: {}", body);

    let search_response: SearchResponse = serde_json::from_str(&body)?;
    Ok(search_response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use task_graph::TaskGraph;

    #[test]
    fn test_is_scrapeable_url() {
        // Test scrapeable URLs (should return true)
        assert!(is_scrapeable_url(
            "https://www.btselem.org/publications/202404_manufacturing_famine"
        ));
        assert!(is_scrapeable_url("https://www.btselem.org/gaza_strip"));
        assert!(is_scrapeable_url("https://example.com/article"));
        assert!(is_scrapeable_url("https://news.com/story.html"));

        // Test non-scrapeable URLs (should return false)
        assert!(!is_scrapeable_url(
            "https://www.btselem.org/download/200503_gaza_prison_english.doc"
        ));
        assert!(!is_scrapeable_url("https://example.com/file.pdf"));
        assert!(!is_scrapeable_url("https://site.com/report.docx"));
        assert!(!is_scrapeable_url("https://site.com/data.xlsx"));
        assert!(!is_scrapeable_url("https://site.com/presentation.pptx"));
        assert!(!is_scrapeable_url("https://site.com/image.jpg"));
        assert!(!is_scrapeable_url("https://site.com/video.mp4"));
        assert!(!is_scrapeable_url("https://site.com/archive.zip"));

        // Test download URLs with documents
        assert!(!is_scrapeable_url("https://site.com/download/report.pdf"));
        assert!(!is_scrapeable_url(
            "https://example.org/download/document.doc"
        ));

        // Test case insensitivity
        assert!(!is_scrapeable_url("https://site.com/FILE.PDF"));
        assert!(!is_scrapeable_url("https://site.com/Document.DOC"));
    }

    #[tokio::test]
    async fn test_retrieve_data_returns_data() {
        let query = "human rights violations".to_string();
        let result = retrieve_data(query).await;

        assert!(result.is_ok(), "API call should succeed");

        let search_response = result.unwrap();
        assert!(
            !search_response.organic.is_empty(),
            "Response should contain organic results"
        );
        assert!(
            !search_response.search_parameters.q.is_empty(),
            "Search parameters should contain query"
        );

        println!("Retrieved {} search results", search_response.organic.len());
        println!("Search query: {}", search_response.search_parameters.q);

        if !search_response.organic.is_empty() {
            println!("First result title: {}", search_response.organic[0].title);
            println!("First result link: {}", search_response.organic[0].link);
            println!(
                "First result snippet: {}",
                search_response.organic[0].snippet
            );
        }
    }

    #[tokio::test]
    async fn test_data_retriever_task() {
        let task = DataRetrieverTask;
        let graph = TaskGraph::new();
        let context = graph.context();

        // Set up the context with an enhanced query
        context
            .set(
                context_vars::ENHANCED_QUERY,
                "Israeli settlements".to_string(),
            )
            .await;

        let result = task.run(context.clone()).await;

        assert!(
            result.is_ok(),
            "DataRetrieverTask should complete successfully"
        );

        // Verify the search results were stored in context
        let search_results: Option<Vec<crate::agent_workflow::OrganicResult>> =
            context.get(context_vars::SEARCH_RESULTS).await;
        assert!(
            search_results.is_some(),
            "Search results should be stored in context"
        );

        let search_results = search_results.unwrap();
        assert!(
            !search_results.is_empty(),
            "Should have retrieved some results"
        );

        println!(
            "DataRetrieverTask completed successfully with {} results",
            search_results.len()
        );
    }

    #[tokio::test]
    async fn test_data_retriever_task_missing_query() {
        let task = DataRetrieverTask;
        let graph = TaskGraph::new();
        let context = graph.context();

        // Don't set the enhanced query - this should fail
        let result = task.run(context).await;

        assert!(
            result.is_err(),
            "DataRetrieverTask should fail without enhanced query"
        );

        if let Err(GraphError::TaskExecutionFailed(msg)) = result {
            assert!(
                msg.contains("Missing enhanced query"),
                "Error message should mention missing query"
            );
        } else {
            panic!("Expected TaskExecutionFailed error");
        }
    }

    #[tokio::test]
    async fn test_retrieve_data_with_multiple_keywords() {
        let query = "Gaza Strip human rights report".to_string();
        let result = retrieve_data(query).await;

        assert!(
            result.is_ok(),
            "API call with multiple keywords should succeed"
        );

        let search_response = result.unwrap();
        assert!(
            !search_response.organic.is_empty(),
            "Response should contain results"
        );

        println!(
            "Retrieved {} results for multi-keyword query",
            search_response.organic.len()
        );

        // Print details about each result
        for (index, result) in search_response.organic.iter().enumerate() {
            println!("Result {}: {}", index + 1, result.title);
            println!("  Link: {}", result.link);
            println!("  Position: {}", result.position);
            if let Some(date) = &result.date {
                println!("  Date: {}", date);
            }
        }
    }

    #[tokio::test]
    async fn test_search_response_structure() {
        let query = "katz gaza starvation".to_string();
        let result = retrieve_data(query).await;

        assert!(result.is_ok(), "API call should succeed");

        let search_response = result.unwrap();

        // Test search parameters structure
        assert_eq!(search_response.search_parameters.engine, "google");
        assert_eq!(search_response.search_parameters.search_type, "search");
        assert!(
            search_response
                .search_parameters
                .q
                .contains("site:www.btselem.org")
        );

        // Test organic results structure
        assert!(
            !search_response.organic.is_empty(),
            "Should have organic results"
        );

        for result in &search_response.organic {
            assert!(!result.title.is_empty(), "Result should have a title");
            assert!(!result.link.is_empty(), "Result should have a link");
            assert!(
                result.link.contains("btselem.org"),
                "Link should be from btselem.org"
            );
            assert!(!result.snippet.is_empty(), "Result should have a snippet");
            assert!(result.position > 0, "Position should be greater than 0");
        }

        println!("Search response structure is valid");
        println!("Total results: {}", search_response.organic.len());
        println!("Query: {}", search_response.search_parameters.q);

        // Show summary of results
        for (i, result) in search_response.organic.iter().enumerate().take(3) {
            println!(
                "{}. {} (Position: {})",
                i + 1,
                result.title,
                result.position
            );
        }
    }
}
