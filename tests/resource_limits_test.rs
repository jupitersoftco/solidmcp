//! Resource Limits Tests
//!
//! Tests for message size limits, session limits, and other resource constraints

use solidmcp::{McpServerBuilder, ResourceLimits};
use serde_json::json;
use std::time::Duration;

mod mcp_test_helpers;
use mcp_test_helpers::*;

/// Test message size limit enforcement
#[tokio::test]
async fn test_message_size_limit() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_test_tracing();
    
    // Create server with small message size limit
    let server = McpServerBuilder::new((), "limit-test", "1.0.0")
        .with_limits(ResourceLimits {
            max_message_size: 1024, // 1KB limit
            ..Default::default()
        })
        .build()
        .await?;
    
    let port = find_available_port().await?;
    
    tokio::spawn(async move {
        let mut server = server;
        server.start(port).await.unwrap();
    });
    
    tokio::time::sleep(Duration::from_millis(300)).await;
    
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}/mcp", port);
    
    // Small message should work
    let small_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
        }
    });
    
    let response = client.post(&url)
        .json(&small_request)
        .send()
        .await?;
    
    assert_eq!(response.status(), 200);
    
    // Large message should fail
    let large_data = "x".repeat(2000); // 2KB of data
    let large_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "test",
        "params": {
            "data": large_data
        }
    });
    
    let response = client.post(&url)
        .header("Cookie", "mcp_session=test")
        .json(&large_request)
        .send()
        .await?;
    
    assert_eq!(response.status(), 200); // JSON-RPC errors still return 200
    let error_response: serde_json::Value = response.json().await?;
    
    assert!(error_response["error"].is_object());
    assert!(error_response["error"]["message"]
        .as_str()
        .unwrap()
        .contains("Message too large"));
    
    Ok(())
}

/// Test session limit enforcement
#[tokio::test]
async fn test_session_limit() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_test_tracing();
    
    // Create server with small session limit
    let server = McpServerBuilder::new((), "session-limit-test", "1.0.0")
        .with_limits(ResourceLimits {
            max_sessions: Some(2), // Only 2 sessions allowed
            ..Default::default()
        })
        .build()
        .await?;
    
    let port = find_available_port().await?;
    
    tokio::spawn(async move {
        let mut server = server;
        server.start(port).await.unwrap();
    });
    
    tokio::time::sleep(Duration::from_millis(300)).await;
    
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}/mcp", port);
    
    // Create init request
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18"
        }
    });
    
    // Create first two sessions - should succeed
    for i in 0..2 {
        let response = client.post(&url)
            .header("Cookie", format!("mcp_session=session{}", i))
            .json(&init_request)
            .send()
            .await?;
        
        assert_eq!(response.status(), 200);
        let result: serde_json::Value = response.json().await?;
        assert!(result["result"].is_object());
    }
    
    // Third session should fail
    let response = client.post(&url)
        .header("Cookie", "mcp_session=session3")
        .json(&init_request)
        .send()
        .await?;
    
    assert_eq!(response.status(), 200); // JSON-RPC errors still return 200
    let error_response: serde_json::Value = response.json().await?;
    
    assert!(error_response["error"].is_object());
    assert!(error_response["error"]["message"]
        .as_str()
        .unwrap()
        .contains("Too many sessions"));
    
    Ok(())
}

/// Test that unlimited limits work correctly
#[tokio::test]
async fn test_unlimited_limits() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_test_tracing();
    
    // Create server with unlimited limits
    let server = McpServerBuilder::new((), "unlimited-test", "1.0.0")
        .with_limits(ResourceLimits::unlimited())
        .build()
        .await?;
    
    let port = find_available_port().await?;
    
    tokio::spawn(async move {
        let mut server = server;
        server.start(port).await.unwrap();
    });
    
    tokio::time::sleep(Duration::from_millis(300)).await;
    
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}/mcp", port);
    
    // Very large message should work with unlimited limits
    let large_data = "x".repeat(10_000); // 10KB of data
    let large_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "data": large_data
        }
    });
    
    let response = client.post(&url)
        .json(&large_request)
        .send()
        .await?;
    
    assert_eq!(response.status(), 200);
    let result: serde_json::Value = response.json().await?;
    assert!(result["result"].is_object());
    
    Ok(())
}