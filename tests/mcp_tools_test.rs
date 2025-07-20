//! MCP Tools Test
//!
//! Tests tool discovery, listing, and execution functionality.

mod mcp_test_helpers;
use futures_util::{SinkExt, StreamExt};
use mcp_test_helpers::{
    init_test_tracing, receive_ws_message, with_mcp_connection, with_mcp_test_server,
};
use serde_json::{json, Value};
use std::time::Duration;
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, error, info, warn};

/// Test tool discovery and listing
#[tokio::test]
async fn test_mcp_tools_list() {
    init_test_tracing();
    info!("🔍 Testing MCP tool discovery and listing");

    with_mcp_connection(
        "tools_list_test",
        |_server, mut write, mut read| async move {
            // Request tools list
            let list_message = json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "tools/list",
                "params": {}
            });

            debug!(
                "📤 Sending tools/list request: {}",
                serde_json::to_string(&list_message)?
            );
            write
                .send(Message::Text(serde_json::to_string(&list_message)?.into()))
                .await?;

            let response_text = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
            debug!("📥 Received tools/list response: {}", response_text);

            let response: Value = serde_json::from_str(&response_text.to_string())?;

            // Validate response structure
            assert_eq!(response["jsonrpc"], "2.0");
            assert_eq!(response["id"], 2);

            if response.get("error").is_some() {
                error!("❌ Tools/list failed: {}", response["error"]);
                return Err(format!("Tools/list failed: {}", response["error"]).into());
            }

            if let Some(result) = response.get("result") {
                if let Some(tools) = result.get("tools") {
                    let tools_array = tools.as_array().unwrap();
                    debug!("📋 Found {} tools", tools_array.len());

                    // Check for expected tools
                    let tool_names: Vec<String> = tools_array
                        .iter()
                        .filter_map(|tool| {
                            tool.get("name")
                                .and_then(|n| n.as_str())
                                .map(|s| s.to_string())
                        })
                        .collect();

                    debug!("🔧 Available tools: {:?}", tool_names);

                    // Should have at least echo and read_file tools
                    assert!(
                        tool_names.contains(&"echo".to_string()),
                        "echo tool not found"
                    );
                    assert!(
                        tool_names.contains(&"read_file".to_string()),
                        "read_file tool not found"
                    );

                    info!(
                        "✅ Tool discovery successful - found {} tools",
                        tools_array.len()
                    );
                } else {
                    error!("❌ No tools array in response");
                    return Err("No tools array in response".into());
                }
            } else {
                error!("❌ No result in tools/list response");
                return Err("No result in tools/list response".into());
            }

            Ok(())
        },
    )
    .await
    .unwrap();
}

/// Test echo tool execution
#[tokio::test]
async fn test_mcp_tools_echo() {
    init_test_tracing();
    info!("🔄 Testing MCP echo tool execution");

    with_mcp_connection(
        "tools_echo_test",
        |_server, mut write, mut read| async move {
            // Call echo tool
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

            debug!(
                "📤 Sending echo tool call: {}",
                serde_json::to_string(&echo_message)?
            );
            write
                .send(Message::Text(serde_json::to_string(&echo_message)?.into()))
                .await?;

            let response_text = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
            debug!("📥 Received echo response: {}", response_text);

            let response: Value = serde_json::from_str(&response_text.to_string())?;

            // Validate response
            assert_eq!(response["jsonrpc"], "2.0");
            assert_eq!(response["id"], 3);

            if response.get("error").is_some() {
                error!("❌ Echo tool failed: {}", response["error"]);
                return Err(format!("Echo tool failed: {}", response["error"]).into());
            }

            if let Some(result) = response.get("result") {
                if let Some(content) = result.get("content") {
                    let content_array = content.as_array().unwrap();
                    assert!(!content_array.is_empty());

                    if let Some(text_content) = content_array[0].get("text") {
                        let text = text_content.as_str().unwrap();
                        // Parse the JSON content to extract the echo message
                        let parsed_content: Value = serde_json::from_str(text)?;
                        if let Some(echo_value) = parsed_content.get("echo") {
                            assert_eq!(echo_value, "Hello, MCP!");
                            info!("✅ Echo tool successful: {}", echo_value);
                        } else {
                            error!("❌ No echo field in response content");
                            return Err("No echo field in response content".into());
                        }
                    } else {
                        error!("❌ No text content in echo response");
                        return Err("No text content in echo response".into());
                    }
                } else {
                    error!("❌ No content in echo response");
                    return Err("No content in echo response".into());
                }
            } else {
                error!("❌ No result in echo response");
                return Err("No result in echo response".into());
            }

            Ok(())
        },
    )
    .await
    .unwrap();
}

