/// Tool pipeline module for the agent.
/// Manages the flow of tool invocations during agent execution.
use crate::error::AgentError;
use crate::traits::tool::{ToolCall, ToolRegistry, ToolSelector};

pub struct ToolPipeline;

impl ToolPipeline {
    /// Invoke a tool by name using the tool registry and selector.
    pub async fn invoke(
        tool_registry: &ToolRegistry,
        _selector: &dyn ToolSelector,
        tool_call: &ToolCall,
    ) -> Result<String, AgentError> {
        // Lookup tool by name
        let tool = tool_registry
            .get(&tool_call.name)
            .ok_or_else(|| AgentError::Runtime(format!("Tool not found: {}", tool_call.name)))?;

        // Invoke the tool with the input
        let result = tool
            .invoke(tool_call.input.clone())
            .await
            .map_err(AgentError::Tool)?;

        // Serialize the result to string
        serde_json::to_string(&result)
            .map_err(|e| AgentError::Runtime(format!("Serialization error: {}", e)))
    }
}
