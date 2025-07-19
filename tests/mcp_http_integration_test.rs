//! MCP HTTP Integration Test
//!
//! Tests both WebSocket and HTTP MCP endpoints to ensure they work together.

mod mcp_test_helpers;
use futures_util::{SinkExt, StreamExt};
use mcp_test_helpers::{init_test_tracing, receive_ws_message, with_mcp_test_server};
use serde_json::{json, Value};
use std::time::Duration;
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, info};

/// Test HTTP MCP endpoint with basic initialize and tools/list
#[tokio::test]
async fn test_mcp_http_integration() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_test_tracing();
    info!("🌐 Testing MCP HTTP integration");

    with_mcp_test_server("http_integration_test", |server| async move {
        let base_url = server.http_url();
        info!("🚀 Testing HTTP endpoint at {}", base_url);

        // Test HTTP initialize
        let client = reqwest::ClientBuilder::new().cookie_store(true).build()?;

        let init_message = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {"name": "http-test", "version": "1.0.0"}
            }
        });

        debug!("📤 Sending HTTP initialize request to {}", base_url);
        let response = client
            .post(format!("{base_url}/mcp"))
            .json(&init_message)
            .send()
            .await?;

        // Print response headers for debugging
        println!("HTTP initialize response headers: {:?}", response.headers());
        println!("HTTP initialize response status: {}", response.status());

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await?;
            println!("HTTP initialize error response: {error_text}");
            return Err(format!("HTTP request failed with status {status}: {error_text}").into());
        }

        let response_json: Value = response.json().await?;

        debug!("📥 HTTP initialize response: {:?}", response_json);

        assert_eq!(response_json["jsonrpc"], "2.0");
        assert_eq!(response_json["id"], 1);
        assert!(response_json.get("result").is_some());
        assert!(response_json["result"]["protocolVersion"]
            .as_str()
            .is_some());

        info!("✅ HTTP initialize successful");

        // Test HTTP tools/list
        let tools_message = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        });

        debug!("📤 Sending HTTP tools/list request");
        let response = client
            .post(format!("{base_url}/mcp"))
            .json(&tools_message)
            .send()
            .await?;

        assert!(response.status().is_success());
        let response_json: Value = response.json().await?;

        debug!("📥 HTTP tools/list response: {:?}", response_json);

        assert_eq!(response_json["jsonrpc"], "2.0");
        assert_eq!(response_json["id"], 2);
        assert!(response_json.get("result").is_some());
        assert!(response_json["result"]["tools"].is_array());

        let tools = response_json["result"]["tools"].as_array().unwrap();
        assert!(!tools.is_empty());

        // Check that we have the expected tools
        let tool_names: Vec<_> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();

        assert!(tool_names.contains(&"echo"));
        assert!(tool_names.contains(&"read_file"));

        info!("✅ HTTP tools/list successful");

        // Test HTTP tool call
        let echo_message = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "echo",
                "arguments": {
                    "message": "Hello from HTTP!"
                }
            }
        });

        debug!("📤 Sending HTTP echo tool call");
        let response = client
            .post(format!("{base_url}/mcp"))
            .json(&echo_message)
            .send()
            .await?;

        assert!(response.status().is_success());
        let response_json: Value = response.json().await?;

        debug!("📥 HTTP echo response: {:?}", response_json);

        assert_eq!(response_json["jsonrpc"], "2.0");
        assert_eq!(response_json["id"], 3);
        assert!(response_json.get("result").is_some());
        assert!(response_json["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("Hello from HTTP!"));

        info!("✅ HTTP tool call successful");

        Ok(())
    })
    .await?;

    Ok(())
}

/// Test that WebSocket still works alongside HTTP
#[tokio::test]
async fn test_mcp_websocket_still_works() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_test_tracing();
    info!("🔌 Testing that WebSocket still works alongside HTTP");

    with_mcp_test_server("websocket_still_works_test", |server| async move {
        let ws_url = server.ws_url();
        info!("🚀 Testing WebSocket at {}", ws_url);

        let (ws_stream, _) = tokio_tungstenite::connect_async(&ws_url).await?;
        let (mut write, mut read) = ws_stream.split();

        // Send initialize message
        let init_message = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {"name": "ws-test", "version": "1.0.0"}
            }
        });

        debug!("📤 Sending WebSocket initialize message");
        write
            .send(Message::Text(serde_json::to_string(&init_message)?))
            .await?;

        // Receive response
        let response_text = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
        debug!("📥 WebSocket initialize response: {}", response_text);

        let response: Value = serde_json::from_str(&response_text)?;
        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["id"], 1);
        assert!(response.get("result").is_some());

        info!("✅ WebSocket initialize successful");

        // Test tools/list via WebSocket
        let tools_message = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        });

        debug!("📤 Sending WebSocket tools/list message");
        write
            .send(Message::Text(serde_json::to_string(&tools_message)?))
            .await?;

        let response_text = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
        debug!("📥 WebSocket tools/list response: {}", response_text);

        let response: Value = serde_json::from_str(&response_text)?;
        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["id"], 2);
        assert!(response.get("result").is_some());
        assert!(response["result"]["tools"].is_array());

        let tools = response["result"]["tools"].as_array().unwrap();
        assert!(!tools.is_empty());

        // Check that we have the expected tools
        let tool_names: Vec<_> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
        assert!(tool_names.contains(&"echo"));
        assert!(tool_names.contains(&"read_file"));

        info!("✅ WebSocket tools/list successful");

        // Test tool call via WebSocket
        let echo_message = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "echo",
                "arguments": {
                    "message": "Hello from WebSocket!"
                }
            }
        });

        debug!("📤 Sending WebSocket echo tool call");
        write
            .send(Message::Text(serde_json::to_string(&echo_message)?))
            .await?;

        let response_text = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
        debug!("📥 WebSocket echo response: {}", response_text);

        let response: Value = serde_json::from_str(&response_text)?;
        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["id"], 3);
        assert!(response.get("result").is_some());
        assert!(response["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("Hello from WebSocket!"));

        info!("✅ WebSocket tool call successful");

        Ok(())
    })
    .await?;

    Ok(())
}

