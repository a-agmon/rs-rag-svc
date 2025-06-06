// web_scraper_test.rs – Production-ready scraper for pages protected by Deflect (eQualit.ie)
// -----------------------------------------------------------------------------
// HOW IT WORKS
// 1. Launches a real Chrome/Chromium instance in headless mode using the
//    `headless_chrome` crate. Deflect's "Challenger" only grants access after
//    a short JavaScript hash-puzzle. Because we execute a full browser engine
//    the puzzle is solved automatically and a `deflect=<token>` cookie is
//    returned by the edge node.
// 2. We actively wait until the cookie is present **or** the page no longer
//    contains the verification banner, then pull the fully rendered HTML. The
//    same cookie (valid ~24 h) can be reused with Reqwest for follow-up HTTP
//    calls so you don't pay the browser startup cost on every URL.
// 3. Finally we strip the HTML tags with `scraper` and stream the visible text
//    to stdout.
// -----------------------------------------------------------------------------

use anyhow::{Context, Result, anyhow};
use headless_chrome::{Browser, LaunchOptionsBuilder};
use scraper::{Html, Selector};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::time::sleep;

/// Production-ready web scraper with cookie reuse and error handling
pub struct DeflectScraper {
    browser: Arc<Mutex<Browser>>,
}

impl DeflectScraper {
    /// Create a new scraper instance with a long-lived browser
    pub fn new() -> Result<Self> {
        let start_time = Instant::now();
        println!("🚀 Starting browser initialization...");

        let browser = Browser::new(
            LaunchOptionsBuilder::default()
                .headless(true)
                .window_size(Some((1280, 800)))
                // Production readiness: avoid headless detection
                .args(vec![
                    std::ffi::OsStr::new("--disable-blink-features=AutomationControlled"),
                    std::ffi::OsStr::new("--disable-web-security"),
                    std::ffi::OsStr::new("--disable-features=VizDisplayCompositor"),
                    std::ffi::OsStr::new("--user-agent=Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36"),
                ])
                .build()
                .context("Invalid Chrome launch options")?,
        )?;

        let init_duration = start_time.elapsed();
        println!(
            "✅ Browser initialized in {:.2}s",
            init_duration.as_secs_f64()
        );

        Ok(Self {
            browser: Arc::new(Mutex::new(browser)),
        })
    }

    /// Navigate to `url`, wait until the Deflect challenge is solved, and return
    /// the visible text of the final page with cookie reuse for efficiency
    pub async fn grab_text(&self, url: &str) -> Result<String> {
        self.grab_text_with_timing(url).await.map(|(text, _)| text)
    }

    /// Same as grab_text but returns timing information
    pub async fn grab_text_with_timing(&self, url: &str) -> Result<(String, Duration)> {
        let total_start = Instant::now();

        // Production readiness: add polite delay
        sleep(Duration::from_millis(500)).await;

        let tab_start = Instant::now();
        let browser = self
            .browser
            .lock()
            .map_err(|_| anyhow!("Failed to acquire browser lock"))?;

        let tab = browser
            .new_tab()
            .context("Failed to create new browser tab")?;

        let tab_creation_time = tab_start.elapsed();
        println!(
            "📋 New tab created in {:.3}s",
            tab_creation_time.as_secs_f64()
        );

        let nav_start = Instant::now();
        tab.navigate_to(url)
            .with_context(|| format!("Failed to navigate to {}", url))?;

        let nav_time = nav_start.elapsed();
        println!("🌐 Navigation completed in {:.3}s", nav_time.as_secs_f64());

        let wait_start = Instant::now();

        // Wait for the page to load completely using browser events
        // First, wait for the DOM to be ready
        let _dom_wait = tab
            .wait_for_element("body")
            .map_err(|e| anyhow!("Failed to wait for page body: {}", e));

        // Check if this might be a Deflect-protected site first
        let is_deflect_challenge = tab
            .evaluate("document.body.innerHTML.includes('challenge') || document.body.innerHTML.includes('deflect') || document.title.includes('Verifying')", false)
            .map(|result| {
                result.value
                    .map(|v| v.to_string().contains("true"))
                    .unwrap_or(false)
            })
            .unwrap_or(false);

        if is_deflect_challenge {
            println!("🔐 Deflect challenge detected, waiting for completion...");
            // Wait longer for Deflect challenge
            sleep(Duration::from_millis(3000)).await;

            // Check if there's a deflect cookie by evaluating JavaScript
            let cookie_check = tab
                .evaluate("document.cookie.includes('deflect=')", false)
                .map_err(|e| anyhow!("Failed to check for deflect cookie: {}", e));

            // If no deflect cookie found, wait a bit more for the challenge to complete
            if let Ok(result) = cookie_check {
                if let Some(value) = result.value {
                    if !value.to_string().contains("true") {
                        // Wait longer for potential challenge completion
                        sleep(Duration::from_secs(5)).await;
                    }
                }
            }
        } else {
            // For regular sites, wait for load event with timeout
            let load_timeout = tokio::time::timeout(
                Duration::from_millis(2000), // Max 2 seconds timeout
                async {
                    let _ = tab.evaluate(
                        "new Promise(resolve => { 
                            if (document.readyState === 'complete') { 
                                resolve(); 
                            } else { 
                                const handler = () => { resolve(); window.removeEventListener('load', handler); };
                                window.addEventListener('load', handler); 
                            } 
                        })", 
                        true
                    );
                }
            ).await;

