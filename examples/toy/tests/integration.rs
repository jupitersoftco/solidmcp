//! End-to-end integration tests for the toy notes server

use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
// Removed unused import
use std::time::Duration;
use tempfile::TempDir;
use tokio::time::timeout;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use toy_notes_server::server::create_toy_server;
// Removed unused import

/// Test helper to receive a WebSocket message with timeout
async fn receive_ws_message<S>(read: &mut S, timeout_duration: Duration) -> Result<String>
where
    S: StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
{
    match timeout(timeout_duration, read.next()).await {
        Ok(Some(Ok(Message::Text(text)))) => Ok(text),
        Ok(Some(Ok(msg))) => Err(anyhow::anyhow!("Unexpected message type: {:?}", msg)),
        Ok(Some(Err(e))) => Err(e.into()),
        Ok(None) => Err(anyhow::anyhow!("Stream closed")),
        Err(_) => Err(anyhow::anyhow!("Timeout waiting for message")),
    }
}

/// Helper to start a toy server on a random port and return the port number
async fn start_toy_server(notes_dir: std::path::PathBuf) -> Result<u16> {
    // Find a random available port
    let port = {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        let port = addr.port();
        // Drop the listener to free the port
        drop(listener);
        port
    };

    // Create the toy server using the high-level API
    let server = create_toy_server(notes_dir).await?;

    // Start the server in the background
    tokio::spawn(async move {
        // Start the server using the high-level API
        server.start(port).await.unwrap();
    });

    // Give the server time to start
    tokio::time::sleep(Duration::from_millis(500)).await;

    Ok(port)
}

#[tokio::test]
async fn test_toy_server_end_to_end() -> Result<()> {
    // Create temporary directory for notes
    let temp_dir = TempDir::new()?;
    let notes_dir = temp_dir.path().to_path_buf();

    // Start the server on a random port
    let port = start_toy_server(notes_dir.clone()).await?;

    // Connect to the WebSocket endpoint
    let url = format!("ws://127.0.0.1:{}/mcp", port);
    let (ws_stream, _) = connect_async(&url).await?;
    let (mut write, mut read) = ws_stream.split();

    // Test 1: Initialize
    let init_message = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {
                "name": "toy-test-client",
                "version": "1.0.0"
            }
        }
    });

    write
        .send(Message::Text(serde_json::to_string(&init_message)?))
        .await?;
    let response = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
    let init_response: Value = serde_json::from_str(&response)?;

    assert_eq!(init_response["jsonrpc"], "2.0");
    assert_eq!(init_response["id"], 1);
    assert!(init_response["result"].is_object());
    assert_eq!(init_response["result"]["protocolVersion"], "2025-06-18");

    println!("✅ Initialize successful");

    // Test 2: List tools
    let list_tools_message = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    });

    write
        .send(Message::Text(serde_json::to_string(&list_tools_message)?))
        .await?;
    let response = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
    let tools_response: Value = serde_json::from_str(&response)?;

    assert_eq!(tools_response["jsonrpc"], "2.0");
    assert_eq!(tools_response["id"], 2);
    assert!(tools_response["result"]["tools"].is_array());

    let tools = tools_response["result"]["tools"].as_array().unwrap();
    // Should show custom tools: add_note, list_notes, add_notification
    assert_eq!(tools.len(), 3);

    // Verify we have the expected tools
    let tool_names: Vec<String> = tools
        .iter()
        .map(|tool| tool["name"].as_str().unwrap().to_string())
        .collect();
    assert!(tool_names.contains(&"add_note".to_string()));
    assert!(tool_names.contains(&"list_notes".to_string()));
    assert!(tool_names.contains(&"add_notification".to_string()));

    println!(
        "✅ Tools list successful - found {} tools: {:?}",
        tools.len(),
        tool_names
    );

    // Test 3: Call add_note tool (custom tool)
    let add_note_message = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "add_note",
            "arguments": {
                "name": "test-note",
                "content": "Hello from toy end-to-end test!"
            }
        }
    });

    write
        .send(Message::Text(serde_json::to_string(&add_note_message)?))
        .await?;
    let response = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
    let add_note_response: Value = serde_json::from_str(&response)?;

    assert_eq!(add_note_response["jsonrpc"], "2.0");
    assert_eq!(add_note_response["id"], 3);
    assert!(add_note_response["result"]["content"].is_array());

    println!("✅ Add note tool call successful");

    // Test 4: Call list_notes tool to verify the note was saved
    let list_notes_message = json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "tools/call",
        "params": {
            "name": "list_notes",
            "arguments": {}
        }
    });

    write
        .send(Message::Text(serde_json::to_string(&list_notes_message)?))
        .await?;
    let response = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
    let list_notes_response: Value = serde_json::from_str(&response)?;

    assert_eq!(list_notes_response["jsonrpc"], "2.0");
    assert_eq!(list_notes_response["id"], 4);
    assert!(list_notes_response["result"]["content"].is_array());

    // Parse the response content to check if our note is listed
    let content_array = list_notes_response["result"]["content"].as_array().unwrap();
    assert!(!content_array.is_empty());
    let content_text = content_array[0]["text"].as_str().unwrap();

    // The response should be a JSON string containing the notes list directly
    let notes_list: Value = serde_json::from_str(content_text).unwrap();
    let notes_array = notes_list.as_array().unwrap();
    assert_eq!(notes_array.len(), 1);
    assert_eq!(notes_array[0], "test-note");

    println!("✅ List notes tool call successful - found note: test-note");

    // Clean up
    write.send(Message::Close(None)).await?;

    println!("\n🎉 End-to-end test passed!");
    Ok(())
}

