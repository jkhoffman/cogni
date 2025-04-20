//! Builder traits for constructing components.
//!
//! This module provides builder traits that enable fluent construction
//! of various components in the Cogni framework.

#[cfg(feature = "tool")]
use crate::traits::tool::ToolConfig;
use std::fmt::Debug;
use std::marker::PhantomData;

/// A trait for building components in a fluent manner.
pub trait Builder {
    /// The type that this builder constructs
    type Output;

    /// Build the component
    fn build(self) -> Result<Self::Output, String>;
}

#[cfg(feature = "tool")]
mod tool_builder {
    use super::*;
    use crate::traits::tool::{Tool, ToolCapability};

    /// Builder for tool creation
    #[derive(Debug)]
    pub struct ToolBuilder<T, I, O, C>
    where
        T: Sized,
        I: Debug,
        O: Debug,
        C: Debug,
    {
        phantom: PhantomData<(T, I, O, C)>,
        _name: String,
        config: C,
        capabilities: Vec<ToolCapability>,
    }

    impl<T, I, O, C> ToolBuilder<T, I, O, C>
    where
        T: Tool<Input = I, Output = O, Config = C>,
        C: ToolConfig,
        I: Debug,
        O: Debug,
    {
        /// Create a new tool builder
        pub fn new(name: impl Into<String>, config: C) -> Self {
            Self {
                _name: name.into(),
                config,
                phantom: PhantomData,
                capabilities: Vec::new(),
            }
        }

        /// Set the tool's description
        pub fn description(mut self, description: impl Into<String>) -> Self {
            self._name = description.into();
            self
        }

        /// Add a capability to the tool
        pub fn capability(mut self, capability: ToolCapability) -> Self {
            self.capabilities.push(capability);
            self
        }

        /// Add multiple capabilities to the tool
        pub fn capabilities(
            mut self,
            capabilities: impl IntoIterator<Item = ToolCapability>,
        ) -> Self {
            self.capabilities.extend(capabilities);
            self
        }
    }

    impl<T, I, O, C> super::Builder for ToolBuilder<T, I, O, C>
    where
        T: Tool<Input = I, Output = O, Config = C>,
        C: ToolConfig,
        I: Debug,
        O: Debug,
    {
        type Output = T;

        fn build(self) -> Result<Self::Output, String> {
            // Validate the configuration
            self.config.validate()?;

            // Create and initialize the tool
            let mut tool =
                T::try_new(self.config).map_err(|e| format!("Failed to create tool: {}", e))?;

            // Initialize the tool
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(tool.initialize())
                .map_err(|e| format!("Failed to initialize tool: {}", e))?;

            Ok(tool)
        }
    }
}

#[cfg(feature = "tool")]
pub use tool_builder::ToolBuilder;
