//! HTTP Session Integration Tests for Toy Example
//!
//! Tests to ensure the toy example works correctly with stateless HTTP clients

use anyhow::Result;
use serde_json::{json, Value};
use std::time::Duration;
use tempfile::TempDir;
use toy_notes_server::server::create_toy_server;

/// Test that toy server works with stateless HTTP clients (like Claude)
#[tokio::test]
async fn test_toy_server_stateless_http_client() -> Result<()> {
    // Create temporary directory for notes
    let temp_dir = TempDir::new()?;
    let notes_dir = temp_dir.path().to_path_buf();

    // Create and start the toy server
    let server = create_toy_server(notes_dir).await?;

    // Find available port
    let port = {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        let port = addr.port();
        drop(listener);
        port
    };

    tokio::spawn(async move {
        server.start(port).await.unwrap();
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Create client without cookie support (like Claude)
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}/mcp", port);

    // Test 1: Initialize
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {
                "name": "claude-test",
                "version": "1.0.0"
            }
        }
    });

    let response = client.post(&url).json(&init_request).send().await?;
    assert_eq!(response.status(), 200);
    let init_response: Value = response.json().await?;
    assert!(init_response["result"].is_object());

    // Test 2: List tools (should work without cookie)
    let tools_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    });

    let response = client.post(&url).json(&tools_request).send().await?;
    assert_eq!(response.status(), 200);
    let tools_response: Value = response.json().await?;

    // Should NOT have error - should have tools
    assert!(tools_response.get("error").is_none());
    assert!(tools_response["result"]["tools"].is_array());

    let tools = tools_response["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 3); // add_note, list_notes, add_notification

    // Test 3: Add a note (should work without cookie)
    let add_note_request = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "add_note",
            "arguments": {
                "name": "test-stateless",
                "content": "This note was created by a stateless client!"
            }
        }
    });

    let response = client.post(&url).json(&add_note_request).send().await?;
    assert_eq!(response.status(), 200);
    let add_response: Value = response.json().await?;
    assert!(add_response["result"].is_object());

    // Test 4: List notes to verify it was saved
    let list_notes_request = json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "tools/call",
        "params": {
            "name": "list_notes",
            "arguments": {}
        }
    });

    let response = client.post(&url).json(&list_notes_request).send().await?;
    assert_eq!(response.status(), 200);
    let list_response: Value = response.json().await?;

    let content = list_response["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    let notes: Vec<String> = serde_json::from_str(content)?;
    assert!(notes.contains(&"test-stateless".to_string()));

    println!("✅ Toy server works correctly with stateless HTTP clients!");
    Ok(())
}

/// Test multiple concurrent stateless clients on toy server
#[tokio::test]
async fn test_toy_server_concurrent_stateless_clients() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let notes_dir = temp_dir.path().to_path_buf();

    let server = create_toy_server(notes_dir).await?;

    let port = {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        let port = addr.port();
        drop(listener);
        port
    };

    tokio::spawn(async move {
        server.start(port).await.unwrap();
    });

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Create multiple stateless clients
    let client1 = reqwest::Client::new();
    let client2 = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}/mcp", port);

    // Initialize both
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

    let resp1 = client1.post(&url).json(&init_request).send().await?;
    let resp2 = client2.post(&url).json(&init_request).send().await?;
    assert_eq!(resp1.status(), 200);
    assert_eq!(resp2.status(), 200);

    // Both should be able to add notes
    let add_note1 = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "add_note",
            "arguments": {
                "name": "client1-note",
                "content": "From client 1"
            }
        }
    });

    let add_note2 = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "add_note",
            "arguments": {
                "name": "client2-note",
                "content": "From client 2"
            }
        }
    });

    let resp1 = client1.post(&url).json(&add_note1).send().await?;
    let resp2 = client2.post(&url).json(&add_note2).send().await?;
    assert_eq!(resp1.status(), 200);
    assert_eq!(resp2.status(), 200);

    // List notes should show both
    let list_request = json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "tools/call",
        "params": {
            "name": "list_notes",
            "arguments": {}
        }
    });

    let response = client1.post(&url).json(&list_request).send().await?;
    let list_response: Value = response.json().await?;

    let content = list_response["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    let notes: Vec<String> = serde_json::from_str(content)?;
    assert!(notes.contains(&"client1-note".to_string()));
    assert!(notes.contains(&"client2-note".to_string()));

    println!("✅ Concurrent stateless clients work correctly!");
    Ok(())
}

/// Test that notifications work with stateless clients
#[tokio::test]
async fn test_toy_server_notifications_stateless() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let notes_dir = temp_dir.path().to_path_buf();

    let server = create_toy_server(notes_dir).await?;

    let port = {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        let port = addr.port();
        drop(listener);
        port
    };

    tokio::spawn(async move {
        server.start(port).await.unwrap();
    });

    tokio::time::sleep(Duration::from_millis(500)).await;

    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}/mcp", port);

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

    // Send notification
    let notification_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "add_notification",
            "arguments": {
                "level": "info",
                "message": "Test notification from stateless client"
            }
        }
    });

    let response = client.post(&url).json(&notification_request).send().await?;
    assert_eq!(response.status(), 200);

    let notif_response: Value = response.json().await?;
    assert!(notif_response["result"].is_object());

    println!("✅ Notifications work with stateless clients!");
    Ok(())
}
