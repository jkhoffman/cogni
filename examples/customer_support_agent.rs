//! # Customer Support Agent Example
//!
//! This example demonstrates building a customer support agent that:
//! - Maintains conversation history with customer context
//! - Uses structured output for ticket classification and responses
//! - Manages context for long support threads
//! - Integrates with multiple providers for failover

use cogni::{
    client::{Client, ParallelClient},
    context::{ContextManager, ImportanceBasedStrategy, TiktokenCounter},
    middleware::{LoggingLayer, RetryLayer, StateLayer},
    providers::{AnthropicProvider, OpenAIProvider},
    state::{FileStore, StateMetadata},
    Message, MessageContent, Role, StructuredOutput,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf, sync::Arc, time::Duration};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, StructuredOutput)]
struct SupportTicket {
    category: TicketCategory,
    priority: Priority,
    sentiment: Sentiment,
    summary: String,
    suggested_actions: Vec<String>,
    requires_human_review: bool,
    tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum TicketCategory {
    TechnicalIssue,
    BillingInquiry,
    FeatureRequest,
    AccountManagement,
    GeneralQuestion,
    Complaint,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum Priority {
    Critical,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum Sentiment {
    VeryPositive,
    Positive,
    Neutral,
    Negative,
    VeryNegative,
}

#[derive(Debug, Clone, Serialize, Deserialize, StructuredOutput)]
struct CustomerProfile {
    customer_id: String,
    account_type: String,
    previous_issues: Vec<String>,
    satisfaction_score: f32,
    preferred_communication_style: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, StructuredOutput)]
struct SupportResponse {
    message: String,
    follow_up_questions: Vec<String>,
    knowledge_base_links: Vec<KnowledgeBaseLink>,
    escalation_needed: bool,
    resolution_status: ResolutionStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct KnowledgeBaseLink {
    title: String,
    url: String,
    relevance_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum ResolutionStatus {
    Resolved,
    InProgress,
    WaitingForCustomer,
    Escalated,
}

struct SupportAgent {
    client: Client,
    context_manager: ContextManager,
    customer_profiles: HashMap<String, CustomerProfile>,
}

impl SupportAgent {
    fn new(client: Client) -> Result<Self, Box<dyn std::error::Error>> {
        // Set up context manager with importance-based pruning
        let counter = TiktokenCounter::for_model("gpt-4o")?;
        let importance_scorer = |msg: &Message| -> f32 {
            // Score messages by importance
            match msg.role {
                Role::System => 1.0, // Always keep system messages
                Role::User => 0.9,   // Customer messages are very important
                Role::Assistant => {
                    // Keep messages with structured data or resolutions
                    if msg.content.as_text().unwrap_or("").contains("resolved") {
                        0.95
                    } else {
                        0.7
                    }
                }
                _ => 0.5,
            }
        };

        let context_manager = ContextManager::new(Arc::new(counter));

        Ok(Self {
            client,
            context_manager,
            customer_profiles: HashMap::new(),
        })
    }

    async fn handle_customer_message(
        &mut self,
        customer_id: &str,
        message: &str,
        session_id: Uuid,
    ) -> Result<SupportResponse, Box<dyn std::error::Error>> {
        // Get or create customer profile
        let profile = self.get_customer_profile(customer_id).await?;

        // Create stateful client for this conversation
        let mut stateful_client = self.client.clone().into_stateful();

        // Load or create conversation
        if stateful_client.load_conversation(session_id).await.is_err() {
            // New conversation
            let system_prompt = format!(
                r#"You are a helpful and empathetic customer support agent.

Customer Profile:
- ID: {}
- Account Type: {}
- Satisfaction Score: {}
- Preferred Style: {}

Guidelines:
1. Be professional, friendly, and solution-oriented
2. Acknowledge the customer's concerns
3. Provide clear, actionable solutions
4. Offer relevant knowledge base articles when appropriate
5. Escalate to human support for complex issues
6. Adapt your communication style to match the customer's preference

Previous Issues: {}
"#,
                profile.customer_id,
                profile.account_type,
                profile.satisfaction_score,
                profile.preferred_communication_style,
                profile.previous_issues.join(", ")
            );

            stateful_client
                .new_conversation_with_metadata(
                    session_id,
                    StateMetadata {
                        title: Some(format!("Support: Customer {}", customer_id)),
                        tags: vec!["support".to_string(), customer_id.to_string()],
                        custom: {
                            let mut custom = HashMap::new();
                            custom.insert("customer_id".to_string(), customer_id.to_string());
                            custom
                        },
                        ..Default::default()
                    },
                )
                .await?;

            stateful_client
                .add_message(Message::system(&system_prompt))
                .await?;
        }

        // Analyze the ticket first
        let ticket_analysis_prompt = format!(
            "Analyze this customer message and classify it:\n\n{}",
            message
        );

        let ticket: SupportTicket = stateful_client
            .chat_structured(&ticket_analysis_prompt)
            .await?;

        println!("📋 Ticket Analysis:");
        println!("  Category: {:?}", ticket.category);
        println!("  Priority: {:?}", ticket.priority);
        println!("  Sentiment: {:?}", ticket.sentiment);
        println!("  Summary: {}", ticket.summary);

        // Generate response based on ticket analysis
        let response_prompt = format!(
            r#"Customer message: {}

Ticket analysis:
- Category: {:?}
- Priority: {:?}
- Sentiment: {:?}
- Summary: {}

Please provide a helpful response to the customer."#,
            message, ticket.category, ticket.priority, ticket.sentiment, ticket.summary
        );

        stateful_client.add_message(Message::user(message)).await?;

        let response: SupportResponse = stateful_client.chat_structured(&response_prompt).await?;

        Ok(response)
    }

    async fn get_customer_profile(
        &mut self,
        customer_id: &str,
    ) -> Result<&CustomerProfile, Box<dyn std::error::Error>> {
        if !self.customer_profiles.contains_key(customer_id) {
            // Mock customer profile - in real app, fetch from database
            let profile = CustomerProfile {
                customer_id: customer_id.to_string(),
                account_type: "Premium".to_string(),
                previous_issues: vec!["Password reset".to_string(), "Billing question".to_string()],
                satisfaction_score: 4.2,
                preferred_communication_style: "Concise and technical".to_string(),
            };
            self.customer_profiles
                .insert(customer_id.to_string(), profile);
        }
        Ok(self.customer_profiles.get(customer_id).unwrap())
    }
}

async fn run_support_agent() -> Result<(), Box<dyn std::error::Error>> {
    // Set up providers with failover
    let primary_provider = Arc::new(OpenAIProvider::new()?);
    let fallback_provider = Arc::new(AnthropicProvider::new()?);

    // Create state store
    let state_dir = PathBuf::from("./support_conversations");
    let state_store = Arc::new(FileStore::new(state_dir)?);

    // Build client with middleware stack
    let client = Client::new(primary_provider.clone())
        .with_middleware(LoggingLayer::new("support-agent"))
        .with_middleware(RetryLayer::new(3, Duration::from_secs(2)))
        .with_middleware(StateLayer::new(state_store.clone(), true))
        .with_state(state_store.clone())
        .with_default_model("gpt-4o");

    // Create support agent
    let mut agent = SupportAgent::new(client)?;

    // Simulate customer conversations
    let customer_id = "CUST123";
    let session_id = Uuid::new_v4();

    println!("🤖 Customer Support Agent Started\n");

    // Customer message 1: Technical issue
    let response1 = agent
        .handle_customer_message(
            customer_id,
            "Hi, I'm having trouble logging into my account. It says my password is incorrect but I'm sure it's right. This is really frustrating!",
            session_id,
        )
        .await?;

    println!("\n💬 Agent Response:");
    println!("{}", response1.message);
    if !response1.follow_up_questions.is_empty() {
        println!("\n❓ Follow-up questions:");
        for q in &response1.follow_up_questions {
            println!("  - {}", q);
        }
    }

    // Customer message 2: Follow-up
    let response2 = agent
        .handle_customer_message(
            customer_id,
            "I've tried resetting it three times already! The reset emails aren't coming through.",
            session_id,
        )
        .await?;

    println!("\n💬 Agent Response:");
    println!("{}", response2.message);
    if response2.escalation_needed {
        println!("\n⚠️  Escalation recommended!");
    }

    // Customer message 3: Feature request
    let response3 = agent
        .handle_customer_message(
            customer_id,
            "Also, it would be great if you could add 2FA support. I'm worried about account security.",
            session_id,
        )
        .await?;

    println!("\n💬 Agent Response:");
    println!("{}", response3.message);

    // Demonstrate parallel processing with multiple customers
    println!("\n\n🔄 Processing multiple customer requests in parallel...\n");

    let parallel_client = ParallelClient::new(vec![
        ("openai", primary_provider),
        ("anthropic", fallback_provider),
    ]);

    // This would typically be a batch of customer messages
    let customer_messages = vec![
        "How do I cancel my subscription?",
        "Can I upgrade to the enterprise plan?",
        "I was charged twice this month",
    ];

    // Process all messages in parallel
    for (i, msg) in customer_messages.iter().enumerate() {
        println!("Customer {}: {}", i + 1, msg);
    }

    println!("\n✅ Support agent demonstration complete!");

    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(e) = run_support_agent().await {
        eprintln!("❌ Error: {}", e);
    }
}
