//! Resource URI Validation Tests
//!
//! Tests for URI pattern validation, scheme handling, and edge cases
//! in resource URI processing across different transport layers.

use {
    anyhow::Result,
    futures_util::{SinkExt, StreamExt},
    serde_json::{json, Value},
    std::{sync::Arc, time::Duration},
    tokio_tungstenite::{connect_async, tungstenite::Message},
    solidmcp::{
        framework::{McpServerBuilder, ResourceProvider},
        handler::{ResourceContent, ResourceInfo},
    },
};

mod mcp_test_helpers;
use mcp_test_helpers::*;

/// Resource provider that validates different URI schemes and patterns
#[derive(Debug)]
struct UriValidationResourceProvider;

#[async_trait::async_trait]
impl ResourceProvider<()> for UriValidationResourceProvider {
    async fn list_resources(&self, _context: Arc<()>) -> Result<Vec<ResourceInfo>> {
        Ok(vec![
            // Standard schemes
            ResourceInfo {
                uri: "file:///path/to/file.txt".to_string(),
                name: "file_resource".to_string(),
                description: Some("File scheme resource".to_string()),
                mime_type: Some("text/plain".to_string()),
            },
            ResourceInfo {
                uri: "http://example.com/resource".to_string(),
                name: "http_resource".to_string(),
                description: Some("HTTP scheme resource".to_string()),
                mime_type: Some("application/json".to_string()),
            },
            ResourceInfo {
                uri: "https://secure.example.com/data".to_string(),
                name: "https_resource".to_string(),
                description: Some("HTTPS scheme resource".to_string()),
                mime_type: Some("application/json".to_string()),
            },
            // Custom schemes
            ResourceInfo {
                uri: "custom://app/data/123".to_string(),
                name: "custom_resource".to_string(),
                description: Some("Custom scheme resource".to_string()),
                mime_type: Some("application/x-custom".to_string()),
            },
            ResourceInfo {
                uri: "mcp://internal/config".to_string(),
                name: "mcp_resource".to_string(),
                description: Some("MCP internal resource".to_string()),
                mime_type: Some("application/json".to_string()),
            },
            // Edge case URIs
            ResourceInfo {
                uri: "scheme://host:8080/path?query=value#fragment".to_string(),
                name: "complex_uri".to_string(),
                description: Some("Complex URI with all components".to_string()),
                mime_type: Some("text/html".to_string()),
            },
            ResourceInfo {
                uri: "unicode://café/naïve-résumé.txt".to_string(),
                name: "unicode_resource".to_string(),
                description: Some("Unicode characters in URI".to_string()),
                mime_type: Some("text/plain".to_string()),
            },
            // Special characters
            ResourceInfo {
                uri: "special://path/with%20spaces/and%2Bplus".to_string(),
                name: "encoded_resource".to_string(),
                description: Some("URL-encoded characters".to_string()),
                mime_type: Some("text/plain".to_string()),
            },
        ])
    }

    async fn read_resource(&self, uri: &str, _context: Arc<()>) -> Result<ResourceContent> {
        // Validate and process different URI schemes
        let content = match uri {
            uri if uri.starts_with("file://") => {
                format!("File content from: {}", uri.strip_prefix("file://").unwrap_or(""))
            }
            uri if uri.starts_with("http://") || uri.starts_with("https://") => {
                format!("Web resource content from: {}", uri)
            }
            uri if uri.starts_with("custom://") => {
                format!("Custom application data from: {}", uri)
            }
            uri if uri.starts_with("mcp://") => {
                format!("MCP internal data from: {}", uri)
            }
            uri if uri.contains("?query=") => {
                format!("Content with query parameters: {}", uri)
            }
            uri if uri.contains("café") => {
                format!("Content with unicode: {}", uri)
            }
            uri if uri.contains("%20") => {
                format!("Content with encoded chars: {}", uri)
            }
            _ => return Err(anyhow::anyhow!("Unsupported URI scheme: {}", uri)),
        };

        // Determine MIME type based on URI
        let mime_type = if uri.ends_with(".txt") {
            Some("text/plain".to_string())
        } else if uri.ends_with(".json") || uri.contains("application/json") {
            Some("application/json".to_string())
        } else if uri.ends_with(".html") {
            Some("text/html".to_string())
        } else {
            Some("text/plain".to_string())
        };

        Ok(ResourceContent {
            uri: uri.to_string(),
            mime_type,
            content,
        })
    }
}

