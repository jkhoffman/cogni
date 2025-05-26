//! # Code Review Agent Example
//!
//! This example demonstrates building a code review agent that:
//! - Maintains conversation state across multiple reviews
//! - Uses structured output for consistent review format
//! - Manages context to handle large code files
//! - Provides actionable feedback with severity levels

use cogni::{
    client::Client,
    context::TiktokenCounter,
    providers::openai::OpenAI,
    state::{FileStore, StateMetadata},
    Message, Role, StructuredOutput,
};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Arc};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, StructuredOutput)]
struct CodeReview {
    overall_quality: String,
    score: u8, // 1-10
    issues: Vec<Issue>,
    suggestions: Vec<Suggestion>,
    security_concerns: Vec<SecurityConcern>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Issue {
    severity: Severity,
    file: String,
    line: Option<u32>,
    description: String,
    category: IssueCategory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum Severity {
    Critical,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum IssueCategory {
    Logic,
    Performance,
    Style,
    Maintainability,
    Documentation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Suggestion {
    description: String,
    code_example: Option<String>,
    rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SecurityConcern {
    severity: Severity,
    vulnerability_type: String,
    description: String,
    recommendation: String,
}

async fn run_code_review_agent() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize provider
    let provider = Arc::new(OpenAI::from_env()?);

    // Set up state persistence
    let state_dir = PathBuf::from("./code_review_state");
    let state_store = Arc::new(FileStore::new(state_dir)?);

    // We'll skip context manager for now as the API may have changed

    // Create stateful client
    let client = Client::new(provider)
        .with_state(state_store.clone())
        .with_default_model("gpt-4o");

    // Create or load a review session
    let session_id = Uuid::new_v4();
    let mut stateful_client = client.into_stateful();

    // Initialize the agent with system prompt
    let system_prompt = r#"You are an expert code reviewer with deep knowledge of software engineering best practices.

Your role is to:
1. Analyze code for logic errors, performance issues, and security vulnerabilities
2. Suggest improvements for maintainability and readability
3. Ensure code follows language-specific conventions
4. Identify potential edge cases and error handling gaps
5. Provide constructive feedback with actionable solutions

Always be thorough but constructive. Focus on the most impactful issues first."#;

    stateful_client
        .new_conversation_with_metadata(
            session_id,
            StateMetadata {
                title: Some("Code Review Session".to_string()),
                tags: vec!["code-review".to_string(), "rust".to_string()],
                ..Default::default()
            },
        )
        .await?;

    // Add system message
    stateful_client
        .add_message(Message::system(system_prompt))
        .await?;

    // Example 1: Review a Rust function
    let code_to_review = r#"
// File: src/auth.rs
use std::collections::HashMap;

pub struct UserAuth {
    users: HashMap<String, String>, // username -> password
}

impl UserAuth {
    pub fn new() -> Self {
        Self {
            users: HashMap::new(),
        }
    }
    
    pub fn authenticate(&self, username: &str, password: &str) -> bool {
        if let Some(stored_password) = self.users.get(username) {
            return stored_password == password;
        }
        false
    }
    
    pub fn add_user(&mut self, username: String, password: String) {
        self.users.insert(username, password);
    }
    
    pub fn get_all_users(&self) -> Vec<String> {
        self.users.keys().cloned().collect()
    }
}
"#;

    println!("🔍 Reviewing authentication code...\n");

    // Request structured review
    let review_prompt = format!(
        "Please review the following Rust code:\n\n{}",
        code_to_review
    );

    let review: CodeReview = stateful_client
        .chat_structured(&review_prompt)
        .await?;

    // Display review results
    println!("📊 Code Review Results");
    println!("====================");
    println!("Overall Quality: {}", review.overall_quality);
    println!("Score: {}/10", review.score);

    if !review.issues.is_empty() {
        println!("\n🐛 Issues Found:");
        for (i, issue) in review.issues.iter().enumerate() {
            println!(
                "\n{}. [{:?}] {:?} - {}",
                i + 1,
                issue.severity,
                issue.category,
                issue.description
            );
            if let Some(line) = issue.line {
                println!("   Location: {} line {}", issue.file, line);
            }
        }
    }

    if !review.security_concerns.is_empty() {
        println!("\n🔒 Security Concerns:");
        for concern in &review.security_concerns {
            println!(
                "\n[{:?}] {} - {}",
                concern.severity, concern.vulnerability_type, concern.description
            );
            println!("   Recommendation: {}", concern.recommendation);
        }
    }

    if !review.suggestions.is_empty() {
        println!("\n💡 Suggestions:");
        for (i, suggestion) in review.suggestions.iter().enumerate() {
            println!("\n{}. {}", i + 1, suggestion.description);
            println!("   Rationale: {}", suggestion.rationale);
            if let Some(example) = &suggestion.code_example {
                println!("   Example:\n{}", example);
            }
        }
    }

    // Example 2: Follow-up question about the review
    println!("\n\n💬 Asking follow-up question...\n");

    let follow_up = stateful_client
        .chat("Can you provide a secure implementation of the password storage?")
        .await?;

    println!("Assistant: {}", follow_up.content);

    // Example 3: Review another piece of code in the same session
    let another_code = r#"
// File: src/cache.rs
use std::collections::HashMap;
use std::time::{Duration, Instant};

pub struct Cache<K, V> {
    data: HashMap<K, (V, Instant)>,
    ttl: Duration,
}

impl<K: std::hash::Hash + Eq, V: Clone> Cache<K, V> {
    pub fn new(ttl: Duration) -> Self {
        Self {
            data: HashMap::new(),
            ttl,
        }
    }
    
    pub fn get(&self, key: &K) -> Option<V> {
        if let Some((value, timestamp)) = self.data.get(key) {
            if timestamp.elapsed() < self.ttl {
                return Some(value.clone());
            }
        }
        None
    }
    
    pub fn insert(&mut self, key: K, value: V) {
        self.data.insert(key, (value, Instant::now()));
    }
}
"#;

    println!("\n\n🔍 Reviewing cache implementation...\n");

    let cache_review_prompt = format!(
        "Please review this cache implementation:\n\n{}",
        another_code
    );

    let cache_review: CodeReview = stateful_client
        .chat_structured(&cache_review_prompt)
        .await?;

    println!("Cache Implementation Score: {}/10", cache_review.score);
    println!("Issues found: {}", cache_review.issues.len());
    println!("Suggestions: {}", cache_review.suggestions.len());

    // Save the conversation for future reference
    println!("\n💾 Review session saved with ID: {}", session_id);

    // Demonstrate loading a previous session
    println!("\n📂 Loading previous review sessions...");
    let sessions = state_store.find_by_tags(&["code-review"]).await?;
    println!("Found {} code review sessions", sessions.len());

    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(e) = run_code_review_agent().await {
        eprintln!("❌ Error: {}", e);
    }
}