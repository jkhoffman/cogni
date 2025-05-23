use anyhow::Result;
use cogni_core::traits::llm::{GenerateOptions, LanguageModel};
use cogni_macros::chat_message;
use cogni_provider_openai::{ChatMessage, OpenAiClient, OpenAiConfig};
use std::env;
use std::io::{self, Write};

#[tokio::main]
async fn main() -> Result<()> {
    // Get API key from environment
    let api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set");

    // Create client
    let config = OpenAiConfig::new(api_key, "gpt-4");
    let client = OpenAiClient::new(config)?;

    println!("Chat with GPT-4 (type 'quit' to exit)");

    let mut conversation = vec![chat_message!(system: "You are a helpful assistant.")];

    loop {
        // Get user input
        print!("> ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input == "quit" {
            break;
        }

        // Add user message to conversation
        conversation.push(ChatMessage {
            role: "user".to_string(),
            content: input.to_string(),
        });

        // Get response from model
        let response = client
            .generate(conversation.clone(), GenerateOptions::default())
            .await?;

        // Print response
        println!("Assistant: {}", response.content);

        // Add response to conversation history
        conversation.push(response);
    }

    Ok(())
}
