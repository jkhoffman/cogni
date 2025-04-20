//! Tool registry for the Cogni framework.
//!
//! This crate provides a registry for tools that can be used by agents.
//! It handles tool discovery, versioning, dependency resolution, and validation.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod validation;

use std::{collections::HashSet, fmt::Debug, sync::Arc};

use anyhow::Result;
use cogni_core::{
    error::{ToolConfigError, ToolError},
    traits::tool::{Tool, ToolCapability, ToolConfig, ToolSpec},
};
use dashmap::DashMap;
use semver::Version;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{error, info, instrument, warn};

pub use validation::{ToolValidator, Validatable, ValidationError};

/// A tool dependency specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDependency {
    /// The name of the tool
    pub name: String,
    /// The version requirement for the tool
    pub version_req: String,
}

/// A tool metadata entry containing information about a registered tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetadata {
    /// The name of the tool
    pub name: String,
    /// The version of the tool
    #[serde(
        serialize_with = "serialize_version",
        deserialize_with = "deserialize_version"
    )]
    pub version: Version,
    /// A description of the tool
    pub description: String,
    /// The capabilities of the tool
    #[serde(skip)]
    pub capabilities: Vec<ToolCapability>,
    /// The dependencies of the tool
    pub dependencies: Vec<ToolDependency>,
    /// The specification of the tool
    pub spec: ToolSpec,
}

// Serialize Version as a string
fn serialize_version<S>(version: &Version, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&version.to_string())
}

// Deserialize Version from a string
fn deserialize_version<'de, D>(deserializer: D) -> Result<Version, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Version::parse(&s).map_err(serde::de::Error::custom)
}

/// Errors that can occur during tool registration or retrieval.
#[derive(Error, Debug)]
pub enum RegistryError {
    /// The tool name is invalid
    #[error("Invalid tool name: {0}")]
    InvalidToolName(String),

    /// The tool version is invalid
    #[error("Invalid tool version: {0}")]
    InvalidToolVersion(String),

    /// The tool is not found
    #[error("Tool not found: {name} (version {version:?})")]
    ToolNotFound {
        name: String,
        version: Option<String>,
    },

    /// The tool already exists
    #[error("Tool already exists: {name} (version {version})")]
    ToolAlreadyExists { name: String, version: String },

    /// The tool has unresolved dependencies
    #[error("Tool has unresolved dependencies: {0:?}")]
    UnresolvedDependencies(Vec<ToolDependency>),

    /// The tool has circular dependencies
    #[error("Tool has circular dependencies: {0:?}")]
    CircularDependencies(Vec<String>),

    /// The tool failed validation
    #[error("Tool validation failed: {0}")]
    ValidationFailed(String),

    /// The tool configuration is invalid
    #[error("Tool configuration is invalid: {0}")]
    InvalidConfig(#[from] ToolConfigError),

    /// A tool operation failed
    #[error("Tool operation failed: {0}")]
    ToolError(#[from] ToolError),

    /// An internal error occurred
    #[error("Internal registry error: {0}")]
    InternalError(String),
}

/// Helper trait combining Tool with Debug to work around trait object limitations
pub trait ToolWithDebug:
    Tool<Input = serde_json::Value, Output = serde_json::Value, Config = ()> + Send + Sync + Debug
{
}

// Implement ToolWithDebug for any type that meets the requirements
impl<T> ToolWithDebug for T where
    T: Tool<Input = serde_json::Value, Output = serde_json::Value, Config = ()>
        + Send
        + Sync
        + Debug
{
}

/// Define a unit config with debug that implements ToolConfig
#[derive(Debug, Clone, Default)]
pub struct EmptyConfig;

impl ToolConfig for EmptyConfig {
    fn validate(&self) -> Result<(), ToolConfigError> {
        Ok(())
    }
}

/// A wrapper for dynamic tools that implements Debug
pub struct DebugBoxedTool {
    /// The actual tool, wrapped in a non-Debug type
    #[allow(dead_code)]
    tool: Box<
        dyn Tool<Input = serde_json::Value, Output = serde_json::Value, Config = EmptyConfig>
            + Send
            + Sync,
    >,
}

impl Debug for DebugBoxedTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DebugBoxedTool")
            .field("tool", &"<dyn Tool>")
            .finish()
    }
}