/// Create test server with URI validation provider
async fn create_uri_test_server() -> Result<solidmcp::McpServer> {
    McpServerBuilder::new((), "uri-test-server", "1.0.0")
        .with_resource_provider(Box::new(UriValidationResourceProvider))
        .build()
        .await
}

/// Test file:// scheme URI handling
#[tokio::test]
async fn test_file_scheme_uri() -> Result<()> {
    init_test_tracing();

    with_mcp_uri_test_server("file_scheme_test", |server| async move {
        let (ws_stream, _) = connect_async(&server.ws_url()).await?;
        let (mut write, mut read) = ws_stream.split();

        // Initialize
        let init_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": { "resources": {} }
            }
        });

        write.send(Message::Text(init_request.to_string().into())).await?;
        receive_ws_message(&mut read, Duration::from_secs(5)).await
            .map_err(|e| anyhow::anyhow!("WebSocket error: {}", e))?;

        // Test file:// URI
        let read_request = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "resources/read",
            "params": {
                "uri": "file:///path/to/file.txt"
            }
        });

        write.send(Message::Text(read_request.to_string().into())).await?;
        let response = receive_ws_message(&mut read, Duration::from_secs(5)).await
            .map_err(|e| anyhow::anyhow!("WebSocket error: {}", e))?;
        let parsed: Value = serde_json::from_str(&response)?;

        assert!(parsed["result"].is_object());
        let content = &parsed["result"]["contents"][0];
        assert_eq!(content["uri"], "file:///path/to/file.txt");
        assert_eq!(content["mimeType"], "text/plain");
        assert!(content["text"].as_str().unwrap().contains("/path/to/file.txt"));

        Ok(())
    }).await
}

/// Test HTTP/HTTPS scheme URI handling
#[tokio::test]
async fn test_http_schemes_uri() -> Result<()> {
    init_test_tracing();

    with_mcp_uri_test_server("http_schemes_test", |server| async move {
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
            .map_err(|e| anyhow::anyhow!("WebSocket error: {}", e))?;

        // Test HTTP URI
        let http_request = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "resources/read",
            "params": {
                "uri": "http://example.com/resource"
            }
        });

        write.send(Message::Text(http_request.to_string().into())).await?;
        let response = receive_ws_message(&mut read, Duration::from_secs(5)).await
            .map_err(|e| anyhow::anyhow!("WebSocket error: {}", e))?;
        let parsed: Value = serde_json::from_str(&response)?;

        let content = &parsed["result"]["contents"][0];
        assert_eq!(content["uri"], "http://example.com/resource");
        assert!(content["text"].as_str().unwrap().contains("http://example.com/resource"));

        // Test HTTPS URI
        let https_request = json!({
            "jsonrpc": "2.0", 
            "id": 3,
            "method": "resources/read",
            "params": {
                "uri": "https://secure.example.com/data"
            }
        });

        write.send(Message::Text(https_request.to_string().into())).await?;
        let response = receive_ws_message(&mut read, Duration::from_secs(5)).await
            .map_err(|e| anyhow::anyhow!("WebSocket error: {}", e))?;
        let parsed: Value = serde_json::from_str(&response)?;

        let content = &parsed["result"]["contents"][0];
        assert_eq!(content["uri"], "https://secure.example.com/data");
        assert!(content["text"].as_str().unwrap().contains("https://secure.example.com/data"));

        Ok(())
    }).await
}

/// Test custom scheme URI handling
#[tokio::test]
async fn test_custom_schemes_uri() -> Result<()> {
    init_test_tracing();

    with_mcp_uri_test_server("custom_schemes_test", |server| async move {
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
            .map_err(|e| anyhow::anyhow!("WebSocket error: {}", e))?;

        // Test custom scheme
        let custom_request = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "resources/read",
            "params": {
                "uri": "custom://app/data/123"
            }
        });

        write.send(Message::Text(custom_request.to_string().into())).await?;
        let response = receive_ws_message(&mut read, Duration::from_secs(5)).await
            .map_err(|e| anyhow::anyhow!("WebSocket error: {}", e))?;
        let parsed: Value = serde_json::from_str(&response)?;

        let content = &parsed["result"]["contents"][0];
        assert_eq!(content["uri"], "custom://app/data/123");
        assert!(content["text"].as_str().unwrap().contains("Custom application data"));

        Ok(())
    }).await
}

