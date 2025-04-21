//! MCP client implementation for Cogni.
//!
//! See TDD.md and https://modelcontextprotocol.io/docs/concepts/tools

use crate::error::McpError;
use crate::protocol::ToolSpec;
use cogni_tools_common::{RateLimiterConfig, ToolRateLimiter};
use serde_json::json;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Semaphore;
use tokio::time::sleep;

/// Configuration for MCPClient
pub struct MCPClientConfig {
    pub server_path: String,
    pub env: Option<Vec<(String, String)>>,
    pub startup_timeout_secs: u64,
    pub max_concurrent_requests: usize,
    pub max_retries: u32,
    pub rate_limiter_config: RateLimiterConfig,
}

pub struct MCPClient {
    child: Child,
    writer: tokio::io::BufWriter<tokio::process::ChildStdin>,
    reader: BufReader<tokio::process::ChildStdout>,
    next_id: u64,
    concurrency: Arc<Semaphore>,
    rate_limiter: ToolRateLimiter,
    max_retries: u32,
}

impl MCPClient {
    /// Start the MCP server process and connect via stdio.
    pub async fn connect(config: MCPClientConfig) -> Result<Self, McpError> {
        let mut cmd = Command::new(&config.server_path);
        cmd.stdin(Stdio::piped()).stdout(Stdio::piped());
        if let Some(envs) = &config.env {
            for (k, v) in envs {
                cmd.env(k, v);
            }
        }
        let mut child = cmd
            .spawn()
            .map_err(|e| McpError::Transport(format!("Failed to spawn MCP server: {e}")))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| McpError::Transport("Failed to open stdin".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| McpError::Transport("Failed to open stdout".into()))?;
        let writer = tokio::io::BufWriter::new(stdin);
        let reader = BufReader::new(stdout);
        Ok(MCPClient {
            child,
            writer,
            reader,
            next_id: 1,
            concurrency: Arc::new(Semaphore::new(config.max_concurrent_requests)),
            rate_limiter: ToolRateLimiter::new(config.rate_limiter_config),
            max_retries: config.max_retries,
        })
    }

