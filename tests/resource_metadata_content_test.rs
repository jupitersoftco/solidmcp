//! Resource Metadata and Content Type Tests
//!
//! Tests for resource metadata handling, MIME type validation, content encoding,
//! and various data formats across different transport layers.

use {
    solidmcp::{McpResult, McpError},
    async_trait::async_trait,
    futures_util::{SinkExt, StreamExt},
    serde_json::{json, Value},
    std::{sync::Arc, time::Duration},
    tokio_tungstenite::{connect_async, tungstenite::Message},
    solidmcp::{
        McpServerBuilder, ResourceProvider,
        ResourceContent, ResourceInfo,
    },
};

mod mcp_test_helpers;
use mcp_test_helpers::*;

/// Resource provider that tests various metadata and content types
#[derive(Debug)]
struct MetadataTestResourceProvider;

#[async_trait]
impl ResourceProvider<()> for MetadataTestResourceProvider {
    async fn list_resources(&self, _context: Arc<()>) -> McpResult<Vec<ResourceInfo>> {
        Ok(vec![
            // Text formats
            ResourceInfo {
                uri: "data://text/plain".to_string(),
                name: "plain_text".to_string(),
                description: Some("Plain text content".to_string()),
                mime_type: Some("text/plain".to_string()),
            },
            ResourceInfo {
                uri: "data://text/markdown".to_string(),
                name: "markdown_content".to_string(),
                description: Some("Markdown formatted content".to_string()),
                mime_type: Some("text/markdown".to_string()),
            },
            ResourceInfo {
                uri: "data://text/html".to_string(),
                name: "html_content".to_string(),
                description: Some("HTML formatted content".to_string()),
                mime_type: Some("text/html".to_string()),
            },
            // Structured data formats
            ResourceInfo {
                uri: "data://application/json".to_string(),
                name: "json_data".to_string(),
                description: Some("JSON structured data".to_string()),
                mime_type: Some("application/json".to_string()),
            },
            ResourceInfo {
                uri: "data://application/xml".to_string(),
                name: "xml_data".to_string(),
                description: Some("XML structured data".to_string()),
                mime_type: Some("application/xml".to_string()),
            },
            ResourceInfo {
                uri: "data://text/csv".to_string(),
                name: "csv_data".to_string(),
                description: Some("CSV tabular data".to_string()),
                mime_type: Some("text/csv".to_string()),
            },
            // Binary-like content represented as text
            ResourceInfo {
                uri: "data://application/base64".to_string(),
                name: "base64_data".to_string(),
                description: Some("Base64 encoded data".to_string()),
                mime_type: Some("application/octet-stream".to_string()),
            },
            // Resource without MIME type
            ResourceInfo {
                uri: "data://unknown/format".to_string(),
                name: "unknown_format".to_string(),
                description: Some("Content with no specified MIME type".to_string()),
                mime_type: None,
            },
            // Resource without description
            ResourceInfo {
                uri: "data://minimal/info".to_string(),
                name: "minimal_info".to_string(),
                description: None,
                mime_type: Some("text/plain".to_string()),
            },
            // Large content resource
            ResourceInfo {
                uri: "data://large/content".to_string(),
                name: "large_content".to_string(),
                description: Some("Large content for size testing".to_string()),
                mime_type: Some("text/plain".to_string()),
            },
        ])
    }

