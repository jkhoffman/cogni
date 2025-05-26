//! # Research Assistant Agent Example
//!
//! This example demonstrates building a research assistant that:
//! - Conducts multi-step research with tool usage
//! - Maintains research context across sessions
//! - Uses structured output for research findings
//! - Implements summarization for long documents

use cogni::{
    client::Client,
    context::{ContextManager, SummarizationStrategy, TiktokenCounter},
    providers::OpenAIProvider,
    state::{FileStore, StateMetadata},
    tools::{builtin::*, ToolExecutor, ToolRegistry},
    Message, MessageContent, Role, ToolCall, ToolResult, StructuredOutput,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, StructuredOutput)]
struct ResearchReport {
    topic: String,
    executive_summary: String,
    key_findings: Vec<Finding>,
    sources: Vec<Source>,
    confidence_level: ConfidenceLevel,
    areas_for_further_research: Vec<String>,
    methodology: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Finding {
    title: String,
    description: String,
    evidence: Vec<String>,
    relevance_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Source {
    title: String,
    url: Option<String>,
    citation: String,
    credibility_score: f32,
    accessed_date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum ConfidenceLevel {
    VeryHigh,
    High,
    Medium,
    Low,
    Speculative,
}

#[derive(Debug, Clone, Serialize, Deserialize, StructuredOutput)]
struct ResearchPlan {
    research_questions: Vec<String>,
    search_queries: Vec<String>,
    required_tools: Vec<String>,
    estimated_steps: u32,
    approach: String,
}

// Custom tool for web search (mock implementation)
struct WebSearchTool;

#[async_trait::async_trait]
impl ToolExecutor for WebSearchTool {
    fn name(&self) -> &str {
        "web_search"
    }

    fn description(&self) -> &str {
        "Search the web for information on a given topic"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The search query"
                },
                "num_results": {
                    "type": "integer",
                    "description": "Number of results to return",
                    "default": 5
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value, String> {
        let query = args["query"].as_str().unwrap_or("");
        let num_results = args["num_results"].as_u64().unwrap_or(5);

        // Mock search results
        let results = vec![
            json!({
                "title": format!("Research paper on {}", query),
                "url": "https://example.com/paper1",
                "snippet": format!("Recent findings about {} show significant developments...", query),
                "date": "2024-01-15"
            }),
            json!({
                "title": format!("Industry report: {}", query),
                "url": "https://example.com/report",
                "snippet": format!("Market analysis of {} indicates growing trends...", query),
                "date": "2024-02-01"
            }),
        ];

        Ok(json!({
            "query": query,
            "results": results.into_iter().take(num_results as usize).collect::<Vec<_>>()
        }))
    }
}

// Custom tool for reading documents
struct DocumentReaderTool;

#[async_trait::async_trait]
impl ToolExecutor for DocumentReaderTool {
    fn name(&self) -> &str {
        "read_document"
    }

    fn description(&self) -> &str {
        "Read and extract content from a document URL"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "URL of the document to read"
                }
            },
            "required": ["url"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value, String> {
        let url = args["url"].as_str().unwrap_or("");

        // Mock document content
        let content = format!(
            "Document from {}\n\nThis is a comprehensive analysis of the topic with multiple sections covering various aspects. The research indicates several key points that are worth noting for further investigation...",
            url
        );

        Ok(json!({
            "url": url,
            "content": content,
            "word_count": 150,
            "extracted_date": "2024-03-01"
        }))
    }
}

async fn run_research_assistant() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize provider
    let provider = Arc::new(OpenAIProvider::new()?);

    // Set up tool registry
    let mut tool_registry = ToolRegistry::new();
    tool_registry.register(Box::new(WebSearchTool));
    tool_registry.register(Box::new(DocumentReaderTool));
    tool_registry.register(Box::new(Calculator));

    // Set up state persistence
    let state_dir = PathBuf::from("./research_sessions");
    let state_store = Arc::new(FileStore::new(state_dir)?);

    // Create context manager with summarization strategy
    let counter = TiktokenCounter::for_model("gpt-4o")?;
    let summarizer_provider = provider.clone();
    let context_manager = ContextManager::new(
        Arc::new(counter)
    );

    // Create client with tools
    let client = Client::new(provider)
        .with_tools(tool_registry)
        .with_state(state_store.clone())
        .with_default_model("gpt-4o");

    // Create research session
    let session_id = Uuid::new_v4();
    let mut stateful_client = client.into_stateful();

    let system_prompt = r#"You are an expert research assistant with access to web search and document reading tools.

Your approach to research:
1. Break down complex topics into specific research questions
2. Use tools systematically to gather information
3. Verify information from multiple sources
4. Synthesize findings into clear, actionable insights
5. Maintain academic rigor and cite all sources
6. Acknowledge limitations and areas of uncertainty

Always create a research plan before starting your investigation."#;

    stateful_client
        .new_conversation_with_metadata(
            session_id,
            StateMetadata {
                title: Some("AI Safety Research".to_string()),
                tags: vec!["research".to_string(), "ai-safety".to_string()],
                ..Default::default()
            },
        )
        .await?;

    stateful_client
        .add_message(Message::system(system_prompt))
        .await?;

    // Step 1: Create research plan
    println!("🔬 Research Assistant Started\n");
    println!("📋 Creating research plan...\n");

    let research_topic = "Recent advances in AI alignment and safety measures in large language models";

    let plan_prompt = format!(
        "Create a research plan for investigating: {}",
        research_topic
    );

    let research_plan: ResearchPlan = stateful_client
        .chat_structured(&plan_prompt)
        .await?;

    println!("Research Plan:");
    println!("  Approach: {}", research_plan.approach);
    println!("  Estimated steps: {}", research_plan.estimated_steps);
    println!("\n  Research Questions:");
    for (i, question) in research_plan.research_questions.iter().enumerate() {
        println!("    {}. {}", i + 1, question);
    }

    // Step 2: Execute research with tools
    println!("\n🔍 Conducting research...\n");

    let research_prompt = format!(
        r#"Please conduct research on: {}

Use the web_search tool to find relevant information, then use read_document to examine promising sources in detail. Focus on recent developments (2023-2024)."#,
        research_topic
    );

    let research_response = stateful_client
        .chat_with_tools(&research_prompt)
        .await?;

    // Process tool calls
    if let Some(tool_calls) = &research_response.tool_calls {
        println!("📡 Executing {} tool calls...", tool_calls.len());
        
        for tool_call in tool_calls {
            println!("  - {}: {}", tool_call.name, tool_call.id);
        }

        // Execute tools and get results
        let tool_results = stateful_client
            .execute_tool_calls(tool_calls)
            .await?;

        // Continue conversation with tool results
        let synthesis_response = stateful_client
            .continue_with_tools(tool_results)
            .await?;

        println!("\n📊 Initial findings gathered");
    }

    // Step 3: Generate structured research report
    println!("\n📝 Generating research report...\n");

    let report_prompt = "Based on your research, please generate a comprehensive research report with all findings, sources, and recommendations.";

    let report: ResearchReport = stateful_client
        .chat_structured(report_prompt)
        .await?;

    // Display report
    println!("=" * 60);
    println!("RESEARCH REPORT: {}", report.topic);
    println!("=" * 60);
    println!("\nEXECUTIVE SUMMARY:");
    println!("{}", report.executive_summary);
    println!("\nCONFIDENCE LEVEL: {:?}", report.confidence_level);

    println!("\nKEY FINDINGS:");
    for (i, finding) in report.key_findings.iter().enumerate() {
        println!("\n{}. {}", i + 1, finding.title);
        println!("   {}", finding.description);
        println!("   Relevance: {:.1}/5.0", finding.relevance_score);
        if !finding.evidence.is_empty() {
            println!("   Evidence:");
            for evidence in &finding.evidence {
                println!("   - {}", evidence);
            }
        }
    }

    println!("\nSOURCES:");
    for source in &report.sources {
        println!("- {} (Credibility: {:.1}/5.0)", source.title, source.credibility_score);
        println!("  {}", source.citation);
    }

    if !report.areas_for_further_research.is_empty() {
        println!("\nAREAS FOR FURTHER RESEARCH:");
        for area in &report.areas_for_further_research {
            println!("- {}", area);
        }
    }

    // Step 4: Follow-up questions
    println!("\n\n💬 Asking follow-up question...\n");

    let follow_up = stateful_client
        .chat("What are the main challenges in implementing these safety measures in production systems?")
        .await?;

    println!("Assistant: {}", follow_up.content);

    // Save research session
    println!("\n💾 Research session saved with ID: {}", session_id);

    // Demonstrate session continuation
    println!("\n📂 Available research sessions:");
    let sessions = state_store.find_by_tags(&["research"]).await?;
    for session in sessions {
        if let Some(title) = session.metadata.title {
            println!("- {} ({})", title, session.id);
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(e) = run_research_assistant().await {
        eprintln!("❌ Error: {}", e);
    }
}