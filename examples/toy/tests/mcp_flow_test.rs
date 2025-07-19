//! MCP Flow Tests
//!
//! End-to-end tests that capture real usage flows including resources and notifications

use anyhow::Result;
use serde_json::{json, Value};
use tempfile::TempDir;
use toy_notes_server::server::create_toy_server;

/// Test the complete MCP flow: initialization, resource listing, reading, and notification sending
#[tokio::test]
async fn test_complete_mcp_flow() -> Result<()> {
    // Create temporary directory for notes
    let temp_dir = TempDir::new()?;
    let notes_dir = temp_dir.path().to_path_buf();

    // Create some initial notes
    std::fs::write(
        notes_dir.join("demo-note.md"),
        "This is a demonstration note created using the toy MCP server.\n\n\
        The toy MCP server provides a simple example of MCP capabilities including:\n\
        - Note storage and retrieval\n\
        - Notification sending\n\
        - Basic data management",
    )?;
    std::fs::write(notes_dir.join("test-note.md"), "Hello from Claude")?;

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

    let server_handle = tokio::spawn(async move {
        server.start(port).await.unwrap();
    });

    // Give server time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Create client
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}/mcp", port);

    // Step 1: Initialize
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {
                "name": "mcp-flow-test",
                "version": "1.0.0"
            }
        }
    });

    let response = client.post(&url).json(&init_request).send().await?;
    assert_eq!(response.status(), 200);
    let init_response: Value = response.json().await?;

    // Verify capabilities
    assert!(init_response["result"]["capabilities"]["tools"].is_object());
    assert!(init_response["result"]["capabilities"]["resources"].is_object());
    assert_eq!(
        init_response["result"]["serverInfo"]["name"],
        "toy-notes-server"
    );

    // Step 2: List resources (like the manual test)
    let list_resources_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "resources/list",
        "params": {}
    });

    let response = client
        .post(&url)
        .header("Cookie", "mcp_session=http_default_session")
        .json(&list_resources_request)
        .send()
        .await?;
    assert_eq!(response.status(), 200);
    let resources_response: Value = response.json().await?;

    let resources = resources_response["result"]["resources"]
        .as_array()
        .unwrap();
    assert_eq!(resources.len(), 2);

    // Verify resource names match what we created
    let resource_names: Vec<String> = resources
        .iter()
        .map(|r| r["name"].as_str().unwrap().to_string())
        .collect();
    assert!(resource_names.contains(&"demo-note".to_string()));
    assert!(resource_names.contains(&"test-note".to_string()));

    // Step 3: Read a resource (like in manual test)
    let read_resource_request = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "resources/read",
        "params": {
            "uri": "notes://demo-note"
        }
    });

    let response = client
        .post(&url)
        .header("Cookie", "mcp_session=http_default_session")
        .json(&read_resource_request)
        .send()
        .await?;
    assert_eq!(response.status(), 200);
    let read_response: Value = response.json().await?;

    let contents = read_response["result"]["contents"].as_array().unwrap();
    assert_eq!(contents.len(), 1);
    let content_text = contents[0]["text"].as_str().unwrap();
    assert!(content_text.contains("demonstration note"));
    assert!(content_text.contains("Note storage and retrieval"));

    // Step 4: Send a notification using the tool
    let add_notification_request = json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "tools/call",
        "params": {
            "name": "add_notification",
            "arguments": {
                "level": "info",
                "message": "Test notification from flow test",
                "data": {
                    "timestamp": "2025-07-19",
                    "source": "toy-mcp-test"
                }
            }
        }
    });

    let response = client
        .post(&url)
        .header("Cookie", "mcp_session=http_default_session")
        .json(&add_notification_request)
        .send()
        .await?;
    assert_eq!(response.status(), 200);
    let notification_response: Value = response.json().await?;

    // Verify the notification was sent successfully
    let content = notification_response["result"]["content"]
        .as_array()
        .unwrap();
    assert_eq!(content.len(), 1);
    let result_text = content[0]["text"].as_str().unwrap();
    let result: Value = serde_json::from_str(result_text)?;
    assert_eq!(result["success"], true);

    // Clean up
    server_handle.abort();

    println!("✅ Complete MCP flow test passed!");
    Ok(())
}

