//! Integration tests for the toy notes server using new framework API

use {
    anyhow::Result,
    chrono,
    futures_util::{SinkExt, StreamExt},
    schemars::JsonSchema,
    serde::{Deserialize, Serialize},
    serde_json::{json, Value},
    std::{fs, path::PathBuf, sync::Arc, time::Duration},
    tempfile::TempDir,
    tokio::time::timeout,
    tokio_tungstenite::{connect_async, tungstenite::Message},
    toy_notes_server::{NotesContext, NotesPromptProvider, NotesResourceProvider},
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
    use solidmcp::McpServerBuilder;

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
        .with_prompt_provider(Box::new(NotesPromptProvider))
        .build()
        .await
}

/// Test helper to receive a WebSocket message with timeout
async fn receive_ws_message<S>(read: &mut S, timeout_duration: Duration) -> Result<String>
where
    S: StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
{
    match timeout(timeout_duration, read.next()).await {
        Ok(Some(Ok(Message::Text(text)))) => Ok(text.to_string()),
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

    write.send(Message::Text(init_request.to_string().into())).await?;
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
        .send(Message::Text(add_note_request.to_string().into()))
        .await?;
    let response = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
    let parsed: Value = serde_json::from_str(&response)?;
    
    assert_eq!(parsed["result"]["data"]["success"], true);
    assert!(parsed["result"]["data"]["message"]
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
    write.send(Message::Text(init_request.to_string().into())).await?;
    receive_ws_message(&mut read, Duration::from_secs(5)).await?;

    // List tools
    let list_tools_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list"
    });

    write
        .send(Message::Text(list_tools_request.to_string().into()))
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
    write.send(Message::Text(init_request.to_string().into())).await?;
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
        .send(Message::Text(add_note_request.to_string().into()))
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
    write.send(Message::Text(init_request.to_string().into())).await?;
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
        .send(Message::Text(add_note_request.to_string().into()))
        .await?;
    receive_ws_message(&mut read, Duration::from_secs(5)).await?;

    // List resources
    let list_resources_request = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "resources/list"
    });

    write
        .send(Message::Text(list_resources_request.to_string().into()))
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
    write.send(Message::Text(init_request.to_string().into())).await?;
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
        .send(Message::Text(add_note_request.to_string().into()))
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
        .send(Message::Text(read_resource_request.to_string().into()))
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
    write.send(Message::Text(init_request.to_string().into())).await?;
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
        .send(Message::Text(read_resource_request.to_string().into()))
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

#[tokio::test]
async fn test_list_prompts() -> Result<()> {
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
    write.send(Message::Text(init_request.to_string().into())).await?;
    receive_ws_message(&mut read, Duration::from_secs(5)).await?;

    // List prompts
    let list_prompts_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "prompts/list"
    });

    write
        .send(Message::Text(list_prompts_request.to_string().into()))
        .await?;
    let response = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
    let parsed: Value = serde_json::from_str(&response)?;

    let prompts = parsed["result"]["prompts"].as_array().unwrap();
    assert_eq!(prompts.len(), 3);

    let prompt_names: Vec<&str> = prompts
        .iter()
        .map(|p| p["name"].as_str().unwrap())
        .collect();

    assert!(prompt_names.contains(&"meeting_notes"));
    assert!(prompt_names.contains(&"task_note"));
    assert!(prompt_names.contains(&"daily_journal"));

    // Check meeting_notes prompt details
    let meeting_prompt = prompts
        .iter()
        .find(|p| p["name"] == "meeting_notes")
        .unwrap();
    assert_eq!(
        meeting_prompt["description"],
        "Template for creating structured meeting notes"
    );

    let arguments = meeting_prompt["arguments"].as_array().unwrap();
    assert_eq!(arguments.len(), 2);
    assert_eq!(arguments[0]["name"], "meeting_title");
    assert_eq!(arguments[0]["required"], true);
    assert_eq!(arguments[1]["name"], "attendees");
    assert_eq!(arguments[1]["required"], false);

    Ok(())
}