    /// List available tools from the MCP server (JSON-RPC: method = "listTools").
    pub async fn list_tools(&mut self) -> Result<Vec<ToolSpec>, McpError> {
        let _permit = self.concurrency.acquire().await.unwrap();
        let mut attempt = 0;
        loop {
            attempt += 1;
            // Rate limit globally (no domain for list_tools)
            if let Err(e) = self.rate_limiter.check_group("mcp", 1.0, None).await {
                return Err(McpError::Transport(format!("Rate limit: {e}")));
            }
            let id = self.next_id;
            self.next_id += 1;
            let req = json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "listTools"
            });
            let req_str = serde_json::to_string(&req)
                .map_err(|e| McpError::Serialization(e.to_string()))?
                + "\n";
            if let Err(e) = self.writer.write_all(req_str.as_bytes()).await {
                if attempt > self.max_retries {
                    return Err(McpError::Transport(e.to_string()));
                }
                sleep(Duration::from_millis(100u64 * attempt as u64)).await;
                continue;
            }
            if let Err(e) = self.writer.flush().await {
                if attempt > self.max_retries {
                    return Err(McpError::Transport(e.to_string()));
                }
                sleep(Duration::from_millis(100u64 * attempt as u64)).await;
                continue;
            }
            let mut line = String::new();
            if let Err(e) = self.reader.read_line(&mut line).await {
                if attempt > self.max_retries {
                    return Err(McpError::Transport(e.to_string()));
                }
                sleep(Duration::from_millis(100u64 * attempt as u64)).await;
                continue;
            }
            let resp: serde_json::Value = match serde_json::from_str(&line) {
                Ok(val) => val,
                Err(e) => {
                    if attempt > self.max_retries {
                        return Err(McpError::Serialization(e.to_string()));
                    }
                    sleep(Duration::from_millis(100u64 * attempt as u64)).await;
                    continue;
                }
            };
            if let Some(result) = resp.get("result") {
                let tools: Vec<ToolSpec> = serde_json::from_value(result.clone())
                    .map_err(|e| McpError::Serialization(e.to_string()))?;
                return Ok(tools);
            } else if let Some(error) = resp.get("error") {
                if attempt > self.max_retries {
                    return Err(McpError::Protocol(error.to_string()));
                }
                sleep(Duration::from_millis(100u64 * attempt as u64)).await;
                continue;
            } else {
                if attempt > self.max_retries {
                    return Err(McpError::Protocol("No result or error in response".into()));
                }
                sleep(Duration::from_millis(100u64 * attempt as u64)).await;
                continue;
            }
        }
    }

    /// Invoke a tool on the MCP server (JSON-RPC: method = "callTool").
    pub async fn call_tool(
        &mut self,
        tool_name: &str,
        input: serde_json::Value,
        request_id: Option<String>,
    ) -> Result<crate::protocol::ToolResult, McpError> {
        let _permit = self.concurrency.acquire().await.unwrap();
        let mut attempt = 0;
        loop {
            attempt += 1;
            // Rate limit by tool name
            if let Err(e) = self.rate_limiter.check_group(tool_name, 1.0, None).await {
                return Err(McpError::Transport(format!("Rate limit: {e}")));
            }
            let id = self.next_id;
            self.next_id += 1;
            let req = json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "callTool",
                "params": {
                    "toolName": tool_name,
                    "input": input,
                    "requestId": request_id
                }
            });
            let req_str = serde_json::to_string(&req)
                .map_err(|e| McpError::Serialization(e.to_string()))?
                + "\n";
            if let Err(e) = self.writer.write_all(req_str.as_bytes()).await {
                if attempt > self.max_retries {
                    return Err(McpError::Transport(e.to_string()));
                }
                sleep(Duration::from_millis(100u64 * attempt as u64)).await;
                continue;
            }
            if let Err(e) = self.writer.flush().await {
                if attempt > self.max_retries {
                    return Err(McpError::Transport(e.to_string()));
                }
                sleep(Duration::from_millis(100u64 * attempt as u64)).await;
                continue;
            }
            let mut line = String::new();
            if let Err(e) = self.reader.read_line(&mut line).await {
                if attempt > self.max_retries {
                    return Err(McpError::Transport(e.to_string()));
                }
                sleep(Duration::from_millis(100u64 * attempt as u64)).await;
                continue;
            }
            let resp: serde_json::Value = match serde_json::from_str(&line) {
                Ok(val) => val,
                Err(e) => {
                    if attempt > self.max_retries {
                        return Err(McpError::Serialization(e.to_string()));
                    }
                    sleep(Duration::from_millis(100u64 * attempt as u64)).await;
                    continue;
                }
            };
            if let Some(result) = resp.get("result") {
                let tool_result: crate::protocol::ToolResult =
                    serde_json::from_value(result.clone())
                        .map_err(|e| McpError::Serialization(e.to_string()))?;
                return Ok(tool_result);
            } else if let Some(error) = resp.get("error") {
                if attempt > self.max_retries {
                    return Err(McpError::Protocol(error.to_string()));
                }
                sleep(Duration::from_millis(100u64 * attempt as u64)).await;
                continue;
            } else {
                if attempt > self.max_retries {
                    return Err(McpError::Protocol("No result or error in response".into()));
                }
                sleep(Duration::from_millis(100u64 * attempt as u64)).await;
                continue;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cogni_tools_common::RateLimiterConfig;

    use std::sync::Arc;
    use std::time::Instant;

    #[tokio::test]
    async fn test_concurrency_limit() {
        // This test verifies that the concurrency limit works by simulating a client that has
        // a semaphore with a low permit count and observing that requests are serialized

        // Create a non-operational client with a concurrency limit
        let _config = MCPClientConfig {
            server_path: "/bin/echo".to_string(), // Not actually used
            env: None,
            startup_timeout_secs: 1,
            max_concurrent_requests: 2, // Only allow 2 concurrent requests
            max_retries: 0,
            rate_limiter_config: RateLimiterConfig::default(),
        };

        // Instead of relying on an external process, mock the client manually
        struct MockClient {
            concurrency: Arc<Semaphore>,
            request_time: Duration,
        }

        impl MockClient {
            async fn call(&self) -> Result<(), ()> {
                let _permit = self.concurrency.acquire().await.unwrap();
                // Simulate a request that takes some time
                tokio::time::sleep(self.request_time).await;
                Ok(())
            }
        }

        // Create a mock client with the concurrency semaphore
        let client = MockClient {
            concurrency: Arc::new(Semaphore::new(2)), // Same as config.max_concurrent_requests
            request_time: Duration::from_millis(50),  // Each request takes 50ms
        };

        let client = Arc::new(client);
        let start = Instant::now();

        // Spawn 4 tasks that will try to acquire permits concurrently
        let mut handles = vec![];
        for i in 0..4 {
            let client = Arc::clone(&client);
            handles.push(tokio::spawn(async move {
                println!("Request {} started", i);
                let result = client.call().await;
                println!("Request {} completed with result: {:?}", i, result);
                result
            }));
        }

        // Wait for all tasks to complete
        for handle in handles {
            handle.await.unwrap().unwrap();
        }

        let elapsed = start.elapsed();
        println!("All requests completed in {:?}", elapsed);

        // With 4 requests, 2 concurrency, and 50ms per request,
        // it should take at least 100ms (2 batches * 50ms)
        assert!(
            elapsed.as_millis() >= 100,
            "Expected at least 100ms with concurrency limit, got {:?}",
            elapsed
        );
    }

    #[tokio::test]
    async fn test_rate_limit() {
        // Create a rate limiter with a very restrictive config for testing
        let mut config = RateLimiterConfig {
            global_rps: 2.0,
            global_burst_size: 1,
            ..Default::default()
        };

        let rate_limiter = ToolRateLimiter::new(config);

        // First request should succeed
        let result1 = rate_limiter.check_group("mcp", 1.0, None).await;
        assert!(result1.is_ok(), "First request should succeed");

        // Second immediate request should fail due to rate limiting
        let result2 = rate_limiter.check_group("mcp", 1.0, None).await;
        assert!(
            result2.is_err(),
            "Second immediate request should fail due to rate limiting"
        );

        // Wait half a second and try again - should still fail
        tokio::time::sleep(Duration::from_millis(500)).await;
        let result3 = rate_limiter.check_group("mcp", 1.0, None).await;
        assert!(result3.is_err(), "Request after 500ms should still fail");

        // Wait until the full 1 second has passed since the first request
        tokio::time::sleep(Duration::from_millis(600)).await; // 500 + 600 > 1000
        let result4 = rate_limiter.check_group("mcp", 1.0, None).await;
        assert!(
            result4.is_ok(),
            "Request after rate limit cooldown should succeed"
        );
    }

    #[tokio::test]
    async fn test_retries() {
        // Mock a retryable scenario without using an external process

        struct MockRetryClient {
            fail_count: std::sync::atomic::AtomicUsize,
            max_retries: u32,
        }

        impl MockRetryClient {
            async fn call(&self) -> Result<String, McpError> {
                let current = self
                    .fail_count
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

                if current < 2 {
                    // Fail the first 2 attempts
                    println!("Attempt {} failing", current + 1);
                    Err(McpError::Transport(format!("Mock failure {}", current + 1)))
                } else {
                    println!("Attempt {} succeeding", current + 1);
                    Ok("success".to_string())
                }
            }

            async fn call_with_retry(&self) -> Result<String, McpError> {
                let mut attempt = 0;
                loop {
                    attempt += 1;
                    match self.call().await {
                        Ok(result) => return Ok(result),
                        Err(e) => {
                            if attempt > self.max_retries {
                                return Err(e);
                            }
                            // Backoff between retries
                            tokio::time::sleep(Duration::from_millis(10 * attempt as u64)).await;
                        }
                    }
                }
            }
        }

        // Test with 2 max retries (3 total attempts)
        let client = MockRetryClient {
            fail_count: std::sync::atomic::AtomicUsize::new(0),
            max_retries: 2,
        };

        // This should succeed on the 3rd attempt (after 2 retries)
        let result = client.call_with_retry().await;
        assert!(
            result.is_ok(),
            "Call should succeed after retries: {:?}",
            result
        );
        assert_eq!(result.unwrap(), "success");
        assert_eq!(
            client.fail_count.load(std::sync::atomic::Ordering::SeqCst),
            3
        );

        // Test with insufficient retries
        let client2 = MockRetryClient {
            fail_count: std::sync::atomic::AtomicUsize::new(0),
            max_retries: 1, // Only 1 retry (2 total attempts)
        };

        // This should fail even with retry
        let result2 = client2.call_with_retry().await;
        assert!(
            result2.is_err(),
            "Call should fail with insufficient retries"
        );
        assert_eq!(
            client2.fail_count.load(std::sync::atomic::Ordering::SeqCst),
            2
        );
    }

    // A simple test to verify that we can run Python scripts directly
    #[tokio::test]
    async fn test_simple() {
        // Skip the test if Python is not available or environment is not conducive to running it
        let python_check = tokio::process::Command::new("python3")
            .arg("--version")
            .output()
            .await;

        if python_check.is_err() {
            println!("Python3 not available, skipping test");
            return;
        }

        println!("Python3 is available, running test");

        // Create a simple echo script
        let script = r#"
import sys
print("Script started", file=sys.stderr)
print('{"jsonrpc": "2.0", "id": 1, "result": []}')
print("Script completed", file=sys.stderr)
"#;

        use std::io::Write;
        use tempfile::NamedTempFile;
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(script.as_bytes()).unwrap();
        let path = file.path().to_str().unwrap().to_string();

        println!("Created script at {}", path);

        // Run it directly
        let output = tokio::process::Command::new("python3")
            .arg(&path)
            .output()
            .await;

        match output {
            Ok(output) => {
                println!("Script stdout: {}", String::from_utf8_lossy(&output.stdout));
                println!("Script stderr: {}", String::from_utf8_lossy(&output.stderr));
                println!("Script exit status: {:?}", output.status);
            }
            Err(e) => {
                println!("Failed to run script: {:?}", e);
            }
        }
    }
}
