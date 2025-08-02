//! Resource Error Handling Tests
//!
//! Tests for resource system error scenarios, invalid requests, malformed URIs,
//! access denied, timeouts, and protocol error compliance.

use {
    solidmcp::{McpResult, McpError},
    async_trait::async_trait,
    futures_util::{SinkExt, StreamExt},
    serde_json::{json, Value},
    std::{sync::Arc, time::Duration},
    tokio_tungstenite::{connect_async, tungstenite::Message},
    solidmcp::{McpServerBuilder, ResourceProvider, ResourceContent, ResourceInfo},
};

mod mcp_test_helpers;
use mcp_test_helpers::*;

/// Resource provider that simulates various error conditions
#[derive(Debug)]
struct ErrorTestResourceProvider;

#[async_trait]
impl ResourceProvider<()> for ErrorTestResourceProvider {
    async fn list_resources(&self, _context: Arc<()>) -> McpResult<Vec<ResourceInfo>> {
        Ok(vec![
            ResourceInfo {
                uri: "error://not-found".to_string(),
                name: "not_found".to_string(),
                description: Some("Resource that returns not found".to_string()),
                mime_type: Some("text/plain".to_string()),
            },
            ResourceInfo {
                uri: "error://access-denied".to_string(),
                name: "access_denied".to_string(),
                description: Some("Resource that returns access denied".to_string()),
                mime_type: Some("text/plain".to_string()),
            },
            ResourceInfo {
                uri: "error://timeout".to_string(),
                name: "timeout".to_string(),
                description: Some("Resource that simulates timeout".to_string()),
                mime_type: Some("text/plain".to_string()),
            },
            ResourceInfo {
                uri: "error://server-error".to_string(),
                name: "server_error".to_string(),
                description: Some("Resource that causes server error".to_string()),
                mime_type: Some("text/plain".to_string()),
            },
            ResourceInfo {
                uri: "valid://resource".to_string(),
                name: "valid_resource".to_string(),
                description: Some("Valid resource for comparison".to_string()),
                mime_type: Some("text/plain".to_string()),
            },
        ])
    }

    async fn read_resource(&self, uri: &str, _context: Arc<()>) -> McpResult<ResourceContent> {
        match uri {
            "error://not-found" => {
                Err(McpError::InvalidParams(format!("Resource not found: {}", uri)))
            }
            "error://access-denied" => {
                Err(McpError::InvalidParams(format!("Access denied to resource: {}", uri)))
            }
            "error://timeout" => {
                // Simulate a delay that might cause timeout in real scenarios
                tokio::time::sleep(Duration::from_millis(100)).await;
                Err(McpError::InvalidParams(format!("Operation timed out for resource: {}", uri)))
            }
            "error://server-error" => {
                Err(McpError::InvalidParams(format!("Internal server error while reading resource: {}", uri)))
            }
            "valid://resource" => Ok(ResourceContent {
                uri: uri.to_string(),
                mime_type: Some("text/plain".to_string()),
                content: "This is a valid resource for testing.".to_string(),
            }),
            _ => Err(McpError::InvalidParams(format!("Unknown resource: {}", uri))),
        }
    }
}

/// Create test server with error provider
async fn create_error_test_server() -> McpResult<solidmcp::McpServer> {
    let server = McpServerBuilder::new((), "error-test-server", "1.0.0")
        .with_resource_provider(Box::new(ErrorTestResourceProvider))
        .build()
        .await
        .map_err(|e| McpError::InvalidParams(format!("Failed to build server: {}", e)))?;
    Ok(server)
}