impl DebugBoxedTool {
    /// Create a new DebugBoxedTool from a Tool
    pub fn new<T>(tool: T) -> Self
    where
        T: Tool<Input = serde_json::Value, Output = serde_json::Value, Config = EmptyConfig>
            + Send
            + Sync
            + 'static,
    {
        Self {
            tool: Box::new(tool),
        }
    }

    /// Get a reference to the inner tool
    pub fn inner(
        &self,
    ) -> &(dyn Tool<Input = serde_json::Value, Output = serde_json::Value, Config = EmptyConfig>
             + Send
             + Sync) {
        self.tool.as_ref()
    }

    /// Get a mutable reference to the inner tool
    pub fn inner_mut(
        &mut self,
    ) -> &mut (dyn Tool<Input = serde_json::Value, Output = serde_json::Value, Config = EmptyConfig>
                 + Send
                 + Sync) {
        self.tool.as_mut()
    }
}

/// Type alias for a boxed dyn Tool that can be stored in the registry.
pub type BoxedTool = DebugBoxedTool;

/// Information about an invocation of a tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInvocation {
    /// The name of the tool
    pub tool_name: String,
    /// The version of the tool
    pub tool_version: String,
    /// The input provided to the tool
    pub input: serde_json::Value,
    /// The output produced by the tool (if successful)
    pub output: Option<serde_json::Value>,
    /// The error produced by the tool (if failed)
    pub error: Option<String>,
    /// The time the invocation started
    pub start_time: chrono::DateTime<chrono::Utc>,
    /// The time the invocation ended
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
    /// The duration of the invocation in milliseconds
    pub duration_ms: Option<u64>,
}

/// The tool registry.
///
/// The registry is responsible for:
/// - Registering tools
/// - Tool discovery and retrieval
/// - Dependency resolution
/// - Tool validation
/// - Invocation tracking
///
/// It uses a concurrent map to store tools and their metadata,
/// allowing for safe concurrent access from multiple threads.
#[derive(Debug, Default)]
pub struct ToolRegistry {
    /// Map from tool name and version to tool instance
    tools: DashMap<(String, Version), Arc<RwLock<BoxedTool>>>,
    /// Map from tool name and version to tool metadata
    metadata: DashMap<(String, Version), ToolMetadata>,
    /// Recent invocations of tools
    invocations: Arc<RwLock<Vec<ToolInvocation>>>,
    /// Maximum number of invocations to track
    max_invocations: usize,
}

impl ToolRegistry {
    /// Create a new tool registry.
    pub fn new() -> Self {
        Self {
            tools: DashMap::new(),
            metadata: DashMap::new(),
            invocations: Arc::new(RwLock::new(Vec::new())),
            max_invocations: 100,
        }
    }

    /// Set the maximum number of invocations to track.
    pub fn set_max_invocations(&mut self, max: usize) {
        self.max_invocations = max;
    }

    /// Register a tool with the registry.
    ///
    /// # Arguments
    /// * `name` - The name of the tool
    /// * `version` - The version of the tool
    /// * `tool` - The tool instance
    /// * `dependencies` - The dependencies of the tool
    ///
    /// # Returns
    /// Returns `Ok(())` if the tool was registered successfully,
    /// or an error if the registration failed.
    ///
    /// # Errors
    /// May return `RegistryError` if:
    /// - The tool name is invalid
    /// - The tool version is invalid
    /// - The tool already exists
    /// - The tool has unresolved dependencies
    /// - The tool has circular dependencies
    /// - The tool failed validation
    /// - The tool configuration is invalid
    #[instrument(skip(self, tool))]
    pub async fn register<T>(
        &self,
        name: &str,
        version: &str,
        mut tool: T,
        dependencies: Vec<ToolDependency>,
    ) -> Result<(), RegistryError>
    where
        T: Tool<Input = serde_json::Value, Output = serde_json::Value, Config = EmptyConfig>
            + Send
            + Sync
            + 'static,
    {
        // Validate the tool name
        if name.is_empty()
            || !name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err(RegistryError::InvalidToolName(name.to_string()));
        }

        // Parse and validate the version
        let version = Version::parse(version)
            .map_err(|_| RegistryError::InvalidToolVersion(version.to_string()))?;

        // Check if the tool already exists
        if self
            .tools
            .contains_key(&(name.to_string(), version.clone()))
        {
            return Err(RegistryError::ToolAlreadyExists {
                name: name.to_string(),
                version: version.to_string(),
            });
        }

