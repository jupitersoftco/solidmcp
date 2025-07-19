//! Regression tests for server info correctness
//!
//! Ensures server identification is consistent and correct

mod mcp_test_helpers;

use anyhow::Result;
use serde_json::{json, Value};

/// Test that server info is correctly reported
#[tokio::test]
async fn test_server_info_correctness() -> Result<()> {
    let server = mcp_test_helpers::McpTestServer::start()
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let url = format!("{}/mcp", server.http_url());

    let client = reqwest::Client::new();

    // Initialize and check server info
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
        }
    });

    let response = client.post(&url).json(&init_request).send().await?;
    let init_response: Value = response.json().await?;

    // Verify server info structure
    assert!(init_response["result"]["serverInfo"].is_object());
    assert!(init_response["result"]["serverInfo"]["name"].is_string());
    assert!(init_response["result"]["serverInfo"]["version"].is_string());

    // Server name should not be empty
    let server_name = init_response["result"]["serverInfo"]["name"]
        .as_str()
        .unwrap();
    assert!(!server_name.is_empty(), "Server name should not be empty");

    println!("✅ Server info correctly reported: {server_name}");
    Ok(())
}

/// Test protocol version negotiation
#[tokio::test]
async fn test_protocol_version_negotiation() -> Result<()> {
    let server = mcp_test_helpers::McpTestServer::start()
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let url = format!("{}/mcp", server.http_url());

    let client = reqwest::Client::new();

    // Test with current protocol version
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
        }
    });

    let response = client.post(&url).json(&init_request).send().await?;
    let init_response: Value = response.json().await?;

    // Server should accept and echo back the protocol version
    assert_eq!(init_response["result"]["protocolVersion"], "2025-06-18");

    println!("✅ Protocol version negotiation works correctly!");
    Ok(())
}

/// Test that server handles re-initialization correctly
#[tokio::test]
async fn test_reinitialization_handling() -> Result<()> {
    let server = mcp_test_helpers::McpTestServer::start()
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let url = format!("{}/mcp", server.http_url());

    let client = reqwest::Client::new();

    // First initialization
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
        }
    });

    let response = client.post(&url).json(&init_request).send().await?;
    assert_eq!(response.status(), 200);

    // Try to reinitialize with same session (should work)
    let init_request2 = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {
                "name": "test-client-2",
                "version": "2.0.0"
            }
        }
    });

    let response = client
        .post(&url)
        .header("Cookie", "mcp_session=http_default_session")
        .json(&init_request2)
        .send()
        .await?;
    let reinit_response: Value = response.json().await?;

    // Should fail with "Already initialized" (MCP rejects duplicate initialization)
    assert!(reinit_response["error"].is_object());
    assert_eq!(reinit_response["error"]["message"], "Already initialized");

    println!("✅ Re-initialization handling works correctly!");
    Ok(())
}
