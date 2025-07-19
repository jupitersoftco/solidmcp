//! Toy server implementation with notification support

use {
    anyhow::Result,
    async_trait::async_trait,
    serde_json::{json, Value},
    solidmcp::{
        ExtendedToolDefinition, HighLevelMcpServer, LogLevel, McpNotification, McpServerBuilder,
        McpTool, ToolContext,
    },
    std::{collections::HashMap, fs, path::PathBuf, sync::Arc},
    tokio::sync::RwLock,
};

/// A simple note storage system
#[derive(Clone)]
pub struct NotesStorage {
    notes_dir: PathBuf,
    notes: Arc<RwLock<HashMap<String, String>>>,
}

impl NotesStorage {
    pub fn new(notes_dir: PathBuf) -> Self {
        Self {
            notes_dir,
            notes: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Load existing notes from disk
    pub async fn load_notes(&self) -> Result<()> {
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

    /// Save a note to disk
    pub async fn save_note(&self, name: &str, content: &str) -> Result<()> {
        let file_path = self.notes_dir.join(format!("{}.md", name));
        fs::write(&file_path, content)?;
        self.notes
            .write()
            .await
            .insert(name.to_string(), content.to_string());
        Ok(())
    }

    /// List all notes
    pub async fn list_notes(&self) -> Vec<String> {
        self.notes.read().await.keys().cloned().collect()
    }
}

/// Tool for adding notes
pub struct AddNoteTool {
    storage: NotesStorage,
}

impl AddNoteTool {
    pub fn new(storage: NotesStorage) -> Self {
        Self { storage }
    }
}

#[async_trait]
impl McpTool for AddNoteTool {
    fn definition(&self) -> ExtendedToolDefinition {
        ExtendedToolDefinition {
            name: "add_note".to_string(),
            description: "Save a note to the notes directory".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "The name of the note (without .md extension)"
                    },
                    "content": {
                        "type": "string",
                        "description": "The content of the note"
                    }
                },
                "required": ["name", "content"]
            }),
            output_schema: json!({
                "type": "object",
                "properties": {
                    "message": {
                        "type": "string"
                    }
                }
            }),
        }
    }

    async fn execute(&self, arguments: Value, context: &ToolContext) -> Result<Value> {
        let name = arguments["name"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'name' parameter"))?;
        let content = arguments["content"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'content' parameter"))?;

        // Save the note
        self.storage.save_note(name, content).await?;

        // Send notification if available
        if let Some(sender) = &context.notification_sender {
            sender.send(McpNotification::LogMessage {
                level: LogLevel::Info,
                logger: Some("notes".to_string()),
                message: format!("Note '{}' has been saved", name),
                data: Some(json!({
                    "note_name": name,
                    "timestamp": chrono::Utc::now().to_rfc3339()
                })),
            })?;

            // Also send a resources list changed notification
            sender.send(McpNotification::ResourcesListChanged)?;
        }

        Ok(json!({
            "message": format!("Note '{}' saved successfully", name)
        }))
    }
}

/// Tool for listing notes
pub struct ListNotesTool {
    storage: NotesStorage,
}

impl ListNotesTool {
    pub fn new(storage: NotesStorage) -> Self {
        Self { storage }
    }
}

#[async_trait]
impl McpTool for ListNotesTool {
    fn definition(&self) -> ExtendedToolDefinition {
        ExtendedToolDefinition {
            name: "list_notes".to_string(),
            description: "List all available notes".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
            output_schema: json!({
                "type": "array",
                "items": {
                    "type": "string"
                }
            }),
        }
    }

    async fn execute(&self, _arguments: Value, _context: &ToolContext) -> Result<Value> {
        let notes = self.storage.list_notes().await;
        Ok(json!(notes))
    }
}

/// Tool for adding notifications
pub struct AddNotificationTool {}

#[async_trait]
impl McpTool for AddNotificationTool {
    fn definition(&self) -> ExtendedToolDefinition {
        ExtendedToolDefinition {
            name: "add_notification".to_string(),
            description: "Send a notification to the client".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "level": {
                        "type": "string",
                        "enum": ["debug", "info", "warning", "error"],
                        "description": "The log level of the notification"
                    },
                    "message": {
                        "type": "string",
                        "description": "The notification message"
                    },
                    "data": {
                        "type": "object",
                        "description": "Optional additional data for the notification"
                    }
                },
                "required": ["level", "message"]
            }),
            output_schema: json!({
                "type": "object",
                "properties": {
                    "success": {
                        "type": "boolean"
                    }
                }
            }),
        }
    }

    async fn execute(&self, arguments: Value, context: &ToolContext) -> Result<Value> {
        let level = arguments["level"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'level' parameter"))?;
        let message = arguments["message"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'message' parameter"))?;
        let data = arguments.get("data").cloned();

        let log_level = match level {
            "debug" => LogLevel::Debug,
            "info" => LogLevel::Info,
            "warning" => LogLevel::Warning,
            "error" => LogLevel::Error,
            _ => return Err(anyhow::anyhow!("Invalid log level: {}", level)),
        };

        // Send notification if available
        if let Some(sender) = &context.notification_sender {
            sender.send(McpNotification::LogMessage {
                level: log_level,
                logger: Some("custom".to_string()),
                message: message.to_string(),
                data,
            })?;
            Ok(json!({
                "success": true
            }))
        } else {
            Err(anyhow::anyhow!("Notification sender not available"))
        }
    }
}

/// Create and configure the toy notes server
pub async fn create_toy_server(notes_dir: PathBuf) -> Result<HighLevelMcpServer> {
    // Create storage
    let storage = NotesStorage::new(notes_dir);
    storage.load_notes().await?;

    // Create tools
    let add_note_tool = AddNoteTool::new(storage.clone());
    let list_notes_tool = ListNotesTool::new(storage.clone());
    let add_notification_tool = AddNotificationTool {};

    // Build server with all tools
    let mut builder = McpServerBuilder::new();
    builder = builder.add_tool(add_note_tool);
    builder = builder.add_tool(list_notes_tool);
    builder = builder.add_tool(add_notification_tool);

    let server = builder.build().await?;

    Ok(server)
}
