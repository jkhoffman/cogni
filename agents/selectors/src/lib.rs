//! Tool selector implementations for the Cogni framework.
//!
//! This module provides various implementations of the `ToolSelector` trait,
//! which is used by agents to select appropriate tools for a given input.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use async_trait::async_trait;
use cogni_core::error::AgentError;
use cogni_core::traits::agent::ToolSelector;
use cogni_core::traits::tool::{Tool, ToolCapability, ToolConfig, ToolSpec};
use cogni_tools_registry::{RegistryError, ToolRegistry};
use regex::Regex;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashSet;
use std::sync::Arc;
use tracing::{debug, instrument, warn};

/// Selector that chooses tools based on their names.
///
/// This selector matches tools whose names exactly match one of the names
/// in the predefined list.
#[derive(Debug, Clone)]
pub struct NameBasedSelector {
    /// The names of the tools to select
    tool_names: HashSet<String>,
}

impl NameBasedSelector {
    /// Create a new name-based selector.
    ///
    /// # Arguments
    /// * `tool_names` - The names of the tools to select
    pub fn new<I, S>(tool_names: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let tool_names = tool_names.into_iter().map(|s| s.into()).collect();
        Self { tool_names }
    }

    /// Add a tool name to the selector.
    ///
    /// # Arguments
    /// * `name` - The name of the tool to add
    pub fn add_tool_name(&mut self, name: impl Into<String>) {
        self.tool_names.insert(name.into());
    }

    /// Remove a tool name from the selector.
    ///
    /// # Arguments
    /// * `name` - The name of the tool to remove
    ///
    /// # Returns
    /// `true` if the name was present and removed, `false` otherwise
    pub fn remove_tool_name(&mut self, name: &str) -> bool {
        self.tool_names.remove(name)
    }
}

#[async_trait]
impl ToolSelector for NameBasedSelector {
    #[instrument(skip(self, _context))]
    async fn select_tools(
        &self,
        _input: &str,
        _context: &serde_json::Value,
    ) -> Result<Vec<String>, AgentError> {
        // Return a sorted list of tool names for deterministic behavior
        let mut names: Vec<String> = self.tool_names.iter().cloned().collect();
        names.sort();
        Ok(names)
    }
}

/// Selector that chooses tools based on regular expression patterns.
///
/// This selector matches tools whose names match specific regex patterns,
/// allowing for more flexible selection than the name-based selector.
#[derive(Debug, Clone)]
pub struct PatternBasedSelector {
    /// The patterns to match against tool names
    patterns: Vec<Regex>,
}

impl PatternBasedSelector {
    /// Create a new pattern-based selector.
    ///
    /// # Arguments
    /// * `patterns` - String patterns to match against tool names
    ///
    /// # Returns
    /// A new selector, or an error if any of the patterns is invalid
    pub fn new<I, S>(patterns: I) -> Result<Self, regex::Error>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut compiled_patterns = Vec::new();
        for pattern in patterns {
            compiled_patterns.push(Regex::new(pattern.as_ref())?);
        }
        Ok(Self {
            patterns: compiled_patterns,
        })
    }

    /// Add a pattern to the selector.
    ///
    /// # Arguments
    /// * `pattern` - The regex pattern to add
    ///
    /// # Returns
    /// Ok(()) on success, or an error if the pattern is invalid
    pub fn add_pattern(&mut self, pattern: &str) -> Result<(), regex::Error> {
        let compiled = Regex::new(pattern)?;
        self.patterns.push(compiled);
        Ok(())
    }
}

#[async_trait]
impl ToolSelector for PatternBasedSelector {
    #[instrument(skip(self, context))]
    async fn select_tools(
        &self,
        _input: &str,
        context: &serde_json::Value,
    ) -> Result<Vec<String>, AgentError> {
        // Extract available tool names from context if present
        let available_tools = if let Some(tools) = context.get("available_tools") {
            if let Some(tools_array) = tools.as_array() {
                tools_array
                    .iter()
                    .filter_map(|t| t.as_str().map(String::from))
                    .collect::<Vec<String>>()
            } else {
                return Err(AgentError::ToolSelectionFailed(
                    "available_tools is not an array".to_string(),
                ));
            }
        } else {
            return Err(AgentError::ToolSelectionFailed(
                "No available_tools in context".to_string(),
            ));
        };

        // Match patterns against available tools
        let mut selected = Vec::new();
        for tool_name in available_tools {
            for pattern in &self.patterns {
                if pattern.is_match(&tool_name) {
                    selected.push(tool_name.clone());
                    break;
                }
            }
        }

        Ok(selected)
    }
}