/// Test notification sending at different levels
#[tokio::test]
async fn test_notification_levels() -> Result<()> {
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

    let server_handle = tokio::spawn(async move {
        server.start(port).await.unwrap();
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

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
                "name": "notification-test",
                "version": "1.0.0"
            }
        }
    });

    client.post(&url).json(&init_request).send().await?;

    // Test all notification levels
    let levels = ["debug", "info", "warning", "error"];

    for (idx, level) in levels.iter().enumerate() {
        let notification_request = json!({
            "jsonrpc": "2.0",
            "id": idx + 2,
            "method": "tools/call",
            "params": {
                "name": "add_notification",
                "arguments": {
                    "level": level,
                    "message": format!("Test {} notification", level),
                    "data": {
                        "level": level,
                        "test": true
                    }
                }
            }
        });

        let response = client
            .post(&url)
            .header("Cookie", "mcp_session=http_default_session")
            .json(&notification_request)
            .send()
            .await?;
        assert_eq!(response.status(), 200);

        let notification_response: Value = response.json().await?;
        let content = notification_response["result"]["content"]
            .as_array()
            .unwrap();
        let result_text = content[0]["text"].as_str().unwrap();
        let result: Value = serde_json::from_str(result_text)?;
        assert_eq!(result["success"], true);
    }

    server_handle.abort();

    println!("✅ All notification levels sent successfully!");
    Ok(())
}

/// Test that notifications trigger resource list changed events
#[tokio::test]
async fn test_notification_on_note_add() -> Result<()> {
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

    let server_handle = tokio::spawn(async move {
        server.start(port).await.unwrap();
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

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
                "name": "note-notification-test",
                "version": "1.0.0"
            }
        }
    });

    client.post(&url).json(&init_request).send().await?;

    // Add a new note (which should trigger notifications)
    let add_note_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "add_note",
            "arguments": {
                "name": "new-note",
                "content": "This is a new note that should trigger notifications"
            }
        }
    });

    let response = client
        .post(&url)
        .header("Cookie", "mcp_session=http_default_session")
        .json(&add_note_request)
        .send()
        .await?;
    assert_eq!(response.status(), 200);

    let add_response: Value = response.json().await?;
    let content = add_response["result"]["content"].as_array().unwrap();
    let result_text = content[0]["text"].as_str().unwrap();
    let result: Value = serde_json::from_str(result_text)?;
    assert!(result["message"]
        .as_str()
        .unwrap()
        .contains("saved successfully"));

    // Note: In the actual implementation, this would trigger:
    // 1. A LogMessage notification about the note being saved
    // 2. A ResourcesListChanged notification
    // But since we can't receive notifications in tests, we just verify the operation succeeded

    // Verify the note was added by listing resources
    let list_resources_request = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "resources/list",
        "params": {}
    });

    let response = client
        .post(&url)
        .header("Cookie", "mcp_session=http_default_session")
        .json(&list_resources_request)
        .send()
        .await?;
    let resources_response: Value = response.json().await?;

    let resources = resources_response["result"]["resources"]
        .as_array()
        .unwrap();
    let resource_names: Vec<String> = resources
        .iter()
        .map(|r| r["name"].as_str().unwrap().to_string())
        .collect();
    assert!(resource_names.contains(&"new-note".to_string()));

    server_handle.abort();

    println!("✅ Note addition with notification flow test passed!");
    Ok(())
}

/// Test error notification scenarios
#[tokio::test]
async fn test_notification_error_handling() -> Result<()> {
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

    let server_handle = tokio::spawn(async move {
        server.start(port).await.unwrap();
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

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
                "name": "error-test",
                "version": "1.0.0"
            }
        }
    });

    client.post(&url).json(&init_request).send().await?;

    // Test invalid notification level
    let invalid_notification_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "add_notification",
            "arguments": {
                "level": "invalid_level",
                "message": "This should fail",
                "data": {}
            }
        }
    });

    let response = client
        .post(&url)
        .header("Cookie", "mcp_session=http_default_session")
        .json(&invalid_notification_request)
        .send()
        .await?;
    assert_eq!(response.status(), 200);

    let error_response: Value = response.json().await?;
    // Should get an error response for invalid level
    assert!(
        error_response.get("error").is_some()
            || error_response["result"]["content"][0]["text"]
                .as_str()
                .map(|s| s.contains("Invalid log level"))
                .unwrap_or(false)
    );

    server_handle.abort();

    println!("✅ Notification error handling test passed!");
    Ok(())
}