/// Test complex URI with all components
#[tokio::test]
async fn test_complex_uri_components() -> Result<()> {
    init_test_tracing();

    with_mcp_uri_test_server("complex_uri_test", |server| async move {
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
            .map_err(|e| anyhow::anyhow!("WebSocket error: {}", e))?;

        // Test complex URI
        let complex_uri = "scheme://host:8080/path?query=value#fragment";
        let complex_request = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "resources/read",
            "params": {
                "uri": complex_uri
            }
        });

        write.send(Message::Text(complex_request.to_string().into())).await?;
        let response = receive_ws_message(&mut read, Duration::from_secs(5)).await
            .map_err(|e| anyhow::anyhow!("WebSocket error: {}", e))?;
        let parsed: Value = serde_json::from_str(&response)?;

        let content = &parsed["result"]["contents"][0];
        assert_eq!(content["uri"], complex_uri);
        assert!(content["text"].as_str().unwrap().contains("query parameters"));

        Ok(())
    }).await
}

/// Test URI with special characters and encoding
#[tokio::test]
async fn test_uri_special_characters() -> Result<()> {
    init_test_tracing();

    with_mcp_uri_test_server("special_chars_test", |server| async move {
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
            .map_err(|e| anyhow::anyhow!("WebSocket error: {}", e))?;

        // Test URL-encoded characters
        let encoded_request = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "resources/read",
            "params": {
                "uri": "special://path/with%20spaces/and%2Bplus"
            }
        });

        write.send(Message::Text(encoded_request.to_string().into())).await?;
        let response = receive_ws_message(&mut read, Duration::from_secs(5)).await
            .map_err(|e| anyhow::anyhow!("WebSocket error: {}", e))?;
        let parsed: Value = serde_json::from_str(&response)?;

        let content = &parsed["result"]["contents"][0];
        assert_eq!(content["uri"], "special://path/with%20spaces/and%2Bplus");
        assert!(content["text"].as_str().unwrap().contains("encoded chars"));

        Ok(())
    }).await
}

/// Test unsupported URI scheme error handling
#[tokio::test]
async fn test_unsupported_uri_scheme() -> Result<()> {
    init_test_tracing();

    with_mcp_uri_test_server("unsupported_scheme_test", |server| async move {
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
            .map_err(|e| anyhow::anyhow!("WebSocket error: {}", e))?;

        // Test unsupported scheme
        let unsupported_request = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "resources/read",
            "params": {
                "uri": "unsupported://invalid/scheme"
            }
        });

        write.send(Message::Text(unsupported_request.to_string().into())).await?;
        let response = receive_ws_message(&mut read, Duration::from_secs(5)).await
            .map_err(|e| anyhow::anyhow!("WebSocket error: {}", e))?;
        let parsed: Value = serde_json::from_str(&response)?;

        // Should return error for unsupported scheme
        assert!(parsed["error"].is_object());
        assert!(parsed["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Resource read error: Resource not found"));

        Ok(())
    }).await
}

// Helper function to create URI test server
async fn start_uri_test_server() -> Result<u16> {
    let port = find_available_port().await
        .map_err(|e| anyhow::anyhow!("Failed to find port: {}", e))?;
    let mut server = create_uri_test_server().await?;
    
    tokio::spawn(async move {
        if let Err(e) = server.start(port).await {
            eprintln!("URI test server error: {e}");
        }
    });

    tokio::time::sleep(Duration::from_millis(100)).await;
    Ok(port)
}

// Custom test helper for URI tests
async fn with_mcp_uri_test_server<F, Fut, T>(
    test_name: &str,
    test_fn: F,
) -> Result<T>
where
    F: FnOnce(McpTestServer) -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    tracing::info!("🚀 Starting MCP URI test server for: {}", test_name);

    let port = start_uri_test_server().await?;
    let server = McpTestServer {
        port,
        server_handle: tokio::spawn(async {}),
    };

    tracing::info!("✅ MCP URI test server started on port {}", server.port);
    let result = test_fn(server).await;
    tracing::info!("🛑 Stopping MCP URI test server for: {}", test_name);

    result
}