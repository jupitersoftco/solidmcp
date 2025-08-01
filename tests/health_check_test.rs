//! Health Check Tests
//!
//! Tests for the health check endpoint functionality

use solidmcp::{McpServerBuilder, HealthStatus};
use serde_json::Value;
use std::time::Duration;

mod mcp_test_helpers;
use mcp_test_helpers::*;

/// Test basic health check endpoint
#[tokio::test]
async fn test_health_endpoint_returns_json() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_test_tracing();
    
    // Create a test server
    let server = McpServerBuilder::new((), "health-test-server", "1.0.0")
        .build()
        .await?;
    
    let port = find_available_port().await?;
    
    tokio::spawn(async move {
        let mut server = server;
        server.start(port).await.unwrap();
    });
    
    tokio::time::sleep(Duration::from_millis(300)).await;
    
    // Test health endpoint
    let client = reqwest::Client::new();
    let health_url = format!("http://127.0.0.1:{}/health", port);
    
    let response = client.get(&health_url).send().await?;
    
    assert_eq!(response.status(), 200);
    
    let health_json: Value = response.json().await?;
    
    // Verify required fields
    assert_eq!(health_json["status"], "healthy");
    assert!(health_json["timestamp"].is_u64());
    assert!(health_json["version"].is_string());
    assert!(health_json["uptime_seconds"].is_u64());
    assert!(health_json["session_count"].is_number());
    
    // Verify metadata
    assert!(health_json["metadata"].is_object());
    assert_eq!(health_json["metadata"]["server_name"], "health-test-server");
    assert_eq!(health_json["metadata"]["protocol_version"], "2025-06-18");
    
    Ok(())
}

/// Test health check performance
#[tokio::test]
async fn test_health_check_performance() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_test_tracing();
    
    let server = McpServerBuilder::new((), "perf-test", "1.0.0")
        .build()
        .await?;
    
    let port = find_available_port().await?;
    
    tokio::spawn(async move {
        let mut server = server;
        server.start(port).await.unwrap();
    });
    
    tokio::time::sleep(Duration::from_millis(300)).await;
    
    let client = reqwest::Client::new();
    let health_url = format!("http://127.0.0.1:{}/health", port);
    
    // Warm up
    client.get(&health_url).send().await?;
    
    // Measure performance
    let start = std::time::Instant::now();
    let response = client.get(&health_url).send().await?;
    let duration = start.elapsed();
    
    assert_eq!(response.status(), 200);
    assert!(
        duration.as_millis() < 100,
        "Health check took {}ms, expected < 100ms",
        duration.as_millis()
    );
    
    Ok(())
}

/// Test health check with active sessions
#[tokio::test]
async fn test_health_with_sessions() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_test_tracing();
    
    let server = McpServerBuilder::new((), "session-test", "1.0.0")
        .build()
        .await?;
    
    let port = find_available_port().await?;
    
    tokio::spawn(async move {
        let mut server = server;
        server.start(port).await.unwrap();
    });
    
    tokio::time::sleep(Duration::from_millis(300)).await;
    
    let client = reqwest::Client::new();
    let mcp_url = format!("http://127.0.0.1:{}/mcp", port);
    let health_url = format!("http://127.0.0.1:{}/health", port);
    
    // Create some sessions
    let init_request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18"
        }
    });
    
    // Create 3 sessions
    for i in 0..3 {
        let response = client.post(&mcp_url)
            .header("Cookie", format!("mcp_session=session_{}", i))
            .json(&init_request)
            .send()
            .await?;
        
        // Verify each session was created successfully
        assert_eq!(response.status(), 200);
    }
    
    // Check health
    let response = client.get(&health_url).send().await?;
    let health: Value = response.json().await?;
    
    assert_eq!(health["status"], "healthy");
    
    // The session count should be at least 1 (could be more if sessions are reused)
    let session_count = health["session_count"].as_u64().unwrap_or(0);
    assert!(session_count >= 1, "Expected at least 1 session, got {}", session_count);
    
    Ok(())
}

/// Test that health check works without authentication
#[tokio::test]
async fn test_health_no_auth_required() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_test_tracing();
    
    let server = McpServerBuilder::new((), "auth-test", "1.0.0")
        .build()
        .await?;
    
    let port = find_available_port().await?;
    
    tokio::spawn(async move {
        let mut server = server;
        server.start(port).await.unwrap();
    });
    
    tokio::time::sleep(Duration::from_millis(300)).await;
    
    // Test without any headers or cookies
    let client = reqwest::Client::builder()
        .default_headers(reqwest::header::HeaderMap::new())
        .build()?;
    
    let health_url = format!("http://127.0.0.1:{}/health", port);
    let response = client.get(&health_url).send().await?;
    
    assert_eq!(response.status(), 200);
    
    let health: Value = response.json().await?;
    assert_eq!(health["status"], "healthy");
    
    Ok(())
}

/// Test health check struct deserialization
#[tokio::test]
async fn test_health_status_struct() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_test_tracing();
    
    let server = McpServerBuilder::new((), "struct-test", "1.0.0")
        .build()
        .await?;
    
    let port = find_available_port().await?;
    
    tokio::spawn(async move {
        let mut server = server;
        server.start(port).await.unwrap();
    });
    
    tokio::time::sleep(Duration::from_millis(300)).await;
    
    let client = reqwest::Client::new();
    let health_url = format!("http://127.0.0.1:{}/health", port);
    
    let response = client.get(&health_url).send().await?;
    let health: HealthStatus = response.json().await?;
    
    assert_eq!(health.status, "healthy");
    assert_eq!(health.version, "1.0.0");
    assert!(health.uptime_seconds >= 0);
    assert_eq!(health.session_count.unwrap_or(0), 0);
    
    Ok(())
}