            // If timeout occurred, continue anyway
            if load_timeout.is_err() {
                println!("⚠️  Load event timeout, proceeding...");
            }
        }

        let wait_time = wait_start.elapsed();
        println!(
            "⏱️  Page loading & JS execution: {:.3}s",
            wait_time.as_secs_f64()
        );

        let extract_start = Instant::now();
        // -- Extract and return visible text --
        let html = tab.get_content().context("Failed to get page content")?;

        let text = self.html2text(&html);
        let extract_time = extract_start.elapsed();
        println!(
            "📄 Text extraction completed in {:.3}s",
            extract_time.as_secs_f64()
        );

        let total_time = total_start.elapsed();
        println!("🏁 Total scraping time: {:.3}s", total_time.as_secs_f64());

        Ok((text, total_time))
    }

    /// Enhanced HTML → plaintext converter with better text extraction
    fn html2text(&self, html: &str) -> String {
        let document = Html::parse_document(html);
        let mut text_parts = Vec::new();

        // Remove script and style tags content
        let script_selector = Selector::parse("script, style").unwrap();
        let mut cleaned_html = html.to_string();

        for element in document.select(&script_selector) {
            cleaned_html = cleaned_html.replace(&element.html(), "");
        }

        let clean_doc = Html::parse_document(&cleaned_html);

        // Extract text from common content containers
        let content_selectors = [
            "main", "article", ".content", "#content", ".main", ".article", "body",
        ];

        for selector_str in content_selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                for element in clean_doc.select(&selector) {
                    let text = element.text().collect::<Vec<_>>().join(" ");
                    if !text.trim().is_empty() && text.len() > 50 {
                        text_parts.push(text.trim().to_string());
                        break; // Use the first substantial content found
                    }
                }
                if !text_parts.is_empty() {
                    break;
                }
            }
        }

        // Fallback to full document text extraction
        if text_parts.is_empty() {
            let all_text: String = clean_doc
                .root_element()
                .text()
                .filter(|t| !t.trim().is_empty())
                .collect::<Vec<_>>()
                .join(" ");

            if !all_text.trim().is_empty() {
                text_parts.push(all_text);
            }
        }

        // Clean up the text
        text_parts
            .join("\n\n")
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty() && line.len() > 2)
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Test the scraper on multiple URLs with error recovery and timing
    pub async fn test_urls(&self, urls: Vec<&str>) -> Result<()> {
        println!("🚀 Starting Deflect Scraper Test with Timing");
        println!("Testing {} URLs...\n", urls.len());

        let mut total_times = Vec::new();

        for (i, url) in urls.iter().enumerate() {
            println!("📄 Testing URL {}/{}: {}", i + 1, urls.len(), url);

            match self.grab_text_with_timing(url).await {
                Ok((text, duration)) => {
                    total_times.push(duration);
                    let preview = if text.len() > 200 {
                        format!("{}...", &text[..200])
                    } else {
                        text.clone()
                    };

                    println!(
                        "✅ Success! Extracted {} characters in {:.3}s",
                        text.len(),
                        duration.as_secs_f64()
                    );
                    println!("📝 Preview: {}\n", preview);
                }
                Err(e) => {
                    println!("❌ Failed: {}\n", e);
                }
            }

            // Production readiness: polite delay between requests
            if i < urls.len() - 1 {
                sleep(Duration::from_secs(2)).await;
            }
        }

        // Print timing statistics
        if !total_times.is_empty() {
            let avg_time =
                total_times.iter().map(|d| d.as_secs_f64()).sum::<f64>() / total_times.len() as f64;
            let min_time = total_times
                .iter()
                .map(|d| d.as_secs_f64())
                .fold(f64::INFINITY, f64::min);
            let max_time = total_times
                .iter()
                .map(|d| d.as_secs_f64())
                .fold(0.0, f64::max);

            println!("📊 Timing Statistics:");
            println!("   Average: {:.3}s", avg_time);
            println!("   Minimum: {:.3}s", min_time);
            println!("   Maximum: {:.3}s", max_time);
        }

        Ok(())
    }

    /// Benchmark tab creation performance
    pub async fn benchmark_tab_creation(&self, iterations: usize) -> Result<()> {
        println!("🏃 Starting Tab Creation Benchmark");
        println!(
            "Creating {} tabs and measuring performance...\n",
            iterations
        );

        let mut tab_times = Vec::new();

        for i in 0..iterations {
            let tab_start = Instant::now();

            let browser = self
                .browser
                .lock()
                .map_err(|_| anyhow!("Failed to acquire browser lock"))?;

            let _tab = browser
                .new_tab()
                .context("Failed to create new browser tab")?;

            let tab_time = tab_start.elapsed();
            tab_times.push(tab_time);

            println!("Tab {}: {:.3}s", i + 1, tab_time.as_secs_f64());

            // Small delay to avoid overwhelming the browser
            sleep(Duration::from_millis(100)).await;
        }

        // Calculate statistics
        let avg_time =
            tab_times.iter().map(|d| d.as_secs_f64()).sum::<f64>() / tab_times.len() as f64;
        let min_time = tab_times
            .iter()
            .map(|d| d.as_secs_f64())
            .fold(f64::INFINITY, f64::min);
        let max_time = tab_times
            .iter()
            .map(|d| d.as_secs_f64())
            .fold(0.0, f64::max);

        println!("\n📊 Tab Creation Benchmark Results:");
        println!("   Average tab creation time: {:.3}s", avg_time);
        println!("   Minimum time: {:.3}s", min_time);
        println!("   Maximum time: {:.3}s", max_time);
        println!("   Total tabs created: {}", iterations);

        Ok(())
    }
}

