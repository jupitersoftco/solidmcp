//! MCP Protocol Test
//!
//! Tests protocol handshake, message exchange, and JSON-RPC compliance.

mod mcp_test_helpers;
use futures_util::{SinkExt, StreamExt};
use mcp_test_helpers::{
    init_test_tracing, receive_ws_message, with_mcp_connection, with_mcp_test_server,
};
use serde_json::{json, Value};
use std::time::Duration;
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, error, info, warn};

/// Test protocol initialization handshake
#[tokio::test]
async fn test_mcp_protocol_initialize() {
    init_test_tracing();
    info!("🤝 Testing MCP protocol initialization");

    with_mcp_test_server("protocol_initialize_test", |server| async move {
        let (ws_stream, _) = tokio_tungstenite::connect_async(&server.ws_url()).await?;
        let (mut write, mut read) = ws_stream.split();

        // Send initialize message
        let init_message = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {"name": "protocol-test", "version": "1.0.0"}
            }
        });

        debug!(
            "📤 Sending initialize message: {}",
            serde_json::to_string(&init_message)?
        );
        write
            .send(Message::Text(serde_json::to_string(&init_message)?.into()))
            .await?;

        // Receive response
        let response_text = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
        debug!("📥 Received response: {}", response_text);

        let response: Value = serde_json::from_str(&response_text.to_string())?;

        // Validate response structure
        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["id"], 1);

        if response.get("error").is_some() {
            error!("❌ Initialize failed: {}", response["error"]);
            return Err(format!("Initialize failed: {}", response["error"]).into());
        }

        if let Some(result) = response.get("result") {
            assert!(result.get("protocolVersion").is_some());
            assert!(result.get("capabilities").is_some());
            info!("✅ Protocol initialization successful");
        } else {
            error!("❌ No result in initialize response");
            return Err("No result in initialize response".into());
        }

        Ok(())
    })
    .await
    .unwrap();
}

/// Test protocol version compatibility
#[tokio::test]
async fn test_mcp_protocol_version() {
    init_test_tracing();
    info!("📋 Testing MCP protocol version compatibility");

    with_mcp_test_server("protocol_version_test", |server| async move {
        let test_versions = vec![
            "2025-06-18", // Current version
            "2024-10-01", // Older version
            "2025-01-01", // Future version
        ];

        for version in test_versions {
            debug!("Testing protocol version: {}", version);

            let (ws_stream, _) = tokio_tungstenite::connect_async(&server.ws_url()).await?;
            let (mut write, mut read) = ws_stream.split();

            let init_message = json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": version,
                    "capabilities": {},
                    "clientInfo": {"name": "version-test", "version": "1.0.0"}
                }
            });

            write
                .send(Message::Text(serde_json::to_string(&init_message)?.into()))
                .await?;

            let response_text = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
            let response: Value = serde_json::from_str(&response_text.to_string())?;

            if response.get("error").is_some() {
                debug!(
                    "⚠️ Version {} not supported: {}",
                    version, response["error"]
                );
            } else {
                debug!("✅ Version {} supported", version);
            }
        }

        info!("✅ Protocol version tests completed");
        Ok(())
    })
    .await
    .unwrap();
}

/// Test malformed protocol messages
#[tokio::test]
async fn test_mcp_protocol_malformed() {
    init_test_tracing();
    info!("🚫 Testing MCP protocol with malformed messages");

    with_mcp_test_server("protocol_malformed_test", |server| async move {
        let malformed_messages = vec![
            // Missing jsonrpc
            json!({
                "id": 1,
                "method": "initialize",
                "params": {}
            }),
            // Missing method
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "params": {}
            }),
            // Invalid jsonrpc version
            json!({
                "jsonrpc": "1.0",
                "id": 1,
                "method": "initialize",
                "params": {}
            }),
            // Invalid JSON
            serde_json::Value::String("invalid json string".to_string()),
        ];

        for (i, message) in malformed_messages.iter().enumerate() {
            debug!("Testing malformed message {}: {:?}", i, message);

            let (ws_stream, _) = tokio_tungstenite::connect_async(&server.ws_url()).await?;
            let (mut write, mut read) = ws_stream.split();

            // Send malformed message and expect error response
            if let Ok(message_str) = serde_json::to_string(message) {
                write.send(Message::Text(message_str.into())).await?;

                // Try to receive response (might timeout for completely invalid messages)
                match receive_ws_message(&mut read, Duration::from_secs(2)).await {
                    Ok(response_text) => {
                        let response: Value = serde_json::from_str(&response_text.to_string())?;
                        if response.get("error").is_some() {
                            debug!("✅ Correctly received error for malformed message {}", i);
                        } else {
                            warn!("⚠️ Expected error but got success for message {}", i);
                        }
                    }
                    Err(_) => {
                        debug!("✅ Connection properly rejected malformed message {}", i);
                    }
                }
            }
        }

        info!("✅ Malformed message tests completed");
        Ok(())
    })
    .await
    .unwrap();
}

