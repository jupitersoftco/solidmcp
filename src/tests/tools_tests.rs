//! MCP Tools Tests
//!
//! Tests for MCP tools functionality like tools/list and tools/call.

#[cfg(test)]
use {crate::protocol_impl::McpProtocolHandlerImpl, serde_json::json};

#[tokio::test]
async fn test_mcp_tools_list() {
    let mut handler = McpProtocolHandlerImpl::new();

    // Initialize first
    let init_message = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {
                "name": "Cursor",
                "version": "1.0.0"
            }
        }
    });
    handler.handle_message(init_message).await.unwrap();

    // Request tools list
    let tools_message = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    });

    let response = handler.handle_message(tools_message).await.unwrap();

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 2);
    assert!(response["result"].is_object());
    assert!(response["result"]["tools"].is_array());

    let tools = response["result"]["tools"].as_array().unwrap();
    assert!(tools.len() >= 2); // Should have echo and read_file

    let tool_names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();

    assert!(tool_names.contains(&"echo"));
    assert!(tool_names.contains(&"read_file"));
}

#[tokio::test]
async fn test_mcp_tool_call() {
    let mut handler = McpProtocolHandlerImpl::new();

    // Initialize first
    let init_message = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {
                "name": "Cursor",
                "version": "1.0.0"
            }
        }
    });
    handler.handle_message(init_message).await.unwrap();

    // Call echo tool
    let echo_message = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "echo",
            "arguments": {
                "message": "Hello from test!"
            }
        }
    });

    let response = handler.handle_message(echo_message).await.unwrap();

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 3);
    assert!(response["result"].is_object());
    assert!(response["result"]["content"].is_array());

    // With the new format, structured data is directly available in the "data" field
    let data = &response["result"]["data"];
    assert_eq!(data["echo"], "Hello from test!");
    
    // The content should also have a human-readable summary
    let content = response["result"]["content"][0]["text"].as_str().unwrap();
    assert!(content.contains("Echo: Hello from test!"));
}

#[tokio::test]
async fn test_mcp_tool_call_without_initialization() {
    let mut handler = McpProtocolHandlerImpl::new();

    let tool_message = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "echo",
            "arguments": {
                "message": "test"
            }
        }
    });

    let result = handler.handle_message(tool_message).await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 1);
    assert!(response["error"].is_object());
    assert!(response["error"]["message"]
        .as_str()
        .unwrap()
        .contains("Not initialized"));
}

#[tokio::test]
async fn test_mcp_unknown_tool() {
    let mut handler = McpProtocolHandlerImpl::new();

    // Initialize first
    let init_message = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {
                "name": "Cursor",
                "version": "1.0.0"
            }
        }
    });
    handler.handle_message(init_message).await.unwrap();

    // Call unknown tool
    let unknown_tool_message = json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "tools/call",
        "params": {
            "name": "unknown_tool",
            "arguments": {}
        }
    });

    let result = handler.handle_message(unknown_tool_message).await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 4);
    assert!(response["error"].is_object());
    let error_message = response["error"]["message"].as_str().unwrap();
    // Unknown tool errors are mapped to "Method not found" per JSON-RPC standards
    assert!(error_message.contains("Method not found"));
}
