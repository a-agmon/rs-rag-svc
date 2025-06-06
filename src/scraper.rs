use anyhow::{Context, Result, anyhow};
use headless_chrome::{Browser, LaunchOptionsBuilder};
use scraper::{Html, Selector};
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::sleep;
use tracing::{debug, warn};

/// Thread-safe web scraper optimized for server use
/// Reuses a single browser instance across multiple async requests
#[derive(Clone)]
pub struct WebScraper {
    browser: std::sync::Arc<Mutex<Browser>>,
}

impl WebScraper {
    /// Create a new scraper with a long-lived browser instance
    /// Call this once at server startup and clone/share the instance
    pub async fn new() -> Result<Self> {
        let browser = Browser::new(
            LaunchOptionsBuilder::default()
                .headless(true)
                .window_size(Some((1280, 800)))
                .args(vec![
                    std::ffi::OsStr::new("--disable-blink-features=AutomationControlled"),
                    std::ffi::OsStr::new("--disable-web-security"),
                    std::ffi::OsStr::new("--disable-features=VizDisplayCompositor"),
                    std::ffi::OsStr::new("--user-agent=Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36"),
                ])
                .build()
                .context("Invalid Chrome launch options")?,
        )?;

        Ok(Self {
            browser: std::sync::Arc::new(Mutex::new(browser)),
        })
    }

    /// Scrape text from a URL - safe to call from multiple threads concurrently
    pub async fn scrape_text(&self, url: &str) -> Result<String> {
        // Brief delay for politeness
        sleep(Duration::from_millis(200)).await;

        // Lock browser and create new tab (async-friendly mutex)
        let tab = {
            let mut browser = self.browser.lock().await;

            // Check if browser process is still alive
            if let Some(pid) = browser.get_process_id() {
                debug!("Browser process ID: {}", pid);
            } else {
                warn!("Browser process ID not available - might be a remote connection");
            }

            // Try to create a tab, and if it fails, try to recreate the browser
            match browser.new_tab() {
                Ok(tab) => tab,
                Err(e) => {
                    warn!(
                        "Failed to create tab, attempting to recreate browser: {}",
                        e
                    );

                    // Try to create a new browser instance
                    let new_browser = Browser::new(
                        LaunchOptionsBuilder::default()
                            .headless(true)
                            .window_size(Some((1280, 800)))
                            .args(vec![
                                std::ffi::OsStr::new("--disable-blink-features=AutomationControlled"),
                                std::ffi::OsStr::new("--disable-web-security"),
                                std::ffi::OsStr::new("--disable-features=VizDisplayCompositor"),
                                std::ffi::OsStr::new("--user-agent=Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36"),
                            ])
                            .build()
                            .context("Invalid Chrome launch options")?,
                    ).context("Failed to recreate browser")?;

                    *browser = new_browser;
                    debug!("Browser recreated successfully");

                    browser
                        .new_tab()
                        .context("Failed to create new browser tab after recreation")?
                }
            }
        }; // Lock is released here

        // Perform scraping operations and ensure tab cleanup
        let result = async {
            // Navigate to URL
            tab.navigate_to(url)
                .with_context(|| format!("Failed to navigate to {}", url))?;

            // Wait for page load
            self.wait_for_page_load(&tab).await?;

            // Extract content
            let html = tab.get_content().context("Failed to get page content")?;
            let text = self.extract_text(&html);

            Ok(text)
        }
        .await;

        // Always close the tab, even if there was an error
        let _ = tab.close_target();

        result
    }

    /// Wait for page to load, handling Deflect challenges automatically
    async fn wait_for_page_load(&self, tab: &headless_chrome::Tab) -> Result<()> {
        // Wait for body element
        let _ = tab
            .wait_for_element("body")
            .map_err(|e| anyhow!("Failed to wait for page body: {}", e));

        // Check for Deflect challenge
        let is_challenge = tab
            .evaluate("document.body.innerHTML.includes('challenge') || document.body.innerHTML.includes('deflect') || document.title.includes('Verifying')", false)
            .map(|result| {
                result.value
                    .map(|v| v.to_string().contains("true"))
                    .unwrap_or(false)
            })
            .unwrap_or(false);

        if is_challenge {
            // Wait for Deflect challenge to complete
            sleep(Duration::from_millis(3000)).await;

            // Check for deflect cookie
            let has_cookie = tab
                .evaluate("document.cookie.includes('deflect=')", false)
                .map(|result| {
                    result
                        .value
                        .map(|v| v.to_string().contains("true"))
                        .unwrap_or(false)
                })
                .unwrap_or(false);

            if !has_cookie {
                sleep(Duration::from_secs(2)).await;
            }
        } else {
            // Regular page load timeout
            let _ = tokio::time::timeout(Duration::from_millis(2000), async {
                let _ = tab.evaluate(
                    "new Promise(resolve => { 
                            if (document.readyState === 'complete') { 
                                resolve(); 
                            } else { 
                                window.addEventListener('load', () => resolve()); 
                            } 
                        })",
                    true,
                );
            })
            .await;
        }

        Ok(())
    }

    /// Extract clean text content from HTML
    fn extract_text(&self, html: &str) -> String {
        let document = Html::parse_document(html);

        // Remove script and style content
        let script_selector = Selector::parse("script, style").unwrap();
        let mut cleaned_html = html.to_string();

        for element in document.select(&script_selector) {
            cleaned_html = cleaned_html.replace(&element.html(), "");
        }

        let clean_doc = Html::parse_document(&cleaned_html);

        // Try content-specific selectors first
        let content_selectors = ["main", "article", ".content", "#content", ".main"];

        for selector_str in content_selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                for element in clean_doc.select(&selector) {
                    let text = element.text().collect::<Vec<_>>().join(" ");
                    if text.trim().len() > 100 {
                        return self.clean_text(&text);
                    }
                }
            }
        }

        // Fallback to full document
        let all_text: String = clean_doc
            .root_element()
            .text()
            .collect::<Vec<_>>()
            .join(" ");

        self.clean_text(&all_text)
    }

    /// Clean and normalize extracted text
    fn clean_text(&self, text: &str) -> String {
        text.lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty() && line.len() > 2)
            .collect::<Vec<_>>()
            .join("\n")
    }
}

// Convenience functions for backward compatibility
pub async fn scrape_url(url: &str) -> Result<String> {
    let scraper = WebScraper::new().await?;
    scraper.scrape_text(url).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_scraper_creation() {
        let scraper = WebScraper::new().await;
        assert!(scraper.is_ok());
    }

    #[tokio::test]
    async fn test_concurrent_scraping() {
        let scraper = WebScraper::new().await.unwrap();

        let urls = vec![
            "https://httpbin.org/html",
            "https://example.com",
            "https://httpbin.org/user-agent",
        ];

        let tasks: Vec<_> = urls
            .into_iter()
            .map(|url| {
                let scraper = scraper.clone();
                tokio::spawn(async move { scraper.scrape_text(url).await })
            })
            .collect();

        let results = futures::future::join_all(tasks).await;

        for result in results {
            let text = result.unwrap().unwrap();
            assert!(!text.is_empty());
        }
    }
}
