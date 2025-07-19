//! WebSocket Integration Tests
//!
//! End-to-end tests for WebSocket protocol implementation

use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use solidmcp::McpServer;
use tokio_tungstenite::{connect_async, tungstenite::Message};

/// Helper to find an available port
async fn find_available_port() -> u16 {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind to port 0");
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    port
}

/// Test basic WebSocket connection and initialization
#[tokio::test]
async fn test_websocket_connection_and_init() -> Result<()> {
    // Start server on random port
    let port = find_available_port().await;
    let mut server = McpServer::new().await?;

    let server_handle = tokio::spawn(async move { server.start(port).await });

    // Give server time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Connect via WebSocket
    let url = format!("ws://127.0.0.1:{port}/mcp");
    let (ws_stream, _) = connect_async(&url).await?;
    let (mut write, mut read) = ws_stream.split();

    // Send initialization request
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {
                "name": "ws-test-client",
                "version": "1.0.0"
            }
        }
    });

    write.send(Message::Text(init_request.to_string())).await?;

    // Read response
    if let Some(Ok(Message::Text(response_text))) = read.next().await {
        let response: Value = serde_json::from_str(&response_text)?;
        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["id"], 1);
        assert!(response["result"]["capabilities"].is_object());
        assert_eq!(response["result"]["protocolVersion"], "2025-06-18");
    } else {
        panic!("Did not receive expected response");
    }

    // Clean up
    server_handle.abort();
    Ok(())
}

/// Test WebSocket message ordering
#[tokio::test]
async fn test_websocket_message_ordering() -> Result<()> {
    let port = find_available_port().await;
    let mut server = McpServer::new().await?;

    let server_handle = tokio::spawn(async move { server.start(port).await });

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let url = format!("ws://127.0.0.1:{port}/mcp");
    let (ws_stream, _) = connect_async(&url).await?;
    let (mut write, mut read) = ws_stream.split();

    // Initialize first
    let init = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18"
        }
    });

    write.send(Message::Text(init.to_string())).await?;
    let _ = read.next().await; // Consume init response

    // Send multiple requests rapidly
    let request_ids = vec![100, 200, 300, 400, 500];
    for id in &request_ids {
        let request = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/list",
            "params": {}
        });
        write.send(Message::Text(request.to_string())).await?;
    }

    // Collect responses
    let mut received_ids = Vec::new();
    for _ in 0..request_ids.len() {
        if let Some(Ok(Message::Text(response_text))) = read.next().await {
            let response: Value = serde_json::from_str(&response_text)?;
            if let Some(id) = response["id"].as_i64() {
                received_ids.push(id);
            }
        }
    }

    // Verify all IDs were received
    assert_eq!(received_ids.len(), request_ids.len());
    for id in request_ids {
        assert!(received_ids.contains(&(id as i64)));
    }

    server_handle.abort();
    Ok(())
}

/// Test WebSocket ping/pong handling
#[tokio::test]
async fn test_websocket_ping_pong() -> Result<()> {
    let port = find_available_port().await;
    let mut server = McpServer::new().await?;

    let server_handle = tokio::spawn(async move { server.start(port).await });

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let url = format!("ws://127.0.0.1:{port}/mcp");
    let (ws_stream, _) = connect_async(&url).await?;
    let (mut write, mut read) = ws_stream.split();

    // Send ping
    write.send(Message::Ping(vec![1, 2, 3])).await?;

    // Should receive pong
    tokio::time::timeout(tokio::time::Duration::from_secs(5), async {
        while let Some(Ok(msg)) = read.next().await {
            if let Message::Pong(data) = msg {
                assert_eq!(data, vec![1, 2, 3]);
                return;
            }
        }
        panic!("Did not receive pong");
    })
    .await?;

    server_handle.abort();
    Ok(())
}

/// Test WebSocket close handling
#[tokio::test]
async fn test_websocket_close_handling() -> Result<()> {
    let port = find_available_port().await;
    let mut server = McpServer::new().await?;

    let server_handle = tokio::spawn(async move { server.start(port).await });

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let url = format!("ws://127.0.0.1:{port}/mcp");
    let (ws_stream, _) = connect_async(&url).await?;
    let (mut write, mut read) = ws_stream.split();

    // Initialize
    let init = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18"
        }
    });

    write.send(Message::Text(init.to_string())).await?;
    let _ = read.next().await;

    // Send close
    write.send(Message::Close(None)).await?;

    // Should receive close acknowledgment
    tokio::time::timeout(tokio::time::Duration::from_secs(5), async {
        while let Some(Ok(msg)) = read.next().await {
            if matches!(msg, Message::Close(_)) {
                return;
            }
        }
    })
    .await?;

    server_handle.abort();
    Ok(())
}

