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
            .get(context_vars::QUERY)
            .await
            .ok_or_else(|| GraphError::TaskExecutionFailed("Missing enhanced query".to_string()))?;

        info!("Data retriever using enhanced query: '[{}]'", query);

        let search_response = retrieve_data(query).await.map_err(|e| {
            GraphError::TaskExecutionFailed(format!("Failed to retrieve data: {}", e))
        })?;

        let empty_vec = Vec::new();
        let items = search_response.items.as_ref().unwrap_or(&empty_vec);
        info!("Retrieved {} search results", items.len());

        // Filter URLs to only include scrapeable ones
        let scrapeable_results: Vec<_> = items
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

const GOOGLE_CSE_BASE_URL: &str = "https://www.googleapis.com/customsearch/v1";
const SEARCH_ENGINE_ID: &str = "966c36571122845c0";

async fn retrieve_data(query: String) -> anyhow::Result<SearchResponse> {
    let api_key = std::env::var("GOOGLE_CSE_API_KEY")
        .map_err(|_| anyhow::anyhow!("GOOGLE_CSE_API_KEY not set"))?;

    let client = reqwest::Client::builder().build()?;

    let url = format!(
        "{}?key={}&cx={}&q={}",
        GOOGLE_CSE_BASE_URL,
        api_key,
        SEARCH_ENGINE_ID,
        urlencoding::encode(&query)
    );

    info!(
        "Executing Google CSE search with URL: {}",
        url.replace(&api_key, "***API_KEY***")
    );

    let response = client.get(&url).send().await?;

    info!("Received response status: {}", response.status());

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(anyhow::anyhow!(
            "Google CSE API request failed: {}",
            error_text
        ));
    }

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
        assert!(is_scrapeable_url("https://example.com"));
        assert!(is_scrapeable_url("https://example.com/article"));
        assert!(is_scrapeable_url("https://news.com/story.html"));

        // Test non-scrapeable URLs (should return false)
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
}
