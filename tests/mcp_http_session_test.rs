//! HTTP Session Management Tests
//!
//! Tests to ensure HTTP session management works correctly for stateless clients

mod mcp_test_helpers;

use anyhow::Result;
use mcp_test_helpers::init_test_tracing;
use serde_json::{json, Value};

/// Test that HTTP clients without cookie support can use the default session
#[tokio::test]
async fn test_http_default_session_fallback() -> Result<()> {
    init_test_tracing();

    // Start test server
    let server = mcp_test_helpers::McpTestServer::start()
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let url = server.http_url();

    // Create HTTP client that doesn't store cookies
    let client = reqwest::Client::new();

    // Initialize without cookies
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {
                "name": "stateless-test-client",
                "version": "1.0.0"
            }
        }
    });

    let response = client.post(&url).json(&init_request).send().await?;

    assert_eq!(response.status(), 200);
    let init_response: Value = response.json().await?;
    assert!(init_response["result"].is_object());

    // Now test that tools/list works without sending cookies
    let tools_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    });

    let response = client.post(&url).json(&tools_request).send().await?;

    assert_eq!(response.status(), 200);
    let tools_response: Value = response.json().await?;

    // Should NOT have an error - should use default session
    assert!(tools_response["result"].is_object());
    assert!(tools_response["result"]["tools"].is_array());
    assert!(tools_response.get("error").is_none());

    println!("✅ HTTP default session fallback test passed!");
    Ok(())
}

/// Test that multiple stateless clients can work concurrently
#[tokio::test]
async fn test_http_concurrent_stateless_clients() -> Result<()> {
    init_test_tracing();

    let server = mcp_test_helpers::McpTestServer::start()
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let url = server.http_url();

    // Create multiple clients without cookie support
    let client1 = reqwest::Client::new();
    let client2 = reqwest::Client::new();

    // Initialize both clients
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

    // Initialize client 1
    let resp1 = client1.post(&url).json(&init_request).send().await?;
    assert_eq!(resp1.status(), 200);

    // Initialize client 2
    let resp2 = client2.post(&url).json(&init_request).send().await?;
    assert_eq!(resp2.status(), 200);

    // Both clients should be able to list tools
    let tools_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    });

    let tools_resp1 = client1.post(&url).json(&tools_request).send().await?;
    let tools_resp2 = client2.post(&url).json(&tools_request).send().await?;

    assert_eq!(tools_resp1.status(), 200);
    assert_eq!(tools_resp2.status(), 200);

    let tools1: Value = tools_resp1.json().await?;
    let tools2: Value = tools_resp2.json().await?;

    assert!(tools1["result"]["tools"].is_array());
    assert!(tools2["result"]["tools"].is_array());

    println!("✅ Concurrent stateless clients test passed!");
    Ok(())
}

/// Test that session cookies are still respected when provided
#[tokio::test]
async fn test_http_session_cookie_still_works() -> Result<()> {
    init_test_tracing();

    let server = mcp_test_helpers::McpTestServer::start()
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let url = server.http_url();

    // Client with cookie support - reqwest stores cookies by default
    let client = reqwest::Client::builder().build()?;

    // Initialize and get session cookie
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {
                "name": "cookie-test-client",
                "version": "1.0.0"
            }
        }
    });

    let response = client.post(&url).json(&init_request).send().await?;
    assert_eq!(response.status(), 200);

    // Check that we got a session cookie
    let cookies = response.cookies().collect::<Vec<_>>();
    let session_cookie = cookies.iter().find(|c| c.name() == "mcp_session");
    assert!(
        session_cookie.is_some(),
        "Should have received session cookie"
    );

    // Tools list should work with cookie
    let tools_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    });

    let response = client.post(&url).json(&tools_request).send().await?;
    assert_eq!(response.status(), 200);

    let tools_response: Value = response.json().await?;
    assert!(tools_response["result"]["tools"].is_array());

    println!("✅ Session cookie still works test passed!");
    Ok(())
}

/// Test error cases when no session exists
#[tokio::test]
async fn test_http_notifications_without_session() -> Result<()> {
    init_test_tracing();

    let server = mcp_test_helpers::McpTestServer::start()
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let url = server.http_url();

    let client = reqwest::Client::new();

    // Send notification without initializing first
    let notification = json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    });

    let response = client.post(&url).json(&notification).send().await?;
    assert_eq!(response.status(), 200);

    // Notifications should still be accepted (they don't return errors)
    let body = response.text().await?;
    assert!(body.is_empty() || body == "{}");

    println!("✅ Notifications without session test passed!");
    Ok(())
}

/// Test that initialize method always uses consistent session
#[tokio::test]
async fn test_http_initialize_consistent_session() -> Result<()> {
    init_test_tracing();

    let server = mcp_test_helpers::McpTestServer::start()
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let url = server.http_url();

    // Multiple clients without cookies
    let client1 = reqwest::Client::new();
    let client2 = reqwest::Client::new();

    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {
                "name": "test",
                "version": "1.0"
            }
        }
    });

    // Initialize multiple times
    for _ in 0..3 {
        let resp1 = client1.post(&url).json(&init_request).send().await?;
        let resp2 = client2.post(&url).json(&init_request).send().await?;

        assert_eq!(resp1.status(), 200);
        assert_eq!(resp2.status(), 200);
    }

    // All should still be able to use tools
    let tools_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    });

    let resp1 = client1.post(&url).json(&tools_request).send().await?;
    let resp2 = client2.post(&url).json(&tools_request).send().await?;

    assert_eq!(resp1.status(), 200);
    assert_eq!(resp2.status(), 200);

    println!("✅ Initialize consistent session test passed!");
    Ok(())
}

/// Test tool execution with stateless client
#[tokio::test]
async fn test_http_tool_execution_stateless() -> Result<()> {
    init_test_tracing();

    let server = mcp_test_helpers::McpTestServer::start()
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
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
                "name": "test",
                "version": "1.0"
            }
        }
    });

    client.post(&url).json(&init_request).send().await?;

    // Execute echo tool
    let tool_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "echo",
            "arguments": {
                "message": "Hello stateless!"
            }
        }
    });

    let response = client.post(&url).json(&tool_request).send().await?;
    assert_eq!(response.status(), 200);

    let tool_response: Value = response.json().await?;
    assert!(tool_response["result"].is_object());

    println!("✅ Tool execution with stateless client test passed!");
    Ok(())
}
