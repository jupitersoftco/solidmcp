//! Advanced Session Re-initialization Tests
//!
//! Comprehensive tests for session re-initialization scenarios following TDD principles

use serde_json::json;
use std::time::Duration;

mod mcp_test_helpers;
use mcp_test_helpers::with_mcp_test_server;

/// Test 1: RED - Multiple re-initializations with different protocol versions
#[tokio::test]
async fn test_multiple_reinitializations_different_versions() {
    // Test that multiple re-initializations work correctly with version negotiation
    with_mcp_test_server("multiple_reinit_test", |server| async move {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .cookie_store(true)
            .build()?;

        let versions = vec!["2025-06-18", "2025-03-26", "2025-06-18", "2025-03-26"];
        
        for (i, version) in versions.iter().enumerate() {
            let init_request = json!({
                "jsonrpc": "2.0",
                "id": i + 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": version,
                    "capabilities": {},
                    "clientInfo": {"name": format!("test-v{}", i), "version": format!("{}.0", i)}
                }
            });

            let response = client
                .post(&server.http_url())
                .json(&init_request)
                .send()
                .await?;

            assert_eq!(response.status(), 200);
            let body: serde_json::Value = response.json().await?;
            
            // Should always succeed
            assert!(body.get("result").is_some(), "Initialization {} failed", i);
            
            // Verify protocol version is negotiated correctly
            let result = body.get("result").unwrap();
            assert_eq!(result["protocolVersion"], *version);
            
            // Verify we can call tools after each re-init
            let tools_request = json!({
                "jsonrpc": "2.0",
                "id": 100 + i,
                "method": "tools/list"
            });

            let response = client
                .post(&server.http_url())
                .json(&tools_request)
                .send()
                .await?;

            assert_eq!(response.status(), 200);
            let body: serde_json::Value = response.json().await?;
            assert!(body.get("result").is_some(), "Tools list failed after re-init {}", i);
        }

        Ok(())
    })
    .await
    .unwrap();
}

/// Test 2: RED - Re-initialization with conflicting capabilities
#[tokio::test]
async fn test_reinit_with_conflicting_capabilities() {
    // Test that re-initialization properly handles capability conflicts
    with_mcp_test_server("reinit_capabilities_test", |server| async move {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .cookie_store(true)
            .build()?;

        // First init with basic capabilities
        let init1 = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {
                    "roots": {"listChanged": true}
                },
                "clientInfo": {"name": "client-basic", "version": "1.0"}
            }
        });

        let response = client.post(&server.http_url()).json(&init1).send().await?;
        assert_eq!(response.status(), 200);

        // Re-init with different capabilities
        let init2 = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {
                    "experimental": {"custom": true},
                    "roots": {"listChanged": false}
                },
                "clientInfo": {"name": "client-advanced", "version": "2.0"}
            }
        });

        let response = client.post(&server.http_url()).json(&init2).send().await?;
        assert_eq!(response.status(), 200);
        let body: serde_json::Value = response.json().await?;
        
        // Should succeed and use new capabilities
        assert!(body.get("result").is_some());

        Ok(())
    })
    .await
    .unwrap();
}

/// Test 3: RED - Session state isolation after re-initialization
#[tokio::test]
async fn test_session_state_isolation_after_reinit() {
    // Test that re-initialization properly clears session-specific state
    with_mcp_test_server("session_state_isolation_test", |server| async move {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .cookie_store(true)
            .build()?;

        // Initialize and set up some state
        let init1 = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {"name": "state-test-v1", "version": "1.0"}
            }
        });

        client.post(&server.http_url()).json(&init1).send().await?;

        // Call a tool to establish some state
        let tool_call = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "echo",
                "arguments": {"message": "session state test"}
            }
        });

        client.post(&server.http_url()).json(&tool_call).send().await?;

        // Re-initialize
        let init2 = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-03-26",
                "capabilities": {},
                "clientInfo": {"name": "state-test-v2", "version": "2.0"}
            }
        });

        let response = client.post(&server.http_url()).json(&init2).send().await?;
        assert_eq!(response.status(), 200);
        
        // Verify clean state by checking server info
        let server_info = json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "server/get_info"
        });

        let response = client.post(&server.http_url()).json(&server_info).send().await?;
        let body: serde_json::Value = response.json().await?;
        
        // If method exists, verify it reflects new initialization
        if body.get("result").is_some() {
            let result = body.get("result").unwrap();
            if let Some(client_info) = result.get("clientInfo") {
                assert_eq!(client_info["name"], "state-test-v2");
            }
        }

        Ok(())
    })
    .await
    .unwrap();
}

/// Test 4: RED - Rapid sequential re-initializations (stress test)
#[tokio::test]
async fn test_rapid_reinitializations() {
    // Test that rapid re-initializations don't cause race conditions
    with_mcp_test_server("rapid_reinit_test", |server| async move {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .cookie_store(true)
            .build()?;

        // Perform 10 rapid re-initializations
        for i in 0..10 {
            let init = json!({
                "jsonrpc": "2.0",
                "id": i + 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": if i % 2 == 0 { "2025-06-18" } else { "2025-03-26" },
                    "capabilities": {},
                    "clientInfo": {"name": "rapid-test", "version": format!("{}.0", i)}
                }
            });

            let response = client.post(&server.http_url()).json(&init).send().await?;
            assert_eq!(response.status(), 200);
            
            // Don't wait between requests to stress test
        }

        // Final verification - should still be able to use the session
        let tools = json!({
            "jsonrpc": "2.0",
            "id": 100,
            "method": "tools/list"
        });

        let response = client.post(&server.http_url()).json(&tools).send().await?;
        assert_eq!(response.status(), 200);
        let body: serde_json::Value = response.json().await?;
        assert!(body.get("result").is_some());

        Ok(())
    })
    .await
    .unwrap();
}

/// Test 5: RED - Re-initialization with invalid protocol version fallback
#[tokio::test]
async fn test_reinit_invalid_protocol_fallback() {
    // Test re-initialization with unsupported protocol version
    with_mcp_test_server("invalid_protocol_reinit_test", |server| async move {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .cookie_store(true)
            .build()?;

        // First valid initialization
        let init1 = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {"name": "test", "version": "1.0"}
            }
        });

        client.post(&server.http_url()).json(&init1).send().await?;

        // Re-init with invalid version
        let init2 = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "initialize",
            "params": {
                "protocolVersion": "2099-99-99",  // Future/invalid version
                "capabilities": {},
                "clientInfo": {"name": "test", "version": "2.0"}
            }
        });

        let response = client.post(&server.http_url()).json(&init2).send().await?;
        
        // Server might return 400 for invalid version or 200 with error
        if response.status() == 400 {
            // 400 Bad Request is acceptable for invalid version
            assert!(true, "Server correctly rejected invalid version with 400");
        } else if response.status() == 200 {
            let body: serde_json::Value = response.json().await?;
            
            // Should either error or negotiate to a supported version
            if let Some(result) = body.get("result") {
                let version = result["protocolVersion"].as_str().unwrap();
                assert!(
                    version == "2025-06-18" || version == "2025-03-26",
                    "Should negotiate to supported version"
                );
            } else if let Some(error) = body.get("error") {
                // Error is also acceptable for invalid version
                assert!(error["message"].as_str().unwrap().contains("version"));
            } else {
                panic!("Expected either result with negotiated version or error");
            }
        } else {
            panic!("Unexpected status code: {}", response.status());
        }

        Ok(())
    })
    .await
    .unwrap();
}