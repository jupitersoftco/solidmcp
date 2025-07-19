//! MCP Integration Tests
//!
//! Tests the Model Context Protocol (MCP) integration including:
//! - HTTP MCP endpoint
//! - WebSocket MCP endpoint
//! - Protocol initialization and tool calls

mod mcp_test_helpers;
use mcp_test_helpers::{init_test_tracing, with_mcp_test_server};
use serde_json::json;
use tokio::time::{sleep, Duration};
use tracing::info;

#[tokio::test]
async fn test_mcp_http_integration() {
    init_test_tracing();
    info!("🧪 Testing MCP HTTP integration");

    with_mcp_test_server("http_integration", |server| async move {
        let port = server.port;
        let client = reqwest::Client::new();

        // Wait for server to be ready
        let mut attempts = 0;
        let max_attempts = 20;
        let mut response_ok = false;

        info!("⏳ Waiting for server to be ready...");
        while attempts < max_attempts {
            sleep(Duration::from_millis(100)).await;
            attempts += 1;

            let response = client
                .get(format!("http://localhost:{port}/health"))
                .timeout(Duration::from_secs(2))
                .send()
                .await;

            match response {
                Ok(response) => {
                    if response.status() == 200 {
                        response_ok = true;
                        break;
                    }
                }
                Err(_) => {
                    // Server not ready yet, continue waiting
                }
            }
        }

        if !response_ok {
            return Err(format!("Server not ready after {max_attempts} attempts").into());
        }

        // Test MCP HTTP endpoint
        info!("🧪 Testing MCP HTTP endpoint...");

        // Test 1: Initialize MCP session
        let init_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {
                    "tools": {}
                },
                "clientInfo": {
                    "name": "test-client",
                    "version": "1.0.0"
                }
            }
        });

        let response = client
            .post(format!("http://localhost:{port}/mcp"))
            .header("Content-Type", "application/json")
            .json(&init_request)
            .send()
            .await
            .expect("Failed to send initialize request");

        assert!(response.status().is_success());
        let init_response: serde_json::Value = response
            .json()
            .await
            .expect("Failed to parse initialize response");

        info!(
            "📄 Initialize response: {}",
            serde_json::to_string_pretty(&init_response).unwrap()
        );
        assert_eq!(init_response["jsonrpc"], "2.0");
        assert_eq!(init_response["id"], 1);
        assert!(init_response["result"].is_object());

        // Test 2: List tools
        let list_tools_request = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list"
        });

        let response = client
            .post(format!("http://localhost:{port}/mcp"))
            .header("Content-Type", "application/json")
            .json(&list_tools_request)
            .send()
            .await
            .expect("Failed to send list tools request");

        assert!(response.status().is_success());
        let list_response: serde_json::Value = response
            .json()
            .await
            .expect("Failed to parse list tools response");

        info!(
            "📄 List tools response: {}",
            serde_json::to_string_pretty(&list_response).unwrap()
        );
        assert_eq!(list_response["jsonrpc"], "2.0");
        assert_eq!(list_response["id"], 2);
        assert!(list_response["result"]["tools"].is_array());

        // Test 3: Call echo tool
        let call_request = json!({
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

        let response = client
            .post(format!("http://localhost:{port}/mcp"))
            .header("Content-Type", "application/json")
            .json(&call_request)
            .send()
            .await
            .expect("Failed to send call request");

        assert!(response.status().is_success());
        let call_response: serde_json::Value = response
            .json()
            .await
            .expect("Failed to parse call response");

        info!(
            "📄 Call response: {}",
            serde_json::to_string_pretty(&call_response).unwrap()
        );
        assert_eq!(call_response["jsonrpc"], "2.0");
        assert_eq!(call_response["id"], 3);
        assert!(call_response["result"]["content"].is_array());

        Ok(())
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn test_mcp_protocol_compliance() {
    init_test_tracing();
    info!("🧪 Testing MCP protocol compliance");

    with_mcp_test_server("protocol_compliance", |server| async move {
        let port = server.port;
        let client = reqwest::Client::new();

        // Wait for server to be ready
        let mut attempts = 0;
        let max_attempts = 20;
        let mut response_ok = false;

        info!("⏳ Waiting for server to be ready...");
        while attempts < max_attempts {
            sleep(Duration::from_millis(100)).await;
            attempts += 1;

            let response = client
                .get(format!("http://localhost:{port}/health"))
                .timeout(Duration::from_secs(2))
                .send()
                .await;

            match response {
                Ok(response) => {
                    if response.status() == 200 {
                        response_ok = true;
                        break;
                    }
                }
                Err(_) => {
                    // Server not ready yet, continue waiting
                }
            }
        }

        if !response_ok {
            return Err(format!("Server not ready after {max_attempts} attempts").into());
        }

        // Test protocol compliance
        info!("🧪 Testing MCP protocol compliance...");

        // Test 1: Invalid JSON-RPC request
        let invalid_request = "invalid json";
        let response = client
            .post(format!("http://localhost:{port}/mcp"))
            .header("Content-Type", "application/json")
            .body(invalid_request)
            .send()
            .await
            .expect("Failed to send invalid request");

        // Should return error (not necessarily 400, depends on implementation)
        assert!(!response.status().is_success());

        // Test 2: Missing method
        let missing_method_request = json!({
            "jsonrpc": "2.0",
            "id": 1
        });

        let response = client
            .post(format!("http://localhost:{port}/mcp"))
            .header("Content-Type", "application/json")
            .json(&missing_method_request)
            .send()
            .await
            .expect("Failed to send missing method request");

        // Should return error for missing method
        assert!(!response.status().is_success());

        // Test 3: Unsupported method
        let unsupported_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "unsupported_method"
        });

        let response = client
            .post(format!("http://localhost:{port}/mcp"))
            .header("Content-Type", "application/json")
            .json(&unsupported_request)
            .send()
            .await
            .expect("Failed to send unsupported method request");

        // Should return error for unsupported method
        assert!(!response.status().is_success());

        Ok(())
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn test_mcp_tool_execution() {
    init_test_tracing();
    info!("🔧 Testing MCP tool execution");

    with_mcp_test_server("tool_execution", |server| async move {
        let port = server.port;
        let client = reqwest::Client::new();

        // Initialize first
        let init_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {
                    "name": "tool-test-client",
                    "version": "1.0.0"
                }
            }
        });

        let response = client
            .post(format!("http://localhost:{port}/mcp"))
            .json(&init_request)
            .send()
            .await?;

        assert!(response.status().is_success());

        // Test echo tool
        let echo_request = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "echo",
                "arguments": {
                    "message": "Integration test echo"
                }
            }
        });

        let response = client
            .post(format!("http://localhost:{port}/mcp"))
            .json(&echo_request)
            .send()
            .await?;

        assert!(response.status().is_success());
        let echo_response: serde_json::Value = response.json().await?;

        assert_eq!(echo_response["jsonrpc"], "2.0");
        assert_eq!(echo_response["id"], 2);
        assert!(echo_response["result"]["content"].is_array());

        // Test read_file tool
        let read_request = json!({
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

        let response = client
            .post(format!("http://localhost:{port}/mcp"))
            .json(&read_request)
            .send()
            .await?;

        assert!(response.status().is_success());
        let read_response: serde_json::Value = response.json().await?;

        assert_eq!(read_response["jsonrpc"], "2.0");
        assert_eq!(read_response["id"], 3);
        assert!(read_response["result"]["content"].is_array());

        // Test unknown tool (should fail)
        let unknown_request = json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": {
                "name": "nonexistent_tool",
                "arguments": {}
            }
        });

        let response = client
            .post(format!("http://localhost:{port}/mcp"))
            .json(&unknown_request)
            .send()
            .await?;

        let unknown_response: serde_json::Value = response.json().await?;
        assert_eq!(unknown_response["jsonrpc"], "2.0");
        assert_eq!(unknown_response["id"], 4);
        // Should have error instead of result
        assert!(unknown_response.get("error").is_some());

        Ok(())
    })
    .await
    .unwrap();
}