// Helper functions for serializing/deserializing ToolCapability
fn serialize_capability<S>(capability: &ToolCapability, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let cap_str = match capability {
        ToolCapability::Stateless => "Stateless",
        ToolCapability::ThreadSafe => "ThreadSafe",
        ToolCapability::NetworkAccess => "NetworkAccess",
        ToolCapability::FileSystem => "FileSystem",
        ToolCapability::CpuIntensive => "CpuIntensive",
        ToolCapability::MemoryIntensive => "MemoryIntensive",
        ToolCapability::Cryptographic => "Cryptographic",
        ToolCapability::GpuAccess => "GpuAccess",
    };
    serializer.serialize_str(cap_str)
}

fn deserialize_capability<'de, D>(deserializer: D) -> Result<ToolCapability, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    match s.as_str() {
        "Stateless" => Ok(ToolCapability::Stateless),
        "ThreadSafe" => Ok(ToolCapability::ThreadSafe),
        "NetworkAccess" => Ok(ToolCapability::NetworkAccess),
        "FileSystem" => Ok(ToolCapability::FileSystem),
        "CpuIntensive" => Ok(ToolCapability::CpuIntensive),
        "MemoryIntensive" => Ok(ToolCapability::MemoryIntensive),
        "Cryptographic" => Ok(ToolCapability::Cryptographic),
        "GpuAccess" => Ok(ToolCapability::GpuAccess),
        _ => Err(serde::de::Error::custom(format!(
            "Unknown tool capability: {}",
            s
        ))),
    }
}

/// Serialization/Deserialization wrapper for ToolCapability
#[derive(Debug, Clone)]
pub struct SerializableCapability(pub ToolCapability);

impl Serialize for SerializableCapability {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serialize_capability(&self.0, serializer)
    }
}

impl<'de> Deserialize<'de> for SerializableCapability {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(SerializableCapability(deserialize_capability(
            deserializer,
        )?))
    }
}

/// Configuration for a capability-based selector.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilitySelectorConfig {
    /// Required capabilities that tools must have
    #[serde(
        serialize_with = "serialize_capability_vec",
        deserialize_with = "deserialize_capability_vec"
    )]
    pub required_capabilities: Vec<ToolCapability>,

    /// Optional capabilities that are preferred but not required
    #[serde(
        serialize_with = "serialize_capability_vec",
        deserialize_with = "deserialize_capability_vec"
    )]
    pub preferred_capabilities: Vec<ToolCapability>,

    /// Capabilities that should not be present
    #[serde(
        serialize_with = "serialize_capability_vec",
        deserialize_with = "deserialize_capability_vec"
    )]
    pub excluded_capabilities: Vec<ToolCapability>,

    /// Maximum number of tools to select
    pub max_tools: Option<usize>,
}

// Helper functions for serializing/deserializing Vec<ToolCapability>
fn serialize_capability_vec<S>(
    capabilities: &[ToolCapability],
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let cap_strings: Vec<String> = capabilities
        .iter()
        .map(|cap| match cap {
            ToolCapability::Stateless => "Stateless".to_string(),
            ToolCapability::ThreadSafe => "ThreadSafe".to_string(),
            ToolCapability::NetworkAccess => "NetworkAccess".to_string(),
            ToolCapability::FileSystem => "FileSystem".to_string(),
            ToolCapability::CpuIntensive => "CpuIntensive".to_string(),
            ToolCapability::MemoryIntensive => "MemoryIntensive".to_string(),
            ToolCapability::Cryptographic => "Cryptographic".to_string(),
            ToolCapability::GpuAccess => "GpuAccess".to_string(),
        })
        .collect();

    cap_strings.serialize(serializer)
}

