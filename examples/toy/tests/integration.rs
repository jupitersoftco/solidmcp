//! Integration tests for the toy notes server using new framework API

use {
    anyhow::Result,
    futures_util::{SinkExt, StreamExt},
    schemars::JsonSchema,
    serde::{Deserialize, Serialize},
    serde_json::{json, Value},
    std::{fs, path::PathBuf, sync::Arc, time::Duration},
    tempfile::TempDir,
    tokio::time::timeout,
    tokio_tungstenite::{connect_async, tungstenite::Message},
    toy_notes_server::{NotesContext, NotesResourceProvider},
};

// Schema definitions
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct AddNote {
    name: String,
    content: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct AddNoteResult {
    message: String,
    success: bool,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct ListNotes {}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct ListNotesResult {
    notes: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct SendNotification {
    level: String,
    message: String,
    data: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct NotificationResult {
    success: bool,
}

/// Create a test server using the new framework with real NotesContext
async fn create_test_server(notes_dir: PathBuf) -> Result<solidmcp::McpServer> {
    use solidmcp::framework::McpServerBuilder;

    // Ensure directory exists
    if !notes_dir.exists() {
        fs::create_dir_all(&notes_dir)?;
    }

    let context = NotesContext::new(notes_dir);
    context.load_notes().await?;

    McpServerBuilder::new(context, "toy-notes-server", "0.1.0")
        .with_tool(
            "add_note",
            "Add a new note",
            |input: AddNote, ctx: Arc<NotesContext>, notify| async move {
                ctx.save_note(&input.name, &input.content).await?;

                // Clean notification API
                notify.info(&format!("Note '{}' added", input.name))?;

                Ok(AddNoteResult {
                    message: format!("Note '{}' added successfully", input.name),
                    success: true,
                })
            },
        )
        .with_tool(
            "list_notes",
            "List all notes",
            |_input: ListNotes, ctx: Arc<NotesContext>, _notify| async move {
                let notes = ctx.list_notes().await;
                Ok(ListNotesResult { notes })
            },
        )
        .with_tool(
            "send_notification",
            "Send a notification",
            |input: SendNotification, _ctx: Arc<NotesContext>, notify| async move {
                // Clean notification API
                match input.level.as_str() {
                    "debug" => notify.debug(&input.message)?,
                    "info" => notify.info(&input.message)?,
                    "warning" => notify.warn(&input.message)?,
                    "error" => notify.error(&input.message)?,
                    _ => return Err(anyhow::anyhow!("Invalid log level: {}", input.level)),
                }

                Ok(NotificationResult { success: true })
            },
        )
        .with_resource_provider(Box::new(NotesResourceProvider))
        .build()
        .await
}

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

/// Helper to start a test server and return the port
async fn start_test_server(notes_dir: PathBuf) -> Result<u16> {
    // Find available port
    let port = {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        let port = addr.port();
        drop(listener);
        port
    };

    // Create and start server
    let mut server = create_test_server(notes_dir).await?;
    tokio::spawn(async move {
        server.start(port).await.unwrap();
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(500)).await;
    Ok(port)
}

#[tokio::test]
async fn test_websocket_basic_flow() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let port = start_test_server(temp_dir.path().to_path_buf()).await?;

    // Connect to WebSocket
    let (ws_stream, _) = connect_async(&format!("ws://127.0.0.1:{}/mcp", port)).await?;
    let (mut write, mut read) = ws_stream.split();

    // Send initialize request
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

    write.send(Message::Text(init_request.to_string())).await?;
    let response = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
    let parsed: Value = serde_json::from_str(&response)?;

    assert_eq!(parsed["result"]["protocolVersion"], "2025-06-18");
    assert_eq!(parsed["result"]["serverInfo"]["name"], "toy-notes-server");

    // Test add_note tool
    let add_note_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "add_note",
            "arguments": {
                "name": "test",
                "content": "This is a test note"
            }
        }
    });

    write
        .send(Message::Text(add_note_request.to_string()))
        .await?;
    let response = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
    let parsed: Value = serde_json::from_str(&response)?;

    assert_eq!(parsed["result"]["success"], true);
    assert!(parsed["result"]["message"]
        .as_str()
        .unwrap()
        .contains("test"));

    Ok(())
}

#[tokio::test]
async fn test_list_tools() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let port = start_test_server(temp_dir.path().to_path_buf()).await?;

    // Connect and initialize
    let (ws_stream, _) = connect_async(&format!("ws://127.0.0.1:{}/mcp", port)).await?;
    let (mut write, mut read) = ws_stream.split();

    // Initialize
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    });
    write.send(Message::Text(init_request.to_string())).await?;
    receive_ws_message(&mut read, Duration::from_secs(5)).await?;

    // List tools
    let list_tools_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list"
    });

    write
        .send(Message::Text(list_tools_request.to_string()))
        .await?;
    let response = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
    let parsed: Value = serde_json::from_str(&response)?;

    let tools = parsed["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 3);

    let tool_names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();

    assert!(tool_names.contains(&"add_note"));
    assert!(tool_names.contains(&"list_notes"));
    assert!(tool_names.contains(&"send_notification"));

    Ok(())
}

