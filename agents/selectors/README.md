# Cogni Agent Tool Selectors

Tool selector implementations for the Cogni framework.

This crate provides various implementations of the `ToolSelector` trait, which is used by agents to select appropriate tools for a given input.

## Included Selectors

- **NameBasedSelector**: Selects tools based on exact name matches from a predefined list.
- **PatternBasedSelector**: Selects tools based on regex pattern matching against tool names.
- **CapabilityBasedSelector**: Selects tools based on their declared capabilities.

## Integration with ToolRegistry

The `ToolSelectorRegistry` class provides integration between selectors and the tool registry system, enabling:

- Validation of selected tools against the registry
- Discovery of tools based on capabilities
- Creation of capability-based selectors using registry data

## Example Usage

```rust
use cogni_agents_selectors::{NameBasedSelector, ToolSelectorRegistry};
use cogni_tools_registry::ToolRegistry;
use std::sync::Arc;

// Create a simple name-based selector
let mut selector = NameBasedSelector::new(vec!["search", "math", "code-executor"]);

// Add another tool
selector.add_tool_name("weather");

// Create a registry integration
let registry = Arc::new(ToolRegistry::new());
let selector_registry = ToolSelectorRegistry::new(registry);

// Validate tools against the registry
async {
    let validated_tools = selector_registry
        .validate_selector(&selector, "user query", &serde_json::Value::Null)
        .await
        .unwrap();
};
```

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option. 