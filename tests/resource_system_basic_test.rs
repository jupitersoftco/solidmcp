//! Basic Resource System Tests
//!
//! Tests for fundamental resource functionality including listing, reading,
//! and basic protocol compliance across HTTP and WebSocket transports.

use {
    async_trait::async_trait,
    futures_util::{SinkExt, StreamExt},
    serde_json::{json, Value},
    std::{sync::Arc, time::Duration},
    tokio_tungstenite::{connect_async, tungstenite::Message},
    solidmcp::{
        McpServerBuilder, ResourceProvider,
        ResourceContent, ResourceInfo,
        McpResult, McpError,
    },
};

mod mcp_test_helpers;
use mcp_test_helpers::*;

/// Simple test resource provider with predictable data
#[derive(Debug)]
struct TestResourceProvider {
    resources: Vec<ResourceInfo>,
}

impl TestResourceProvider {
    fn new() -> Self {
        Self {
            resources: vec![
                ResourceInfo {
                    uri: "test://simple".to_string(),
                    name: "simple".to_string(),
                    description: Some("Simple test resource".to_string()),
                    mime_type: Some("text/plain".to_string()),
                },
                ResourceInfo {
                    uri: "test://markdown".to_string(),
                    name: "markdown".to_string(),
                    description: Some("Markdown test resource".to_string()),
                    mime_type: Some("text/markdown".to_string()),
                },
                ResourceInfo {
                    uri: "file:///test/file.txt".to_string(),
                    name: "file.txt".to_string(),
                    description: None,
                    mime_type: Some("text/plain".to_string()),
                },
            ],
        }
    }
}

#[async_trait]
impl ResourceProvider<()> for TestResourceProvider {
    async fn list_resources(&self, _context: Arc<()>) -> McpResult<Vec<ResourceInfo>> {
        Ok(self.resources.clone())
    }

    async fn read_resource(&self, uri: &str, _context: Arc<()>) -> McpResult<ResourceContent> {
        match uri {
            "test://simple" => Ok(ResourceContent {
                uri: uri.to_string(),
                mime_type: Some("text/plain".to_string()),
                content: "Hello, simple resource!".to_string(),
            }),
            "test://markdown" => Ok(ResourceContent {
                uri: uri.to_string(),
                mime_type: Some("text/markdown".to_string()),
                content: "# Markdown Resource\n\nThis is a **markdown** resource.".to_string(),
            }),
            "file:///test/file.txt" => Ok(ResourceContent {
                uri: uri.to_string(),
                mime_type: Some("text/plain".to_string()),
                content: "File content from disk".to_string(),
            }),
            _ => Err(McpError::UnknownResource(uri.to_string())),
        }
    }
}

/// Create a test server with resource provider
async fn create_resource_test_server(context: ()) -> Result<solidmcp::McpServer, Box<dyn std::error::Error + Send + Sync>> {
    McpServerBuilder::new(context, "resource-test-server", "1.0.0")
        .with_resource_provider(Box::new(TestResourceProvider::new()))
        .build()
        .await
}

/// Test basic resources/list functionality via WebSocket
#[tokio::test]
async fn test_websocket_resources_list() -> anyhow::Result<()> {
    init_test_tracing();

    with_mcp_test_server("resource_list_ws", |server| async move {
        // Connect to WebSocket
        let (ws_stream, _) = connect_async(&server.ws_url()).await?;
        let (mut write, mut read) = ws_stream.split();

        // Initialize connection
        let init_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": { "resources": {} },
                "clientInfo": { "name": "test-client", "version": "1.0.0" }
            }
        });

        write.send(Message::Text(init_request.to_string().into())).await?;
        let _init_response = receive_ws_message(&mut read, Duration::from_secs(5)).await
            .map_err(|e| anyhow::anyhow!("WebSocket error: {}", e))?;

        // Test resources/list
        let list_request = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "resources/list",
            "params": {}
        });

        write.send(Message::Text(list_request.to_string().into())).await?;
        let response = receive_ws_message(&mut read, Duration::from_secs(5)).await
            .map_err(|e| anyhow::anyhow!("WebSocket error: {}", e))?;
        let parsed: Value = serde_json::from_str(&response)?;

        // Verify response structure
        assert_eq!(parsed["jsonrpc"], "2.0");
        assert_eq!(parsed["id"], 2);
        assert!(parsed["result"].is_object());

        let resources = parsed["result"]["resources"].as_array().unwrap();
        assert_eq!(resources.len(), 3);

        // Verify first resource
        let simple_resource = &resources[0];
        assert_eq!(simple_resource["uri"], "test://simple");
        assert_eq!(simple_resource["name"], "simple");
        assert_eq!(simple_resource["description"], "Simple test resource");
        assert_eq!(simple_resource["mimeType"], "text/plain");

        Ok(())
    }).await
}

