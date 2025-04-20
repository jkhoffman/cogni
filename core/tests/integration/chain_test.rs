//! Integration tests for chain execution.

use super::*;
use cogni_core::{
    chain::{Chain, ChainConfig},
    traits::{llm::GenerateOptions, memory::Role},
};
use time::OffsetDateTime;

#[tokio::test]
async fn test_basic_chain_execution() -> Result<(), Error> {
    // Set up test data
    let llm_responses = vec![
        "Hello, I'm an AI assistant".to_string(),
        "I can help you with that".to_string(),
    ];
    let tool_responses = vec![
        "Tool processed: query1".to_string(),
        "Tool processed: query2".to_string(),
    ];
    let memory_entries = vec![MemoryEntry {
        role: Role::User,
        content: "Initial message".to_string(),
        timestamp: OffsetDateTime::now_utc(),
    }];

    // Create test harness
    let harness = TestBuilder::new()
        .with_llm_responses(llm_responses)
        .with_tool_responses(tool_responses)
        .with_memory_entries(memory_entries)
        .build();

    // Run the test scenario
    harness
        .run(|h| async move {
            // Create a chain
            let mut chain = Chain::new();

            // Add an LLM step
            chain = chain
                .add_llm(
                    h.llm(),
                    "What can you help me with?",
                    Some(std::time::Duration::from_secs(5)),
                )
                .await;

            // Add a tool step
            chain = chain
                .add_tool(h.tool(), Some(std::time::Duration::from_secs(5)))
                .await;

            // Execute the chain
            let result = chain.execute("Test input").await?;

            // Verify the result
            assert!(result.contains("Tool processed"));

            // Verify memory was updated
            let session = SessionId::new("test");
            let entries = h.memory().load(&session, 10).await?;
            assert!(!entries.is_empty());

            Ok(())
        })
        .await
}

#[tokio::test]
async fn test_parallel_chain_execution() -> Result<(), Error> {
    // Set up test data for parallel chains
    let llm_responses = vec![
        "Response 1".to_string(),
        "Response 2".to_string(),
        "Final response".to_string(),
    ];
    let tool_responses = vec![
        "Tool 1: processed".to_string(),
        "Tool 2: processed".to_string(),
    ];

    // Create test harness
    let harness = TestBuilder::new()
        .with_llm_responses(llm_responses)
        .with_tool_responses(tool_responses)
        .build();

    // Run the test scenario
    harness
        .run(|h| async move {
            // Create parallel chains
            let chain1 = Chain::new()
                .add_llm(
                    h.llm(),
                    "Chain 1 prompt",
                    Some(std::time::Duration::from_secs(5)),
                )
                .await;

            let chain2 = Chain::new()
                .add_tool(h.tool(), Some(std::time::Duration::from_secs(5)))
                .await;

            // Create main chain with parallel execution
            let main_chain = Chain::new()
                .add_parallel(vec![chain1, chain2])
                .await
                .add_llm(
                    h.llm(),
                    "Combine results",
                    Some(std::time::Duration::from_secs(5)),
                )
                .await;

            // Execute the chain
            let result = main_chain.execute("Test input").await?;

            // Verify the result contains the final response
            assert!(result.contains("Final response"));

            Ok(())
        })
        .await
}

#[tokio::test]
async fn test_chain_error_handling() -> Result<(), Error> {
    // Create test harness with no responses (will cause errors)
    let harness = TestBuilder::new().build();

    // Run the test scenario
    harness
        .run(|h| async move {
            let chain = Chain::new()
                .add_llm(
                    h.llm(),
                    "This should fail",
                    Some(std::time::Duration::from_secs(1)),
                )
                .await;

            // Execute the chain and expect an error
            let result = chain.execute("Test input").await;
            assert!(result.is_err());

            if let Err(err) = result {
                assert!(matches!(err, Error::Llm(_)));
            }

            Ok(())
        })
        .await
}

#[tokio::test]
async fn test_chain_timeout() -> Result<(), Error> {
    // Create test harness with a very short timeout
    let config = TestConfig {
        timeout_secs: 1,
        ..Default::default()
    };

    let harness = TestBuilder::new().with_config(config).build();

    // Run the test scenario
    let result = harness
        .run(|h| async move {
            let chain = Chain::new()
                .add_llm(
                    h.llm(),
                    "This should timeout",
                    Some(std::time::Duration::from_secs(2)),
                )
                .await;

            // Sleep longer than the timeout
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;

            chain.execute("Test input").await
        })
        .await;

    assert!(matches!(result, Err(Error::Timeout)));
    Ok(())
}
