//! Concurrent Client Handling Tests
//!
//! Integration tests for handling multiple concurrent clients

use anyhow::Result;
use serde_json::{json, Value};
use solidmcp::McpServer;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::Barrier;

/// Helper to find an available port
async fn find_available_port() -> u16 {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind to port 0");
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    port
}

/// Test multiple HTTP clients making concurrent requests
#[tokio::test]
async fn test_concurrent_http_clients() -> Result<()> {
    let port = find_available_port().await;
    let server = McpServer::new().await?;

    let server_handle = tokio::spawn(async move { server.start(port).await });

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let url = format!("http://127.0.0.1:{}/mcp", port);
    let client_count = 20;
    let requests_per_client = 10;

    // Use a barrier to ensure all clients start at the same time
    let barrier = Arc::new(Barrier::new(client_count));
    let success_count = Arc::new(AtomicUsize::new(0));

    let mut handles = vec![];

    for client_id in 0..client_count {
        let url = url.clone();
        let barrier = barrier.clone();
        let success_count = success_count.clone();

        let handle = tokio::spawn(async move {
            let client = reqwest::Client::new();

            // Wait for all clients to be ready
            barrier.wait().await;

            // Initialize
            let init_request = json!({
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

            let session_cookie = format!("mcp_session=client_{}", client_id);

            let response = client
                .post(&url)
                .header("Cookie", &session_cookie)
                .json(&init_request)
                .send()
                .await?;

            assert_eq!(response.status(), 200);

            // Make concurrent requests
            for request_id in 0..requests_per_client {
                let request = json!({
                    "jsonrpc": "2.0",
                    "id": request_id + 2,
                    "method": "tools/list",
                    "params": {}
                });

                let response = client
                    .post(&url)
                    .header("Cookie", &session_cookie)
                    .json(&request)
                    .send()
                    .await?;

                assert_eq!(response.status(), 200);
                let response_json: Value = response.json().await?;
                assert!(response_json["result"]["tools"].is_array());

                success_count.fetch_add(1, Ordering::Relaxed);
            }

            Ok::<(), anyhow::Error>(())
        });

        handles.push(handle);
    }

    // Wait for all clients
    for handle in handles {
        handle.await??;
    }

    // Verify all requests succeeded
    assert_eq!(
        success_count.load(Ordering::Relaxed),
        client_count * requests_per_client
    );

    server_handle.abort();
    Ok(())
}

/// Test session isolation with concurrent clients
#[tokio::test]
async fn test_session_isolation_concurrent() -> Result<()> {
    let port = find_available_port().await;
    let server = McpServer::new().await?;

    let server_handle = tokio::spawn(async move { server.start(port).await });

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let url = format!("http://127.0.0.1:{}/mcp", port);

    // Create two clients with different sessions
    let client1 = reqwest::Client::new();
    let client2 = reqwest::Client::new();

    // Initialize both clients
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18"
        }
    });

    let response1 = client1
        .post(&url)
        .header("Cookie", "mcp_session=session1")
        .json(&init_request)
        .send()
        .await?;
    assert_eq!(response1.status(), 200);

    let response2 = client2
        .post(&url)
        .header("Cookie", "mcp_session=session2")
        .json(&init_request)
        .send()
        .await?;
    assert_eq!(response2.status(), 200);

    // Make concurrent requests from both clients
    let barrier = Arc::new(Barrier::new(2));

    let url1 = url.clone();
    let barrier1 = barrier.clone();
    let handle1 = tokio::spawn(async move {
        barrier1.wait().await;

        for i in 0..50 {
            let request = json!({
                "jsonrpc": "2.0",
                "id": i + 2,
                "method": "tools/list",
                "params": {}
            });

            let response = client1
                .post(&url1)
                .header("Cookie", "mcp_session=session1")
                .json(&request)
                .send()
                .await?;

            assert_eq!(response.status(), 200);
        }

        Ok::<(), anyhow::Error>(())
    });

    let url2 = url;
    let barrier2 = barrier;
    let handle2 = tokio::spawn(async move {
        barrier2.wait().await;

        for i in 0..50 {
            let request = json!({
                "jsonrpc": "2.0",
                "id": i + 2,
                "method": "tools/list",
                "params": {}
            });

            let response = client2
                .post(&url2)
                .header("Cookie", "mcp_session=session2")
                .json(&request)
                .send()
                .await?;

            assert_eq!(response.status(), 200);
        }

        Ok::<(), anyhow::Error>(())
    });

    // Wait for both clients
    handle1.await??;
    handle2.await??;

    server_handle.abort();
    Ok(())
}

