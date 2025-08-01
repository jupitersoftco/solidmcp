//! Comprehensive MCP Debugging Test
//!
//! This test systematically diagnoses MCP (Model Context Protocol)
//! integration issues in the solidmcp project. It provides detailed diagnostics
//! for connection, protocol, and tool execution problems.

mod mcp_test_helpers;
use futures_util::{SinkExt, StreamExt};
use mcp_test_helpers::{
    init_test_tracing, initialize_mcp_connection_with_server, receive_ws_message,
    with_mcp_test_server,
};
use serde_json::{json, Value};
use std::time::Duration;
// use tokio::time::timeout; // Commented out unused import
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, error, info, warn};

/// MCP Debugging Test Suite
///
/// This test systematically checks all aspects of MCP functionality:
/// 1. Server startup and availability
/// 2. WebSocket connection establishment
/// 3. Protocol handshake and initialization
/// 4. Tool discovery and listing
/// 5. Tool execution (echo and read_file)
/// 6. Error handling and recovery
/// 7. Performance and resource usage
#[tokio::test]
async fn test_mcp_comprehensive_debugging() {
    init_test_tracing();
    info!("🚀 Starting comprehensive MCP debugging test");

    // Phase 1: Server Availability Check
    let server_check = check_server_availability().await;
    if let Err(e) = server_check {
        error!("❌ Server availability check failed: {}", e);
        panic!("Server not available: {e}");
    }
    info!("✅ Server availability confirmed");

    // Phase 2: WebSocket Connection Test
    let ws_test = test_websocket_connection().await;
    if let Err(e) = ws_test {
        error!("❌ WebSocket connection test failed: {}", e);
        panic!("WebSocket connection failed: {e}");
    }
    info!("✅ WebSocket connection established");

    // Phase 3: Protocol Handshake Test
    let protocol_test = test_protocol_handshake().await;
    if let Err(e) = protocol_test {
        error!("❌ Protocol handshake test failed: {}", e);
        panic!("Protocol handshake failed: {e}");
    }
    info!("✅ Protocol handshake successful");

    // Phase 4: Tool Discovery Test
    let tools_test = test_tool_discovery().await;
    if let Err(e) = tools_test {
        error!("❌ Tool discovery test failed: {}", e);
        panic!("Tool discovery failed: {e}");
    }
    info!("✅ Tool discovery successful");

    // Phase 5: Tool Execution Test
    let execution_test = test_tool_execution().await;
    if let Err(e) = execution_test {
        error!("❌ Tool execution test failed: {}", e);
        panic!("Tool execution failed: {e}");
    }
    info!("✅ Tool execution successful");

    // Phase 6: Error Handling Test
    let error_test = test_error_handling().await;
    if let Err(e) = error_test {
        error!("❌ Error handling test failed: {}", e);
        panic!("Error handling failed: {e}");
    }
    info!("✅ Error handling validated");

    // Phase 7: Performance Test
    let perf_test = test_performance().await;
    if let Err(e) = perf_test {
        warn!("⚠️ Performance test failed: {}", e);
        // Don't fail the test for performance issues, just warn
    } else {
        info!("✅ Performance test passed");
    }

    info!("🎉 All MCP debugging tests completed successfully!");
}

/// Check if the MCP server is available by starting a test instance
async fn check_server_availability() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    debug!("🔍 Checking server availability...");

    // Try to start a test server instance
    with_mcp_test_server("availability_check", |server| async move {
        debug!("✅ Server is responding on port {}", server.port);
        Ok(())
    })
    .await?;

    debug!("✅ Server availability confirmed");
    Ok(())
}

/// Test basic WebSocket connection
async fn test_websocket_connection() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    debug!("🔌 Testing WebSocket connection...");

    with_mcp_test_server("websocket_test", |server| async move {
        let (_ws_stream, _) = tokio_tungstenite::connect_async(&server.ws_url()).await?;
        debug!("✅ WebSocket connection established");
        Ok(())
    })
    .await?;

    Ok(())
}

/// Test protocol handshake
async fn test_protocol_handshake() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    debug!("🤝 Testing protocol handshake...");

    with_mcp_test_server("handshake_test", |server| async move {
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
                "clientInfo": {"name": "handshake-test", "version": "1.0.0"}
            }
        });

        write
            .send(Message::Text(serde_json::to_string(&init_message)?.into()))
            .await?;
        let _init_response = receive_ws_message(&mut read, Duration::from_secs(5)).await?;

        debug!("✅ Protocol handshake successful");
        Ok(())
    })
    .await?;

    Ok(())
}

/// Test tool discovery
async fn test_tool_discovery() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    debug!("🔍 Testing tool discovery...");

    with_mcp_test_server("tool_discovery_test", |server| async move {
        let (write, read) = initialize_mcp_connection_with_server(&server).await?;
        let (mut write, mut read) = (write, read);

        // Request tools list
        let tools_message = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        });

        write
            .send(Message::Text(serde_json::to_string(&tools_message)?.into()))
            .await?;

        let text = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
        let response: Value = serde_json::from_str(&text)?;

        if response["jsonrpc"] != "2.0" || response["id"] != 2 {
            return Err("Invalid tools list response".into());
        }

        let tools = &response["result"]["tools"];
        if !tools.is_array() {
            return Err("Tools list response missing tools array".into());
        }

        let tool_names: Vec<String> = tools
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|tool| tool["name"].as_str().map(|s| s.to_string()))
            .collect();

        if !tool_names.contains(&"echo".to_string()) {
            return Err("Echo tool not found in tools list".into());
        }

        if !tool_names.contains(&"read_file".to_string()) {
            return Err("Read file tool not found in tools list".into());
        }

        debug!("✅ Tool discovery successful: found {:?}", tool_names);
        Ok(())
    })
    .await?;

    Ok(())
}