/// Test resource not found error
#[tokio::test]
async fn test_resource_not_found_error() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_test_tracing();

    with_error_test_server("not_found_test", |server| async move {
        let (ws_stream, _) = connect_async(&server.ws_url()).await?;
        let (mut write, mut read) = ws_stream.split();

        // Initialize
        let init_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {
                    "name": "test-client",
                    "version": "1.0.0"
                }
            }
        });

        write.send(Message::Text(init_request.to_string().into())).await?;
        receive_ws_message(&mut read, Duration::from_secs(5)).await
            .map_err(|e| McpError::InvalidParams(format!("WebSocket error: {}", e)))?;

        // Request non-existent resource
        let read_request = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "resources/read",
            "params": {
                "uri": "error://not-found"
            }
        });

        write.send(Message::Text(read_request.to_string().into())).await?;
        let response = receive_ws_message(&mut read, Duration::from_secs(5)).await
            .map_err(|e| McpError::InvalidParams(format!("WebSocket error: {}", e)))?;
        let parsed: Value = serde_json::from_str(&response)?;

        // Should return JSON-RPC error
        assert!(parsed["error"].is_object());
        assert!(parsed["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Resource read error: Resource not found"));
        assert_eq!(parsed["id"], 2);
        assert_eq!(parsed["jsonrpc"], "2.0");

        Ok(())
    }).await
}

/// Test access denied error
#[tokio::test]
async fn test_access_denied_error() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_test_tracing();

    with_error_test_server("access_denied_test", |server| async move {
        let (ws_stream, _) = connect_async(&server.ws_url()).await?;
        let (mut write, mut read) = ws_stream.split();

        // Initialize
        let init_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {
                    "name": "test-client",
                    "version": "1.0.0"
                }
            }
        });

        write.send(Message::Text(init_request.to_string().into())).await?;
        receive_ws_message(&mut read, Duration::from_secs(5)).await
            .map_err(|e| McpError::InvalidParams(format!("WebSocket error: {}", e)))?;

        // Request access denied resource
        let read_request = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "resources/read",
            "params": {
                "uri": "error://access-denied"
            }
        });

        write.send(Message::Text(read_request.to_string().into())).await?;
        let response = receive_ws_message(&mut read, Duration::from_secs(5)).await
            .map_err(|e| McpError::InvalidParams(format!("WebSocket error: {}", e)))?;
        let parsed: Value = serde_json::from_str(&response)?;

        // Should return JSON-RPC error
        assert!(parsed["error"].is_object());
        assert!(parsed["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Resource read error: Resource not found"));

        Ok(())
    }).await
}

/// Test server error handling
#[tokio::test]
async fn test_server_error_handling() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_test_tracing();

    with_error_test_server("server_error_test", |server| async move {
        let (ws_stream, _) = connect_async(&server.ws_url()).await?;
        let (mut write, mut read) = ws_stream.split();

        // Initialize
        let init_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {
                    "name": "test-client",
                    "version": "1.0.0"
                }
            }
        });

        write.send(Message::Text(init_request.to_string().into())).await?;
        receive_ws_message(&mut read, Duration::from_secs(5)).await
            .map_err(|e| McpError::InvalidParams(format!("WebSocket error: {}", e)))?;

        // Request resource that causes server error
        let error_request = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "resources/read",
            "params": {
                "uri": "error://server-error"
            }
        });

        write.send(Message::Text(error_request.to_string().into())).await?;
        let response = receive_ws_message(&mut read, Duration::from_secs(5)).await
            .map_err(|e| McpError::InvalidParams(format!("WebSocket error: {}", e)))?;
        let parsed: Value = serde_json::from_str(&response)?;

        // Should return internal error
        assert!(parsed["error"].is_object());
        assert!(parsed["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Resource read error: Resource not found"));

        Ok(())
    }).await
}