#[tokio::test]
async fn test_note_persistence() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let notes_dir = temp_dir.path().to_path_buf();
    let port = start_test_server(notes_dir.clone()).await?;

    // Connect and initialize
    let (ws_stream, _) = connect_async(&format!("ws://127.0.0.1:{}/mcp", port)).await?;
    let (mut write, mut read) = ws_stream.split();

    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    });
    write.send(Message::Text(init_request.to_string())).await?;
    receive_ws_message(&mut read, Duration::from_secs(5)).await?;

    // Add a note
    let add_note_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "add_note",
            "arguments": {
                "name": "persistent_test",
                "content": "This note should persist to disk"
            }
        }
    });

    write
        .send(Message::Text(add_note_request.to_string()))
        .await?;
    receive_ws_message(&mut read, Duration::from_secs(5)).await?;

    // Check file was created
    let note_file = notes_dir.join("persistent_test.md");
    assert!(note_file.exists());

    let content = fs::read_to_string(&note_file)?;
    assert_eq!(content, "This note should persist to disk");

    Ok(())
}

#[tokio::test]
async fn test_list_resources() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let port = start_test_server(temp_dir.path().to_path_buf()).await?;

    // Connect and initialize
    let (ws_stream, _) = connect_async(&format!("ws://127.0.0.1:{}/mcp", port)).await?;
    let (mut write, mut read) = ws_stream.split();

    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    });
    write.send(Message::Text(init_request.to_string())).await?;
    receive_ws_message(&mut read, Duration::from_secs(5)).await?;

    // Add a test note first
    let add_note_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "add_note",
            "arguments": {
                "name": "resource_test",
                "content": "This note will be exposed as a resource"
            }
        }
    });

    write
        .send(Message::Text(add_note_request.to_string()))
        .await?;
    receive_ws_message(&mut read, Duration::from_secs(5)).await?;

    // List resources
    let list_resources_request = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "resources/list"
    });

    write
        .send(Message::Text(list_resources_request.to_string()))
        .await?;
    let response = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
    let parsed: Value = serde_json::from_str(&response)?;

    let resources = parsed["result"]["resources"].as_array().unwrap();
    assert_eq!(resources.len(), 1);

    let resource = &resources[0];
    assert_eq!(resource["uri"], "note://resource_test");
    assert_eq!(resource["name"], "resource_test");
    assert_eq!(resource["mimeType"], "text/markdown");
    assert!(resource["description"]
        .as_str()
        .unwrap()
        .contains("Markdown note"));

    Ok(())
}

#[tokio::test]
async fn test_read_resource() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let port = start_test_server(temp_dir.path().to_path_buf()).await?;

    // Connect and initialize
    let (ws_stream, _) = connect_async(&format!("ws://127.0.0.1:{}/mcp", port)).await?;
    let (mut write, mut read) = ws_stream.split();

    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    });
    write.send(Message::Text(init_request.to_string())).await?;
    receive_ws_message(&mut read, Duration::from_secs(5)).await?;

    // Add a test note
    let note_content = "# Resource Test\n\nThis note is accessible as an MCP resource.";
    let add_note_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "add_note",
            "arguments": {
                "name": "markdown_note",
                "content": note_content
            }
        }
    });

    write
        .send(Message::Text(add_note_request.to_string()))
        .await?;
    receive_ws_message(&mut read, Duration::from_secs(5)).await?;

    // Read the resource
    let read_resource_request = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "resources/read",
        "params": {
            "uri": "note://markdown_note"
        }
    });

    write
        .send(Message::Text(read_resource_request.to_string()))
        .await?;
    let response = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
    let parsed: Value = serde_json::from_str(&response)?;

    let result = &parsed["result"];
    let contents = result["contents"].as_array().unwrap();
    assert_eq!(contents.len(), 1);

    let content = &contents[0];
    assert_eq!(content["uri"], "note://markdown_note");
    assert_eq!(content["mimeType"], "text/markdown");
    assert_eq!(content["text"], note_content);

    Ok(())
}

#[tokio::test]
async fn test_resource_not_found() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let port = start_test_server(temp_dir.path().to_path_buf()).await?;

    // Connect and initialize
    let (ws_stream, _) = connect_async(&format!("ws://127.0.0.1:{}/mcp", port)).await?;
    let (mut write, mut read) = ws_stream.split();

    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    });
    write.send(Message::Text(init_request.to_string())).await?;
    receive_ws_message(&mut read, Duration::from_secs(5)).await?;

    // Try to read a non-existent resource
    let read_resource_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "resources/read",
        "params": {
            "uri": "note://nonexistent"
        }
    });

    write
        .send(Message::Text(read_resource_request.to_string()))
        .await?;
    let response = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
    let parsed: Value = serde_json::from_str(&response)?;

    // Should get an error response
    assert!(parsed["error"].is_object());
    assert!(parsed["error"]["message"]
        .as_str()
        .unwrap()
        .contains("Resource not found"));

    Ok(())
}
