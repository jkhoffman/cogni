#!/bin/bash
set -e

# Get the script directory and go to project root
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd "$SCRIPT_DIR/.."

# Check for Python and Rust
if ! command -v python3 &> /dev/null; then
    echo "Python 3 is required but not found"
    exit 1
fi

if ! command -v cargo &> /dev/null; then
    echo "Cargo is required but not found"
    exit 1
fi

# Build the Rust client example
echo "Building Rust client..."
cargo build --example simple_client

# Run the mock server in the background
echo "Starting mock server..."
cd examples
python3 mock_server.py > /tmp/mcp_server.log 2>&1 &
SERVER_PID=$!

# Function to cleanup when the script exits
function cleanup {
    echo "Shutting down mock server (PID: $SERVER_PID)..."
    kill $SERVER_PID 2>/dev/null || true
}

# Set the trap for cleanup
trap cleanup EXIT

# Give the server a moment to start
sleep 1

# Run the client
echo "Running client..."
echo "Client will connect to the mock server, list tools, and make some example calls"
echo "--------------------------------------------------------------------"
RUST_LOG=info cargo run --example simple_client

echo "--------------------------------------------------------------------"
echo "Demo complete!"
echo "Server logs available at /tmp/mcp_server.log" 