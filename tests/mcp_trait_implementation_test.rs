//! MCP Trait Implementation Coverage Tests
//!
//! Tests that verify the McpHandler trait integration and custom handler behavior

mod mcp_test_helpers;

use solidmcp::{McpResult, McpError};
use async_trait::async_trait;
use mcp_test_helpers::init_test_tracing;
use serde_json::{json, Value};
use solidmcp::{
    McpContext, McpHandler, ToolDefinition,
    McpServer,
};
use std::sync::Arc;

/// Simple test handler to verify trait implementation
struct TestMcpHandler;

#[async_trait]
impl McpHandler for TestMcpHandler {
    async fn list_tools(&self, _context: &McpContext) -> McpResult<Vec<ToolDefinition>> {
        Ok(vec![ToolDefinition {
            name: "test_tool".to_string(),
            description: "A test tool for trait verification".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "message": {"type": "string"}
                }
            }),
            output_schema: json!({
                "type": "object",
                "properties": {
                    "response": {"type": "string"}
                },
                "required": ["response"]
            }),
        }])
    }

    async fn call_tool(
        &self,
        name: &str,
        arguments: Value,
        _context: &McpContext,
    ) -> McpResult<Value> {
        match name {
            "test_tool" => {
                let message = arguments["message"].as_str().unwrap_or("default");
                Ok(json!({
                    "response": format!("Processed: {}", message)
                }))
            }
            _ => Err(McpError::InvalidParams(format!("Unknown tool: {}", name))),
        }
    }
}

#[tokio::test]
async fn test_custom_trait_implementation() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_test_tracing();

    // Create server with custom handler
    let handler = Arc::new(TestMcpHandler);
    let mut server = McpServer::with_handler(handler).await?;

    // Start server on random port
    let port = mcp_test_helpers::find_available_port()
        .await
        .map_err(|e| McpError::InvalidParams(format!("{}", e)))?;

    tokio::spawn(async move {
        server.start(port).await.unwrap();
    });

    // Give server time to start
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // Test that the custom handler is working
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{port}/mcp");

    // Test tools list
    let tools_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    });

    let response = client.post(&url).json(&tools_request).send().await?;
    assert_eq!(response.status(), 200);

    let tools_response: Value = response.json().await?;
    assert_eq!(tools_response["jsonrpc"], "2.0");
    assert_eq!(tools_response["id"], 1);

    let tools = tools_response["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0]["name"], "test_tool");

    // Test tool execution
    let tool_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "test_tool",
            "arguments": {
                "message": "Hello trait!"
            }
        }
    });

    let response = client.post(&url).json(&tool_request).send().await?;
    assert_eq!(response.status(), 200);

    let tool_response: Value = response.json().await?;
    assert_eq!(tool_response["jsonrpc"], "2.0");
    assert_eq!(tool_response["id"], 2);

    // Verify custom tool response
    assert!(tool_response["result"].is_object());

    println!("✅ Custom trait implementation test passed!");
    Ok(())
}

#[tokio::test]
async fn test_trait_error_handling() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_test_tracing();

    // Create server with custom handler
    let handler = Arc::new(TestMcpHandler);
    let mut server = McpServer::with_handler(handler).await?;

    // Start server on random port
    let port = mcp_test_helpers::find_available_port()
        .await
        .map_err(|e| McpError::InvalidParams(format!("{}", e)))?;

    tokio::spawn(async move {
        server.start(port).await.unwrap();
    });

    // Give server time to start
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{port}/mcp");

    // Test unknown tool error
    let tool_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "unknown_tool",
            "arguments": {}
        }
    });

    let response = client.post(&url).json(&tool_request).send().await?;
    assert_eq!(response.status(), 200);

    let tool_response: Value = response.json().await?;
    assert_eq!(tool_response["jsonrpc"], "2.0");
    assert_eq!(tool_response["id"], 1);

    // Should have error, not result
    assert!(tool_response["error"].is_object());
    assert!(tool_response["result"].is_null());

    println!("✅ Trait error handling test passed!");
    Ok(())
}