/// Test that HTTP and WebSocket can work together on the same server
#[tokio::test]
async fn test_mcp_dual_transport() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_test_tracing();
    info!("🔄 Testing dual transport (HTTP + WebSocket) on same server");

    with_mcp_test_server("dual_transport_test", |server| async move {
        let base_url = server.http_url();
        let ws_url = server.ws_url();

        info!(
            "🚀 Testing dual transport - HTTP: {}, WebSocket: {}",
            base_url, ws_url
        );

        // Test HTTP initialize
        let client = reqwest::ClientBuilder::new().cookie_store(true).build()?;

        let init_message = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {"name": "dual-test", "version": "1.0.0"}
            }
        });

        debug!("📤 Sending HTTP initialize request");
        let response = client
            .post(format!("{base_url}/mcp"))
            .json(&init_message)
            .send()
            .await?;

        assert!(response.status().is_success());
        let response_json: Value = response.json().await?;
        assert_eq!(response_json["jsonrpc"], "2.0");
        assert_eq!(response_json["id"], 1);

        info!("✅ HTTP initialize successful");

        // Test WebSocket on same server
        let (ws_stream, _) = tokio_tungstenite::connect_async(&ws_url).await?;
        let (mut write, mut read) = ws_stream.split();

        let ws_init_message = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {"name": "dual-test-ws", "version": "1.0.0"}
            }
        });

        debug!("📤 Sending WebSocket initialize message");
        write
            .send(Message::Text(serde_json::to_string(&ws_init_message)?))
            .await?;

        let response_text = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
        let response: Value = serde_json::from_str(&response_text)?;
        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["id"], 2);

        info!("✅ WebSocket initialize successful");

        // Test HTTP tool call
        let echo_message = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "echo",
                "arguments": {
                    "message": "HTTP dual test"
                }
            }
        });

        let response = client
            .post(format!("{base_url}/mcp"))
            .json(&echo_message)
            .send()
            .await?;

        assert!(response.status().is_success());
        let response_json: Value = response.json().await?;
        assert_eq!(response_json["id"], 3);
        assert!(response_json["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("HTTP dual test"));

        info!("✅ HTTP tool call successful");

        // Test WebSocket tool call
        let ws_echo_message = json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": {
                "name": "echo",
                "arguments": {
                    "message": "WebSocket dual test"
                }
            }
        });

        write
            .send(Message::Text(serde_json::to_string(&ws_echo_message)?))
            .await?;

        let response_text = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
        let response: Value = serde_json::from_str(&response_text)?;
        assert_eq!(response["id"], 4);
        assert!(response["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("WebSocket dual test"));

        info!("✅ WebSocket tool call successful");

        Ok(())
    })
    .await?;

    Ok(())
}

/// Test HTTP MCP error handling scenarios
#[tokio::test]
async fn test_mcp_http_error_handling() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_test_tracing();
    info!("🚨 Testing MCP HTTP error handling");

    with_mcp_test_server("http_error_handling_test", |server| async move {
        let base_url = server.http_url();
        info!("🚀 Testing HTTP error handling at {}", base_url);

        let client = reqwest::ClientBuilder::new().cookie_store(true).build()?;

        // Test 1: Invalid JSON-RPC version
        let invalid_version_message = json!({
            "jsonrpc": "1.0", // Invalid version
            "id": 1,
            "method": "initialize",
            "params": {}
        });

        debug!("📤 Testing invalid JSON-RPC version");
        let response = client
            .post(format!("{base_url}/mcp"))
            .json(&invalid_version_message)
            .send()
            .await?;

        // Should return an error response
        let response_json: Value = response.json().await?;
        assert_eq!(response_json["jsonrpc"], "2.0");
        assert!(response_json.get("error").is_some());

        info!("✅ Invalid JSON-RPC version handled correctly");

        // Test 2: Missing method
        let missing_method_message = json!({
            "jsonrpc": "2.0",
            "id": 2
            // Missing method
        });

        debug!("📤 Testing missing method");
        let response = client
            .post(format!("{base_url}/mcp"))
            .json(&missing_method_message)
            .send()
            .await?;

        let response_json: Value = response.json().await?;
        assert_eq!(response_json["jsonrpc"], "2.0");
        assert!(response_json.get("error").is_some());

        info!("✅ Missing method handled correctly");

        // Test 3: Unknown method
        let unknown_method_message = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "unknown/method",
            "params": {}
        });

        debug!("📤 Testing unknown method");
        let response = client
            .post(format!("{base_url}/mcp"))
            .json(&unknown_method_message)
            .send()
            .await?;

        let response_json: Value = response.json().await?;
        assert_eq!(response_json["jsonrpc"], "2.0");
        assert!(response_json.get("error").is_some());
        assert_eq!(response_json["error"]["code"], -32601); // Method not found

        info!("✅ Unknown method handled correctly");

        // Test 4: Invalid Content-Type
        debug!("📤 Testing invalid Content-Type");
        let response = client
            .post(format!("{base_url}/mcp"))
            .header("Content-Type", "text/plain")
            .body("invalid json")
            .send()
            .await?;

        // Should return 400 or handle gracefully
        assert!(!response.status().is_success());

        info!("✅ Invalid Content-Type handled correctly");

        Ok(())
    })
    .await?;

    Ok(())
}