fn deserialize_capability_vec<'de, D>(deserializer: D) -> Result<Vec<ToolCapability>, D::Error>
where
    D: Deserializer<'de>,
{
    let string_vec: Vec<String> = Vec::deserialize(deserializer)?;
    let mut capabilities = Vec::with_capacity(string_vec.len());

    for s in string_vec {
        let capability = match s.as_str() {
            "Stateless" => ToolCapability::Stateless,
            "ThreadSafe" => ToolCapability::ThreadSafe,
            "NetworkAccess" => ToolCapability::NetworkAccess,
            "FileSystem" => ToolCapability::FileSystem,
            "CpuIntensive" => ToolCapability::CpuIntensive,
            "MemoryIntensive" => ToolCapability::MemoryIntensive,
            "Cryptographic" => ToolCapability::Cryptographic,
            "GpuAccess" => ToolCapability::GpuAccess,
            _ => {
                return Err(serde::de::Error::custom(format!(
                    "Unknown tool capability: {}",
                    s
                )))
            }
        };
        capabilities.push(capability);
    }

    Ok(capabilities)
}

/// Selector that chooses tools based on their capabilities.
///
/// This selector matches tools that have specific capabilities, allowing
/// for selection based on tool characteristics rather than names.
#[derive(Debug)]
pub struct CapabilityBasedSelector {
    /// Configuration for capability selection
    config: CapabilitySelectorConfig,
    /// Reference to the tool registry
    registry: Arc<ToolRegistry>,
}

impl CapabilityBasedSelector {
    /// Create a new capability-based selector.
    ///
    /// # Arguments
    /// * `config` - Configuration for capability selection
    /// * `registry` - Reference to the tool registry
    pub fn new(config: CapabilitySelectorConfig, registry: Arc<ToolRegistry>) -> Self {
        Self { config, registry }
    }

    /// Update the selector configuration.
    ///
    /// # Arguments
    /// * `config` - New configuration for capability selection
    pub fn update_config(&mut self, config: CapabilitySelectorConfig) {
        self.config = config;
    }
}

#[async_trait]
impl ToolSelector for CapabilityBasedSelector {
    #[instrument(skip(self, _context))]
    async fn select_tools(
        &self,
        _input: &str,
        _context: &serde_json::Value,
    ) -> Result<Vec<String>, AgentError> {
        // Get all tools from the registry
        let all_tools = self.registry.get_all_tools();

        // Filter tools based on capabilities
        let mut selected = Vec::new();

        for (name, _, metadata) in all_tools {
            let tool_capabilities = metadata.capabilities;

            // Check if the tool has all required capabilities
            let has_required = self
                .config
                .required_capabilities
                .iter()
                .all(|req| tool_capabilities.contains(req));

            if !has_required {
                continue;
            }

            // Check if the tool has any excluded capabilities
            let has_excluded = self
                .config
                .excluded_capabilities
                .iter()
                .any(|excl| tool_capabilities.contains(excl));

            if has_excluded {
                continue;
            }

            // Calculate how many preferred capabilities the tool has
            let preferred_count = self
                .config
                .preferred_capabilities
                .iter()
                .filter(|pref| tool_capabilities.contains(pref))
                .count();

            // Store the tool with its preference score for sorting later
            selected.push((name, preferred_count));
        }

        // Sort by number of preferred capabilities (descending)
        selected.sort_by(|a, b| b.1.cmp(&a.1));

        // Apply max tools limit if specified
        if let Some(max) = self.config.max_tools {
            selected.truncate(max);
        }

        // Extract just the names
        let result = selected.into_iter().map(|(name, _)| name).collect();

        Ok(result)
    }
}

/// Integration between ToolSelector and ToolRegistry.
///
/// This struct provides methods to connect tool selectors with the tool registry,
/// enabling discovery and validation of selected tools.
#[derive(Debug)]
pub struct ToolSelectorRegistry {
    /// Reference to the tool registry
    registry: Arc<ToolRegistry>,
}

impl ToolSelectorRegistry {
    /// Create a new tool selector registry.
    ///
    /// # Arguments
    /// * `registry` - Reference to the tool registry
    pub fn new(registry: Arc<ToolRegistry>) -> Self {
        Self { registry }
    }

