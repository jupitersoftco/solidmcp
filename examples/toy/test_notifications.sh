#!/bin/bash

# Test script for MCP notification demonstration

echo "🧪 Testing MCP Notification Support"
echo "=================================="

# Initialize the connection
echo -e "\n1️⃣ Initializing connection..."
curl -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "initialize",
    "params": {
      "protocolVersion": "0.1.0",
      "capabilities": {},
      "clientInfo": {
        "name": "test-client",
        "version": "1.0.0"
      }
    }
  }' | jq .

# List available tools
echo -e "\n2️⃣ Listing available tools..."
curl -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 2,
    "method": "tools/list"
  }' | jq .

# Test the add_notification tool
echo -e "\n3️⃣ Testing add_notification tool..."
curl -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 3,
    "method": "tools/call",
    "params": {
      "name": "add_notification",
      "arguments": {
        "level": "info",
        "message": "Test notification from the client",
        "data": {
          "timestamp": "'$(date -u +"%Y-%m-%dT%H:%M:%SZ")'",
          "source": "test-script"
        }
      }
    }
  }' | jq .

# Add a note (which should trigger a notification)
echo -e "\n4️⃣ Adding a note (should trigger notification)..."
curl -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 4,
    "method": "tools/call",
    "params": {
      "name": "add_note",
      "arguments": {
        "name": "test-note",
        "content": "This is a test note that should trigger a notification!"
      }
    }
  }' | jq .

# List notes
echo -e "\n5️⃣ Listing notes..."
curl -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 5,
    "method": "tools/call",
    "params": {
      "name": "list_notes",
      "arguments": {}
    }
  }' | jq .

echo -e "\n✅ Test complete!"
echo "Note: To see actual notifications, connect with a WebSocket client."