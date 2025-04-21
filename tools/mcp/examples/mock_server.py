#!/usr/bin/env python3
"""
Mock MCP server for testing the Rust MCP client.
This implements a simple JSON-RPC server that responds to MCP protocol methods.
"""

import json
import logging
import random
import sys
import time
from typing import Dict, List, Any

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger("mock_server")

# Available tools in our mock server
AVAILABLE_TOOLS = {
    "example": {
        "description": "A simple example tool",
        "parameters": {
            "message": {
                "type": "string",
                "description": "A message to echo back"
            }
        }
    },
    "weather": {
        "description": "Get weather information for a location",
        "parameters": {
            "location": {
                "type": "string",
                "description": "The location to get weather for"
            }
        }
    },
    "calculator": {
        "description": "Perform basic calculations",
        "parameters": {
            "expression": {
                "type": "string",
                "description": "The mathematical expression to evaluate"
            }
        }
    }
}

def handle_list_tools(params):
    """Handle a request to list available tools"""
    logger.info("Listing available tools")
    return AVAILABLE_TOOLS

def handle_example_tool(params):
    """Handle a request to the example tool"""
    message = params.get("message", "No message provided")
    logger.info(f"Example tool called with message: {message}")
    return {
        "status": "success",
        "message": f"Echo: {message}",
        "timestamp": time.time()
    }

def handle_weather_tool(params):
    """Handle a request to the weather tool"""
    location = params.get("location", "Unknown")
    logger.info(f"Weather tool called for location: {location}")
    
    # Simulate getting weather data
    weather_conditions = ["Sunny", "Cloudy", "Rainy", "Snowy", "Windy"]
    temperature = random.randint(0, 35)
    condition = random.choice(weather_conditions)
    
    return {
        "location": location,
        "temperature": temperature,
        "condition": condition,
        "humidity": random.randint(30, 90),
        "timestamp": time.time()
    }

def handle_calculator_tool(params):
    """Handle a request to the calculator tool"""
    expression = params.get("expression", "")
    logger.info(f"Calculator tool called with expression: {expression}")
    
    try:
        # Note: eval is used for demonstration only, would be unsafe in production
        result = eval(expression)
        return {
            "expression": expression,
            "result": result
        }
    except Exception as e:
        return {
            "error": str(e)
        }

def handle_call_tool(params):
    """Handle a request to call a specific tool"""
    tool_name = params.get("tool_name")
    tool_params = params.get("params", {})
    
    if tool_name == "example":
        return handle_example_tool(tool_params)
    elif tool_name == "weather":
        return handle_weather_tool(tool_params)
    elif tool_name == "calculator":
        return handle_calculator_tool(tool_params)
    else:
        raise ValueError(f"Unknown tool: {tool_name}")

def handle_request(request):
    """Handle a JSON-RPC request"""
    method = request.get("method")
    params = request.get("params", {})
    request_id = request.get("id")
    
    try:
        if method == "list_tools":
            result = handle_list_tools(params)
            return {"jsonrpc": "2.0", "result": result, "id": request_id}
        elif method == "call_tool":
            result = handle_call_tool(params)
            return {"jsonrpc": "2.0", "result": result, "id": request_id}
        else:
            return {
                "jsonrpc": "2.0", 
                "error": {"code": -32601, "message": f"Method not found: {method}"}, 
                "id": request_id
            }
    except Exception as e:
        logger.error(f"Error handling request: {e}")
        return {
            "jsonrpc": "2.0",
            "error": {"code": -32000, "message": str(e)},
            "id": request_id
        }

def main():
    """Main function to run the mock server"""
    logger.info("Starting MCP mock server")
    
    try:
        # Process requests from stdin and respond to stdout
        for line in sys.stdin:
            if not line.strip():
                continue
                
            try:
                request = json.loads(line)
                logger.info(f"Received request: {request}")
                
                response = handle_request(request)
                logger.info(f"Sending response: {response}")
                
                # Send the response
                print(json.dumps(response))
                sys.stdout.flush()
                
            except json.JSONDecodeError:
                logger.error(f"Invalid JSON: {line}")
                error_response = {
                    "jsonrpc": "2.0",
                    "error": {"code": -32700, "message": "Parse error"},
                    "id": None
                }
                print(json.dumps(error_response))
                sys.stdout.flush()
                
    except KeyboardInterrupt:
        logger.info("Server shutting down")
    
    logger.info("Server terminated")

if __name__ == "__main__":
    main() 