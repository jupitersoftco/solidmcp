//! Race Condition Test
//!
//! Tests for race conditions in session management

use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Barrier;
use tokio_tungstenite::tungstenite::Message;

mod mcp_test_helpers;
use mcp_test_helpers::{receive_ws_message, with_mcp_test_server};

#[tokio::test]
async fn test_concurrent_initialization_race() {
    // Test that concurrent initialization attempts don't cause race conditions
    with_mcp_test_server("concurrent_init_race", |server| async move {
        let barrier = Arc::new(Barrier::new(3)); // 3 concurrent connections
        let mut handles = vec![];

        for i in 0..3 {
            let ws_url = server.ws_url();
            let barrier = barrier.clone();

            let handle = tokio::spawn(async move {
                let (ws_stream, _) = tokio_tungstenite::connect_async(&ws_url).await.unwrap();
                let (mut write, mut read) = ws_stream.split();

                // Wait for all connections to be ready
                barrier.wait().await;

                // All send initialization at the same time
                let init = json!({
                    "jsonrpc": "2.0",
                    "id": i + 1,
                    "method": "initialize",
                    "params": {
                        "protocolVersion": "2025-06-18",
                        "capabilities": {},
                        "clientInfo": {"name": format!("client-{}", i), "version": "1.0"}
                    }
                });

                write
                    .send(Message::Text(serde_json::to_string(&init).unwrap().into()))
                    .await
                    .unwrap();

                let response = receive_ws_message(&mut read, Duration::from_secs(5))
                    .await
                    .unwrap();
                let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();

                // All should succeed without race conditions
                assert!(parsed.get("result").is_some(), "Client {} init failed", i);

                // Test concurrent tool calls
                let tools = json!({
                    "jsonrpc": "2.0",
                    "id": i + 10,
                    "method": "tools/list"
                });

                write
                    .send(Message::Text(serde_json::to_string(&tools).unwrap().into()))
                    .await
                    .unwrap();
                let response = receive_ws_message(&mut read, Duration::from_secs(5))
                    .await
                    .unwrap();
                let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();

                assert!(
                    parsed.get("result").is_some(),
                    "Client {} tools/list failed",
                    i
                );
            });

            handles.push(handle);
        }

        // Wait for all to complete
        for handle in handles {
            handle.await.unwrap();
        }

        Ok(())
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn test_session_state_isolation() {
    // Test that session states don't interfere with each other
    with_mcp_test_server("session_isolation_race", |server| async move {
        // Create two connections
        let (ws1, _) = tokio_tungstenite::connect_async(&server.ws_url()).await?;
        let (mut write1, mut read1) = ws1.split();

        let (ws2, _) = tokio_tungstenite::connect_async(&server.ws_url()).await?;
        let (mut write2, mut read2) = ws2.split();

        // Initialize first with one version
        let init1 = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {"name": "client1", "version": "1.0"}
            }
        });

        write1
            .send(Message::Text(serde_json::to_string(&init1)?.into()))
            .await?;
        let response1 = receive_ws_message(&mut read1, Duration::from_secs(2)).await?;
        let parsed1: serde_json::Value = serde_json::from_str(&response1)?;
        assert_eq!(parsed1["result"]["protocolVersion"], "2025-06-18");

        // Initialize second with different version
        let init2 = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-03-26",
                "capabilities": {},
                "clientInfo": {"name": "client2", "version": "1.0"}
            }
        });

        write2
            .send(Message::Text(serde_json::to_string(&init2)?.into()))
            .await?;
        let response2 = receive_ws_message(&mut read2, Duration::from_secs(2)).await?;
        let parsed2: serde_json::Value = serde_json::from_str(&response2)?;
        assert_eq!(parsed2["result"]["protocolVersion"], "2025-03-26");

        // Verify sessions are isolated - send tools/list to both
        let tools = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list"
        });

        write1
            .send(Message::Text(serde_json::to_string(&tools)?.into()))
            .await?;
        write2
            .send(Message::Text(serde_json::to_string(&tools)?.into()))
            .await?;

        let _resp1 = receive_ws_message(&mut read1, Duration::from_secs(2)).await?;
        let _resp2 = receive_ws_message(&mut read2, Duration::from_secs(2)).await?;

        // Both should succeed independently
        Ok(())
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn test_http_session_mutex_contention() {
    // Test that HTTP sessions don't have mutex contention issues
    with_mcp_test_server("http_mutex_contention", |server| async move {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .cookie_store(true)
            .build()?;

        // Initialize session
        let init = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {}
            }
        });

        client.post(&server.http_url()).json(&init).send().await?;

        // Send multiple concurrent requests on same session
        let mut handles = vec![];

        for i in 0..10 {
            let client = client.clone();
            let url = server.http_url();

            let handle = tokio::spawn(async move {
                let request = json!({
                    "jsonrpc": "2.0",
                    "id": i + 10,
                    "method": "tools/list"
                });

                let response = client.post(&url).json(&request).send().await.unwrap();
                assert_eq!(response.status(), 200);

                let body: serde_json::Value = response.json().await.unwrap();
                assert!(body.get("result").is_some());
            });

            handles.push(handle);
        }

        // Wait for all requests
        for handle in handles {
            handle.await?;
        }

        Ok(())
    })
    .await
    .unwrap();
}
