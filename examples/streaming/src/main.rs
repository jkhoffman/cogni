use anyhow::Result;
use cogni_core::traits::llm::{GenerateOptions, LanguageModel};
use cogni_core::LanguageModel as _;
use cogni_macros::chat_message;
use cogni_provider_openai::{ChatMessage, OpenAiClient, OpenAiConfig};
use futures::StreamExt;
use std::io::Write;
use tokio::time::{sleep, Duration};

const MAX_RETRIES: u32 = 3;
const RETRY_DELAY_MS: u64 = 100;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Initialize the OpenAI client
    let api_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set");
    let config = OpenAiConfig::new(api_key, "gpt-3.5-turbo");
    let llm = OpenAiClient::new(config)?;

    // Create a simple prompt using the chat_message macro
    let prompt = vec![chat_message!(user: "Tell me a story about a wise AI and a curious human.")];

    // Stream the response
    println!("Streaming response:\n");
    let mut stream = llm
        .stream_generate(prompt, GenerateOptions::default())
        .await?;

    let mut had_error = false;
    let mut consecutive_errors = 0;
    while let Some(token) = stream.next().await {
        match token {
            Ok(t) => {
                if had_error {
                    // Add a space after error to separate continued text
                    print!(" ");
                    had_error = false;
                }
                print!("{}", t);
                std::io::stdout().flush()?;
                consecutive_errors = 0;
            }
            Err(e) => {
                consecutive_errors += 1;
                if consecutive_errors > MAX_RETRIES {
                    eprintln!("\nToo many consecutive errors, stopping stream: {}", e);
                    break;
                }
                if e.to_string().contains("EOF while parsing") {
                    // For JSON parsing errors, wait briefly and continue
                    sleep(Duration::from_millis(RETRY_DELAY_MS)).await;
                    had_error = true;
                    continue;
                }
                eprintln!("\nError during streaming: {}", e);
                had_error = true;
            }
        }
    }
    println!("\n\nStream completed.");

    Ok(())
}
