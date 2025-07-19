//! Toy Notes Server - New Framework Demonstration
//!
//! A simple note-taking server built with the new SolidMCP framework demonstrating
//! minimal boilerplate and automatic tool registration.

use {
    anyhow::Result,
    schemars::JsonSchema,
    serde::{Deserialize, Serialize},
    serde_json::Value,
    solidmcp::{framework::McpServerBuilder, LogLevel},
    std::{collections::HashMap, env, fs, path::PathBuf, sync::Arc},
    tokio::sync::RwLock,
    tracing::info,
};

/// Notes context for the server
#[derive(Debug)]
struct NotesContext {
    notes_dir: PathBuf,
    notes: RwLock<HashMap<String, String>>,
}

impl NotesContext {
    fn new(notes_dir: PathBuf) -> Self {
        Self {
            notes_dir,
            notes: RwLock::new(HashMap::new()),
        }
    }

    async fn load_notes(&self) -> Result<()> {
        if !self.notes_dir.exists() {
            fs::create_dir_all(&self.notes_dir)?;
        }

        let mut notes = self.notes.write().await;
        for entry in fs::read_dir(&self.notes_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("md") {
                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                    let content = fs::read_to_string(&path)?;
                    notes.insert(name.to_string(), content);
                }
            }
        }
        Ok(())
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

// Input/Output schemas using schemars
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct AddNote {
    /// The name of the note
    name: String,
    /// The content in markdown format
    content: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct NoteResult {
    message: String,
    success: bool,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct ListNotes {
    // Empty - no input needed
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct NotesList {
    notes: Vec<String>,
    count: usize,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct SendNotification {
    #[schemars(regex(pattern = r"^(debug|info|warning|error)$"))]
    level: String,
    message: String,
    data: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct NotificationResult {
    success: bool,
    sent_at: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    info!("🚀 Starting Toy Notes Server with New Framework");

    // Setup notes directory
    let notes_dir = env::var("NOTES_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let mut path = env::current_dir().unwrap();
            path.push("notes");
            path
        });

    info!("📁 Notes directory: {}", notes_dir.display());

    // Create context and load existing notes
    let context = NotesContext::new(notes_dir);
    context.load_notes().await?;

    // Build server using new framework - minimal boilerplate!
    let mut server = McpServerBuilder::new(context, "toy-notes-server", "0.1.0")
        .with_tool(
            "add_note",
            "Add a new note",
            |input: AddNote, ctx: Arc<NotesContext>, mcp| {
                let notification_sender = mcp.notification_sender.clone();
                async move {
                    ctx.save_note(&input.name, &input.content).await?;

                    // Send notification easily
                    if let Some(sender) = notification_sender {
                        let _ = sender.send(solidmcp::McpNotification::LogMessage {
                            level: LogLevel::Info,
                            logger: Some("notes".to_string()),
                            message: format!("Note '{}' saved", input.name),
                            data: None,
                        });
                    }

                    Ok(NoteResult {
                        message: format!("Note '{}' saved successfully", input.name),
                        success: true,
                    })
                }
            },
        )
        .with_tool(
            "list_notes",
            "List all available notes",
            |_input: ListNotes, ctx: Arc<NotesContext>, _mcp| async move {
                let notes = ctx.list_notes().await;
                Ok(NotesList {
                    count: notes.len(),
                    notes,
                })
            },
        )
        .with_tool(
            "send_notification",
            "Send a notification to the client",
            |input: SendNotification, _ctx: Arc<NotesContext>, mcp| {
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

                    Ok(NotificationResult {
                        success: true,
                        sent_at: chrono::Utc::now().to_rfc3339(),
                    })
                }
            },
        )
        .build()
        .await?;

    // Get port and start server
    let port = env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);

    info!("🌐 Starting server on port {}", port);
    info!("📡 Connect via:");
    info!("   WebSocket: ws://localhost:{}/mcp", port);
    info!("   HTTP: http://localhost:{}/mcp", port);
    info!("");
    info!("💡 This server demonstrates the new SolidMCP framework:");
    info!("   - Automatic tool registration and routing");
    info!("   - Compile-time schema generation with schemars");
    info!("   - Zero boilerplate initialization");
    info!("   - Type-safe tool handlers");
    info!("");
    info!("Available tools (auto-generated with JSON schemas):");
    info!("  - add_note: Add a new note with validation");
    info!("  - list_notes: List all available notes");
    info!("  - send_notification: Send notifications to client");

    server.start(port).await?;
    Ok(())
}