#[tokio::test]
async fn test_toy_server_http_endpoint() -> Result<()> {
    // Create temporary directory for notes
    let temp_dir = TempDir::new()?;
    let notes_dir = temp_dir.path().to_path_buf();

    // Start the server on a random port
    let port = start_toy_server(notes_dir.clone()).await?;

    // Test HTTP endpoint
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}/mcp", port);

    // Test initialization via HTTP
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {
                "name": "toy-http-test-client",
                "version": "1.0.0"
            }
        }
    });

    let response = client.post(&url).json(&init_request).send().await?;

    assert_eq!(response.status(), 200);
    let init_response: Value = response.json().await?;

    assert_eq!(init_response["jsonrpc"], "2.0");
    assert_eq!(init_response["id"], 1);
    assert!(init_response["result"].is_object());

    println!("✅ HTTP initialization successful");

    // Test tools list via HTTP
    let tools_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    });

    let response = client.post(&url).json(&tools_request).send().await?;

    assert_eq!(response.status(), 200);
    let tools_response: Value = response.json().await?;

    assert_eq!(tools_response["jsonrpc"], "2.0");
    assert_eq!(tools_response["id"], 2);
    assert!(tools_response["result"]["tools"].is_array());

    let tools = tools_response["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 3); // Should have 3 custom tools

    println!(
        "✅ HTTP tools list successful - found {} tools",
        tools.len()
    );
    println!("\n🎉 HTTP endpoint test passed!");

    Ok(())
}

#[test]
fn test_notes_directory_creation() {
    // This is a simple test to verify the notes directory logic
    use std::path::PathBuf;

    let notes_dir = std::env::var("NOTES_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let mut path = std::env::current_dir().unwrap();
            path.push("notes");
            path
        });

    assert!(notes_dir.is_absolute());
    println!("Notes directory would be: {}", notes_dir.display());
}

#[tokio::test]
async fn test_toy_server_note_storage() -> Result<()> {
    use toy_notes_server::server::NotesStorage;

    // Create temporary directory for notes
    let temp_dir = TempDir::new()?;
    let notes_dir = temp_dir.path().to_path_buf();

    // Test the storage directly
    let storage = NotesStorage::new(notes_dir.clone());
    storage.load_notes().await?;

    // Save a note
    storage
        .save_note("test-note", "This is a test note content")
        .await?;

    // List notes
    let notes = storage.list_notes().await;
    assert_eq!(notes.len(), 1);
    assert!(notes.contains(&"test-note".to_string()));

    // Verify file was created
    let note_file = notes_dir.join("test-note.md");
    assert!(note_file.exists());

    let content = std::fs::read_to_string(&note_file)?;
    assert_eq!(content, "This is a test note content");

    println!("✅ Note storage test passed!");
    Ok(())
}