/// Test handling of large WebSocket messages
#[tokio::test]
async fn test_websocket_large_messages() -> Result<()> {
    let port = find_available_port().await;
    let mut server = McpServer::new().await?;

    let server_handle = tokio::spawn(async move { server.start(port).await });

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let url = format!("ws://127.0.0.1:{port}/mcp");
    let (ws_stream, _) = connect_async(&url).await?;
    let (mut write, mut read) = ws_stream.split();

    // Create large message
    let mut large_array = Vec::new();
    for i in 0..10000 {
        large_array.push(format!("item_{i}"));
    }

    let large_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0",
                "metadata": large_array
            }
        }
    });

    write.send(Message::Text(large_request.to_string())).await?;

    // Should receive response
    if let Some(Ok(Message::Text(response_text))) = read.next().await {
        let response: Value = serde_json::from_str(&response_text)?;
        assert_eq!(response["jsonrpc"], "2.0");
        assert!(response["result"].is_object());
    } else {
        panic!("Did not receive response for large message");
    }

    server_handle.abort();
    Ok(())
}

/// Test concurrent WebSocket connections
#[tokio::test]
async fn test_concurrent_websocket_connections() -> Result<()> {
    let port = find_available_port().await;
    let mut server = McpServer::new().await?;

    let server_handle = tokio::spawn(async move { server.start(port).await });

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Create multiple concurrent connections
    let mut handles = vec![];

    for client_id in 0..5 {
        let url = format!("ws://127.0.0.1:{port}/mcp");

        let handle = tokio::spawn(async move {
            let (ws_stream, _) = connect_async(&url).await?;
            let (mut write, mut read) = ws_stream.split();

            // Each client initializes
            let init = json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2025-06-18",
                    "clientInfo": {
                        "name": format!("client-{}", client_id),
                        "version": "1.0.0"
                    }
                }
            });

            write.send(Message::Text(init.to_string())).await?;

            // Read initialization response
            if let Some(Ok(Message::Text(response_text))) = read.next().await {
                let response: Value = serde_json::from_str(&response_text)?;
                assert_eq!(response["result"]["protocolVersion"], "2025-06-18");
            }

            // Make some requests
            for i in 0..10 {
                let request = json!({
                    "jsonrpc": "2.0",
                    "id": i + 2,
                    "method": "tools/list",
                    "params": {}
                });

                write.send(Message::Text(request.to_string())).await?;

                if let Some(Ok(Message::Text(response_text))) = read.next().await {
                    let response: Value = serde_json::from_str(&response_text)?;
                    assert!(response["result"]["tools"].is_array());
                }
            }

            Ok::<(), anyhow::Error>(())
        });

        handles.push(handle);
    }

    // Wait for all clients to complete
    for handle in handles {
        handle.await??;
    }

    server_handle.abort();
    Ok(())
}

/// Test WebSocket reconnection
#[tokio::test]
async fn test_websocket_reconnection() -> Result<()> {
    let port = find_available_port().await;
    let mut server = McpServer::new().await?;

    let server_handle = tokio::spawn(async move { server.start(port).await });

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let url = format!("ws://127.0.0.1:{port}/mcp");

    // First connection
    {
        let (ws_stream, _) = connect_async(&url).await?;
        let (mut write, mut read) = ws_stream.split();

        let init = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18"
            }
        });

        write.send(Message::Text(init.to_string())).await?;
        let _ = read.next().await;

        // Close connection
        write.send(Message::Close(None)).await?;
    }

    // Second connection (reconnect)
    {
        let (ws_stream, _) = connect_async(&url).await?;
        let (mut write, mut read) = ws_stream.split();

        let init = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18"
            }
        });

        write.send(Message::Text(init.to_string())).await?;

        if let Some(Ok(Message::Text(response_text))) = read.next().await {
            let response: Value = serde_json::from_str(&response_text)?;
            assert_eq!(response["result"]["protocolVersion"], "2025-06-18");
        }
    }

    server_handle.abort();
    Ok(())
}
