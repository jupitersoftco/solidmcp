//! Resource Integration Tests for Toy Example
//!
//! Tests to ensure resources are properly exposed

use anyhow::Result;
use serde_json::{json, Value};
use tempfile::TempDir;
use toy_notes_server::server::create_toy_server;

/// Test that resources are listed correctly
#[tokio::test]
async fn test_toy_server_resources() -> Result<()> {
    // Create temporary directory for notes
    let temp_dir = TempDir::new()?;
    let notes_dir = temp_dir.path().to_path_buf();

    // Create some test notes
    std::fs::write(notes_dir.join("note1.md"), "Content of note 1")?;
    std::fs::write(notes_dir.join("note2.md"), "Content of note 2")?;

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
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Create client
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
                "name": "resource-test",
                "version": "1.0.0"
            }
        }
    });

    let response = client.post(&url).json(&init_request).send().await?;
    assert_eq!(response.status(), 200);
    let init_response: Value = response.json().await?;

    // Check that resources capability is advertised
    println!("Init response: {:?}", init_response);
    assert!(init_response["result"]["capabilities"]["resources"].is_object());

    // List resources
    let resources_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "resources/list",
        "params": {}
    });

    let response = client
        .post(&url)
        .header("Cookie", "mcp_session=http_default_session")
        .json(&resources_request)
        .send()
        .await?;
    assert_eq!(response.status(), 200);
    let resources_response: Value = response.json().await?;

    // Should have 2 resources
    let resources = resources_response["result"]["resources"]
        .as_array()
        .unwrap();
    assert_eq!(resources.len(), 2);

    // Check resource details
    let resource_names: Vec<String> = resources
        .iter()
        .map(|r| r["name"].as_str().unwrap().to_string())
        .collect();
    assert!(resource_names.contains(&"note1".to_string()));
    assert!(resource_names.contains(&"note2".to_string()));

    // Check URIs
    for resource in resources {
        let uri = resource["uri"].as_str().unwrap();
        assert!(uri.starts_with("notes://"));
    }

    println!("✅ Resources are correctly exposed!");
    Ok(())
}

/// Test reading a resource
#[tokio::test]
async fn test_toy_server_read_resource() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let notes_dir = temp_dir.path().to_path_buf();

    // Create a test note
    std::fs::write(
        notes_dir.join("test-note.md"),
        "# Test Note\n\nThis is test content.",
    )?;

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
                "name": "resource-test",
                "version": "1.0.0"
            }
        }
    });

    client.post(&url).json(&init_request).send().await?;

    // Read resource
    let read_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "resources/read",
        "params": {
            "uri": "notes://test-note"
        }
    });

    let response = client
        .post(&url)
        .header("Cookie", "mcp_session=http_default_session")
        .json(&read_request)
        .send()
        .await?;
    assert_eq!(response.status(), 200);
    let read_response: Value = response.json().await?;

    // Check content
    let contents = read_response["result"]["contents"].as_array().unwrap();
    assert_eq!(contents.len(), 1);
    let content = contents[0]["text"].as_str().unwrap();
    assert_eq!(content, "# Test Note\n\nThis is test content.");

    // Check mime type
    let mime_type = contents[0]["mimeType"].as_str().unwrap();
    assert_eq!(mime_type, "text/markdown");

    println!("✅ Resource reading works correctly!");
    Ok(())
}