    async fn read_resource(&self, uri: &str, _context: Arc<()>) -> McpResult<ResourceContent> {
        match uri {
            "data://text/plain" => Ok(ResourceContent {
                uri: uri.to_string(),
                mime_type: Some("text/plain".to_string()),
                content: "This is plain text content with special characters: äöü, 日本語, 🚀".to_string(),
            }),
            "data://text/markdown" => Ok(ResourceContent {
                uri: uri.to_string(),
                mime_type: Some("text/markdown".to_string()),
                content: "# Markdown Content\n\n## Features\n\n- **Bold text**\n- *Italic text*\n- [Link](https://example.com)\n- `Code snippet`\n\n```rust\nfn hello() {\n    println!(\"Hello, world!\");\n}\n```".to_string(),
            }),
            "data://text/html" => Ok(ResourceContent {
                uri: uri.to_string(),
                mime_type: Some("text/html".to_string()),
                content: "<!DOCTYPE html>\n<html>\n<head><title>Test</title></head>\n<body>\n<h1>HTML Content</h1>\n<p>This is <strong>HTML</strong> with <em>formatting</em>.</p>\n<a href=\"https://example.com\">Link</a>\n</body>\n</html>".to_string(),
            }),
            "data://application/json" => Ok(ResourceContent {
                uri: uri.to_string(),
                mime_type: Some("application/json".to_string()),
                content: json!({
                    "name": "Test Data",
                    "version": "1.0.0",
                    "features": ["json", "parsing", "validation"],
                    "metadata": {
                        "created": "2025-01-31T00:00:00Z",
                        "author": "test"
                    },
                    "numbers": [1, 2, 3.14, -42],
                    "boolean": true,
                    "null_value": null
                }).to_string(),
            }),
            "data://application/xml" => Ok(ResourceContent {
                uri: uri.to_string(),
                mime_type: Some("application/xml".to_string()),
                content: "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<root>\n  <item id=\"1\">\n    <name>Test Item</name>\n    <value>42</value>\n    <active>true</active>\n  </item>\n  <item id=\"2\">\n    <name>Another Item</name>\n    <value>3.14</value>\n    <active>false</active>\n  </item>\n</root>".to_string(),
            }),
            "data://text/csv" => Ok(ResourceContent {
                uri: uri.to_string(),
                mime_type: Some("text/csv".to_string()),
                content: "name,age,city,score\nAlice,30,New York,95.5\nBob,25,London,87.2\nCharlie,35,Tokyo,92.1\nDiana,28,\"San Francisco\",89.7".to_string(),
            }),
            "data://application/base64" => Ok(ResourceContent {
                uri: uri.to_string(),
                mime_type: Some("application/octet-stream".to_string()),
                content: "SGVsbG8sIFdvcmxkISBUaGlzIGlzIGEgYmFzZTY0IGVuY29kZWQgbWVzc2FnZS4gSXQgY29udGFpbnMgc3BlY2lhbCBjaGFyYWN0ZXJzOiDDpMO2w7wsIOaXpeacrOiqniwg8J+agA==".to_string(),
            }),
            "data://unknown/format" => Ok(ResourceContent {
                uri: uri.to_string(),
                mime_type: None,
                content: "This content has no specified MIME type and could be anything.".to_string(),
            }),
            "data://minimal/info" => Ok(ResourceContent {
                uri: uri.to_string(),
                mime_type: Some("text/plain".to_string()),
                content: "Minimal resource info.".to_string(),
            }),
            "data://large/content" => {
                // Generate large content (about 50KB)
                let mut large_content = String::new();
                for i in 0..1000 {
                    large_content.push_str(&format!("Line {}: This is a long line of text to test large content handling in the resource system. It contains enough text to make the overall content size significant for testing purposes.\n", i + 1));
                }
                Ok(ResourceContent {
                    uri: uri.to_string(),
                    mime_type: Some("text/plain".to_string()),
                    content: large_content,
                })
            },
            _ => Err(McpError::InvalidParams(format!("Resource not found: {}", uri))),
        }
    }
}

/// Create test server with metadata provider
async fn create_metadata_test_server() -> McpResult<solidmcp::McpServer> {
    McpServerBuilder::new((), "metadata-test-server", "1.0.0")
        .with_resource_provider(Box::new(MetadataTestResourceProvider))
        .build()
        .await
}

/// Test various MIME types and content formats
#[tokio::test]
async fn test_mime_types_and_formats() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_test_tracing();

    with_metadata_test_server("mime_types_test", |server| async move {
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
            .map_err(|e| McpError::InvalidParams(format!("WebSocket error: {}", e)))?;

        // Test different content types
        let test_cases = vec![
            ("data://text/plain", "text/plain", "special characters"),
            ("data://text/markdown", "text/markdown", "# Markdown"),
            ("data://text/html", "text/html", "<!DOCTYPE html>"),
            ("data://application/json", "application/json", "\"name\""),
            ("data://application/xml", "application/xml", "<?xml version"),
            ("data://text/csv", "text/csv", "name,age,city"),
        ];

        for (uri, expected_mime, content_check) in test_cases {
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

            assert!(parsed["result"].is_object());
            let content = &parsed["result"]["contents"][0];
            assert_eq!(content["uri"], uri);
            assert_eq!(content["mimeType"], expected_mime);
            assert!(content["text"].as_str().unwrap().contains(content_check));
        }

        Ok(())
    }).await
}