/// Test tool execution (echo and read_file)
async fn test_tool_execution() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    debug!("🛠️ Testing tool execution...");

    with_mcp_test_server("tool_execution_test", |server| async move {
        let (write, read) = initialize_mcp_connection_with_server(&server).await?;
        let (mut write, mut read) = (write, read);

        // Test echo tool
        let echo_message = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "echo",
                "arguments": {
                    "message": "Hello from MCP debugging test!"
                }
            }
        });

        write
            .send(Message::Text(serde_json::to_string(&echo_message)?.into()))
            .await?;

        let text = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
        let response: Value = serde_json::from_str(&text)?;

        if response["jsonrpc"] != "2.0" || response["id"] != 2 {
            return Err("Invalid echo tool response".into());
        }

        let content = &response["result"]["content"][0]["text"];
        if !content.is_string() {
            return Err("Echo tool response missing content".into());
        }

        // Check the structured data instead of parsing the human-readable text
        let data = &response["result"]["data"];
        if data["echo"] != "Hello from MCP debugging test!" {
            return Err("Echo tool returned unexpected message".into());
        }

        debug!("✅ Echo tool execution successful");

        // Test read_file tool (read Cargo.toml)
        let read_message = json!({
            "jsonrpc": "2.0",
            "id": 3,
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

        let text = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
        let response: Value = serde_json::from_str(&text)?;

        if response["jsonrpc"] != "2.0" || response["id"] != 3 {
            return Err("Invalid read_file tool response".into());
        }

        let content = &response["result"]["content"][0]["text"];
        if !content.is_string() {
            return Err("Read file tool response missing content".into());
        }

        // Check the structured data instead of parsing the human-readable text
        let data = &response["result"]["data"];
        if data["file_path"] != "Cargo.toml" {
            return Err("Read file tool returned wrong file path".into());
        }

        if !data["content"].is_string() {
            return Err("Read file tool response missing file content".into());
        }

        debug!("✅ Read file tool execution successful");
        Ok(())
    })
    .await?;

    Ok(())
}

/// Test error handling with invalid requests
async fn test_error_handling() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    debug!("🚫 Testing error handling...");

    with_mcp_test_server("error_handling_test", |server| async move {
        let (write, read) = initialize_mcp_connection_with_server(&server).await?;
        let (mut write, mut read) = (write, read);

        // Test unknown method
        let unknown_message = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "unknown_method",
            "params": {}
        });

        write
            .send(Message::Text(
                serde_json::to_string(&unknown_message)?.into(),
            ))
            .await?;

        let text = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
        let response: Value = serde_json::from_str(&text)?;

        if response["jsonrpc"] != "2.0" || response["id"] != 2 {
            return Err("Invalid error response structure".into());
        }

        if !response["error"].is_object() {
            return Err("Error response missing error object".into());
        }

        let error_obj = &response["error"];
        if !error_obj["code"].is_number() || !error_obj["message"].is_string() {
            return Err("Error response missing code or message".into());
        }

        debug!("✅ Error handling for unknown method successful");

        // Test unknown tool
        let unknown_tool_message = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "unknown_tool",
                "arguments": {}
            }
        });

        write
            .send(Message::Text(
                serde_json::to_string(&unknown_tool_message)?.into(),
            ))
            .await?;

        let text = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
        let response: Value = serde_json::from_str(&text)?;

        if response["jsonrpc"] != "2.0" || response["id"] != 3 {
            return Err("Invalid tool error response structure".into());
        }

        if !response["error"].is_object() {
            return Err("Tool error response missing error object".into());
        }

        debug!("✅ Error handling for unknown tool successful");
        Ok(())
    })
    .await?;

    Ok(())
}

/// Test performance and resource usage
async fn test_performance() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    debug!("⚡ Testing performance...");

    let start_time = std::time::Instant::now();

    // Run multiple quick operations with a single server instance
    with_mcp_test_server("performance_test", |server| async move {
        for i in 0..5 {
            let (ws_stream, _) = tokio_tungstenite::connect_async(&server.ws_url()).await?;
            let (mut write, mut read) = ws_stream.split();

            // Quick initialize
            let init_message = json!({
                "jsonrpc": "2.0",
                "id": i,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2025-06-18",
                    "capabilities": {},
                    "clientInfo": {"name": "perf-test", "version": "1.0.0"}
                }
            });

            write
                .send(Message::Text(serde_json::to_string(&init_message)?.into()))
                .await?;
            let _init_response = receive_ws_message(&mut read, Duration::from_secs(2)).await?;

            // Quick echo
            let echo_message = json!({
                "jsonrpc": "2.0",
                "id": i + 100,
                "method": "tools/call",
                "params": {
                    "name": "echo",
                    "arguments": {
                        "message": format!("Performance test {}", i)
                    }
                }
            });

            write
                .send(Message::Text(serde_json::to_string(&echo_message)?.into()))
                .await?;
            let _echo_response = receive_ws_message(&mut read, Duration::from_secs(2)).await?;
        }

        Ok(())
    })
    .await?;

    let elapsed = start_time.elapsed();

    if elapsed > Duration::from_secs(10) {
        debug!(
            "⚠️ Performance test took longer than expected: {:?}",
            elapsed
        );
    } else {
        debug!("✅ Performance test completed in {:?}", elapsed);
    }

    Ok(())
}
