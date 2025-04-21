# Technical Design Document: Model Context Protocol (MCP) Integration

## Overview

This document describes the design and implementation of the MCP (Model Context Protocol) integration for the Cogni framework. MCP is a protocol for connecting Large Language Models (LLMs) to external tools and resources, allowing for a standardized interface between LLMs and various tools.

## Goals

- Implement a Rust client for MCP compliant with the [MCP specification](https://modelcontextprotocol.io/specification/2025-03-26)
- Enable bidirectional communication with MCP servers over stdio
- Integrate existing Cogni tools with MCP
- Provide concurrency control, rate limiting, and retry mechanisms
- Ensure proper error handling and resource management

## Architecture

The MCP integration consists of the following components:

1. **Protocol Layer** - JSON-RPC types and serialization/deserialization
2. **Transport Layer** - Communication with MCP servers via stdio
3. **Client Layer** - High-level interface for connecting to and interacting with MCP servers
4. **Routing Layer** - Mapping between MCP tool calls and Cogni tools
5. **Error Handling** - MCP-specific error types and mapping to Cogni errors

### Component Diagram

```
┌───────────────┐     ┌──────────────┐
│    Cogni      │     │ MCP Server   │
│  Application  │     │              │
└───────┬───────┘     └──────┬───────┘
        │                    │
┌───────▼───────┐     ┌──────▼───────┐
│  MCP Client    │◄────►   stdio      │
└───────┬───────┘     └──────────────┘
        │
┌───────▼───────┐
│  MCP Router   │
└───────┬───────┘
        │
┌───────▼───────┐
│  Cogni Tools  │
└───────────────┘
```

## Components

### Protocol Layer

Implements the MCP JSON-RPC protocol with the following types:
- `ToolSpec` - Schema for tool capabilities
- `ToolCall` - Request to execute a tool
- `ToolResult` - Result of a tool execution
- `ErrorResponse` - Standard error format

### Transport Layer

Handles the low-level communication with MCP servers:
- Spawns and manages subprocess for MCP server
- Reads/writes JSON-RPC messages via stdin/stdout
- Handles process lifecycle

### Client Layer

Provides a high-level interface for MCP:
- `connect()` - Starts server process and establishes connection
- `list_tools()` - Retrieves available tools from server
- `call_tool()` - Invokes a tool with arguments
- Implements concurrency control with semaphores
- Handles rate limiting for tool calls
- Implements retry logic for transient failures

### Routing Layer

Maps between MCP tool calls and Cogni tools:
- Registers Cogni tools for MCP exposure
- Translates between MCP and Cogni tool formats
- Handles serialization of inputs/outputs

### Error Handling

Defines MCP-specific error types and maps them to Cogni errors:
- Transport errors
- Protocol errors
- Serialization errors
- Tool execution errors

## Implementation Details

### Concurrency

The client uses Tokio semaphores to limit the number of concurrent requests to the MCP server. This prevents overwhelming the server and ensures orderly processing of requests.

```rust
let _permit = self.concurrency.acquire().await.unwrap();
// Process request
```

### Rate Limiting

Rate limiting is implemented using the existing `ToolRateLimiter` from the common tools package. This provides:
- Global rate limits across all tools
- Per-tool rate limits
- Configurable burst allowances

### Retries

The client implements a retry mechanism for transient failures:
- Exponential backoff between retry attempts
- Configurable maximum number of retries
- Different retry strategies based on error type

## Testing Strategy

Testing for the MCP integration includes:
- Unit tests for protocol serialization/deserialization
- Mock server tests for client functionality
- Integration tests with sample MCP servers
- Performance tests for concurrency and rate limiting
- Error handling tests for retry mechanisms

## Future Work

- Add support for additional MCP features (prompts, sampling)
- Implement additional transports (WebSocket, gRPC)
- Add observability and telemetry
- Implement streaming responses

## References

- [MCP Specification](https://modelcontextprotocol.io/specification/2025-03-26)
- [MCP Python Reference Implementation](https://github.com/modelcontextprotocol/python-sdk)
- [MCP TypeScript Reference Implementation](https://github.com/modelcontextprotocol/typescript-sdk) 