    /// Validate that all tools selected by a selector exist in the registry.
    ///
    /// # Arguments
    /// * `selector` - The tool selector to validate
    /// * `input` - Input to use for tool selection
    /// * `context` - Context to use for tool selection
    ///
    /// # Returns
    /// Ok(Vec<String>) with validated tool names on success, or an error if validation fails
    #[instrument(skip(self, selector, context))]
    pub async fn validate_selector(
        &self,
        selector: &dyn ToolSelector,
        input: &str,
        context: &serde_json::Value,
    ) -> Result<Vec<String>, AgentError> {
        // Get selected tools
        let selected_tools = selector.select_tools(input, context).await?;

        // Validate each tool exists
        let mut validated_tools = Vec::new();
        for tool_name in selected_tools {
            match self.registry.get_metadata(&tool_name, None) {
                Ok(_) => {
                    validated_tools.push(tool_name);
                }
                Err(RegistryError::ToolNotFound { name, .. }) => {
                    warn!("Selected tool not found in registry: {}", name);
                    // Skip this tool
                }
                Err(e) => {
                    return Err(AgentError::ToolSelectionFailed(format!(
                        "Error validating tool {}: {}",
                        tool_name, e
                    )));
                }
            }
        }

        if validated_tools.is_empty() {
            debug!("No valid tools selected for input: {}", input);
        }

        Ok(validated_tools)
    }

    /// Get all tools that match the capabilities specified in the config.
    ///
    /// # Arguments
    /// * `config` - Configuration specifying required, preferred, and excluded capabilities
    ///
    /// # Returns
    /// A list of tool names that match the capability requirements
    pub fn get_tools_by_capabilities(&self, config: &CapabilitySelectorConfig) -> Vec<String> {
        let selector = CapabilityBasedSelector::new(config.clone(), Arc::clone(&self.registry));

        // Use a tokio runtime to execute the async method
        let rt = tokio::runtime::Runtime::new().unwrap();
        match rt.block_on(selector.select_tools("", &serde_json::Value::Null)) {
            Ok(tools) => tools,
            Err(_) => Vec::new(),
        }
    }

    /// Create a capability-based selector with the registry.
    ///
    /// # Arguments
    /// * `config` - Configuration for capability selection
    ///
    /// # Returns
    /// A new capability-based selector
    pub fn create_capability_selector(
        &self,
        config: CapabilitySelectorConfig,
    ) -> CapabilityBasedSelector {
        CapabilityBasedSelector::new(config, Arc::clone(&self.registry))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cogni_core::traits::tool::{Tool, ToolConfig, ToolSpec};
    use cogni_tools_registry::ToolMetadata;
    use cogni_tools_registry::ToolRegistry;
    use std::sync::Arc;

    // Mock tool implementation for testing
    struct MockTool;

    #[async_trait]
    impl Tool for MockTool {
        type Input = serde_json::Value;
        type Output = serde_json::Value;
        type Config = ();

        fn try_new(_config: Self::Config) -> Result<Self, cogni_core::error::ToolConfigError> {
            Ok(Self)
        }

        async fn initialize(&mut self) -> Result<(), cogni_core::error::ToolError> {
            Ok(())
        }

        async fn shutdown(&mut self) -> Result<(), cogni_core::error::ToolError> {
            Ok(())
        }

        fn capabilities(&self) -> Vec<ToolCapability> {
            vec![ToolCapability::Stateless, ToolCapability::ThreadSafe]
        }

        async fn invoke(
            &self,
            input: Self::Input,
        ) -> Result<Self::Output, cogni_core::error::ToolError> {
            Ok(input)
        }

        fn spec(&self) -> ToolSpec {
            ToolSpec {
                name: "mock-tool".to_string(),
                description: "A mock tool for testing".to_string(),
                input_schema: serde_json::json!({"type": "string"}),
                output_schema: serde_json::json!({"type": "string"}),
                examples: vec![],
            }
        }
    }

    #[tokio::test]
    async fn test_name_based_selector() {
        let selector = NameBasedSelector::new(vec!["tool1", "tool2", "tool3"]);

        let result = selector
            .select_tools("test input", &serde_json::Value::Null)
            .await
            .unwrap();

        assert_eq!(result.len(), 3);
        assert!(result.contains(&"tool1".to_string()));
        assert!(result.contains(&"tool2".to_string()));
        assert!(result.contains(&"tool3".to_string()));
    }

    #[tokio::test]
    async fn test_pattern_based_selector() {
        let selector = PatternBasedSelector::new(vec!["^test", "search$"]).unwrap();

        let context = serde_json::json!({
            "available_tools": [
                "test-tool",
                "other-tool",
                "web-search"
            ]
        });

        let result = selector.select_tools("query", &context).await.unwrap();

        assert_eq!(result.len(), 2);
        assert!(result.contains(&"test-tool".to_string()));
        assert!(result.contains(&"web-search".to_string()));
    }

    // More tests would be implemented here
}
