use anyhow::Result;
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json;

#[derive(Debug, Deserialize, Serialize)]
struct GoogleSearchResponse {
    kind: String,
    items: Option<Vec<GoogleSearchItem>>,
    #[serde(rename = "searchInformation")]
    search_information: Option<SearchInformation>,
    queries: Option<Queries>,
}

#[derive(Debug, Deserialize, Serialize)]
struct GoogleSearchItem {
    title: String,
    link: String,
    snippet: String,
    #[serde(rename = "displayLink")]
    display_link: Option<String>,
    #[serde(rename = "formattedUrl")]
    formatted_url: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct SearchInformation {
    #[serde(rename = "totalResults")]
    total_results: String,
    #[serde(rename = "searchTime")]
    search_time: f64,
}

#[derive(Debug, Deserialize, Serialize)]
struct Queries {
    request: Option<Vec<RequestQuery>>,
    #[serde(rename = "nextPage")]
    next_page: Option<Vec<RequestQuery>>,
}

#[derive(Debug, Deserialize, Serialize)]
struct RequestQuery {
    title: String,
    #[serde(rename = "totalResults")]
    total_results: String,
    #[serde(rename = "searchTerms")]
    search_terms: String,
    count: u32,
    #[serde(rename = "startIndex")]
    start_index: u32,
}

const GOOGLE_CSE_BASE_URL: &str = "https://www.googleapis.com/customsearch/v1";

async fn test_google_search(api_key: &str, cx: &str, query: &str) -> Result<GoogleSearchResponse> {
    let client = reqwest::Client::new();

    let url = format!(
        "{}?key={}&cx={}&q={}",
        GOOGLE_CSE_BASE_URL,
        api_key,
        cx,
        urlencoding::encode(query)
    );

    println!("🔍 Making request to Google Custom Search API");
    println!("Query: {}", query);
    println!("URL: {}", url.replace(api_key, "***API_KEY***"));

    let response = client.get(&url).send().await?;

    let status = response.status();
    println!("Response Status: {}", status);

    if !status.is_success() {
        let error_text = response.text().await?;
        println!("❌ Error Response: {}", error_text);
        return Err(anyhow::anyhow!("API request failed: {}", error_text));
    }

    let body = response.text().await?;
    println!("✅ Response received ({} bytes)", body.len());

    // Dump the raw response content
    println!("\n📄 Raw Response Content:");
    println!("{}", "─".repeat(60));
    println!("{}", body);
    println!("{}", "─".repeat(60));

    let search_response: GoogleSearchResponse = serde_json::from_str(&body)?;
    Ok(search_response)
}

async fn compare_with_serper(query: &str) -> Result<()> {
    println!("\n🔄 Comparing with Serper API (if available)...");

    if let Ok(serper_key) = std::env::var("SERPER_API_KEY") {
        let client = reqwest::Client::new();

        let serper_url = "https://google.serper.dev/search";
        let serper_query = format!("{}+site:www.btselem.org", query);

        let serper_response = client
            .get(&format!(
                "{}?q={}&apiKey={}&num=5",
                serper_url,
                urlencoding::encode(&serper_query),
                serper_key
            ))
            .send()
            .await?;

        if serper_response.status().is_success() {
            let serper_body = serper_response.text().await?;
            println!("📊 Serper API Response Size: {} bytes", serper_body.len());

            // Parse and count results
            if let Ok(serper_json) = serde_json::from_str::<serde_json::Value>(&serper_body) {
                if let Some(organic) = serper_json.get("organic").and_then(|o| o.as_array()) {
                    println!("📊 Serper Results Count: {}", organic.len());
                } else {
                    println!("📊 Serper Results: Unable to parse organic results");
                }
            }
        } else {
            println!("❌ Serper API request failed: {}", serper_response.status());
        }
    } else {
        println!("⚠️  SERPER_API_KEY not set, skipping comparison");
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Google Programmable Search Engine API Test");
    println!("{}", "=".repeat(50));

    // Your provided API key
    let api_key = "AIzaSyCkKlLvDYL8WFsGD3ZgEQGEqhFwJjmevsI";

    // You'll need to create a Custom Search Engine and get the CX ID
    // For now, let's try a few common test approaches

    let test_queries = vec![
        "human rights violations Gaza",
        "Israeli settlements West Bank",
        "Palestine conflict recent news",
    ];

    // Your Custom Search Engine ID
    let test_cx_ids = vec![
        "966c36571122845c0", // Your CSE ID
    ];

    println!("⚠️  NOTE: You need to:");
    println!("1. Create a Custom Search Engine at: https://cse.google.com/cse/");
    println!("2. Get your Search Engine ID (CX parameter)");
    println!("3. Replace the CX ID in this test");
    println!();

    for cx in &test_cx_ids {
        if cx.contains("xxxxxxxxx") {
            println!("⏭️  Skipping placeholder CX ID: {}", cx);
            continue;
        }

        println!("🔧 Testing with CX: {}", cx);

        for query in &test_queries {
            println!("\n{}", "─".repeat(40));

            match test_google_search(api_key, cx, query).await {
                Ok(response) => {
                    println!("✅ Search successful!");

                    if let Some(search_info) = &response.search_information {
                        println!("📊 Total Results: {}", search_info.total_results);
                        println!("⏱️  Search Time: {:.3}s", search_info.search_time);
                    }

                    if let Some(items) = &response.items {
                        println!("📋 Results Count: {}", items.len());

                        for (i, item) in items.iter().enumerate().take(3) {
                            println!("\n{}. {}", i + 1, item.title);
                            println!("   🔗 {}", item.link);
                            println!(
                                "   📝 {}",
                                if item.snippet.len() > 100 {
                                    format!("{}...", &item.snippet[..100])
                                } else {
                                    item.snippet.clone()
                                }
                            );
                        }
                    } else {
                        println!("⚠️  No search results returned");
                    }

                    // Dump formatted response
                    println!("\n📋 Formatted Response:");
                    println!("{}", "─".repeat(60));
                    println!("{}", serde_json::to_string_pretty(&response)?);
                    println!("{}", "─".repeat(60));

                    // Compare with Serper
                    //compare_with_serper(query).await?;
                }
                Err(error) => {
                    println!("❌ Search failed: {}", error);

                    // Check for common issues
                    let error_str = error.to_string();
                    if error_str.contains("Invalid Value") {
                        println!("💡 Hint: The CX (Search Engine ID) might be invalid");
                        println!("   Create a CSE at: https://cse.google.com/cse/");
                    } else if error_str.contains("keyInvalid") {
                        println!(
                            "💡 Hint: API key might be invalid or doesn't have Custom Search API enabled"
                        );
                        println!(
                            "   Enable it at: https://console.cloud.google.com/apis/library/customsearch.googleapis.com"
                        );
                    } else if error_str.contains("quotaExceeded") {
                        println!("💡 Hint: API quota exceeded. Free tier allows 100 queries/day");
                    }
                }
            }
        }

        break; // Only test first valid CX
    }

    println!("\n{}", "=".repeat(50));
    println!("🏁 Test completed!");
    println!("\n💡 Next Steps:");
    println!("1. Create your Custom Search Engine: https://cse.google.com/cse/");
    println!("2. Configure it to search the web or specific sites");
    println!("3. Get your CX (Search Engine) ID");
    println!("4. Update your codebase to replace Serper with Google CSE");

    Ok(())
}
