# Example Test Results

## Summary

Tested all examples in the cogni crate. Found several issues requiring attention:

### Critical Issues
1. **Schema validation errors** in structured output examples
2. **Streaming error loop** causing timeout
3. **Wrong model names** for Anthropic provider

### Non-Critical Issues
1. Missing example files referenced in directory structure
2. Ollama provider errors (expected if not running locally)

## Detailed Results

### ✅ Working Examples

1. **basic_openai.rs** - SUCCESS
   - Simple OpenAI request works correctly
   - Response received without issues

2. **provider_failover.rs** - SUCCESS
   - Failover mechanism works as designed
   - Falls back from Ollama to OpenAI correctly

3. **streaming_openai.rs** - SUCCESS
   - Basic streaming functionality works
   - Generates haiku correctly

4. **structured_output_with_fallback.rs** - SUCCESS
   - Demonstrates proper fallback handling
   - Falls back from structured output to JSON mode when model doesn't support it

5. **tools_api_demo.rs** - SUCCESS
   - Tool calling works correctly
   - Tool registry functions properly

6. **client/multi_provider_client_example.rs** - SUCCESS
   - All client functionality works
   - Multiple providers can be used through client API

7. **client/request_builder_example.rs** - SUCCESS
   - Request builder API works correctly
   - All examples complete successfully

8. **client/simple_chat_example.rs** - SUCCESS
   - Simple chat functionality works
   - Conversation handling is correct

### ❌ Failed Examples

1. **agentic_combined_example.rs** - ERROR
   ```
   Error: Network { message: "HTTP 400 Bad Request: {
     "error": {
       "message": "Invalid schema for response_format 'response': In context=(), 'additionalProperties' is required to be supplied and to be false.",
       "type": "invalid_request_error",
       "param": "response_format",
       "code": null
     }
   }"
   ```
   **Issue**: Schema missing required `additionalProperties: false`

2. **structured_output_derive.rs** - ERROR
   ```
   Error: Network { message: "HTTP 400 Bad Request: {
     "error": {
       "message": "Invalid schema for response_format 'response': In context=(), 'required' is required to be supplied and to be an array including every key in properties. Missing 'humidity'.",
       "type": "invalid_request_error",
       "param": "response_format",
       "code": null
     }
   }"
   ```
   **Issue**: Schema validation - all properties must be in required array

3. **structured_output_example.rs** - ERROR
   ```
   Error: Network { message: "HTTP 400 Bad Request: {
     "error": {
       "message": "Invalid schema for response_format 'response': In context=(), 'required' is required to be supplied and to be an array including every key in properties. Missing 'age'.",
       "type": "invalid_request_error",
       "param": "response_format",
       "code": null
     }
   }"
   ```
   **Issue**: Same schema validation issue

4. **client/streaming_example.rs** - ERROR/TIMEOUT
   ```
   Error: Network error: Stream ended
   ```
   **Issue**: Enters infinite error loop, times out after 2 minutes

### ⚠️ Configuration Issues

1. **multi_provider.rs** - PARTIAL FAILURE
   - OpenAI: Works correctly
   - Anthropic: Fails with "model: gpt-4 not found"
   - Ollama: Expected failure (not running locally)
   **Issue**: Using OpenAI model name for Anthropic provider

2. **multi_provider_tools.rs** - PARTIAL FAILURE
   - Same issues as multi_provider.rs
   - Tool calling works for OpenAI

### 🔍 Missing Examples

The following examples were listed in directory structure but not found:
- context_management_example.rs
- custom_tools_example.rs
- middleware_example.rs
- middleware_simple.rs
- stateful_conversation.rs
- test_tools_locally.rs
- tool_execution_example.rs
- client/advanced_patterns_example.rs
- client/combined_features_example.rs

## Recommendations

1. **Fix Schema Validation**
   - Ensure all schemas include `additionalProperties: false`
   - Include all properties in the `required` array or make them optional

2. **Fix Anthropic Provider**
   - Use correct model names for Anthropic (e.g., "claude-3-opus-20240229" instead of "gpt-4")

3. **Debug Streaming Error**
   - Investigate why client/streaming_example.rs enters an error loop
   - Add proper error handling to prevent infinite loops

4. **Update Examples List**
   - Remove references to non-existent examples
   - Or implement the missing examples if they're intended

5. **Add Error Context**
   - Consider adding more descriptive error messages for schema validation errors
   - Help users understand what's wrong with their schemas