/// Test resource metadata completeness
#[tokio::test]
async fn test_resource_metadata() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_test_tracing();

    with_metadata_test_server("metadata_test", |server| async move {
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
            .map_err(|e| McpError::InvalidParams(format!("WebSocket error: {}", e)))?;

        // List resources to check metadata
        let list_request = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "resources/list",
            "params": {}
        });

        write.send(Message::Text(list_request.to_string().into())).await?;
        let response = receive_ws_message(&mut read, Duration::from_secs(5)).await
            .map_err(|e| McpError::InvalidParams(format!("WebSocket error: {}", e)))?;
        let parsed: Value = serde_json::from_str(&response)?;

        let resources = parsed["result"]["resources"].as_array().unwrap();
        assert_eq!(resources.len(), 10);

        // Find and verify specific resources
        let json_resource = resources.iter()
            .find(|r| r["name"] == "json_data")
            .unwrap();
        assert_eq!(json_resource["uri"], "data://application/json");
        assert_eq!(json_resource["mimeType"], "application/json");
        assert_eq!(json_resource["description"], "JSON structured data");

        // Check resource without description
        let minimal_resource = resources.iter()
            .find(|r| r["name"] == "minimal_info")
            .unwrap();
        assert_eq!(minimal_resource["uri"], "data://minimal/info");
        assert_eq!(minimal_resource["mimeType"], "text/plain");
        assert!(minimal_resource["description"].is_null());

        // Check resource without MIME type
        let unknown_resource = resources.iter()
            .find(|r| r["name"] == "unknown_format")
            .unwrap();
        assert_eq!(unknown_resource["uri"], "data://unknown/format");
        assert!(unknown_resource["mimeType"].is_null());

        Ok(())
    }).await
}

/// Test large content handling
#[tokio::test]
async fn test_large_content_handling() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_test_tracing();

    with_metadata_test_server("large_content_test", |server| async move {
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
            .map_err(|e| McpError::InvalidParams(format!("WebSocket error: {}", e)))?;

        // Request large content
        let read_request = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "resources/read",
            "params": {
                "uri": "data://large/content"
            }
        });

        write.send(Message::Text(read_request.to_string().into())).await?;
        let response = receive_ws_message(&mut read, Duration::from_secs(10)).await
            .map_err(|e| McpError::InvalidParams(format!("WebSocket error: {}", e)))?;
        let parsed: Value = serde_json::from_str(&response)?;

        assert!(parsed["result"].is_object());
        let content = &parsed["result"]["contents"][0];
        assert_eq!(content["uri"], "data://large/content");
        assert_eq!(content["mimeType"], "text/plain");
        
        let text_content = content["text"].as_str().unwrap();
        assert!(text_content.len() > 40000); // Should be around 50KB
        assert!(text_content.contains("Line 1:"));
        assert!(text_content.contains("Line 1000:"));

        Ok(())
    }).await
}

/// Test content with Unicode and special characters
#[tokio::test]
async fn test_unicode_content() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_test_tracing();

    with_metadata_test_server("unicode_test", |server| async move {
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
            .map_err(|e| McpError::InvalidParams(format!("WebSocket error: {}", e)))?;

        // Request content with Unicode
        let read_request = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "resources/read",
            "params": {
                "uri": "data://text/plain"
            }
        });

        write.send(Message::Text(read_request.to_string().into())).await?;
        let response = receive_ws_message(&mut read, Duration::from_secs(5)).await
            .map_err(|e| McpError::InvalidParams(format!("WebSocket error: {}", e)))?;
        let parsed: Value = serde_json::from_str(&response)?;

        let content = &parsed["result"]["contents"][0];
        let text = content["text"].as_str().unwrap();
        
        // Verify Unicode characters are preserved
        assert!(text.contains("äöü"));
        assert!(text.contains("日本語"));
        assert!(text.contains("🚀"));

        Ok(())
    }).await
}

// Helper function to create metadata test server
async fn start_metadata_test_server() -> McpResult<u16> {
    let port = find_available_port().await
        .map_err(|e| McpError::InvalidParams(format!("Failed to find port: {}", e)))?;
    let mut server = create_metadata_test_server().await?;
    
    tokio::spawn(async move {
        if let Err(e) = server.start(port).await {
            eprintln!("Metadata test server error: {e}");
        }
    });

    tokio::time::sleep(Duration::from_millis(100)).await;
    Ok(port)
}

// Custom test helper for metadata tests
async fn with_metadata_test_server<F, Fut, T>(
    test_name: &str,
    test_fn: F,
) -> McpResult<T>
where
    F: FnOnce(McpTestServer) -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<T>>,
{
    tracing::info!("🚀 Starting MCP metadata test server for: {}", test_name);

    let port = start_metadata_test_server().await?;
    let server = McpTestServer {
        port,
        server_handle: tokio::spawn(async {}),
    };

    tracing::info!("✅ MCP metadata test server started on port {}", server.port);
    let result = test_fn(server).await;
    tracing::info!("🛑 Stopping MCP metadata test server for: {}", test_name);

    result
}