        // Validate dependencies
        self.validate_dependencies(&dependencies).await?;

        // Initialize the tool
        tool.initialize().await.map_err(RegistryError::ToolError)?;

        // Create tool metadata
        let capabilities = tool.capabilities();
        let spec = tool.spec();
        let metadata = ToolMetadata {
            name: name.to_string(),
            version: version.clone(),
            description: spec.description.clone(),
            capabilities,
            dependencies,
            spec,
        };

        // Store the tool in the registry
        let boxed_tool = DebugBoxedTool::new(tool);
        let tool_lock = Arc::new(RwLock::new(boxed_tool));

        self.tools
            .insert((name.to_string(), version.clone()), tool_lock);
        self.metadata.insert((name.to_string(), version), metadata);

        info!("Tool registered: {}", name);
        Ok(())
    }

    /// Get a tool by name and version.
    ///
    /// # Arguments
    /// * `name` - The name of the tool
    /// * `version` - The version of the tool (or None for latest)
    ///
    /// # Returns
    /// Returns `Ok(Arc<RwLock<BoxedTool>>)` if the tool was found,
    /// or an error if the tool was not found.
    ///
    /// # Errors
    /// May return `RegistryError::ToolNotFound` if the tool was not found.
    #[instrument(skip(self))]
    pub fn get_tool(
        &self,
        name: &str,
        version: Option<&str>,
    ) -> Result<Arc<RwLock<BoxedTool>>, RegistryError> {
        match version {
            Some(version_str) => {
                // Parse the version
                let version = Version::parse(version_str)
                    .map_err(|_| RegistryError::InvalidToolVersion(version_str.to_string()))?;

                // Get the tool instance
                self.tools
                    .get(&(name.to_string(), version.clone()))
                    .map(|tool| tool.value().clone())
                    .ok_or_else(|| RegistryError::ToolNotFound {
                        name: name.to_string(),
                        version: Some(version_str.to_string()),
                    })
            }
            None => {
                // Find the latest version
                let mut latest_version: Option<Version> = None;
                let mut latest_tool: Option<Arc<RwLock<BoxedTool>>> = None;

                for entry in self.tools.iter() {
                    let key = entry.key();
                    if key.0 == name
                        && (latest_version.is_none() || key.1 > *latest_version.as_ref().unwrap())
                    {
                        latest_version = Some(key.1.clone());
                        latest_tool = Some(entry.value().clone());
                    }
                }

                latest_tool.ok_or_else(|| RegistryError::ToolNotFound {
                    name: name.to_string(),
                    version: None,
                })
            }
        }
    }

    /// Get metadata for a tool by name and version.
    ///
    /// # Arguments
    /// * `name` - The name of the tool
    /// * `version` - The version of the tool (or None for latest)
    ///
    /// # Returns
    /// Returns `Ok(ToolMetadata)` if the tool was found,
    /// or an error if the tool was not found.
    ///
    /// # Errors
    /// May return `RegistryError::ToolNotFound` if the tool was not found.
    #[instrument(skip(self))]
    pub fn get_metadata(
        &self,
        name: &str,
        version: Option<&str>,
    ) -> Result<ToolMetadata, RegistryError> {
        match version {
            Some(version_str) => {
                // Parse the version
                let version = Version::parse(version_str)
                    .map_err(|_| RegistryError::InvalidToolVersion(version_str.to_string()))?;

                // Get the metadata
                self.metadata
                    .get(&(name.to_string(), version.clone()))
                    .map(|metadata| metadata.value().clone())
                    .ok_or_else(|| RegistryError::ToolNotFound {
                        name: name.to_string(),
                        version: Some(version_str.to_string()),
                    })
            }
            None => {
                // Find the latest version
                let mut latest_version: Option<Version> = None;
                let mut latest_metadata: Option<ToolMetadata> = None;

                for entry in self.metadata.iter() {
                    let key = entry.key();
                    if key.0 == name
                        && (latest_version.is_none() || key.1 > *latest_version.as_ref().unwrap())
                    {
                        latest_version = Some(key.1.clone());
                        latest_metadata = Some(entry.value().clone());
                    }
                }

                latest_metadata.ok_or_else(|| RegistryError::ToolNotFound {
                    name: name.to_string(),
                    version: None,
                })
            }
        }
    }

    /// Get all registered tools.
    ///
    /// # Returns
    /// Returns a vector of tuples containing the tool name, version, and metadata.
    pub fn get_all_tools(&self) -> Vec<(String, String, ToolMetadata)> {
        self.metadata
            .iter()
            .map(|entry| {
                let key = entry.key();
                let metadata = entry.value();
                (key.0.clone(), key.1.to_string(), metadata.clone())
            })
            .collect()
    }

    /// Invoke a tool by name and version.
    ///
    /// # Arguments
    /// * `name` - The name of the tool
    /// * `version` - The version of the tool (or None for latest)
    /// * `input` - The input to the tool
    ///
    /// # Returns
    /// Returns `Ok(serde_json::Value)` on success, or an error on failure.
    ///
    /// # Errors
    /// May return `RegistryError` if:
    /// - The tool was not found
    /// - The tool invocation failed
    #[instrument(skip(self, input))]
    pub async fn invoke(
        &self,
        name: &str,
        version: Option<&str>,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, RegistryError> {
        let tool_lock = self.get_tool(name, version)?;
        let version_str = match version {
            Some(v) => v.to_string(),
            None => self.get_metadata(name, None)?.version.to_string(),
        };

        // Create an invocation record
        let mut invocation = ToolInvocation {
            tool_name: name.to_string(),
            tool_version: version_str,
            input: input.clone(),
            output: None,
            error: None,
            start_time: chrono::Utc::now(),
            end_time: None,
            duration_ms: None,
        };

        let result = {
            let tool = tool_lock.read().await;
            tool.inner().invoke(input).await
        };

        // Update the invocation record
        let now = chrono::Utc::now();
        invocation.end_time = Some(now);
        invocation.duration_ms = Some((now - invocation.start_time).num_milliseconds() as u64);

        // Store the result or error
        match &result {
            Ok(output) => {
                invocation.output = Some(output.clone());
            }
            Err(err) => {
                invocation.error = Some(err.to_string());
            }
        }

        // Add the invocation to the history
        self.record_invocation(invocation).await;

        // Return the result or propagate the error
        result.map_err(RegistryError::ToolError)
    }

    /// Get recent tool invocations.
    ///
    /// # Returns
    /// Returns a vector of tool invocations.
    pub async fn get_invocations(&self) -> Vec<ToolInvocation> {
        let invocations = self.invocations.read().await;
        invocations.clone()
    }

    /// Unregister a tool by name and version.
    ///
    /// # Arguments
    /// * `name` - The name of the tool
    /// * `version` - The version of the tool
    ///
    /// # Returns
    /// Returns `Ok(())` if the tool was unregistered successfully,
    /// or an error if the tool was not found.
    ///
    /// # Errors
    /// May return `RegistryError::ToolNotFound` if the tool was not found.
    #[instrument(skip(self))]
    pub async fn unregister(&self, name: &str, version: &str) -> Result<(), RegistryError> {
        // Parse the version
        let version_parsed = Version::parse(version)
            .map_err(|_| RegistryError::InvalidToolVersion(version.to_string()))?;

        // Check if the tool exists
        if !self
            .tools
            .contains_key(&(name.to_string(), version_parsed.clone()))
        {
            return Err(RegistryError::ToolNotFound {
                name: name.to_string(),
                version: Some(version.to_string()),
            });
        }

        // Check if other tools depend on this tool
        for entry in self.metadata.iter() {
            let other_key = entry.key();
            let other_metadata = entry.value();

            // Skip the tool itself
            if other_key.0 == name && other_key.1 == version_parsed {
                continue;
            }

            // Check if this tool is a dependency
            for dependency in &other_metadata.dependencies {
                if dependency.name == name {
                    let version_req =
                        semver::VersionReq::parse(&dependency.version_req).map_err(|e| {
                            RegistryError::InternalError(format!(
                                "Invalid version requirement: {}",
                                e
                            ))
                        })?;

                    if version_req.matches(&version_parsed) {
                        return Err(RegistryError::InternalError(format!(
                            "Cannot unregister tool {}@{} because it is a dependency of {}@{}",
                            name, version, other_key.0, other_key.1
                        )));
                    }
                }
            }
        }

        // Remove the tool and metadata
        if let Some((_, mut tool_lock)) = self
            .tools
            .remove(&(name.to_string(), version_parsed.clone()))
        {
            // Call shutdown on the tool if we can get exclusive access
            if let Some(tool_ref) = Arc::get_mut(&mut tool_lock) {
                let tool = tool_ref.get_mut();
                if let Err(err) = tool.inner_mut().shutdown().await {
                    error!("Failed to shutdown tool {}@{}: {}", name, version, err);
                }
            }
        }

        self.metadata.remove(&(name.to_string(), version_parsed));

        info!("Tool unregistered: {}@{}", name, version);
        Ok(())
    }

    /// Validate a set of tool dependencies.
    ///
    /// # Arguments
    /// * `dependencies` - The dependencies to validate
    ///
    /// # Returns
    /// Returns `Ok(())` if all dependencies are valid,
    /// or an error if any dependency is invalid.
    ///
    /// # Errors
    /// May return `RegistryError` if:
    /// - Any dependency is not found
    /// - There are circular dependencies
    async fn validate_dependencies(
        &self,
        dependencies: &[ToolDependency],
    ) -> Result<(), RegistryError> {
        // Check that all dependencies exist
        let mut unresolved_deps = Vec::new();

        for dependency in dependencies {
            let version_req = semver::VersionReq::parse(&dependency.version_req)
                .map_err(|_| RegistryError::InvalidToolVersion(dependency.version_req.clone()))?;

            let mut found = false;
            for metadata_entry in self.metadata.iter() {
                let key = metadata_entry.key();
                if key.0 == dependency.name && version_req.matches(&key.1) {
                    found = true;
                    break;
                }
            }

            if !found {
                unresolved_deps.push(dependency.clone());
            }
        }

        if !unresolved_deps.is_empty() {
            return Err(RegistryError::UnresolvedDependencies(unresolved_deps));
        }

        // Check for circular dependencies
        for dependency in dependencies {
            let mut visited = HashSet::new();
            let mut path = Vec::new();

            if let Err(cycle) =
                self.check_circular_dependencies(dependency, &mut visited, &mut path)
            {
                return Err(RegistryError::CircularDependencies(cycle));
            }
        }

        Ok(())
    }

    /// Check for circular dependencies starting from a given dependency.
    ///
    /// # Arguments
    /// * `dependency` - The dependency to check
    /// * `visited` - Set of already visited dependencies
    /// * `path` - Current dependency path
    ///
    /// # Returns
    /// Returns `Ok(())` if no circular dependencies are found,
    /// or an error with the cycle if a circular dependency is found.
    fn check_circular_dependencies(
        &self,
        dependency: &ToolDependency,
        visited: &mut HashSet<String>,
        path: &mut Vec<String>,
    ) -> Result<(), Vec<String>> {
        let name = &dependency.name;

        // If we've already visited this dependency in the current path, we have a cycle
        if path.contains(name) {
            let cycle_start = path.iter().position(|n| n == name).unwrap();
            let mut cycle = path[cycle_start..].to_vec();
            cycle.push(name.clone());
            return Err(cycle);
        }

        // If we've already visited this dependency in a different path, it's safe
        if visited.contains(name) {
            return Ok(());
        }

        // Mark the dependency as visited and add it to the current path
        visited.insert(name.clone());
        path.push(name.clone());

        // Get the metadata for this dependency
        for metadata_entry in self.metadata.iter() {
            let key = metadata_entry.key();
            let metadata = metadata_entry.value();

            if key.0 == *name {
                // Check each of its dependencies
                for dep in &metadata.dependencies {
                    self.check_circular_dependencies(dep, visited, path)?
                }
            }
        }

        // Remove the dependency from the current path
        path.pop();

        Ok(())
    }

    /// Record a tool invocation in the history.
    ///
    /// # Arguments
    /// * `invocation` - The invocation to record
    async fn record_invocation(&self, invocation: ToolInvocation) {
        let mut invocations = self.invocations.write().await;
        invocations.push(invocation);

        // Truncate the history if it's too long
        if invocations.len() > self.max_invocations {
            let excess = invocations.len() - self.max_invocations;
            invocations.drain(0..excess);
        }
    }

    #[allow(dead_code)]
    async fn get_tool_instance(
        &self,
        name: &str,
        version: Option<&str>,
    ) -> Result<Arc<RwLock<BoxedTool>>, RegistryError> {
        let tool_arc = self.get_tool(name, version)?;

        Ok(tool_arc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    

    // A simple implementation of ToolConfig for the tests
    #[derive(Debug, Default, Clone, Serialize, Deserialize)]
    struct MockToolConfig {}

    impl ToolConfig for MockToolConfig {
        fn validate(&self) -> Result<(), ToolConfigError> {
            Ok(())
        }
    }

    // Mock tool for testing
    #[derive(Debug)]
    struct MockTool {
        initialized: bool,
    }

    #[async_trait]
    impl Tool for MockTool {
        type Input = serde_json::Value;
        type Output = serde_json::Value;
        type Config = EmptyConfig;

        async fn initialize(&mut self) -> Result<(), ToolError> {
            self.initialized = true;
            Ok(())
        }

        async fn shutdown(&mut self) -> Result<(), ToolError> {
            self.initialized = false;
            Ok(())
        }

        fn capabilities(&self) -> Vec<ToolCapability> {
            vec![ToolCapability::Stateless]
        }

        async fn invoke(&self, input: Self::Input) -> Result<Self::Output, ToolError> {
            // Return the input directly for test compatibility
            Ok(input)
        }

        fn spec(&self) -> ToolSpec {
            ToolSpec {
                name: "mock_tool".into(),
                description: "A mock tool for testing".into(),
                input_schema: serde_json::json!({}),
                output_schema: serde_json::json!({}),
                examples: vec![],
            }
        }

        fn try_new(_config: Self::Config) -> Result<Self, ToolConfigError>
        where
            Self: Sized,
        {
            Ok(Self { initialized: false })
        }
    }

    impl MockTool {
        fn new() -> Self {
            Self { initialized: false }
        }

        fn into_boxed_dynamic(self) -> BoxedTool {
            DebugBoxedTool::new(self)
        }
    }

    #[derive(Debug)]
    struct MockToolWrapper {
        tool: MockTool,
    }

    impl MockToolWrapper {
        fn new(tool: MockTool) -> Self {
            Self { tool }
        }
    }

    #[async_trait]
    impl Tool for MockToolWrapper {
        type Input = serde_json::Value;
        type Output = serde_json::Value;
        type Config = EmptyConfig;

        async fn initialize(&mut self) -> Result<(), ToolError> {
            self.tool.initialize().await
        }

        async fn shutdown(&mut self) -> Result<(), ToolError> {
            self.tool.shutdown().await
        }

        fn capabilities(&self) -> Vec<ToolCapability> {
            self.tool.capabilities()
        }

        async fn invoke(&self, input: Self::Input) -> Result<Self::Output, ToolError> {
            self.tool.invoke(input).await
        }

        fn spec(&self) -> ToolSpec {
            self.tool.spec()
        }

        fn try_new(_config: Self::Config) -> Result<Self, ToolConfigError>
        where
            Self: Sized,
        {
            Err(ToolConfigError::MissingField {
                field_name: "MockToolWrapper cannot be created directly".to_string(),
            })
        }
    }

    #[tokio::test]
    async fn test_registry_basic_operations() {
        let registry = ToolRegistry::new();
        let tool = MockTool::new();

        // Register a tool directly using the adapter
        registry
            .register("test-tool", "1.0.0", MockToolWrapper::new(tool), vec![])
            .await
            .expect("Failed to register tool");

        // Get the tool
        let tool_lock = registry
            .get_tool("test-tool", Some("1.0.0"))
            .expect("Failed to get tool");

        // Get metadata
        let metadata = registry
            .get_metadata("test-tool", Some("1.0.0"))
            .expect("Failed to get metadata");

        assert_eq!(metadata.name, "test-tool");
        assert_eq!(metadata.version.to_string(), "1.0.0");
        assert_eq!(metadata.description, "A mock tool for testing");

        // Invoke the tool
        let result = registry
            .invoke("test-tool", Some("1.0.0"), serde_json::json!("hello"))
            .await
            .expect("Failed to invoke tool");

        assert_eq!(result, serde_json::json!("hello"));

        // Get invocations
        let invocations = registry.get_invocations().await;
        assert_eq!(invocations.len(), 1);
        assert_eq!(invocations[0].tool_name, "test-tool");
        assert_eq!(invocations[0].tool_version, "1.0.0");

        // Unregister the tool
        registry
            .unregister("test-tool", "1.0.0")
            .await
            .expect("Failed to unregister tool");

        // Verify the tool is gone
        assert!(registry.get_tool("test-tool", Some("1.0.0")).is_err());
    }

    #[tokio::test]
    async fn test_registry_version_resolution() {
        let registry = ToolRegistry::new();

        // Register multiple versions
        registry
            .register(
                "multi-version",
                "1.0.0",
                MockToolWrapper::new(MockTool::new()),
                vec![],
            )
            .await
            .expect("Failed to register tool");

        registry
            .register(
                "multi-version",
                "1.1.0",
                MockToolWrapper::new(MockTool::new()),
                vec![],
            )
            .await
            .expect("Failed to register tool");

        registry
            .register(
                "multi-version",
                "2.0.0",
                MockToolWrapper::new(MockTool::new()),
                vec![],
            )
            .await
            .expect("Failed to register tool");

        // Get latest version
        let metadata = registry
            .get_metadata("multi-version", None)
            .expect("Failed to get latest metadata");

        assert_eq!(metadata.version.to_string(), "2.0.0");

        // Get specific version
        let metadata = registry
            .get_metadata("multi-version", Some("1.1.0"))
            .expect("Failed to get specific metadata");

        assert_eq!(metadata.version.to_string(), "1.1.0");
    }

    #[tokio::test]
    async fn test_registry_dependency_validation() {
        let registry = ToolRegistry::new();

        // Register a tool
        registry
            .register(
                "dep-a",
                "1.0.0",
                MockToolWrapper::new(MockTool::new()),
                vec![],
            )
            .await
            .expect("Failed to register tool");

        // Register a tool that depends on the first
        let dependencies = vec![ToolDependency {
            name: "dep-a".into(),
            version_req: "^1.0.0".into(),
        }];

        registry
            .register(
                "dep-b",
                "1.0.0",
                MockToolWrapper::new(MockTool::new()),
                dependencies,
            )
            .await
            .expect("Failed to register tool with valid dependencies");

        // Try to register a tool with invalid dependencies
        let invalid_dependencies = vec![ToolDependency {
            name: "non-existent".into(),
            version_req: "^1.0.0".into(),
        }];

        let result = registry
            .register(
                "dep-c",
                "1.0.0",
                MockToolWrapper::new(MockTool::new()),
                invalid_dependencies,
            )
            .await;
        assert!(result.is_err());

        // Try to unregister a tool that others depend on
        let result = registry.unregister("dep-a", "1.0.0").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_registry_circular_dependencies() {
        // Create a registry
        let registry = ToolRegistry::new();

        // Register the first tool
        registry
            .register(
                "cycle-a",
                "1.0.0",
                MockToolWrapper::new(MockTool::new()),
                vec![],
            )
            .await
            .unwrap();

        // Register the second tool with a dependency on the first
        registry
            .register(
                "cycle-b",
                "1.0.0",
                MockToolWrapper::new(MockTool::new()),
                vec![ToolDependency {
                    name: "cycle-a".into(),
                    version_req: ">=1.0.0".into(),
                }],
            )
            .await
            .unwrap();

        // Register the third tool with a dependency on the second
        registry
            .register(
                "cycle-c",
                "1.0.0",
                MockToolWrapper::new(MockTool::new()),
                vec![ToolDependency {
                    name: "cycle-b".into(),
                    version_req: ">=1.0.0".into(),
                }],
            )
            .await
            .unwrap();

        // Check for circular dependency with direct access to the check method
        let circular_dep = ToolDependency {
            name: "cycle-c".into(),
            version_req: ">=1.0.0".into(),
        };

        // Track path and visited nodes to detect cycles
        let mut visited = HashSet::new();
        let mut path = Vec::new();

        // Add the current tool to the path
        path.push("cycle-a".to_string());
        visited.insert("cycle-a".to_string());

        // Check the dependency directly
        let result = registry.check_circular_dependencies(&circular_dep, &mut visited, &mut path);

        // This should detect a cycle
        assert!(result.is_err(), "Expected circular dependency detection");
        if let Err(cycle_path) = result {
            assert!(
                cycle_path.len() >= 3,
                "Expected at least 3 nodes in the cycle"
            );
            assert!(
                cycle_path.contains(&"cycle-a".to_string()),
                "cycle should contain cycle-a"
            );
            assert!(
                cycle_path.contains(&"cycle-b".to_string()),
                "cycle should contain cycle-b"
            );
            assert!(
                cycle_path.contains(&"cycle-c".to_string()),
                "cycle should contain cycle-c"
            );
        }
    }
}
