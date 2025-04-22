use anyhow::{Context, Result};
use log::{error, info};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

fn main() -> Result<()> {
    // Initialize logger
    env_logger::init();

    info!("Starting MCP mock server client example");

    // Start the mock server process
    let mock_server_path = "mock_server.py";
    let mut server = Command::new("python3")
        .arg(mock_server_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .context("Failed to start mock server")?;

    let server_stdin = server.stdin.take().expect("Failed to open server stdin");
    let server_stdout = server.stdout.take().expect("Failed to open server stdout");
    let mut reader = BufReader::new(server_stdout);
    let mut writer = server_stdin;

    // Example 1: List available tools
    info!("Listing available tools...");
    let list_tools_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "list_tools"
    });

    send_request(&mut writer, &list_tools_request)?;
    let response = read_response(&mut reader)?;

    if let Some(error) = response.get("error") {
        error!("Error listing tools: {}", error);
    } else if let Some(result) = response.get("result") {
        info!("Available tools: {}", serde_json::to_string_pretty(result)?);
    }

    // Example 2: Call the example tool
    info!("Calling the example tool...");
    let call_example_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "call_tool",
        "params": {
            "tool_name": "example",
            "params": {
                "message": "Hello from Rust client!"
            }
        }
    });

    send_request(&mut writer, &call_example_request)?;
    let response = read_response(&mut reader)?;

    if let Some(error) = response.get("error") {
        error!("Error calling example tool: {}", error);
    } else if let Some(result) = response.get("result") {
        info!(
            "Example tool response: {}",
            serde_json::to_string_pretty(result)?
        );
    }

    // Example 3: Call the weather tool
    info!("Calling the weather tool...");
    let call_weather_request = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "call_tool",
        "params": {
            "tool_name": "weather",
            "params": {
                "location": "San Francisco"
            }
        }
    });

    send_request(&mut writer, &call_weather_request)?;
    let response = read_response(&mut reader)?;

    if let Some(error) = response.get("error") {
        error!("Error calling weather tool: {}", error);
    } else if let Some(result) = response.get("result") {
        info!(
            "Weather tool response: {}",
            serde_json::to_string_pretty(result)?
        );
    }

    // Example 4: Call the calculator tool
    info!("Calling the calculator tool...");
    let call_calculator_request = json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "call_tool",
        "params": {
            "tool_name": "calculator",
            "params": {
                "expression": "2 * (3 + 4) - 5"
            }
        }
    });

    send_request(&mut writer, &call_calculator_request)?;
    let response = read_response(&mut reader)?;

    if let Some(error) = response.get("error") {
        error!("Error calling calculator tool: {}", error);
    } else if let Some(result) = response.get("result") {
        info!(
            "Calculator tool response: {}",
            serde_json::to_string_pretty(result)?
        );
    }

    // Example 5: Call a non-existent tool (to demonstrate error handling)
    info!("Calling a non-existent tool (should return an error)...");
    let call_nonexistent_request = json!({
        "jsonrpc": "2.0",
        "id": 5,
        "method": "call_tool",
        "params": {
            "tool_name": "nonexistent_tool",
            "params": {}
        }
    });

    send_request(&mut writer, &call_nonexistent_request)?;
    let response = read_response(&mut reader)?;

    if let Some(error) = response.get("error") {
        error!("Error response as expected: {}", error);
    } else {
        error!(
            "Expected an error but got a successful response: {:?}",
            response
        );
    }

    // Example 6: Send an invalid method
    info!("Sending an invalid method (should return an error)...");
    let invalid_method_request = json!({
        "jsonrpc": "2.0",
        "id": 6,
        "method": "invalid_method"
    });

    send_request(&mut writer, &invalid_method_request)?;
    let response = read_response(&mut reader)?;

    if let Some(error) = response.get("error") {
        error!("Error response as expected: {}", error);
    } else {
        error!(
            "Expected an error but got a successful response: {:?}",
            response
        );
    }

    // Wait a moment before terminating to make sure logs are flushed
    thread::sleep(Duration::from_millis(500));

    // Terminate the server process gracefully
    drop(writer); // Close stdin to signal the server to shut down
    let status = server.wait().context("Failed to wait for server to exit")?;
    info!("Server exited with status: {}", status);

    Ok(())
}

fn send_request(writer: &mut impl Write, request: &Value) -> Result<()> {
    let request_str = serde_json::to_string(request)?;
    info!("Sending request: {}", request_str);
    writeln!(writer, "{}", request_str)?;
    writer.flush()?;
    Ok(())
}

fn read_response(reader: &mut impl BufRead) -> Result<Value> {
    let mut response_str = String::new();
    reader.read_line(&mut response_str)?;

    if response_str.trim().is_empty() {
        return Err(anyhow::anyhow!("Received empty response from server"));
    }

    info!("Received response: {}", response_str.trim());
    let response: Value =
        serde_json::from_str(&response_str).context("Failed to parse JSON response")?;
    Ok(response)
}
