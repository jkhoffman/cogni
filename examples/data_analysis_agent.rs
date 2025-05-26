//! # Data Analysis Agent Example
//!
//! This example demonstrates building a data analysis agent that:
//! - Analyzes datasets and provides insights
//! - Uses structured output for analysis results
//! - Maintains analysis history for iterative exploration
//! - Generates visualizations and statistical summaries

use cogni::{
    client::Client,
    context::{ContextManager, SlidingWindowStrategy, TiktokenCounter},
    middleware::CacheLayer,
    providers::OpenAIProvider,
    state::{MemoryStore, StateMetadata},
    Message, MessageContent, Role, StructuredOutput,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc, time::Duration};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, StructuredOutput)]
struct DataAnalysisReport {
    dataset_summary: DatasetSummary,
    statistical_analysis: StatisticalAnalysis,
    patterns_found: Vec<Pattern>,
    anomalies: Vec<Anomaly>,
    insights: Vec<Insight>,
    recommended_visualizations: Vec<Visualization>,
    next_steps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DatasetSummary {
    rows: usize,
    columns: usize,
    column_types: HashMap<String, DataType>,
    missing_values: HashMap<String, usize>,
    data_quality_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum DataType {
    Numeric,
    Categorical,
    DateTime,
    Text,
    Boolean,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StatisticalAnalysis {
    numeric_summaries: HashMap<String, NumericSummary>,
    categorical_summaries: HashMap<String, CategoricalSummary>,
    correlations: Vec<Correlation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct NumericSummary {
    mean: f64,
    median: f64,
    std_dev: f64,
    min: f64,
    max: f64,
    quartiles: [f64; 3],
    skewness: f64,
    kurtosis: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CategoricalSummary {
    unique_values: usize,
    mode: String,
    frequency_distribution: HashMap<String, usize>,
    entropy: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Correlation {
    variable1: String,
    variable2: String,
    coefficient: f64,
    p_value: f64,
    interpretation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Pattern {
    pattern_type: PatternType,
    description: String,
    confidence: f32,
    affected_columns: Vec<String>,
    impact: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum PatternType {
    Trend,
    Seasonality,
    Clustering,
    LinearRelationship,
    NonLinearRelationship,
    TimeSeries,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Anomaly {
    anomaly_type: AnomalyType,
    description: String,
    severity: Severity,
    location: String,
    suggested_action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum AnomalyType {
    Outlier,
    MissingData,
    DataTypeMismatch,
    UnexpectedPattern,
    DataQualityIssue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum Severity {
    Critical,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Insight {
    title: String,
    description: String,
    business_impact: String,
    confidence: f32,
    supporting_evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Visualization {
    chart_type: ChartType,
    title: String,
    variables: Vec<String>,
    purpose: String,
    priority: Priority,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum ChartType {
    Histogram,
    ScatterPlot,
    LineChart,
    BarChart,
    Heatmap,
    BoxPlot,
    TimeSeries,
    PairPlot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum Priority {
    Essential,
    Recommended,
    Optional,
}

#[derive(Debug, Clone, Serialize, Deserialize, StructuredOutput)]
struct QueryPlan {
    analysis_steps: Vec<AnalysisStep>,
    required_computations: Vec<String>,
    estimated_complexity: String,
    prerequisites: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AnalysisStep {
    step_number: u32,
    description: String,
    method: String,
    expected_output: String,
}

async fn run_data_analysis_agent() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize provider
    let provider = Arc::new(OpenAIProvider::new()?);

    // Set up state with in-memory store for this example
    let state_store = Arc::new(MemoryStore::new());

    // Create context manager
    let counter = TiktokenCounter::for_model("gpt-4o")?;
    let context_manager = ContextManager::new(
        Arc::new(counter)
    );

    // Create client with caching for repeated analyses
    let client = Client::new(provider)
        .with_middleware(CacheLayer::new(100, Duration::from_secs(3600)))
        .with_state(state_store.clone())
        .with_default_model("gpt-4o");

    // Create analysis session
    let session_id = Uuid::new_v4();
    let mut stateful_client = client.into_stateful();

    let system_prompt = r#"You are an expert data analyst and statistician with deep knowledge of:
- Statistical analysis and hypothesis testing
- Machine learning and pattern recognition
- Data visualization best practices
- Business intelligence and insights generation

Your approach:
1. First understand the data structure and quality
2. Perform appropriate statistical analyses
3. Identify patterns, trends, and anomalies
4. Generate actionable business insights
5. Recommend visualization strategies
6. Suggest next steps for deeper analysis

Always consider the business context and practical implications of your findings."#;

    stateful_client
        .new_conversation_with_metadata(
            session_id,
            StateMetadata {
                title: Some("Sales Data Analysis".to_string()),
                tags: vec!["data-analysis".to_string(), "sales".to_string()],
                ..Default::default()
            },
        )
        .await?;

    stateful_client
        .add_message(Message::system(system_prompt))
        .await?;

    println!("📊 Data Analysis Agent Started\n");

    // Example 1: Analyze sales dataset
    let dataset_description = r#"
I have a sales dataset with the following structure:
- Date: Daily records from 2023-01-01 to 2024-03-01
- Product_ID: 150 unique products
- Category: 8 product categories (Electronics, Clothing, Home, Sports, etc.)
- Sales_Amount: Revenue in USD
- Units_Sold: Number of units
- Customer_Segment: B2B, B2C, Enterprise
- Region: North, South, East, West, Central
- Discount_Percentage: 0-50%
- Customer_Satisfaction: 1-5 rating
- Return_Rate: Percentage of returns

The dataset has 45,000 rows with some missing values in Customer_Satisfaction (5%) and occasional data quality issues.
"#;

    // Step 1: Create analysis plan
    println!("📋 Creating analysis plan...\n");

    let plan_prompt = format!(
        "Create a comprehensive analysis plan for this dataset:\n{}",
        dataset_description
    );

    let analysis_plan: QueryPlan = stateful_client.chat_structured(&plan_prompt).await?;

    println!("Analysis Plan:");
    println!("  Complexity: {}", analysis_plan.estimated_complexity);
    println!("\n  Steps:");
    for step in &analysis_plan.analysis_steps {
        println!(
            "    {}. {} (Method: {})",
            step.step_number, step.description, step.method
        );
    }

    // Step 2: Perform comprehensive analysis
    println!("\n🔍 Performing data analysis...\n");

    let analysis_prompt = format!(
        r#"Please perform a comprehensive analysis of this sales dataset:

{}

Focus on:
1. Overall data quality and summary statistics
2. Sales trends and seasonality patterns
3. Product and category performance
4. Customer segment analysis
5. Regional variations
6. Impact of discounts on sales and returns
7. Customer satisfaction correlations
8. Anomalies and outliers
9. Predictive insights for future sales"#,
        dataset_description
    );

    let analysis_report: DataAnalysisReport =
        stateful_client.chat_structured(&analysis_prompt).await?;

    // Display analysis results
    println!("=" * 60);
    println!("DATA ANALYSIS REPORT");
    println!("=" * 60);

    println!("\n📈 DATASET SUMMARY:");
    println!("  Rows: {}", analysis_report.dataset_summary.rows);
    println!("  Columns: {}", analysis_report.dataset_summary.columns);
    println!(
        "  Data Quality Score: {:.1}/10",
        analysis_report.dataset_summary.data_quality_score
    );

    println!("\n🔍 KEY PATTERNS FOUND:");
    for (i, pattern) in analysis_report.patterns_found.iter().enumerate() {
        println!(
            "\n{}. {:?}: {}",
            i + 1,
            pattern.pattern_type,
            pattern.description
        );
        println!("   Confidence: {:.0}%", pattern.confidence * 100.0);
        println!("   Impact: {}", pattern.impact);
    }

    println!("\n⚠️  ANOMALIES DETECTED:");
    for anomaly in &analysis_report.anomalies {
        println!(
            "\n[{:?}] {:?}: {}",
            anomaly.severity, anomaly.anomaly_type, anomaly.description
        );
        println!("   Location: {}", anomaly.location);
        println!("   Action: {}", anomaly.suggested_action);
    }

    println!("\n💡 BUSINESS INSIGHTS:");
    for (i, insight) in analysis_report.insights.iter().enumerate() {
        println!("\n{}. {}", i + 1, insight.title);
        println!("   {}", insight.description);
        println!("   Business Impact: {}", insight.business_impact);
        println!("   Confidence: {:.0}%", insight.confidence * 100.0);
    }

    println!("\n📊 RECOMMENDED VISUALIZATIONS:");
    for viz in &analysis_report.recommended_visualizations {
        println!(
            "\n[{:?}] {:?}: {}",
            viz.priority, viz.chart_type, viz.title
        );
        println!("   Purpose: {}", viz.purpose);
        println!("   Variables: {}", viz.variables.join(", "));
    }

    // Step 3: Deep dive into specific finding
    println!("\n\n🔬 Deep dive analysis...\n");

    let deep_dive = stateful_client
        .chat(
            "Based on the patterns found, can you provide a detailed analysis of the relationship between discount rates and customer satisfaction? Include statistical significance and business recommendations.",
        )
        .await?;

    println!("Deep Dive Analysis:");
    println!("{}", deep_dive.content);

    // Step 4: Predictive analysis
    println!("\n\n📈 Predictive insights...\n");

    #[derive(Debug, Clone, Serialize, Deserialize, StructuredOutput)]
    struct PredictiveAnalysis {
        forecast_period: String,
        predicted_trends: Vec<PredictedTrend>,
        risk_factors: Vec<RiskFactor>,
        opportunities: Vec<Opportunity>,
        confidence_intervals: HashMap<String, (f64, f64)>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct PredictedTrend {
        metric: String,
        direction: String,
        magnitude: f64,
        drivers: Vec<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct RiskFactor {
        risk: String,
        probability: f32,
        impact: String,
        mitigation: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct Opportunity {
        description: String,
        potential_value: String,
        requirements: Vec<String>,
        timeline: String,
    }

    let predictive_analysis: PredictiveAnalysis = stateful_client
        .chat_structured(
            "Based on the analysis, provide predictive insights for the next quarter including trends, risks, and opportunities.",
        )
        .await?;

    println!("PREDICTIVE ANALYSIS: {}", predictive_analysis.forecast_period);
    println!("\nPredicted Trends:");
    for trend in &predictive_analysis.predicted_trends {
        println!(
            "- {}: {} by {:.1}%",
            trend.metric, trend.direction, trend.magnitude
        );
    }

    println!("\nOpportunities:");
    for opp in &predictive_analysis.opportunities {
        println!("- {}", opp.description);
        println!("  Potential Value: {}", opp.potential_value);
    }

    // Save analysis session
    println!("\n💾 Analysis session saved with ID: {}", session_id);

    // Show ability to continue analysis later
    println!("\n📂 Previous analyses available for comparison:");
    let sessions = state_store.list().await?;
    println!("Found {} analysis sessions", sessions.len());

    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(e) = run_data_analysis_agent().await {
        eprintln!("❌ Error: {}", e);
    }
}