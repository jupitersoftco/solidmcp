//! Integration tests for the toy notes server using new framework API

use {
    anyhow::Result,
    futures_util::{SinkExt, StreamExt},
    schemars::JsonSchema,
    serde::{Deserialize, Serialize},
    serde_json::{json, Value},
    solidmcp::{framework::McpServerBuilder, LogLevel},
    std::{collections::HashMap, fs, path::PathBuf, sync::Arc, time::Duration},
    tempfile::TempDir,
    tokio::{sync::RwLock, time::timeout},
    tokio_tungstenite::{connect_async, tungstenite::Message},
};

/// Test context for notes server
#[derive(Debug)]
struct TestNotesContext {
    notes_dir: PathBuf,
    notes: RwLock<HashMap<String, String>>,
}

impl TestNotesContext {
    fn new(notes_dir: PathBuf) -> Self {
        Self {
            notes_dir,
            notes: RwLock::new(HashMap::new()),
        }
    }

    async fn save_note(&self, name: &str, content: &str) -> Result<()> {
        let file_path = self.notes_dir.join(format!("{}.md", name));
        fs::write(&file_path, content)?;
        self.notes
            .write()
            .await
            .insert(name.to_string(), content.to_string());
        Ok(())
    }

    async fn list_notes(&self) -> Vec<String> {
        self.notes.read().await.keys().cloned().collect()
    }
}

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

/// Create a test server using the new framework
async fn create_test_server(notes_dir: PathBuf) -> Result<solidmcp::McpServer> {
    // Ensure directory exists
    if !notes_dir.exists() {
        fs::create_dir_all(&notes_dir)?;
    }

    let context = TestNotesContext::new(notes_dir);

    McpServerBuilder::new(context, "toy-notes-server", "0.1.0")
        .with_tool(
            "add_note",
            "Add a new note",
            |input: AddNote, ctx: Arc<TestNotesContext>, mcp| {
                let notification_sender = mcp.notification_sender.clone();
                async move {
                    ctx.save_note(&input.name, &input.content).await?;

                    // Send notification
                    if let Some(sender) = notification_sender {
                        let _ = sender.send(solidmcp::McpNotification::LogMessage {
                            level: LogLevel::Info,
                            logger: Some("notes".to_string()),
                            message: format!("Note '{}' added", input.name),
                            data: None,
                        });
                    }

                    Ok(AddNoteResult {
                        message: format!("Note '{}' added successfully", input.name),
                        success: true,
                    })
                }
            },
        )
        .with_tool(
            "list_notes",
            "List all notes",
            |_input: ListNotes, ctx: Arc<TestNotesContext>, _mcp| async move {
                let notes = ctx.list_notes().await;
                Ok(ListNotesResult { notes })
            },
        )
        .with_tool(
            "send_notification",
            "Send a notification",
            |input: SendNotification, _ctx: Arc<TestNotesContext>, mcp| {
                let notification_sender = mcp.notification_sender.clone();
                async move {
                    let level = match input.level.as_str() {
                        "debug" => LogLevel::Debug,
                        "info" => LogLevel::Info,
                        "warning" => LogLevel::Warning,
                        "error" => LogLevel::Error,
                        _ => return Err(anyhow::anyhow!("Invalid log level: {}", input.level)),
                    };

                    if let Some(sender) = notification_sender {
                        let _ = sender.send(solidmcp::McpNotification::LogMessage {
                            level,
                            logger: Some("custom".to_string()),
                            message: input.message,
                            data: input.data,
                        });
                    }

                    Ok(NotificationResult { success: true })
                }
            },
        )
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