/// Test basic resources/read functionality via WebSocket
#[tokio::test]
async fn test_websocket_resources_read() -> anyhow::Result<()> {
    init_test_tracing();

    with_mcp_test_server("resource_read_ws", |server| async move {
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
                "clientInfo": { "name": "test-client", "version": "1.0.0" }
            }
        });

        write.send(Message::Text(init_request.to_string().into())).await?;
        receive_ws_message(&mut read, Duration::from_secs(5)).await
            .map_err(|e| anyhow::anyhow!("WebSocket error: {}", e))?;

        // Test resources/read
        let read_request = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "resources/read",
            "params": {
                "uri": "test://markdown"
            }
        });

        write.send(Message::Text(read_request.to_string().into())).await?;
        let response = receive_ws_message(&mut read, Duration::from_secs(5)).await
            .map_err(|e| anyhow::anyhow!("WebSocket error: {}", e))?;
        let parsed: Value = serde_json::from_str(&response)?;

        // Verify response structure
        assert_eq!(parsed["jsonrpc"], "2.0");
        assert_eq!(parsed["id"], 2);
        assert!(parsed["result"].is_object());

        let contents = parsed["result"]["contents"].as_array().unwrap();
        assert_eq!(contents.len(), 1);

        let content = &contents[0];
        assert_eq!(content["uri"], "test://markdown");
        assert_eq!(content["mimeType"], "text/markdown");
        assert_eq!(content["text"], "# Markdown Resource\n\nThis is a **markdown** resource.");

        Ok(())
    }).await
}

/// Test resources functionality via HTTP
#[tokio::test]
async fn test_http_resources_list() -> anyhow::Result<()> {
    init_test_tracing();

    with_mcp_test_server("resource_list_http", |server| async move {
        let client = reqwest::Client::new();

        // Initialize session
        let init_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": { "resources": {} },
                "clientInfo": { "name": "test-client", "version": "1.0.0" }
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

        // Test resources/list
        let list_request = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "resources/list",
            "params": {}
        });

        let response = client
            .post(&server.http_url())
            .header("Cookie", cookies)
            .json(&list_request)
            .send()
            .await?;

        assert!(response.status().is_success());
        let parsed: Value = response.json().await?;

        // Verify resources list
        let resources = parsed["result"]["resources"].as_array().unwrap();
        assert_eq!(resources.len(), 3);

        // Check resource URIs are present
        let uris: Vec<&str> = resources.iter()
            .map(|r| r["uri"].as_str().unwrap())
            .collect();
        
        assert!(uris.contains(&"test://simple"));
        assert!(uris.contains(&"test://markdown"));
        assert!(uris.contains(&"file:///test/file.txt"));

        Ok(())
    }).await
}

/// Test resource not found error handling
#[tokio::test]
async fn test_resource_not_found() -> anyhow::Result<()> {
    init_test_tracing();

    with_mcp_test_server("resource_not_found", |server| async move {
        let (ws_stream, _) = connect_async(&server.ws_url()).await?;
        let (mut write, mut read) = ws_stream.split();

        // Initialize
        let init_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        });

        write.send(Message::Text(init_request.to_string().into())).await?;
        receive_ws_message(&mut read, Duration::from_secs(5)).await
            .map_err(|e| anyhow::anyhow!("WebSocket error: {}", e))?;

        // Try to read non-existent resource
        let read_request = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "resources/read",
            "params": {
                "uri": "test://nonexistent"
            }
        });

        write.send(Message::Text(read_request.to_string().into())).await?;
        let response = receive_ws_message(&mut read, Duration::from_secs(5)).await
            .map_err(|e| anyhow::anyhow!("WebSocket error: {}", e))?;
        let parsed: Value = serde_json::from_str(&response)?;

        // Should return error
        assert!(parsed["error"].is_object());
        assert!(parsed["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Resource not found"));

        Ok(())
    }).await
}

// Override the test server creation to use our resource provider
async fn start_test_server_with_resources() -> Result<u16, Box<dyn std::error::Error + Send + Sync>> {
    let port = find_available_port().await
        .map_err(|e| anyhow::anyhow!("Failed to find port: {}", e))?;
    let mut server = create_resource_test_server(()).await?;
    
    tokio::spawn(async move {
        if let Err(e) = server.start(port).await {
            eprintln!("Test server error: {e}");
        }
    });

    tokio::time::sleep(Duration::from_millis(100)).await;
    Ok(port)
}

// Test helper that uses our custom server
async fn with_mcp_test_server<F, Fut, T>(
    test_name: &str,
    test_fn: F,
) -> Result<T, Box<dyn std::error::Error + Send + Sync>>
where
    F: FnOnce(McpTestServer) -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<T>>,
{
    tracing::info!("🚀 Starting MCP resource test server for: {}", test_name);

    let port = start_test_server_with_resources().await?;
    let server = McpTestServer {
        port,
        server_handle: tokio::spawn(async {}), // Dummy handle since we spawn above
    };

    tracing::info!("✅ MCP resource test server started on port {}", server.port);
    let result = test_fn(server).await;
    tracing::info!("🛑 Stopping MCP resource test server for: {}", test_name);

    result
}