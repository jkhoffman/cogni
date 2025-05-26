//! Example demonstrating context management with automatic message pruning

use cogni_client::Client;
use cogni_context::{ContextManager, SlidingWindowStrategy, TiktokenCounter};
use cogni_core::Message;
use cogni_providers::OpenAI;
use std::error::Error;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize OpenAI provider
    let api_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set");
    let provider = OpenAI::with_api_key(api_key);
    let client = Client::new(provider);

    // Create a token counter for the model
    let counter = Arc::new(TiktokenCounter::for_model("gpt-4")?);
    println!(
        "Model context window: {} tokens",
        counter.model_context_window()
    );

    // Create a context manager with sliding window strategy
    let strategy = Arc::new(SlidingWindowStrategy::new(true, 5)); // Keep system message and last 5 messages
    let context_manager = Arc::new(
        ContextManager::new(counter.clone())
            .with_reserve_output_tokens(500) // Reserve 500 tokens for the response
            .with_strategy(strategy),
    );

    // Simulate a long conversation
    let mut messages = vec![
        Message::system("You are a helpful assistant."),
        Message::user("Let's have a conversation about space exploration."),
        Message::assistant("I'd be happy to discuss space exploration with you! It's a fascinating topic that encompasses everything from the history of spaceflight to current missions and future plans for exploring our solar system and beyond. What aspect of space exploration interests you most?"),
        Message::user("Tell me about the Apollo missions."),
        Message::assistant("The Apollo program was NASA's human spaceflight program that successfully landed astronauts on the Moon between 1969 and 1972. Here are the key highlights:\n\n1. **Apollo 11** (July 1969): Neil Armstrong and Buzz Aldrin became the first humans to walk on the Moon\n2. **Apollo 13** (April 1970): The famous \"successful failure\" - the crew survived an explosion and returned safely\n3. **Apollo 17** (December 1972): The last crewed Moon landing, with Harrison Schmitt as the only scientist-astronaut to visit\n\nThe program achieved President Kennedy's goal of landing humans on the Moon and returning them safely before the end of the 1960s."),
        Message::user("What about modern space missions?"),
        Message::assistant("Modern space exploration is more diverse and international than ever:\n\n**Mars Exploration:**\n- NASA's Perseverance rover and Ingenuity helicopter\n- China's Zhurong rover\n- UAE's Hope orbiter\n\n**Commercial Space:**\n- SpaceX's reusable rockets and Starship development\n- Blue Origin's New Shepherd and New Glenn\n- Virgin Galactic's space tourism\n\n**International Missions:**\n- James Webb Space Telescope (NASA/ESA/CSA)\n- International Space Station ongoing operations\n- Artemis program to return to the Moon\n\n**Future Plans:**\n- Mars sample return mission\n- Europa Clipper to study Jupiter's moon\n- Gateway lunar space station"),
        Message::user("Can you explain how rockets work?"),
        Message::assistant("Rockets work on Newton's third law of motion: for every action, there's an equal and opposite reaction.\n\n**Basic Principle:**\nRockets expel mass (propellant) at high velocity in one direction, which pushes the rocket in the opposite direction.\n\n**Key Components:**\n1. **Propellant**: Fuel + Oxidizer\n2. **Combustion Chamber**: Where fuel burns\n3. **Nozzle**: Shapes and accelerates exhaust\n\n**Types:**\n- **Solid rockets**: Simple, reliable, can't be throttled\n- **Liquid rockets**: Complex, controllable, restartable\n- **Hybrid rockets**: Combine solid fuel with liquid oxidizer\n\n**Staging**: Multi-stage rockets drop empty sections to improve efficiency\n\n**Key Equation**: The Tsiolkovsky rocket equation relates velocity change to exhaust velocity and mass ratio."),
        Message::user("What's the future of space exploration?"),
    ];

    // Check token count before pruning
    let total_tokens = counter.count_messages(&messages);
    println!(
        "\nOriginal conversation: {} messages, {} tokens",
        messages.len(),
        total_tokens
    );

    // Send request with context management
    let response = client
        .request()
        .with_context_manager(context_manager.clone())
        .with_message(messages[0].clone()) // System message
        .with_message(messages[1].clone())
        .with_message(messages[2].clone())
        .with_message(messages[3].clone())
        .with_message(messages[4].clone())
        .with_message(messages[5].clone())
        .with_message(messages[6].clone())
        .with_message(messages[7].clone())
        .with_message(messages[8].clone())
        .with_message(messages[9].clone())
        .send()
        .await?;

    println!("\nResponse: {}", response.content);

    // Demonstrate manual context fitting
    let fitted_messages = context_manager.fit_messages(messages.clone()).await?;
    println!(
        "\nAfter pruning: {} messages, {} tokens",
        fitted_messages.len(),
        counter.count_messages(&fitted_messages)
    );

    println!("\nMessages retained:");
    for (i, msg) in fitted_messages.iter().enumerate() {
        let preview = msg
            .content
            .as_ref()
            .and_then(|c| c.as_text())
            .map(|text| {
                if text.len() > 50 {
                    format!("{}...", &text[..50])
                } else {
                    text.to_string()
                }
            })
            .unwrap_or_default();
        println!("  {}: {:?} - {}", i, msg.role, preview);
    }

    Ok(())
}
