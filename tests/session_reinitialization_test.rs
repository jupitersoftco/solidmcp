//! Session Re-initialization Test
//!
//! Tests proper handling of session re-initialization scenarios

use serde_json::json;
use std::time::Duration;

mod mcp_test_helpers;
use mcp_test_helpers::with_mcp_test_server;

#[tokio::test]
async fn test_session_reinitialize_clears_state() {
    // Test that re-initializing a session properly clears all previous state
    with_mcp_test_server("session_reinit_test", |server| async move {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .cookie_store(true) // Enable cookie jar for session tracking
            .build()?;

        // First initialization
        let init_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {"name": "test-v1", "version": "1.0"}
            }
        });

        let response = client
            .post(&server.http_url())
            .json(&init_request)
            .send()
            .await?;

        assert_eq!(response.status(), 200);
        let body: serde_json::Value = response.json().await?;
        assert!(body.get("result").is_some());

        // Verify we can call tools
        let tools_request = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list"
        });

        let response = client
            .post(&server.http_url())
            .json(&tools_request)
            .send()
            .await?;

        assert_eq!(response.status(), 200);

        // Second initialization with different client info
        let reinit_request = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-03-26",  // Different version
                "capabilities": {},
                "clientInfo": {"name": "test-v2", "version": "2.0"}
            }
        });

        let response = client
            .post(&server.http_url())
            .json(&reinit_request)
            .send()
            .await?;

        assert_eq!(response.status(), 200);
        let body: serde_json::Value = response.json().await?;

        println!("Re-init response: {:?}", body);

        // Should succeed and return the negotiated version
        let result = body.get("result").expect("Should have result");
        assert_eq!(result["protocolVersion"], "2025-03-26");

        // Verify we can still call tools after re-init
        let tools_request = json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/list"
        });

        let response = client
            .post(&server.http_url())
            .json(&tools_request)
            .send()
            .await?;

        assert_eq!(response.status(), 200);

        Ok(())
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn test_concurrent_session_isolation() {
    // Test that different sessions don't interfere with each other
    with_mcp_test_server("concurrent_sessions_test", |server| async move {
        // Client 1
        let client1 = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()?;

        // Client 2
        let client2 = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()?;

        // Initialize client 1
        let init1 = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {"name": "client1", "version": "1.0"}
            }
        });

        let response = client1.post(&server.http_url()).json(&init1).send().await?;
        assert_eq!(response.status(), 200);

        // Initialize client 2 with different version
        let init2 = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-03-26",
                "capabilities": {},
                "clientInfo": {"name": "client2", "version": "1.0"}
            }
        });

        let response = client2.post(&server.http_url()).json(&init2).send().await?;
        assert_eq!(response.status(), 200);

        // Both clients should be able to call tools independently
        let tools_request = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list"
        });

        let response1 = client1
            .post(&server.http_url())
            .json(&tools_request)
            .send()
            .await?;
        assert_eq!(response1.status(), 200);

        let response2 = client2
            .post(&server.http_url())
            .json(&tools_request)
            .send()
            .await?;
        assert_eq!(response2.status(), 200);

        Ok(())
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn test_uninitialized_session_rejection() {
    // Test that uninitialized sessions can't call methods
    with_mcp_test_server("uninitialized_test", |server| async move {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()?;

        // Try to call tools without initialization
        let tools_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/list"
        });

        let response = client
            .post(&server.http_url())
            .json(&tools_request)
            .send()
            .await?;

        assert_eq!(response.status(), 200); // JSON-RPC errors return 200
        let body: serde_json::Value = response.json().await?;

        // Should have an error
        let error = body.get("error").expect("Should have error");
        assert_eq!(error["code"], -32002); // Not initialized error code

        Ok(())
    })
    .await
    .unwrap();
}
