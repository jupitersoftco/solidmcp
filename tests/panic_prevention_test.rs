//! Panic Prevention Test
//!
//! Tests that the server handles malformed input gracefully without panicking

use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use std::time::Duration;
use tokio_tungstenite::tungstenite::Message;

mod mcp_test_helpers;
use mcp_test_helpers::{receive_ws_message, with_mcp_test_server};

#[tokio::test]
async fn test_malformed_json_no_panic() {
    // Test that malformed JSON doesn't cause panic
    with_mcp_test_server("malformed_json_test", |server| async move {
        let (ws_stream, _) = tokio_tungstenite::connect_async(&server.ws_url()).await?;
        let (mut write, mut read) = ws_stream.split();

        // Send malformed JSON
        let malformed_messages = vec![
            "{invalid json}",
            "{'single': 'quotes'}",
            "{\"unterminated\": ",
            "null",
            "[]",
            "42",
        ];

        for msg in malformed_messages {
            write.send(Message::Text(msg.to_string().into())).await?;

            let response = receive_ws_message(&mut read, Duration::from_secs(2)).await?;
            let parsed: serde_json::Value = serde_json::from_str(&response)?;

            // Should get error response, not panic
            assert!(parsed.get("error").is_some());
            let error_code = parsed["error"]["code"].as_i64().unwrap();
            // Accept both parse error (-32700) and internal error (-32603)
            assert!(error_code == -32700 || error_code == -32603);
        }

        Ok(())
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn test_missing_fields_no_panic() {
    // Test that missing required fields don't cause panic
    with_mcp_test_server("missing_fields_test", |server| async move {
        let (ws_stream, _) = tokio_tungstenite::connect_async(&server.ws_url()).await?;
        let (mut write, mut read) = ws_stream.split();

        // Messages with missing required fields
        let incomplete_messages = vec![
            json!({}),                                                       // Empty object
            json!({"jsonrpc": "2.0"}),                                       // Missing method
            json!({"method": "initialize"}),                                 // Missing jsonrpc
            json!({"jsonrpc": "2.0", "method": "tools/call"}), // Missing params for tools/call
            json!({"jsonrpc": "2.0", "method": "tools/call", "params": {}}), // Missing name in params
        ];

        for msg in incomplete_messages {
            write
                .send(Message::Text(serde_json::to_string(&msg)?.into()))
                .await?;

            let response = receive_ws_message(&mut read, Duration::from_secs(2)).await?;
            let parsed: serde_json::Value = serde_json::from_str(&response)?;

            // Should get error response, not panic
            assert!(parsed.get("error").is_some());
            let error_code = parsed["error"]["code"].as_i64().unwrap();
            // Accept various error codes: invalid request, invalid params, internal error, or not initialized
            assert!(
                error_code == -32600
                    || error_code == -32602
                    || error_code == -32603
                    || error_code == -32002,
                "Unexpected error code: {}",
                error_code
            );
        }

        Ok(())
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn test_tools_list_response_no_panic() {
    // Test that tools/list response handling doesn't panic
    with_mcp_test_server("tools_list_panic_test", |server| async move {
        let (ws_stream, _) = tokio_tungstenite::connect_async(&server.ws_url()).await?;
        let (mut write, mut read) = ws_stream.split();

        // Initialize first
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
        let _response = receive_ws_message(&mut read, Duration::from_secs(2)).await?;

        // Request tools list
        let tools_list = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list"
        });

        write
            .send(Message::Text(serde_json::to_string(&tools_list)?.into()))
            .await?;
        let response = receive_ws_message(&mut read, Duration::from_secs(2)).await?;
        let parsed: serde_json::Value = serde_json::from_str(&response)?;

        // Should have result with tools array, not panic
        assert!(parsed.get("result").is_some());
        let result = &parsed["result"];
        assert!(result.get("tools").is_some());

        // This line was using unwrap() - verify it doesn't panic
        let tools = result["tools"].as_array();
        assert!(tools.is_some());

        Ok(())
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn test_websocket_binary_message_no_panic() {
    // Test that binary WebSocket messages don't cause panic
    with_mcp_test_server("binary_message_test", |server| async move {
        let (ws_stream, _) = tokio_tungstenite::connect_async(&server.ws_url()).await?;
        let (mut write, mut read) = ws_stream.split();

        // Send binary message
        write
            .send(Message::Binary(vec![0, 1, 2, 3, 4].into()))
            .await?;

        // Should handle gracefully, not panic
        // The server logs it but doesn't respond to binary messages
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Send a valid message to ensure connection is still alive
        let ping = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {}
            }
        });

        write
            .send(Message::Text(serde_json::to_string(&ping)?.into()))
            .await?;
        let response = receive_ws_message(&mut read, Duration::from_secs(2)).await?;
        let parsed: serde_json::Value = serde_json::from_str(&response)?;

        // Should get valid response, proving server didn't panic
        assert!(parsed.get("result").is_some());

        Ok(())
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn test_http_large_json_no_panic() {
    // Test that large JSON responses don't cause panic
    with_mcp_test_server("large_json_test", |server| async move {
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

        let response = client.post(&server.http_url()).json(&init).send().await?;

        assert_eq!(response.status(), 200);

        // Create a tool call with very large message
        let large_message = "x".repeat(100_000); // 100KB
        let tool_call = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "echo",
                "arguments": {
                    "message": large_message
                }
            }
        });

        let response = client
            .post(&server.http_url())
            .json(&tool_call)
            .send()
            .await?;

        assert_eq!(response.status(), 200);

        // Should handle large response without panic
        let body: serde_json::Value = response.json().await?;
        assert!(body.get("result").is_some() || body.get("error").is_some());

        Ok(())
    })
    .await
    .unwrap();
}
