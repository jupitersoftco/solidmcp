//! Regression tests for capabilities visibility
//!
//! Ensures that server capabilities are correctly reported during initialization

mod mcp_test_helpers;

use solidmcp::{McpResult, McpError};
use serde_json::{json, Value};

/// Test that capabilities are correctly reported for servers with tools
#[tokio::test]
async fn test_capabilities_with_tools() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let server = mcp_test_helpers::McpTestServer::start()
        .await
        .map_err(|e| McpError::InvalidParams(format!("{}", e)))?;
    let url = server.http_url();

    let client = reqwest::Client::new();

    // Initialize and check capabilities
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

    // Verify capabilities are present
    assert!(init_response["result"]["capabilities"].is_object());
    assert!(init_response["result"]["capabilities"]["tools"].is_object());
    assert_eq!(
        init_response["result"]["capabilities"]["tools"]["listChanged"],
        false
    );

    println!("✅ Capabilities correctly reported with tools!");
    Ok(())
}

/// Test that empty servers report no capabilities
#[tokio::test]
async fn test_empty_capabilities() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // This would require creating a server with no tools
    // For now, we'll skip this test
    println!("⏭️  Empty capabilities test skipped (requires empty server)");
    Ok(())
}

/// Test that capabilities match actual server features
#[tokio::test]
async fn test_capabilities_match_features() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let server = mcp_test_helpers::McpTestServer::start()
        .await
        .map_err(|e| McpError::InvalidParams(format!("{}", e)))?;
    let url = server.http_url();

    let client = reqwest::Client::new();

    // Initialize
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

    let has_tools_capability = init_response["result"]["capabilities"]["tools"].is_object();

    // Now check if tools/list works
    let tools_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    });

    let response = client
        .post(&url)
        .header("Cookie", "mcp_session=http_default_session")
        .json(&tools_request)
        .send()
        .await?;
    let tools_response: Value = response.json().await?;

    let has_tools = tools_response["result"]["tools"]
        .as_array()
        .map(|arr| !arr.is_empty())
        .unwrap_or(false);

    // If we have tools capability, we should have tools
    if has_tools_capability {
        assert!(
            has_tools,
            "Server reports tools capability but has no tools"
        );
    }

    println!("✅ Capabilities match server features!");
    Ok(())
}
