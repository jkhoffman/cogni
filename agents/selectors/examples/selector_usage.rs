use async_trait::async_trait;
use cogni_agents_selectors::{
    CapabilityBasedSelector, CapabilitySelectorConfig, NameBasedSelector, PatternBasedSelector,
    ToolSelectorRegistry,
};
use cogni_core::error::ToolConfigError;
use cogni_core::error::ToolError;
use cogni_core::traits::tool::{Tool, ToolCapability, ToolConfig};
use cogni_tools_registry::ToolRegistry;
use serde_json::json;
use std::sync::Arc;

// Sample tool implementation
#[derive(Debug, Clone)]
struct SearchTool;

#[async_trait]
impl Tool for SearchTool {
    type Input = serde_json::Value;
    type Output = serde_json::Value;
    type Config = ();

    fn try_new(_config: Self::Config) -> Result<Self, ToolConfigError> {
        Ok(Self)
    }

    async fn initialize(&mut self) -> Result<(), ToolError> {
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), ToolError> {
        Ok(())
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![
            ToolCapability::Stateless,
            ToolCapability::ThreadSafe,
            ToolCapability::NetworkAccess,
        ]
    }

    async fn invoke(&self, input: Self::Input) -> Result<Self::Output, ToolError> {
        // Just echo back the input in a real implementation
        Ok(json!({
            "query": input,
            "results": ["Sample result 1", "Sample result 2"]
        }))
    }
}

// Sample tool implementation
#[derive(Debug, Clone)]
struct MathTool;

#[async_trait]
impl Tool for MathTool {
    type Input = serde_json::Value;
    type Output = serde_json::Value;
    type Config = ();

    fn try_new(_config: Self::Config) -> Result<Self, ToolConfigError> {
        Ok(Self)
    }

    async fn initialize(&mut self) -> Result<(), ToolError> {
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), ToolError> {
        Ok(())
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![
            ToolCapability::Stateless,
            ToolCapability::ThreadSafe,
            ToolCapability::CpuIntensive,
        ]
    }

    async fn invoke(&self, input: Self::Input) -> Result<Self::Output, ToolError> {
        // Just echo back the input in a real implementation
        Ok(json!({
            "expression": input,
            "result": 42
        }))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a registry and register tools
    let registry = Arc::new(ToolRegistry::new());

    // Register search tool
    registry
        .register("search", "1.0.0", SearchTool, vec![])
        .await?;

    // Register math tool
    registry.register("math", "1.0.0", MathTool, vec![]).await?;

    println!("Registered tools:");
    for (name, version, metadata) in registry.get_all_tools() {
        println!(
            "- {} (v{}): {} with {} capabilities",
            name,
            version,
            metadata.description,
            metadata.capabilities.len()
        );
    }

    // Create a name-based selector
    let name_selector = NameBasedSelector::new(vec!["search", "weather"]);

    // Create a pattern-based selector
    let pattern_selector = PatternBasedSelector::new(vec!["^ma", "search$"]).unwrap();

    // Create a capability-based selector
    let capability_config = CapabilitySelectorConfig {
        required_capabilities: vec![ToolCapability::Stateless],
        preferred_capabilities: vec![ToolCapability::NetworkAccess],
        excluded_capabilities: vec![],
        max_tools: Some(3),
    };
    let capability_selector =
        CapabilityBasedSelector::new(capability_config, Arc::clone(&registry));

    // Create a selector registry
    let selector_registry = ToolSelectorRegistry::new(Arc::clone(&registry));

    // Test name-based selector
    println!("\nTesting name-based selector:");
    let selected_tools = name_selector.select_tools("some query", &json!({})).await?;
    println!("Selected tools: {:?}", selected_tools);

    // Validate tools against registry
    let validated_tools = selector_registry
        .validate_selector(&name_selector, "some query", &json!({}))
        .await?;
    println!("Validated tools: {:?}", validated_tools);

    // Test pattern-based selector
    println!("\nTesting pattern-based selector:");
    let context = json!({
        "available_tools": ["math", "search", "other-tool"]
    });
    let selected_tools = pattern_selector
        .select_tools("some query", &context)
        .await?;
    println!("Selected tools: {:?}", selected_tools);

    // Test capability-based selector
    println!("\nTesting capability-based selector:");
    let selected_tools = capability_selector
        .select_tools("some query", &json!({}))
        .await?;
    println!("Selected tools: {:?}", selected_tools);

    Ok(())
}
