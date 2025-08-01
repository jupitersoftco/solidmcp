//! MCP Notification System Tests
//!
//! Tests that verify notification functionality using trait-based handlers

use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};
use solidmcp::{
    handler::{McpContext, McpHandler, ToolDefinition},
    McpServer,
};
use std::sync::Arc;
use std::time::Duration;

/// Test handler with add_notification tool
struct NotificationTestHandler;

#[async_trait]
impl McpHandler for NotificationTestHandler {
    async fn list_tools(&self, _context: &McpContext) -> Result<Vec<ToolDefinition>> {
        Ok(vec![ToolDefinition {
            name: "add_notification".to_string(),
            description: "Add a test notification".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "level": {"type": "string"},
                    "message": {"type": "string"},
                    "data": {"type": "object"}
                },
                "required": ["level", "message"]
            }),
            output_schema: json!({
                "type": "object",
                "properties": {
                    "content": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "type": {"type": "string"},
                                "text": {"type": "string"}
                            }
                        }
                    }
                }
            }),
        }])
    }

    async fn call_tool(
        &self,
        name: &str,
        arguments: Value,
        _context: &McpContext,
    ) -> Result<Value> {
        match name {
            "add_notification" => {
                let level = arguments["level"].as_str().unwrap_or("info");
                if !matches!(level, "info" | "warning" | "error") {
                    return Err(anyhow::anyhow!("Invalid log level: {}", level));
                }

                let message = arguments["message"].as_str().unwrap_or("").to_string();
                Ok(json!({
                    "content": [{
                        "type": "text",
                        "text": serde_json::to_string(&json!({
                            "success": true,
                            "level": level,
                            "message": message
                        }))?
                    }]
                }))
            }
            _ => Err(anyhow::anyhow!("Unknown tool: {}", name)),
        }
    }
}

#[tokio::test]
async fn test_notification_tool_execution() -> Result<()> {
    // Create server with notification handler
    let handler = Arc::new(NotificationTestHandler);
    let mut server = McpServer::with_handler(handler).await?;

    // Find available port and start server
    let port = {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        let port = addr.port();
        drop(listener);
        port
    };

    tokio::spawn(async move {
        server.start(port).await.unwrap();
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Test HTTP endpoint for notification tool
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{port}/mcp");

    // Initialize session
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {
                "name": "notification-test-client",
                "version": "1.0.0"
            }
        }
    });

    let response = client.post(&url).json(&init_request).send().await?;
    assert_eq!(response.status(), 200);
    let init_response: Value = response.json().await?;
    assert_eq!(init_response["jsonrpc"], "2.0");

    // Test add_notification tool
    let notification_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "add_notification",
            "arguments": {
                "level": "info",
                "message": "Test notification message",
                "data": {"key": "value"}
            }
        }
    });

    let response = client.post(&url).json(&notification_request).send().await?;
    assert_eq!(response.status(), 200);

    let notification_response: Value = response.json().await?;
    assert_eq!(notification_response["jsonrpc"], "2.0");
    assert_eq!(notification_response["id"], 2);

    // Check if response indicates success
    assert!(notification_response["result"].is_object());
    let result = &notification_response["result"];

    // The result should be formatted as MCP tool response
    assert!(result["content"].is_array());
    let content = result["content"].as_array().unwrap();
    assert!(!content.is_empty());

    // Parse the actual response
    let content_text = content[0]["text"].as_str().unwrap();
    let response_data: Value = serde_json::from_str(content_text).unwrap();
    assert_eq!(response_data["success"], true);

    println!("✅ Notification tool execution test passed!");
    Ok(())
}

#[tokio::test]
async fn test_notification_tool_validation() -> Result<()> {
    // Create server with notification handler
    let handler = Arc::new(NotificationTestHandler);
    let mut server = McpServer::with_handler(handler).await?;

    // Find available port and start server
    let port = {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        let port = addr.port();
        drop(listener);
        port
    };

    tokio::spawn(async move {
        server.start(port).await.unwrap();
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(300)).await;

    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{port}/mcp");

    // Test with invalid log level
    let invalid_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "add_notification",
            "arguments": {
                "level": "invalid_level",
                "message": "Test message"
            }
        }
    });

    let response = client.post(&url).json(&invalid_request).send().await?;
    assert_eq!(response.status(), 200);

    let error_response: Value = response.json().await?;
    assert_eq!(error_response["jsonrpc"], "2.0");
    assert_eq!(error_response["id"], 1);

    // Should return an error for invalid log level
    assert!(error_response["error"].is_object());
    let error = &error_response["error"];
    assert!(error["message"]
        .as_str()
        .unwrap()
        .contains("Invalid log level"));

    println!("✅ Notification tool validation test passed!");
    Ok(())
}
