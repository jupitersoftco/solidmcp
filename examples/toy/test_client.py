#!/usr/bin/env python3
"""Test client for the toy MCP server"""

import json
import requests
import sys

BASE_URL = "http://localhost:3000/mcp"

def send_request(method, params=None, id=1):
    """Send a JSON-RPC request to the MCP server"""
    payload = {
        "jsonrpc": "2.0",
        "method": method,
        "id": id
    }
    if params:
        payload["params"] = params
    
    headers = {
        "Content-Type": "application/json",
        "Accept": "application/json"
    }
    
    print(f"\n📤 Sending: {method}")
    print(f"   Params: {json.dumps(params, indent=2) if params else 'None'}")
    
    response = requests.post(BASE_URL, json=payload, headers=headers)
    result = response.json()
    
    print(f"📥 Response: {json.dumps(result, indent=2)}")
    return result

def main():
    print("🚀 Testing Toy MCP Server")
    print(f"   Server: {BASE_URL}")
    
    # 1. Initialize the session
    print("\n1️⃣ Initializing session...")
    init_result = send_request("initialize", {
        "protocolVersion": "2024-11-05",
        "capabilities": {},
        "clientInfo": {
            "name": "test-client",
            "version": "1.0.0"
        }
    })
    
    # 2. List available tools
    print("\n2️⃣ Listing available tools...")
    tools_result = send_request("tools/list", id=2)
    
    # 3. List notes (should be empty initially)
    print("\n3️⃣ Listing notes...")
    list_result = send_request("tools/call", {
        "name": "list_notes",
        "arguments": {}
    }, id=3)
    
    # 4. Add a note
    print("\n4️⃣ Adding a note...")
    add_result = send_request("tools/call", {
        "name": "add_note",
        "arguments": {
            "name": "test-note",
            "content": "# Test Note\n\nThis is a test note created by the MCP test client."
        }
    }, id=4)
    
    # 5. List notes again
    print("\n5️⃣ Listing notes again...")
    list_result2 = send_request("tools/call", {
        "name": "list_notes",
        "arguments": {}
    }, id=5)
    
    # 6. Send a notification
    print("\n6️⃣ Sending a notification...")
    notify_result = send_request("tools/call", {
        "name": "add_notification",
        "arguments": {
            "level": "info",
            "message": "Test notification from client",
            "data": {"test": True, "timestamp": "2025-07-19"}
        }
    }, id=6)
    
    print("\n✅ Test complete!")

if __name__ == "__main__":
    main()