//! MCP Protocol Core Tests
//!
//! Tests for basic MCP protocol functionality like initialization and error handling.

#[cfg(test)]
use {
    crate::protocol_impl::McpProtocolHandlerImpl, crate::protocol_testable::McpProtocolHandler,
    serde_json::json,
};

#[tokio::test]
async fn test_mcp_protocol_handler_creation() {
    let handler = McpProtocolHandlerImpl::new();
    assert!(!handler.is_initialized());
    assert_eq!(handler.protocol_version(), "2025-06-18");
}

#[tokio::test]
async fn test_mcp_initialize() {
    let mut handler = McpProtocolHandlerImpl::new();

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

    let response = handler.handle_message(init_message).await.unwrap();

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 1);
    assert!(response["result"].is_object());
    assert_eq!(response["result"]["protocolVersion"], "2025-06-18");
    assert_eq!(response["result"]["serverInfo"]["name"], "mcp-server");

    assert!(handler.is_initialized());
}

#[tokio::test]
async fn test_mcp_tools_list_without_initialization() {
    let mut handler = McpProtocolHandlerImpl::new();

    let tools_message = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    });

    let result = handler.handle_message(tools_message).await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 1);
    assert!(response["error"].is_object());
    assert_eq!(response["error"]["code"], -32002);
    assert!(response["error"]["message"]
        .as_str()
        .unwrap()
        .contains("Not initialized"));
}

#[tokio::test]
async fn test_mcp_unknown_method() {
    let mut handler = McpProtocolHandlerImpl::new();

    let unknown_message = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "unknown_method",
        "params": {}
    });

    let result = handler.handle_message(unknown_message).await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 1);
    assert!(response["error"].is_object());
    let error_message = response["error"]["message"].as_str().unwrap();
    assert!(error_message.contains("Method not found") || error_message.contains("Unknown method"));
}

#[tokio::test]
async fn test_mcp_error_response_creation() {
    let handler = McpProtocolHandlerImpl::new();
    let error_response = handler.create_error_response(json!(1), -32601, "Method not found");

    assert_eq!(error_response["jsonrpc"], "2.0");
    assert_eq!(error_response["id"], 1);
    assert!(error_response["error"].is_object());
    assert_eq!(error_response["error"]["code"], -32601);
    assert_eq!(error_response["error"]["message"], "Method not found");
}

#[tokio::test]
async fn test_mcp_protocol_version_mismatch() {
    let mut handler = McpProtocolHandlerImpl::new();

    let init_message = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-01-01", // Different version
            "capabilities": {},
            "clientInfo": {
                "name": "Cursor",
                "version": "1.0.0"
            }
        }
    });

    // Should return an error for unsupported version
    let response = handler.handle_message(init_message).await.unwrap();
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 1);
    assert!(response["error"].is_object());
    assert!(response["error"]["message"]
        .as_str()
        .unwrap()
        .contains("Unsupported protocol version"));
    assert!(!handler.is_initialized()); // Should not be initialized
}

#[tokio::test]
async fn test_jsonrpc_error_response_for_unknown_method() {
    use crate::protocol_impl::McpProtocolHandlerImpl;
    use crate::protocol_testable::McpProtocolHandler;
    use serde_json::json;

    let mut handler = McpProtocolHandlerImpl::new();
    let unknown_message = json!({
        "jsonrpc": "2.0",
        "id": 42,
        "method": "unknown_method",
        "params": {}
    });
    let result = handler.handle_message(unknown_message).await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 42);
    assert!(response["error"].is_object());
    let error_message = response["error"]["message"].as_str().unwrap();
    assert!(error_message.contains("Method not found") || error_message.contains("Unknown method"));
}