#[tokio::test]
async fn test_get_meeting_notes_prompt() -> Result<()> {
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
    write.send(Message::Text(init_request.to_string().into())).await?;
    receive_ws_message(&mut read, Duration::from_secs(5)).await?;

    // Get meeting_notes prompt with arguments
    let get_prompt_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "prompts/get",
        "params": {
            "name": "meeting_notes",
            "arguments": {
                "meeting_title": "Weekly Team Sync",
                "attendees": "Alice, Bob, Charlie"
            }
        }
    });

    write
        .send(Message::Text(get_prompt_request.to_string().into()))
        .await?;
    let response = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
    let parsed: Value = serde_json::from_str(&response)?;

    let result = &parsed["result"];
    let messages = result["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 1);

    let message = &messages[0];
    assert_eq!(message["role"], "user");

    let content = message["content"]["text"].as_str().unwrap();
    assert!(content.contains("# Weekly Team Sync"));
    assert!(content.contains("Alice, Bob, Charlie"));
    assert!(content.contains("## Agenda"));
    assert!(content.contains("## Action Items"));

    Ok(())
}

#[tokio::test]
async fn test_get_task_note_prompt() -> Result<()> {
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
    write.send(Message::Text(init_request.to_string().into())).await?;
    receive_ws_message(&mut read, Duration::from_secs(5)).await?;

    // Get task_note prompt with arguments
    let get_prompt_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "prompts/get",
        "params": {
            "name": "task_note",
            "arguments": {
                "task_name": "Implement user authentication",
                "priority": "high",
                "due_date": "2025-01-30"
            }
        }
    });

    write
        .send(Message::Text(get_prompt_request.to_string().into()))
        .await?;
    let response = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
    let parsed: Value = serde_json::from_str(&response)?;

    let result = &parsed["result"];
    let messages = result["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 1);

    let message = &messages[0];
    assert_eq!(message["role"], "user");

    let content = message["content"]["text"].as_str().unwrap();
    assert!(content.contains("# Task: Implement user authentication"));
    assert!(content.contains("**Priority**: high"));
    assert!(content.contains("**Due Date**: 2025-01-30"));
    assert!(content.contains("## Requirements"));
    assert!(content.contains("## Progress"));

    Ok(())
}

#[tokio::test]
async fn test_get_daily_journal_prompt() -> Result<()> {
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
    write.send(Message::Text(init_request.to_string().into())).await?;
    receive_ws_message(&mut read, Duration::from_secs(5)).await?;

    // Get daily_journal prompt with custom date
    let get_prompt_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "prompts/get",
        "params": {
            "name": "daily_journal",
            "arguments": {
                "date": "2025-01-19"
            }
        }
    });

    write
        .send(Message::Text(get_prompt_request.to_string().into()))
        .await?;
    let response = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
    let parsed: Value = serde_json::from_str(&response)?;

    let result = &parsed["result"];
    let messages = result["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 1);

    let message = &messages[0];
    assert_eq!(message["role"], "user");

    let content = message["content"]["text"].as_str().unwrap();
    assert!(content.contains("# Daily Journal - 2025-01-19"));
    assert!(content.contains("## How I'm Feeling"));
    assert!(content.contains("## Accomplishments"));
    assert!(content.contains("## Gratitude"));

    Ok(())
}

#[tokio::test]
async fn test_get_daily_journal_prompt_default_date() -> Result<()> {
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
    write.send(Message::Text(init_request.to_string().into())).await?;
    receive_ws_message(&mut read, Duration::from_secs(5)).await?;

    // Get daily_journal prompt without date (should use today)
    let get_prompt_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "prompts/get",
        "params": {
            "name": "daily_journal",
            "arguments": {}
        }
    });

    write
        .send(Message::Text(get_prompt_request.to_string().into()))
        .await?;
    let response = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
    let parsed: Value = serde_json::from_str(&response)?;

    let result = &parsed["result"];
    let messages = result["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 1);

    let message = &messages[0];
    assert_eq!(message["role"], "user");

    let content = message["content"]["text"].as_str().unwrap();
    // Should contain today's date in YYYY-MM-DD format
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    assert!(content.contains(&format!("# Daily Journal - {}", today)));

    Ok(())
}

#[tokio::test]
async fn test_get_nonexistent_prompt() -> Result<()> {
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
    write.send(Message::Text(init_request.to_string().into())).await?;
    receive_ws_message(&mut read, Duration::from_secs(5)).await?;

    // Try to get a non-existent prompt
    let get_prompt_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "prompts/get",
        "params": {
            "name": "nonexistent_prompt",
            "arguments": {}
        }
    });

    write
        .send(Message::Text(get_prompt_request.to_string().into()))
        .await?;
    let response = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
    let parsed: Value = serde_json::from_str(&response)?;

    // Should get an error response
    assert!(parsed["error"].is_object());
    assert!(parsed["error"]["message"]
        .as_str()
        .unwrap()
        .contains("Prompt not found"));

    Ok(())
}
