//! Example demonstrating combined agentic features:
//! - Stateful conversation management
//! - Context-aware message pruning
//! - Structured output generation
//!
//! This example shows how to build an intelligent agent that maintains conversation state,
//! manages context windows efficiently, and produces structured responses.

use cogni::{
    client::{Client, StatefulClient},
    context::{ContextManager, SlidingWindowStrategy, TiktokenCounter},
    providers::openai::OpenAI,
    state::FileStore,
    Error, Message, Provider, Request, ResponseFormat, StructuredOutput,
};
use serde::{Deserialize, Serialize};
use std::{path::Path, sync::Arc};

// Define structured output types
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DataAnalysis {
    summary: String,
    key_insights: Vec<String>,
    metrics: AnalysisMetrics,
    recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AnalysisMetrics {
    total_revenue: f64,
    growth_percentage: f32,
    top_performing_category: String,
    risk_score: u8,
}

impl StructuredOutput for DataAnalysis {
    fn schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "summary": {
                    "type": "string",
                    "description": "High-level summary of the analysis"
                },
                "key_insights": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "List of important insights discovered"
                },
                "metrics": {
                    "type": "object",
                    "properties": {
                        "total_revenue": {
                            "type": "number",
                            "description": "Total revenue in dollars"
                        },
                        "growth_percentage": {
                            "type": "number",
                            "description": "Growth percentage compared to previous period"
                        },
                        "top_performing_category": {
                            "type": "string",
                            "description": "Best performing product category"
                        },
                        "risk_score": {
                            "type": "integer",
                            "minimum": 0,
                            "maximum": 100,
                            "description": "Risk assessment score (0-100)"
                        }
                    },
                    "required": ["total_revenue", "growth_percentage", "top_performing_category", "risk_score"]
                },
                "recommendations": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Actionable recommendations based on analysis"
                }
            },
            "required": ["summary", "key_insights", "metrics", "recommendations"]
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // 1. Initialize provider
    let api_key =
        std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY environment variable must be set");

    let provider = OpenAI::with_api_key(api_key.clone());

    // 2. Create client with state persistence
    let state_store = Arc::new(
        FileStore::new(Path::new("./agent_conversations")).expect("Failed to create state store"),
    );

    let client = Client::new(provider.clone());
    let mut agent = client.with_state(state_store.clone());

    // 3. Set up context management
    let context_manager = ContextManager::new(Arc::new(
        TiktokenCounter::for_model("gpt-4o-mini").expect("Failed to create token counter"),
    ))
    .with_max_tokens(8000) // Reserve space within model's context window
    .with_reserve_output_tokens(1000) // Reserve tokens for the response
    .with_strategy(Arc::new(SlidingWindowStrategy::new(true, 10))); // Keep system + last 10

    // 4. Create or load conversation
    let conversation_id = agent.new_conversation().await?;
    println!("Starting new conversation with ID: {}", conversation_id);

    // Initialize conversation with system prompt and metadata
    let system_message = Message::system(
        "You are a data analysis assistant specializing in business metrics and insights. \
         Analyze data thoroughly and provide structured insights with actionable recommendations.",
    );

    // Add system message to the conversation
    if let Some(state) = agent.current_state_mut() {
        state.add_message(system_message);
    }

    // Update metadata
    if let Some(state) = agent.current_state_mut() {
        state.set_title("Q4 Sales Analysis Session");
        state.add_tag("sales");
        state.add_tag("analysis");
        state.add_tag("q4-2024");
        state
            .metadata
            .custom
            .insert("department".to_string(), "sales".to_string());
        state
            .metadata
            .custom
            .insert("analyst".to_string(), "ai-agent".to_string());
    }

    // 5. Simulate a conversation with multiple interactions
    println!("\n=== Starting Analysis Conversation ===\n");

    // First interaction: Upload initial data
    let response1 = agent
        .chat(
            "I have sales data for Q4 2024. Total revenue was $2.3M across 5 product categories: \
               Electronics ($800K), Clothing ($600K), Home & Garden ($400K), Sports ($300K), \
               and Books ($200K). This represents a 15% increase from Q3.",
        )
        .await?;
    println!("Agent: {}\n", response1.content);

    // Second interaction: Dive deeper
    let response2 = agent
        .chat(
            "The Electronics category grew 25% while Books declined by 10%. \
               We also saw increased customer acquisition costs in Sports and Home & Garden. \
               Customer retention improved across all categories except Books.",
        )
        .await?;
    println!("Agent: {}\n", response2.content);

    // Third interaction: Add context about competition
    let response3 = agent
        .chat(
            "Our main competitor launched a books subscription service in October, \
               and we've seen market share erosion in that category. However, our electronics \
               margins improved due to direct supplier relationships established in September.",
        )
        .await?;
    println!("Agent: {}\n", response3.content);

    // 6. Request structured analysis with context management
    println!("\n=== Generating Structured Analysis ===\n");

    // The context manager will ensure the conversation fits within token limits
    let analysis: DataAnalysis = chat_structured_with_context(
        &mut agent,
        &provider,
        "Based on our entire conversation about Q4 sales performance, provide a comprehensive \
         data analysis with key insights, metrics, and strategic recommendations.",
        context_manager,
        "gpt-4o", // Use a model that supports structured output
    )
    .await?;

    // 7. Display structured results
    println!("📊 Analysis Summary: {}\n", analysis.summary);

    println!("🔍 Key Insights:");
    for (i, insight) in analysis.key_insights.iter().enumerate() {
        println!("   {}. {}", i + 1, insight);
    }

    println!("\n📈 Metrics:");
    println!(
        "   Total Revenue: ${:.2}M",
        analysis.metrics.total_revenue / 1_000_000.0
    );
    println!("   Growth: {:.1}%", analysis.metrics.growth_percentage);
    println!(
        "   Top Category: {}",
        analysis.metrics.top_performing_category
    );
    println!("   Risk Score: {}/100", analysis.metrics.risk_score);

    println!("\n💡 Recommendations:");
    for (i, rec) in analysis.recommendations.iter().enumerate() {
        println!("   {}. {}", i + 1, rec);
    }

    // 8. Save conversation state
    agent.save().await?;
    println!("\n✅ Conversation saved with ID: {}", conversation_id);

    // 9. Demonstrate loading and continuing conversation
    println!("\n=== Loading and Continuing Conversation ===\n");

    // Create a new client and agent to demonstrate loading
    let provider2 = OpenAI::with_api_key(api_key);
    let client2 = Client::new(provider2);
    let mut agent2 = client2.with_state(state_store.clone());
    agent2.load_conversation(conversation_id).await?;

    let follow_up = agent2
        .chat("What specific actions should we take for the Books category?")
        .await?;
    println!("Follow-up response: {}", follow_up.content);

    // 10. List all conversations
    println!("\n=== All Conversations ===");
    let conversations = agent2.list_conversations().await?;
    for conv in conversations {
        println!(
            "- {} | {} | {} messages | {}",
            conv.id,
            conv.metadata
                .title
                .unwrap_or_else(|| "Untitled".to_string()),
            conv.messages.len(),
            conv.updated_at.format("%Y-%m-%d %H:%M")
        );
    }

    Ok(())
}

// Helper function to request structured output with context management
async fn chat_structured_with_context<P: Provider, T: StructuredOutput>(
    agent: &mut StatefulClient<P>,
    provider: &P,
    message: &str,
    context_manager: ContextManager,
    model: &str,
) -> Result<T, Error> {
    // Add the new message to conversation
    let user_message = Message::user(message);
    if let Some(state) = agent.current_state_mut() {
        state.add_message(user_message.clone());
    }

    // Get all messages and apply context management
    let all_messages = agent
        .current_state()
        .map(|s| s.messages.clone())
        .unwrap_or_default();
    let pruned_messages = context_manager.fit_messages(all_messages).await?;

    // Build and send request with structured output
    let request = Request::builder()
        .messages(pruned_messages)
        .model(model)
        .response_format(ResponseFormat::JsonSchema {
            schema: T::schema(),
            strict: true,
        })
        .build();

    let response = provider.request(request).await?;

    // Parse structured response
    let structured = response.parse_structured::<T>()?;

    // Add assistant response to conversation state
    if let Some(state) = agent.current_state_mut() {
        state.add_message(Message::assistant(&response.content));
    }

    // Auto-save if enabled
    agent.save().await?;

    Ok(structured)
}
