/// Examples demonstrating common agent integration patterns:
/// 1. Agent + Chain composition
/// 2. Agent + Tool invocation
/// 3. Agent + Memory integration
///
/// These examples illustrate how to compose agents with chains, invoke tools,
/// and integrate memory backends in a concise and clear manner.

/// Agent Integration Patterns Example
///
/// This example demonstrates common patterns for integrating agents with chains,
/// tools, and memory without relying on external implementations.

/// Example 1: Agent + Chain composition pattern
///
/// Demonstrates how to compose an Agent with a Chain to process input sequentially.
fn example_agent_chain_composition() {
    // Create a simple chain that processes data in steps
    let chain = SimpleChain::new();

    // Create an agent that uses the chain
    let mut agent = SimpleAgent::with_chain(chain);

    // Execute the agent on some input
    let result = agent.process("Process this data through steps");

    println!("Agent + Chain result: {}", result);
}

/// Example 2: Agent + Tool invocation
///
/// Demonstrates how an Agent can invoke external tools as part of its processing.
fn example_agent_tool_invocation() {
    // Create a simple agent
    let mut agent = SimpleAgent::new();

    // Add a tool to the agent
    agent.add_tool("calculator", |input| {
        // Simple calculator tool implementation
        println!("Calculator tool input: {}", input);
        if input.contains("add") {
            let parts: Vec<&str> = input.split_whitespace().collect();
            println!("Parts: {:?}", parts);
            if parts.len() >= 4 {
                // calculator add 5 7
                if let (Ok(a), Ok(b)) = (parts[2].parse::<i32>(), parts[3].parse::<i32>()) {
                    println!("Calculated sum: {} + {} = {}", a, b, a + b);
                    return format!("Sum: {}", a + b);
                }
            }
        }
        "I couldn't calculate that.".to_string()
    });

    // Print registered tools
    agent.print_tools();

    // Execute agent with input that triggers tool usage
    let result = agent.process("calculator add 5 7");

    println!("Agent + Tool result: {}", result);
}

/// Example 3: Agent + Memory integration
///
/// Demonstrates how an Agent can integrate with a memory backend to store and
/// retrieve context during execution.
fn example_agent_memory_integration() {
    // Create an agent with memory support
    let mut agent = SimpleAgent::with_memory();

    // First interaction
    let result1 = agent.process("My name is Alice");
    println!("Agent response 1: {}", result1);

    // Second interaction that uses memory
    let result2 = agent.process("What's my name?");
    println!("Agent response 2: {}", result2);
}

/// A simple agent implementation for the examples
struct SimpleAgent {
    chain: Option<SimpleChain>,
    tools: std::collections::HashMap<String, Box<dyn Fn(&str) -> String + Send + Sync>>,
    memory: Vec<String>,
}

impl SimpleAgent {
    /// Create a new agent
    fn new() -> Self {
        Self {
            chain: None,
            tools: std::collections::HashMap::new(),
            memory: Vec::new(),
        }
    }

    /// Create an agent with a chain
    fn with_chain(chain: SimpleChain) -> Self {
        Self {
            chain: Some(chain),
            tools: std::collections::HashMap::new(),
            memory: Vec::new(),
        }
    }

    /// Create an agent with memory
    fn with_memory() -> Self {
        Self {
            chain: None,
            tools: std::collections::HashMap::new(),
            memory: Vec::new(),
        }
    }

    /// Add a tool to the agent
    fn add_tool<F>(&mut self, name: &str, handler: F)
    where
        F: Fn(&str) -> String + Send + Sync + 'static,
    {
        self.tools.insert(name.to_string(), Box::new(handler));
    }

    /// Process input and return a response
    fn process(&mut self, input: &str) -> String {
        // Store input in memory
        self.memory.push(input.to_string());

        // Check if we should use a tool
        for (tool_name, tool_fn) in &self.tools {
            if input.starts_with(tool_name) {
                return tool_fn(input);
            }
        }

        // Use the chain if available
        if let Some(chain) = &self.chain {
            return chain.process(input);
        }

        // Handle memory-based queries
        if input.contains("name") && input.contains("?") {
            for memory in &self.memory {
                if memory.contains("My name is") {
                    let parts: Vec<&str> = memory.split("My name is").collect();
                    if parts.len() > 1 {
                        return format!("Your name is{}", parts[1]);
                    }
                }
            }
        }

        // Default response
        format!("Processed: {}", input)
    }

    /// Print registered tools
    fn print_tools(&self) {
        println!("Registered tools:");
        for (name, _) in &self.tools {
            println!("{}", name);
        }
    }
}

/// A simple chain implementation for the examples
struct SimpleChain {
    steps: Vec<Box<dyn Fn(&str) -> String + Send + Sync>>,
}

impl SimpleChain {
    /// Create a new chain
    fn new() -> Self {
        let mut chain = Self { steps: Vec::new() };

        // Add some default steps
        chain.add_step(|input| format!("Step 1: {}", input));
        chain.add_step(|input| format!("Step 2: {}", input));

        chain
    }

    /// Add a step to the chain
    fn add_step<F>(&mut self, step: F)
    where
        F: Fn(&str) -> String + Send + Sync + 'static,
    {
        self.steps.push(Box::new(step));
    }

    /// Process input through all steps
    fn process(&self, input: &str) -> String {
        let mut result = input.to_string();

        for step in &self.steps {
            result = step(&result);
        }

        result
    }
}

fn main() {
    println!("Agent Integration Patterns Examples\n");

    println!("1. Agent + Chain Composition:");
    example_agent_chain_composition();
    println!();

    println!("2. Agent + Tool Invocation:");
    example_agent_tool_invocation();
    println!();

    println!("3. Agent + Memory Integration:");
    example_agent_memory_integration();
    println!();
}
