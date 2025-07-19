//! MCP Protocol Core Tests
//!
//! Tests for basic MCP protocol functionality like initialization and error handling.

#[cfg(test)]
use {
    crate::protocol_impl::McpProtocolHandler,
    crate::handlers::{SolidMcpHandler, Handler},
    serde_json::json,
};

#[tokio::test]
async fn test_mcp_protocol_handler_creation() {
    let handler = McpProtocolHandler::new();
    assert!(!handler.is_initialized());
    assert_eq!(handler.protocol_version(), "2024-11-05");
}

#[tokio::test]
async fn test_mcp_initialize() {
    let mut handler = McpProtocolHandler::new();
    
    let init_message = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
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
    assert_eq!(response["result"]["protocolVersion"], "2024-11-05");
    assert_eq!(response["result"]["serverInfo"]["name"], "solidmcp");
    
    assert!(handler.is_initialized());
}

#[tokio::test]
async fn test_mcp_tools_list_without_initialization() {
    let mut handler = McpProtocolHandler::new();
    
    let tools_message = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    });

    let result = handler.handle_message(tools_message).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not initialized"));
}

#[tokio::test]
async fn test_mcp_unknown_method() {
    let mut handler = McpProtocolHandler::new();
    
    let unknown_message = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "unknown_method",
        "params": {}
    });

    let result = handler.handle_message(unknown_message).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Unknown method"));
}

#[tokio::test]
async fn test_mcp_error_response_creation() {
    let handler = McpProtocolHandler::new();
    let error_response = handler.create_error_response(
        json!(1), 
        -32601, 
        "Method not found"
    );
    
    assert_eq!(error_response["jsonrpc"], "2.0");
    assert_eq!(error_response["id"], 1);
    assert!(error_response["error"].is_object());
    assert_eq!(error_response["error"]["code"], -32601);
    assert_eq!(error_response["error"]["message"], "Method not found");
}

#[tokio::test]
async fn test_mcp_protocol_version_mismatch() {
    let mut handler = McpProtocolHandler::new();
    
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

    // Should still succeed but log the mismatch
    let response = handler.handle_message(init_message).await.unwrap();
    assert!(response["result"].is_object());
    assert!(handler.is_initialized());
}

#[tokio::test]
async fn test_jsonrpc_error_response_for_unknown_method() {
    use crate::protocol_impl::{McpProtocolHandler, McpError};
    use serde_json::json;

    let mut handler = McpProtocolHandler::new();
    let unknown_message = json!({
        "jsonrpc": "2.0",
        "id": 42,
        "method": "unknown_method",
        "params": {}
    });
    let result = handler.handle_message(unknown_message).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    let mcp_err = err.downcast_ref::<McpError>().expect("Should be McpError");
    assert!(matches!(mcp_err, McpError::UnknownMethod(_)));
}