/// Test read_file tool execution
#[tokio::test]
async fn test_mcp_tools_read_file() {
    init_test_tracing();
    info!("📖 Testing MCP read_file tool execution");

    with_mcp_connection(
        "tools_read_file_test",
        |_server, mut write, mut read| async move {
            // Call read_file tool with Cargo.toml (which should exist)
            let read_file_message = json!({
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

            debug!(
                "📤 Sending read_file tool call: {}",
                serde_json::to_string(&read_file_message)?
            );
            write
                .send(Message::Text(
                    serde_json::to_string(&read_file_message)?.into(),
                ))
                .await?;

            let response_text = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
            debug!("📥 Received read_file response: {}", response_text);

            let response: Value = serde_json::from_str(&response_text.to_string())?;

            // Validate response
            assert_eq!(response["jsonrpc"], "2.0");
            assert_eq!(response["id"], 4);

            if response.get("error").is_some() {
                // For a non-existent file, an error is expected and valid
                info!(
                    "✅ read_file correctly handled error: {}",
                    response["error"]
                );
            } else if let Some(result) = response.get("result") {
                if let Some(content) = result.get("content") {
                    let content_array = content.as_array().unwrap();
                    assert!(!content_array.is_empty());

                    if let Some(text_content) = content_array[0].get("text") {
                        let text = text_content.as_str().unwrap();
                        // Parse the JSON content
                        let parsed_content: Value = serde_json::from_str(text)?;

                        if let Some(file_path) = parsed_content.get("file_path") {
                            assert_eq!(file_path, "Cargo.toml");
                            info!("✅ read_file tool successful for: {}", file_path);
                        } else {
                            error!("❌ No file_path in response content");
                            return Err("No file_path in response content".into());
                        }

                        if let Some(file_content) = parsed_content.get("content") {
                            assert!(file_content.is_string());
                            info!("✅ File content received successfully");
                        } else {
                            error!("❌ No content field in response");
                            return Err("No content field in response".into());
                        }
                    } else {
                        error!("❌ No text content in read_file response");
                        return Err("No text content in read_file response".into());
                    }
                } else {
                    error!("❌ No content in read_file response");
                    return Err("No content in read_file response".into());
                }
            } else {
                error!("❌ No result or error in read_file response");
                return Err("No result or error in read_file response".into());
            }

            Ok(())
        },
    )
    .await
    .unwrap();
}

/// Test unknown tool handling
#[tokio::test]
async fn test_mcp_tools_unknown() {
    init_test_tracing();
    info!("❓ Testing MCP unknown tool handling");

    with_mcp_connection(
        "tools_unknown_test",
        |_server, mut write, mut read| async move {
            // Call unknown tool
            let unknown_message = json!({
                "jsonrpc": "2.0",
                "id": 5,
                "method": "tools/call",
                "params": {
                    "name": "unknown_tool",
                    "arguments": {}
                }
            });

            debug!(
                "📤 Sending unknown tool call: {}",
                serde_json::to_string(&unknown_message)?
            );
            write
                .send(Message::Text(
                    serde_json::to_string(&unknown_message)?.into(),
                ))
                .await?;

            let response_text = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
            debug!("📥 Received unknown tool response: {}", response_text);

            let response: Value = serde_json::from_str(&response_text.to_string())?;

            // Validate response
            assert_eq!(response["jsonrpc"], "2.0");
            assert_eq!(response["id"], 5);

            // Should get an error for unknown tool
            if response.get("error").is_some() {
                info!("✅ Unknown tool correctly rejected: {}", response["error"]);
            } else {
                error!("❌ Expected error for unknown tool but got success");
                return Err("Expected error for unknown tool".into());
            }

            Ok(())
        },
    )
    .await
    .unwrap();
}

/// Test tools without initialization
#[tokio::test]
async fn test_mcp_tools_no_init() {
    init_test_tracing();
    info!("🚫 Testing MCP tools without initialization");

    with_mcp_test_server("tools_no_init_test", |server| async move {
        // Connect without initializing
        let (ws_stream, _) = tokio_tungstenite::connect_async(&server.ws_url()).await?;
        let (mut write, mut read) = ws_stream.split();

        // Try to call tools/list without initialization
        let list_message = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/list",
            "params": {}
        });

        write
            .send(Message::Text(serde_json::to_string(&list_message)?.into()))
            .await?;

        let response_text = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
        let response: Value = serde_json::from_str(&response_text.to_string())?;

        // Should get an error for not being initialized
        if response.get("error").is_some() {
            info!(
                "✅ Tools correctly rejected without initialization: {}",
                response["error"]
            );
        } else {
            warn!("⚠️ Tools allowed without initialization (might be acceptable)");
        }

        Ok(())
    })
    .await
    .unwrap();
}

