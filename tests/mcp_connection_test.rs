//! MCP Connection Test
//!
//! Tests WebSocket connection establishment and basic connectivity issues.
//! Uses isolated test servers to avoid conflicts with production services.

mod mcp_test_helpers;
use futures_util::{SinkExt, StreamExt};
use mcp_test_helpers::{
    init_test_tracing, receive_ws_message, with_mcp_connection, with_mcp_test_server,
};
use serde_json::{json, Value};
use std::time::Duration;
use tokio_tungstenite::tungstenite::Message;
use tracing::info;

/// Test basic MCP WebSocket connection with isolated server
#[tokio::test]
async fn test_mcp_connection_basic() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_test_tracing();
    info!("🔌 Testing basic MCP WebSocket connection with isolated server");

    with_mcp_connection(
        "test_mcp_connection_basic",
        |_server, mut write, mut read| async move {
            // Test tools/list request
            let tools_message = json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "tools/list",
                "params": {}
            });

            write
                .send(Message::Text(serde_json::to_string(&tools_message)?.into()))
                .await?;
            let response_text = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
            let response: Value = serde_json::from_str(&response_text.to_string())?;

            assert_eq!(response["jsonrpc"], "2.0");
            assert_eq!(response["id"], 2);
            assert!(response["result"]["tools"].is_array());

            let tools = response["result"]["tools"].as_array().unwrap();
            assert!(
                !tools.is_empty(),
                "Should have at least echo and read_file tools"
            );
            info!("📋 Available tools: {:?}", tools.len());

            // Test echo tool
            let echo_message = json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "tools/call",
                "params": {
                    "name": "echo",
                    "arguments": {
                        "message": "Hello, MCP!"
                    }
                }
            });

            write
                .send(Message::Text(serde_json::to_string(&echo_message)?.into()))
                .await?;
            let echo_response_text = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
            let echo_response: Value = serde_json::from_str(&echo_response_text)?;

            assert_eq!(echo_response["jsonrpc"], "2.0");
            assert_eq!(echo_response["id"], 3);
            assert!(echo_response["result"].is_object());
            info!("🔊 Echo result received successfully");

            // Test read_file tool
            let read_message = json!({
                "jsonrpc": "2.0",
                "id": 4,
                "method": "tools/call",
                "params": {
                    "name": "read_file",
                    "arguments": {
                        "file_path": "Cargo.toml"
                    }
                }
            });

            write
                .send(Message::Text(serde_json::to_string(&read_message)?.into()))
                .await?;
            let read_response_text = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
            let read_response: Value = serde_json::from_str(&read_response_text)?;

            assert_eq!(read_response["jsonrpc"], "2.0");
            assert_eq!(read_response["id"], 4);
            assert!(read_response["result"].is_object());
            info!("📖 Read result received successfully");

            Ok(())
        },
    )
    .await?;

    Ok(())
}

/// Test MCP connection with malformed URLs
#[tokio::test]
async fn test_mcp_connection_malformed() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_test_tracing();
    info!("🔗 Testing MCP connection with malformed URLs");

    // Test connection to malformed URLs
    let malformed_urls = vec![
        "ws://invalid-host:9999/mcp",
        "ws://localhost:99999/mcp",  // Invalid port
        "http://localhost:8080/mcp", // Wrong protocol
        "ws://localhost:8080",       // Missing path
    ];

    for url in malformed_urls {
        let result = tokio_tungstenite::connect_async(url).await;
        assert!(
            result.is_err(),
            "Should fail to connect to malformed URL: {url}"
        );
        info!("✅ Correctly failed to connect to malformed URL: {}", url);
    }

    Ok(())
}

/// Test MCP connection timeout scenarios
#[tokio::test]
async fn test_mcp_connection_timeout() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_test_tracing();
    info!("⏰ Testing MCP connection timeout scenarios");

    // Test connection to non-existent server with timeout
    let timeout_result = tokio::time::timeout(
        Duration::from_secs(2),
        tokio_tungstenite::connect_async("ws://localhost:9999/mcp"),
    )
    .await;

    // Should either timeout or fail to connect
    match timeout_result {
        Ok(Ok(_)) => panic!("Should not connect to non-existent server"),
        Ok(Err(_)) => info!("✅ Correctly failed to connect to non-existent server"),
        Err(_) => info!("✅ Correctly timed out connecting to non-existent server"),
    }

    Ok(())
}

/// Test unknown tool error handling
#[tokio::test]
async fn test_mcp_unknown_tool_error() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_test_tracing();
    info!("❌ Testing unknown tool error handling");

    with_mcp_connection(
        "test_unknown_tool_error",
        |_server, mut write, mut read| async move {
            // Test unknown tool (should return error)
            let unknown_message = json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "tools/call",
                "params": {
                    "name": "unknown_tool",
                    "arguments": {}
                }
            });

            write
                .send(Message::Text(
                    serde_json::to_string(&unknown_message)?.into(),
                ))
                .await?;
            let unknown_response_text =
                receive_ws_message(&mut read, Duration::from_secs(2)).await?;
            let unknown_response: Value = serde_json::from_str(&unknown_response_text)?;

            assert_eq!(unknown_response["jsonrpc"], "2.0");
            assert_eq!(unknown_response["id"], 1);
            // Should have error instead of result
            assert!(unknown_response.get("error").is_some());
            assert!(unknown_response.get("result").is_none());

            let error = &unknown_response["error"];
            assert!(error["code"].is_number());
            assert!(error["message"].is_string());
            info!("✅ Correctly failed for unknown tool: {}", error["message"]);

            Ok(())
        },
    )
    .await?;

    Ok(())
}

/// Test connection lifecycle - connect, use, disconnect
#[tokio::test]
async fn test_mcp_connection_lifecycle() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_test_tracing();
    info!("🔄 Testing MCP connection lifecycle");

    with_mcp_test_server("test_connection_lifecycle", |server| async move {
        // Connect to server
        let (ws_stream, _) = tokio_tungstenite::connect_async(&server.ws_url()).await?;
        let (mut write, mut read) = ws_stream.split();
        info!("✅ Connected to MCP server");

        // Initialize connection
        let init_message = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {"name": "test-client", "version": "1.0.0"}
            }
        });

        write
            .send(Message::Text(serde_json::to_string(&init_message)?.into()))
            .await?;
        let _init_response = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
        info!("✅ Initialized connection");

        // Use connection for a simple request
        let tools_message = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        });

        write
            .send(Message::Text(serde_json::to_string(&tools_message)?.into()))
            .await?;
        let _tools_response = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
        info!("✅ Used connection successfully");

        // Close connection gracefully
        write.send(Message::Close(None)).await?;
        info!("✅ Closed connection gracefully");

        Ok(())
    })
    .await?;

    Ok(())
}