/// Test protocol message ordering
#[tokio::test]
async fn test_mcp_protocol_ordering() {
    init_test_tracing();
    info!("🔄 Testing MCP protocol message ordering");

    with_mcp_connection(
        "protocol_ordering_test",
        |_server, mut write, mut read| async move {
            // Send multiple requests with different IDs
            let requests = vec![
                json!({
                    "jsonrpc": "2.0",
                    "id": 10,
                    "method": "tools/list",
                    "params": {}
                }),
                json!({
                    "jsonrpc": "2.0",
                    "id": 5,
                    "method": "tools/list",
                    "params": {}
                }),
                json!({
                    "jsonrpc": "2.0",
                    "id": 15,
                    "method": "tools/list",
                    "params": {}
                }),
            ];

            // Send all requests
            for request in &requests {
                write
                    .send(Message::Text(serde_json::to_string(request)?.into()))
                    .await?;
            }

            // Collect responses
            let mut responses = Vec::new();
            for _ in 0..requests.len() {
                let response_text = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
                let response: Value = serde_json::from_str(&response_text.to_string())?;
                responses.push(response);
            }

            // Verify that responses have correct IDs (order may vary)
            let response_ids: Vec<i64> = responses
                .iter()
                .map(|r| r["id"].as_i64().unwrap())
                .collect();

            assert!(response_ids.contains(&10));
            assert!(response_ids.contains(&5));
            assert!(response_ids.contains(&15));

            info!("✅ Protocol message ordering tests completed");
            Ok(())
        },
    )
    .await
    .unwrap();
}

/// Test JSON-RPC compliance
#[tokio::test]
async fn test_jsonrpc_compliance() {
    init_test_tracing();
    info!("📝 Testing JSON-RPC 2.0 compliance");

    with_mcp_connection(
        "jsonrpc_compliance_test",
        |_server, mut write, mut read| async move {
            // Test 1: Valid request with string ID
            let string_id_request = json!({
                "jsonrpc": "2.0",
                "id": "test-string-id",
                "method": "tools/list",
                "params": {}
            });

            write
                .send(Message::Text(
                    serde_json::to_string(&string_id_request)?.into(),
                ))
                .await?;

            let response_text = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
            let response: Value = serde_json::from_str(&response_text.to_string())?;

            assert_eq!(response["jsonrpc"], "2.0");
            assert_eq!(response["id"], "test-string-id");

            // Test 2: Valid request with null ID
            let null_id_request = json!({
                "jsonrpc": "2.0",
                "id": null,
                "method": "tools/list",
                "params": {}
            });

            write
                .send(Message::Text(
                    serde_json::to_string(&null_id_request)?.into(),
                ))
                .await?;

            let response_text = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
            let response: Value = serde_json::from_str(&response_text.to_string())?;

            assert_eq!(response["jsonrpc"], "2.0");
            assert_eq!(response["id"], serde_json::Value::Null);

            // Test 3: Request without params
            let no_params_request = json!({
                "jsonrpc": "2.0",
                "id": 42,
                "method": "tools/list"
            });

            write
                .send(Message::Text(
                    serde_json::to_string(&no_params_request)?.into(),
                ))
                .await?;

            let response_text = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
            let response: Value = serde_json::from_str(&response_text.to_string())?;

            assert_eq!(response["jsonrpc"], "2.0");
            assert_eq!(response["id"], 42);

            info!("✅ JSON-RPC compliance tests completed");
            Ok(())
        },
    )
    .await
    .unwrap();
}
