use cogni_core::traits::tool::Tool;
use cogni_tool_search::{SearchConfig, SearchInput, SearchTool};
use std::env;

#[tokio::main]
async fn main() {
    let api_key = env::var("SERPAPI_KEY").expect("SERPAPI_KEY not set");
    let config = SearchConfig {
        api_key,
        base_url: "https://serpapi.com/search".to_string(),
        rate_limit: 5.0,
        cache_duration: 60,
    };
    let mut tool = SearchTool::new(config);
    tool.initialize().await.expect("Failed to initialize tool");
    let input = SearchInput {
        query: "rust programming".to_string(),
        max_results: Some(3),
    };
    let output = tool.invoke(input).await.expect("Search failed");
    println!("Search results:");
    for (i, result) in output.results.iter().enumerate() {
        println!(
            "{}. {}\n   {}\n   {}\n",
            i + 1,
            result.title,
            result.url,
            result.snippet
        );
    }
}