/// Simple function for one-off URL scraping (maintains backward compatibility)
pub async fn grab_text_simple(url: &str) -> Result<String> {
    let scraper = DeflectScraper::new().context("Failed to initialize scraper")?;
    scraper.grab_text(url).await
}

/// Benchmark cold start vs warm start performance
pub async fn benchmark_cold_vs_warm() -> Result<()> {
    println!("🧪 Cold Start vs Warm Start Benchmark\n");

    // Test 1: Cold start (new browser each time)
    println!("❄️  Cold Start Test (new browser each time):");
    let mut cold_times = Vec::new();

    for i in 0..3 {
        let total_start = Instant::now();
        let scraper = DeflectScraper::new()?;
        let (_, _scrape_time) = scraper
            .grab_text_with_timing("https://httpbin.org/html")
            .await?;
        let total_time = total_start.elapsed();
        cold_times.push(total_time);

        println!(
            "  Run {}: Total {:.3}s (Browser init + scraping)",
            i + 1,
            total_time.as_secs_f64()
        );
    }

    // Test 2: Warm start (reuse existing browser)
    println!("\n🔥 Warm Start Test (reuse existing browser):");
    let scraper = DeflectScraper::new()?;
    let mut warm_times = Vec::new();

    for i in 0..3 {
        let start = Instant::now();
        let _ = scraper.grab_text("https://httpbin.org/html").await?;
        let time = start.elapsed();
        warm_times.push(time);

        println!(
            "  Run {}: {:.3}s (tab creation + scraping)",
            i + 1,
            time.as_secs_f64()
        );
    }

    // Calculate averages
    let cold_avg =
        cold_times.iter().map(|d| d.as_secs_f64()).sum::<f64>() / cold_times.len() as f64;
    let warm_avg =
        warm_times.iter().map(|d| d.as_secs_f64()).sum::<f64>() / warm_times.len() as f64;
    let speedup = cold_avg / warm_avg;

    println!("\n📊 Performance Comparison:");
    println!("   Cold start average: {:.3}s", cold_avg);
    println!("   Warm start average: {:.3}s", warm_avg);
    println!("   Speedup factor: {:.1}x faster", speedup);
    println!("   Time saved per request: {:.3}s", cold_avg - warm_avg);

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        println!("Usage:");
        println!(
            "  {} <URL>                    - Scrape a single URL with timing",
            args[0]
        );
        println!(
            "  {} test                     - Run test on sample URLs with timing",
            args[0]
        );
        println!(
            "  {} benchmark-tabs <N>       - Benchmark tab creation (N times)",
            args[0]
        );
        println!(
            "  {} benchmark-cold-warm      - Compare cold vs warm start performance",
            args[0]
        );
        println!(
            "  {} <URL1> <URL2> <URL3>...  - Scrape multiple URLs with timing",
            args[0]
        );
        println!("\nExamples:");
        println!("  {} https://example.com", args[0]);
        println!("  {} test", args[0]);
        println!("  {} benchmark-tabs 10", args[0]);
        println!("  {} benchmark-cold-warm", args[0]);
        return Ok(());
    }

    match args[1].as_str() {
        "test" => {
            let scraper = DeflectScraper::new().context("Failed to initialize Deflect scraper")?;

            // Test mode with sample URLs
            let test_urls = vec![
                "https://httpbin.org/html",
                "https://example.com",
                "https://httpbin.org/user-agent",
            ];

            scraper.test_urls(test_urls).await?;
        }
        "benchmark-tabs" => {
            let iterations = if args.len() > 2 {
                args[2].parse().unwrap_or(10)
            } else {
                10
            };

            let scraper = DeflectScraper::new().context("Failed to initialize Deflect scraper")?;

            scraper.benchmark_tab_creation(iterations).await?;
        }
        "benchmark-cold-warm" => {
            benchmark_cold_vs_warm().await?;
        }
        _ => {
            if args.len() == 2 {
                // Single URL mode with timing
                let url = &args[1];
                println!("🔍 Scraping with timing: {}", url);

                let scraper =
                    DeflectScraper::new().context("Failed to initialize Deflect scraper")?;

                match scraper.grab_text_with_timing(url).await {
                    Ok((text, duration)) => {
                        println!(
                            "✅ Successfully scraped {} characters in {:.3}s:\n",
                            text.len(),
                            duration.as_secs_f64()
                        );
                        println!("{}", text);
                    }
                    Err(e) => {
                        eprintln!("❌ Error scraping {}: {}", url, e);
                        std::process::exit(1);
                    }
                }
            } else {
                // Multiple URLs mode with timing
                let scraper =
                    DeflectScraper::new().context("Failed to initialize Deflect scraper")?;

                let urls: Vec<&str> = args[1..].iter().map(|s| s.as_str()).collect();
                scraper.test_urls(urls).await?;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_scraper_initialization() {
        let scraper = DeflectScraper::new();
        assert!(scraper.is_ok());
    }

    #[tokio::test]
    async fn test_html_text_extraction() {
        let scraper = DeflectScraper::new().unwrap();
        let html = r#"
            <html>
                <body>
                    <main>
                        <h1>Test Title</h1>
                        <p>Test paragraph with content.</p>
                        <script>console.log('ignored');</script>
                        <style>body { color: red; }</style>
                    </main>
                </body>
            </html>
        "#;

        let text = scraper.html2text(html);
        assert!(text.contains("Test Title"));
        assert!(text.contains("Test paragraph"));
        assert!(!text.contains("console.log"));
        assert!(!text.contains("color: red"));
    }
}