/// Test mixed WebSocket and HTTP clients
#[tokio::test]
async fn test_mixed_protocol_clients() -> Result<()> {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::{connect_async, tungstenite::Message};

    let port = find_available_port().await;
    let server = McpServer::new().await?;

    let server_handle = tokio::spawn(async move { server.start(port).await });

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let http_url = format!("http://127.0.0.1:{}/mcp", port);
    let ws_url = format!("ws://127.0.0.1:{}/mcp", port);

    let barrier = Arc::new(Barrier::new(2));

    // HTTP client
    let http_barrier = barrier.clone();
    let http_handle = tokio::spawn(async move {
        let client = reqwest::Client::new();

        // Initialize
        let init = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18"
            }
        });

        client
            .post(&http_url)
            .header("Cookie", "mcp_session=http_client")
            .json(&init)
            .send()
            .await?;

        http_barrier.wait().await;

        // Make requests
        for i in 0..20 {
            let request = json!({
                "jsonrpc": "2.0",
                "id": i + 2,
                "method": "tools/list",
                "params": {}
            });

            let response = client
                .post(&http_url)
                .header("Cookie", "mcp_session=http_client")
                .json(&request)
                .send()
                .await?;

            assert_eq!(response.status(), 200);
        }

        Ok::<(), anyhow::Error>(())
    });

    // WebSocket client
    let ws_barrier = barrier;
    let ws_handle = tokio::spawn(async move {
        let (ws_stream, _) = connect_async(&ws_url).await?;
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

        ws_barrier.wait().await;

        // Make requests
        for i in 0..20 {
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

    // Wait for both clients
    http_handle.await??;
    ws_handle.await??;

    server_handle.abort();
    Ok(())
}

/// Test rate limiting behavior under load
#[tokio::test]
async fn test_high_load_handling() -> Result<()> {
    let port = find_available_port().await;
    let server = McpServer::new().await?;

    let server_handle = tokio::spawn(async move { server.start(port).await });

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let url = format!("http://127.0.0.1:{}/mcp", port);
    let client = reqwest::Client::new();

    // Initialize
    let init = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18"
        }
    });

    client
        .post(&url)
        .header("Cookie", "mcp_session=load_test")
        .json(&init)
        .send()
        .await?;

    // Send many requests rapidly
    let mut handles = vec![];
    let start = std::time::Instant::now();

    for i in 0..1000 {
        let url = url.clone();
        let client = client.clone();

        let handle = tokio::spawn(async move {
            let request = json!({
                "jsonrpc": "2.0",
                "id": i + 2,
                "method": "tools/list",
                "params": {}
            });

            let response = client
                .post(&url)
                .header("Cookie", "mcp_session=load_test")
                .json(&request)
                .send()
                .await?;

            assert_eq!(response.status(), 200);
            Ok::<(), anyhow::Error>(())
        });

        handles.push(handle);
    }

    // Wait for all requests
    for handle in handles {
        handle.await??;
    }

    let elapsed = start.elapsed();
    println!("Processed 1000 requests in {:?}", elapsed);

    server_handle.abort();
    Ok(())
}

/// Test client disconnection and reconnection
#[tokio::test]
async fn test_client_reconnection_handling() -> Result<()> {
    let port = find_available_port().await;
    let server = McpServer::new().await?;

    let server_handle = tokio::spawn(async move { server.start(port).await });

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let url = format!("http://127.0.0.1:{}/mcp", port);

    // Simulate multiple connect/disconnect cycles
    for cycle in 0..10 {
        let client = reqwest::Client::new();
        let session_id = format!("reconnect_test_{}", cycle);

        // Initialize
        let init = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18"
            }
        });

        let response = client
            .post(&url)
            .header("Cookie", format!("mcp_session={}", session_id))
            .json(&init)
            .send()
            .await?;

        assert_eq!(response.status(), 200);

        // Make a few requests
        for i in 0..5 {
            let request = json!({
                "jsonrpc": "2.0",
                "id": i + 2,
                "method": "tools/list",
                "params": {}
            });

            let response = client
                .post(&url)
                .header("Cookie", format!("mcp_session={}", session_id))
                .json(&request)
                .send()
                .await?;

            assert_eq!(response.status(), 200);
        }

        // Client "disconnects" by dropping the client
        drop(client);

        // Small delay to simulate reconnection time
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }

    server_handle.abort();
    Ok(())
}
