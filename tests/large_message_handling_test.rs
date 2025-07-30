//! Large Message Handling Test
//!
//! Tests that the server properly handles large messages

use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use std::time::Duration;
use tokio_tungstenite::tungstenite::Message;

mod mcp_test_helpers;
use mcp_test_helpers::{receive_ws_message, with_mcp_test_server};

const KB: usize = 1024;
const MB: usize = 1024 * KB;

#[tokio::test]
async fn test_large_websocket_message() {
    // Test WebSocket handling of large messages
    with_mcp_test_server("large_ws_message", |server| async move {
        let (ws_stream, _) = tokio_tungstenite::connect_async(&server.ws_url()).await?;
        let (mut write, mut read) = ws_stream.split();

        // Initialize
        let init = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {}
            }
        });

        write
            .send(Message::Text(serde_json::to_string(&init)?.into()))
            .await?;
        let _response = receive_ws_message(&mut read, Duration::from_secs(5)).await?;

        // Test various message sizes
        let test_sizes = vec![
            (10 * KB, "10KB"),
            (100 * KB, "100KB"),
            (500 * KB, "500KB"),
            (1 * MB, "1MB"),
        ];

        for (size, label) in test_sizes {
            let large_text = "x".repeat(size);
            let echo_message = json!({
                "jsonrpc": "2.0",
                "id": 10 + size,
                "method": "tools/call",
                "params": {
                    "name": "echo",
                    "arguments": {
                        "message": large_text
                    }
                }
            });

            write
                .send(Message::Text(serde_json::to_string(&echo_message)?.into()))
                .await?;

            let response = receive_ws_message(&mut read, Duration::from_secs(10)).await?;
            let parsed: serde_json::Value = serde_json::from_str(&response)?;

            assert!(
                parsed.get("result").is_some() || parsed.get("error").is_some(),
                "Failed to handle {} message",
                label
            );

            println!("✅ Successfully handled {} message", label);
        }

        Ok(())
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn test_large_http_request() {
    // Test HTTP handling of large requests
    with_mcp_test_server("large_http_request", |server| async move {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;

        // Initialize
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

        // Test various payload sizes
        let test_sizes = vec![
            (10 * KB, "10KB"),
            (100 * KB, "100KB"),
            (500 * KB, "500KB"),
            (1 * MB, "1MB"),
            (2 * MB, "2MB"),
        ];

        for (size, label) in test_sizes {
            let large_text = "y".repeat(size);
            let tool_call = json!({
                "jsonrpc": "2.0",
                "id": 100 + size,
                "method": "tools/call",
                "params": {
                    "name": "echo",
                    "arguments": {
                        "message": large_text
                    }
                }
            });

            let response = client
                .post(&server.http_url())
                .json(&tool_call)
                .send()
                .await?;

            assert_eq!(response.status(), 200, "Failed on {} request", label);

            // Read response body
            let body: serde_json::Value = response.json().await?;
            assert!(
                body.get("result").is_some() || body.get("error").is_some(),
                "Invalid response for {} request",
                label
            );

            println!("✅ Successfully handled {} HTTP request", label);
        }

        Ok(())
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn test_message_size_limits() {
    // Test that extremely large messages are rejected gracefully
    with_mcp_test_server("message_size_limits", |server| async move {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()?;

        // Initialize
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

        // Test message that's too large (10MB)
        let huge_text = "z".repeat(10 * MB);
        let huge_request = json!({
            "jsonrpc": "2.0",
            "id": 999,
            "method": "tools/call",
            "params": {
                "name": "echo",
                "arguments": {
                    "message": huge_text
                }
            }
        });

        let result = client
            .post(&server.http_url())
            .json(&huge_request)
            .send()
            .await;

        // Should either succeed or fail gracefully (not panic)
        match result {
            Ok(response) => {
                println!(
                    "Server accepted 10MB message with status: {}",
                    response.status()
                );
                assert!(response.status() == 200 || response.status() == 413);
            }
            Err(e) => {
                println!("Client rejected 10MB message: {}", e);
                // This is also acceptable - client-side rejection
            }
        }

        Ok(())
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn test_chunked_large_response() {
    // Test that large responses with progress tokens use chunked encoding
    with_mcp_test_server("chunked_large_response", |server| async move {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;

        // Initialize
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

        // Large message with progress token
        let large_text = "w".repeat(500 * KB);
        let request = json!({
            "jsonrpc": "2.0",
            "id": 1000,
            "method": "tools/call",
            "params": {
                "name": "echo",
                "arguments": {
                    "message": large_text
                },
                "_meta": {
                    "progressToken": "large-response-token"
                }
            }
        });

        let response = client
            .post(&server.http_url())
            .json(&request)
            .send()
            .await?;

        assert_eq!(response.status(), 200);

        // Should use chunked encoding for large response with progress token
        assert_eq!(
            response
                .headers()
                .get("transfer-encoding")
                .map(|v| v.to_str().unwrap()),
            Some("chunked")
        );

        // Consume the response
        let _body: serde_json::Value = response.json().await?;

        println!("✅ Large response with progress token uses chunked encoding");

        Ok(())
    })
    .await
    .unwrap();
}