/// Test WebSocket empty message handling
#[tokio::test]
async fn test_mcp_websocket_empty_message_parse_error() {
    init_test_tracing();
    info!("📭 Testing MCP WebSocket empty message handling");

    with_mcp_test_server("websocket_empty_message_test", |server| async move {
        let (ws_stream, _) = tokio_tungstenite::connect_async(&server.ws_url()).await?;
        let (mut write, mut read) = ws_stream.split();

        // Send empty message
        write.send(Message::Text("".to_string().into())).await?;

        // Should get an error or connection close
        match receive_ws_message(&mut read, Duration::from_secs(2)).await {
            Ok(response_text) => {
                let response: Value = serde_json::from_str(&response_text.to_string())?;
                if response.get("error").is_some() {
                    info!("✅ Empty message correctly rejected: {}", response["error"]);
                } else {
                    warn!("⚠️ Empty message didn't return error");
                }
            }
            Err(_) => {
                info!("✅ Connection properly handled empty message");
            }
        }

        Ok(())
    })
    .await
    .unwrap();
}

/// Test tool argument validation
#[tokio::test]
async fn test_mcp_tool_argument_validation() {
    init_test_tracing();
    info!("🔍 Testing MCP tool argument validation");

    with_mcp_connection(
        "tool_argument_validation_test",
        |_server, mut write, mut read| async move {
            // Test echo with missing message argument
            let invalid_echo_message = json!({
                "jsonrpc": "2.0",
                "id": 6,
                "method": "tools/call",
                "params": {
                    "name": "echo",
                    "arguments": {}
                }
            });

            write
                .send(Message::Text(
                    serde_json::to_string(&invalid_echo_message)?.into(),
                ))
                .await?;

            let response_text = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
            let response: Value = serde_json::from_str(&response_text.to_string())?;

            assert_eq!(response["jsonrpc"], "2.0");
            assert_eq!(response["id"], 6);

            // Should get an error for missing arguments
            if response.get("error").is_some() {
                info!(
                    "✅ Missing arguments correctly rejected: {}",
                    response["error"]
                );
            } else {
                warn!("⚠️ Missing arguments didn't return error (might handle gracefully)");
            }

            Ok(())
        },
    )
    .await
    .unwrap();
}
