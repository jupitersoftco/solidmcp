//! MCP Notifications Tests
//!
//! Tests for MCP notification handling like cancel notifications.

#[cfg(test)]
use {
    crate::protocol_impl::McpProtocolHandlerImpl, crate::protocol_testable::McpProtocolHandler,
    serde_json::json,
};

#[tokio::test]
async fn test_mcp_cancel_notification() {
    let mut handler = McpProtocolHandlerImpl::new();

    let cancel_message = json!({
        "jsonrpc": "2.0",
        "method": "notifications/cancel",
        "params": {
            "requestId": "test-request"
        }
    });

    let response = handler.handle_message(cancel_message).await.unwrap();
    // For notifications without ID, the response should just be the result
    assert!(response.is_object());
}

#[tokio::test]
async fn test_mcp_cancel_notification_with_id() {
    let mut handler = McpProtocolHandlerImpl::new();

    let cancel_message = json!({
        "jsonrpc": "2.0",
        "id": 42,
        "method": "notifications/cancel",
        "params": {
            "requestId": "test-request-with-id"
        }
    });

    let response = handler.handle_message(cancel_message).await.unwrap();
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 42);
    assert!(response["result"].is_object());
}
