//! Example of using stateful client for conversation persistence

use cogni_client::{Client, StatefulClient};
use cogni_providers::OpenAI;
use cogni_state::{FileStore, MemoryStore};
use std::env;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize provider
    let api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set");
    let provider = OpenAI::with_api_key(api_key);
    let client = Client::new(provider);

    // Example 1: In-memory state store
    println!("=== Example 1: In-Memory State Store ===");
    {
        let store = Arc::new(MemoryStore::new());
        let mut stateful = client.clone().with_state(store.clone());

        // Start a new conversation
        let conversation_id = stateful.new_conversation().await?;
        println!("Started conversation: {}", conversation_id);

        // Set conversation metadata
        if let Some(state) = stateful.current_state_mut() {
            state.set_title("Product Support Chat");
            state.add_tag("support");
            state.add_tag("product-inquiry");
            state.set_custom("customer_id", "12345");
        }

        // Have a conversation
        let response1 = stateful.chat("Hi, I need help with my order").await?;
        println!("Assistant: {}", response1.content);

        let response2 = stateful.chat("The order number is ORD-789").await?;
        println!("Assistant: {}", response2.content);

        // Check token usage
        if let Some(state) = stateful.current_state() {
            if let Some(tokens) = state.metadata.token_count {
                println!("Total tokens used: {}", tokens);
            }
        }

        // List all conversations
        println!("\nAll conversations in memory:");
        for conv in stateful.list_conversations().await? {
            println!(
                "- {} ({}): {} messages",
                conv.id,
                conv.metadata
                    .title
                    .unwrap_or_else(|| "Untitled".to_string()),
                conv.messages.len()
            );
        }
    }

    // Example 2: File-based state store
    println!("\n=== Example 2: File-Based State Store ===");
    {
        let store = Arc::new(FileStore::new("./conversations")?);
        let mut stateful = client.with_state(store.clone());

        // Create a conversation
        let conversation_id = stateful.new_conversation().await?;
        println!("Created conversation: {}", conversation_id);

        // Chat
        let response = stateful.chat("What's the weather like today?").await?;
        println!("Assistant: {}", response.content);

        // Save explicitly (auto-save is enabled by default)
        stateful.save().await?;
        println!("Conversation saved to disk");

        // Clear current conversation
        stateful.clear_current();

        // Load the conversation back
        stateful.load_conversation(conversation_id).await?;
        println!("Loaded conversation from disk");

        // Continue the conversation
        let response = stateful.chat("What about tomorrow?").await?;
        println!("Assistant: {}", response.content);

        // Find conversations by tags
        if let Some(state) = stateful.current_state_mut() {
            state.add_tag("weather");
        }
        stateful.save().await?;

        let weather_convos = stateful.find_by_tags(&["weather".to_string()]).await?;
        println!("\nConversations tagged 'weather': {}", weather_convos.len());
    }

    // Example 3: Using State Middleware
    println!("\n=== Example 3: State Middleware ===");
    {
        use cogni_middleware::{ProviderExt, StateLayer};

        let store = Arc::new(MemoryStore::new());
        let layer = StateLayer::new(store);

        // Apply middleware to provider
        let stateful_provider = provider.layer(layer);

        // Now all requests through this provider will have automatic state management
        println!("State middleware configured - all requests will be tracked");
    }

    Ok(())
}