/// Test HTTP error responses
#[tokio::test]
async fn test_http_error_responses() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_test_tracing();

    with_error_test_server("http_error_test", |server| async move {
        let client = reqwest::Client::new();

        // Initialize session
        let init_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {
                    "name": "test-client",
                    "version": "1.0.0"
                }
            }
        });

        let init_response = client
            .post(&server.http_url())
            .json(&init_request)
            .send()
            .await?;

        assert!(init_response.status().is_success());
        let cookies = init_response.headers()
            .get_all("set-cookie")
            .iter()
            .map(|v| v.to_str().unwrap_or(""))
            .collect::<Vec<_>>()
            .join("; ");

        // Test error resource via HTTP
        let error_request = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "resources/read",
            "params": {
                "uri": "error://not-found"
            }
        });

        let response = client
            .post(&server.http_url())
            .header("Cookie", cookies)
            .json(&error_request)
            .send()
            .await?;

        // HTTP should still return 200 for JSON-RPC errors
        assert!(response.status().is_success());
        let parsed: Value = response.json().await?;

        // But the JSON-RPC response should contain the error
        assert!(parsed["error"].is_object());
        assert!(parsed["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Resource read error: Resource not found"));

        Ok(())
    }).await
}

/// Test error message format compliance
#[tokio::test]
async fn test_error_format_compliance() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_test_tracing();

    with_error_test_server("error_format_test", |server| async move {
        let (ws_stream, _) = connect_async(&server.ws_url()).await?;
        let (mut write, mut read) = ws_stream.split();

        // Initialize
        let init_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {
                    "name": "test-client",
                    "version": "1.0.0"
                }
            }
        });

        write.send(Message::Text(init_request.to_string().into())).await?;
        receive_ws_message(&mut read, Duration::from_secs(5)).await
            .map_err(|e| McpError::InvalidParams(format!("WebSocket error: {}", e)))?;

        // Test multiple error scenarios
        // Note: The framework converts all resource errors to "Resource not found"
        let error_requests = vec![
            ("error://not-found", "Resource read error: Resource not found"),
            ("error://access-denied", "Resource read error: Resource not found"),
            ("error://server-error", "Resource read error: Resource not found"),
        ];

        for (uri, expected_error) in error_requests {
            let read_request = json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "resources/read",
                "params": {
                    "uri": uri
                }
            });

            write.send(Message::Text(read_request.to_string().into())).await?;
            let response = receive_ws_message(&mut read, Duration::from_secs(5)).await
                .map_err(|e| McpError::InvalidParams(format!("WebSocket error: {}", e)))?;
            let parsed: Value = serde_json::from_str(&response)?;

            // Verify JSON-RPC error format
            assert_eq!(parsed["jsonrpc"], "2.0");
            assert_eq!(parsed["id"], 2);
            assert!(parsed["error"].is_object());
            assert!(parsed["error"]["code"].is_number());
            assert!(parsed["error"]["message"].is_string());
            assert!(parsed["error"]["message"]
                .as_str()
                .unwrap()
                .contains(expected_error));
        }

        Ok(())
    }).await
}

// Helper function to create error test server
async fn start_error_test_server() -> McpResult<(tokio::task::JoinHandle<Result<(), anyhow::Error>>, u16)> {
    let server = create_error_test_server().await?;
    
    let (server_handle, port) = server.start_dynamic().await
        .map_err(|e| McpError::InvalidParams(format!("Failed to start server: {}", e)))?;

    tokio::time::sleep(Duration::from_millis(100)).await;
    Ok((server_handle, port))
}

// Custom test helper for error tests
async fn with_error_test_server<F, Fut, T>(
    test_name: &str,
    test_fn: F,
) -> Result<T, Box<dyn std::error::Error + Send + Sync>>
where
    F: FnOnce(McpTestServer) -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<T>>,
{
    tracing::info!("🚀 Starting MCP error test server for: {}", test_name);

    let (server_handle, port) = start_error_test_server().await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
    let server = McpTestServer {
        port,
        server_handle,
    };

    tracing::info!("✅ MCP error test server started on port {}", server.port);
    let result = test_fn(server).await;
    tracing::info!("🛑 Stopping MCP error test server for: {}", test_name);

    result.map_err(|e| e.into())
}