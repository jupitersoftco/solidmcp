//! Typed handler implementation demonstrating schemars usage

use {
    anyhow::Result,
    async_trait::async_trait,
    schemars::JsonSchema,
    serde::{Deserialize, Serialize},
    serde_json::{json, Value},
    solidmcp::{
        LogLevel, McpContext, McpHandler, McpNotification, PromptArgument, PromptContent,
        PromptInfo, PromptMessage, ResourceContent, ResourceInfo, ToolDefinition,
        TypedToolDefinition,
    },
    std::{collections::HashMap, fs, path::PathBuf, sync::Arc},
    tokio::sync::RwLock,
};

/// Input schema for adding a note
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AddNoteInput {
    /// The name of the note (without .md extension)
    pub name: String,
    /// The content of the note in markdown format
    pub content: String,
}

/// Output schema for adding a note
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AddNoteOutput {
    /// Success message
    pub message: String,
    /// Timestamp when the note was created
    pub timestamp: String,
}

/// Input schema for listing notes (empty object)
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListNotesInput {
    // Empty - no parameters needed
}

/// Output schema for listing notes
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListNotesOutput {
    /// List of note names
    pub notes: Vec<String>,
    /// Total count of notes
    pub count: usize,
}

/// Input schema for reading a note
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReadNoteInput {
    /// The name of the note to read
    pub name: String,
}

/// Output schema for reading a note
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReadNoteOutput {
    /// The name of the note
    pub name: String,
    /// The content of the note
    pub content: String,
    /// File size in bytes
    pub size: usize,
}

/// Input schema for deleting a note
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DeleteNoteInput {
    /// The name of the note to delete
    pub name: String,
}

/// Output schema for deleting a note
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DeleteNoteOutput {
    /// Success message
    pub message: String,
    /// Name of the deleted note
    pub deleted_note: String,
}

/// Input schema for sending notifications
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SendNotificationInput {
    /// Log level for the notification
    #[schemars(regex(pattern = r"^(debug|info|warning|error)$"))]
    pub level: String,
    /// The notification message
    pub message: String,
    /// Optional additional data
    pub data: Option<Value>,
}

/// Output schema for sending notifications
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SendNotificationOutput {
    /// Whether the notification was sent successfully
    pub success: bool,
    /// Timestamp when the notification was sent
    pub sent_at: String,
}

/// Typed notes handler demonstrating schemars usage
#[derive(Clone)]
pub struct TypedNotesHandler {
    notes_dir: PathBuf,
    notes: Arc<RwLock<HashMap<String, String>>>,
}

impl TypedNotesHandler {
    /// Create a new typed notes handler
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
    async fn save_note(&self, name: &str, content: &str) -> Result<()> {
        let file_path = self.notes_dir.join(format!("{}.md", name));
        fs::write(&file_path, content)?;
        self.notes
            .write()
            .await
            .insert(name.to_string(), content.to_string());
        Ok(())
    }

    /// Delete a note from disk
    async fn delete_note(&self, name: &str) -> Result<()> {
        let file_path = self.notes_dir.join(format!("{}.md", name));
        if file_path.exists() {
            fs::remove_file(&file_path)?;
        }
        self.notes.write().await.remove(name);
        Ok(())
    }

    /// Get note content
    async fn get_note(&self, name: &str) -> Option<String> {
        self.notes.read().await.get(name).cloned()
    }

    /// List all note names
    async fn list_note_names(&self) -> Vec<String> {
        self.notes.read().await.keys().cloned().collect()
    }
}

#[async_trait]
impl McpHandler for TypedNotesHandler {
    async fn initialize(&self, _params: Value, _context: &McpContext) -> Result<Value> {
        Ok(json!({
            "protocolVersion": "2025-06-18",
            "capabilities": {
                "tools": {},
                "resources": {},
                "prompts": {}
            },
            "serverInfo": {
                "name": "typed-notes-server",
                "version": "0.1.0",
                "description": "A note-taking server demonstrating schemars usage"
            }
        }))
    }

    async fn list_tools(&self, _context: &McpContext) -> Result<Vec<ToolDefinition>> {
        Ok(vec![
            TypedToolDefinition::<AddNoteInput>::new(
                "add_note",
                "Save a note to the notes directory",
            )
            .to_tool_definition(),
            TypedToolDefinition::<ListNotesInput>::new("list_notes", "List all available notes")
                .to_tool_definition(),
            TypedToolDefinition::<ReadNoteInput>::new(
                "read_note",
                "Read the content of a specific note",
            )
            .to_tool_definition(),
            TypedToolDefinition::<DeleteNoteInput>::new(
                "delete_note",
                "Delete a note from the notes directory",
            )
            .to_tool_definition(),
            TypedToolDefinition::<SendNotificationInput>::new(
                "send_notification",
                "Send a notification to the client",
            )
            .to_tool_definition(),
        ])
    }

    async fn call_tool(&self, name: &str, arguments: Value, context: &McpContext) -> Result<Value> {
        match name {
            "add_note" => {
                // Parse and validate input using serde + schemars
                let input: AddNoteInput = serde_json::from_value(arguments)?;

                // Save the note
                self.save_note(&input.name, &input.content).await?;

                // Send notification if available
                if let Some(sender) = &context.notification_sender {
                    sender.send(McpNotification::LogMessage {
                        level: LogLevel::Info,
                        logger: Some("notes".to_string()),
                        message: format!("Note '{}' has been saved", input.name),
                        data: Some(json!({
                            "note_name": input.name,
                            "timestamp": chrono::Utc::now().to_rfc3339()
                        })),
                    })?;
                    sender.send(McpNotification::ResourcesListChanged)?;
                }

                let output = AddNoteOutput {
                    message: format!("Note '{}' saved successfully", input.name),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                };

                Ok(serde_json::to_value(output)?)
            }

            "list_notes" => {
                let _input: ListNotesInput = serde_json::from_value(arguments)?;
                let notes = self.list_note_names().await;

                let output = ListNotesOutput {
                    count: notes.len(),
                    notes,
                };

                Ok(serde_json::to_value(output)?)
            }

            "read_note" => {
                let input: ReadNoteInput = serde_json::from_value(arguments)?;

                if let Some(content) = self.get_note(&input.name).await {
                    let output = ReadNoteOutput {
                        name: input.name,
                        size: content.len(),
                        content,
                    };
                    Ok(serde_json::to_value(output)?)
                } else {
                    Err(anyhow::anyhow!("Note not found: {}", input.name))
                }
            }

            "delete_note" => {
                let input: DeleteNoteInput = serde_json::from_value(arguments)?;

                // Check if note exists first
                if self.get_note(&input.name).await.is_none() {
                    return Err(anyhow::anyhow!("Note not found: {}", input.name));
                }

                // Delete the note
                self.delete_note(&input.name).await?;

                // Send notification if available
                if let Some(sender) = &context.notification_sender {
                    sender.send(McpNotification::LogMessage {
                        level: LogLevel::Info,
                        logger: Some("notes".to_string()),
                        message: format!("Note '{}' has been deleted", input.name),
                        data: Some(json!({
                            "deleted_note": input.name,
                            "timestamp": chrono::Utc::now().to_rfc3339()
                        })),
                    })?;
                    sender.send(McpNotification::ResourcesListChanged)?;
                }

                let output = DeleteNoteOutput {
                    message: format!("Note '{}' deleted successfully", input.name),
                    deleted_note: input.name,
                };

                Ok(serde_json::to_value(output)?)
            }

            "send_notification" => {
                let input: SendNotificationInput = serde_json::from_value(arguments)?;

                let log_level = match input.level.as_str() {
                    "debug" => LogLevel::Debug,
                    "info" => LogLevel::Info,
                    "warning" => LogLevel::Warning,
                    "error" => LogLevel::Error,
                    _ => return Err(anyhow::anyhow!("Invalid log level: {}", input.level)),
                };

                if let Some(sender) = &context.notification_sender {
                    sender.send(McpNotification::LogMessage {
                        level: log_level,
                        logger: Some("custom".to_string()),
                        message: input.message,
                        data: input.data,
                    })?;

                    let output = SendNotificationOutput {
                        success: true,
                        sent_at: chrono::Utc::now().to_rfc3339(),
                    };

                    Ok(serde_json::to_value(output)?)
                } else {
                    Err(anyhow::anyhow!("Notification sender not available"))
                }
            }

            _ => Err(anyhow::anyhow!("Unknown tool: {}", name)),
        }
    }

    async fn list_resources(&self, _context: &McpContext) -> Result<Vec<ResourceInfo>> {
        let notes = self.notes.read().await;
        let mut resources = Vec::new();

        for (name, _content) in notes.iter() {
            resources.push(ResourceInfo {
                uri: format!("notes://{}", name),
                name: name.clone(),
                description: Some(format!("Note: {}", name)),
                mime_type: Some("text/markdown".to_string()),
            });
        }

        Ok(resources)
    }

    async fn read_resource(&self, uri: &str, _context: &McpContext) -> Result<ResourceContent> {
        if let Some(name) = uri.strip_prefix("notes://") {
            if let Some(content) = self.get_note(name).await {
                Ok(ResourceContent {
                    uri: uri.to_string(),
                    mime_type: Some("text/markdown".to_string()),
                    content,
                })
            } else {
                Err(anyhow::anyhow!("Note not found: {}", name))
            }
        } else {
            Err(anyhow::anyhow!("Invalid note URI: {}", uri))
        }
    }

    async fn list_prompts(&self, _context: &McpContext) -> Result<Vec<PromptInfo>> {
        Ok(vec![PromptInfo {
            name: "create_note_template".to_string(),
            description: Some("Generate a template for creating a new note".to_string()),
            arguments: vec![
                PromptArgument {
                    name: "topic".to_string(),
                    description: Some("The topic or subject of the note".to_string()),
                    required: true,
                },
                PromptArgument {
                    name: "format".to_string(),
                    description: Some(
                        "The format style (e.g., 'outline', 'journal', 'meeting')".to_string(),
                    ),
                    required: false,
                },
            ],
        }])
    }

    async fn get_prompt(
        &self,
        name: &str,
        arguments: Option<Value>,
        _context: &McpContext,
    ) -> Result<PromptContent> {
        match name {
            "create_note_template" => {
                let args = arguments.unwrap_or_default();
                let topic = args
                    .get("topic")
                    .and_then(|v| v.as_str())
                    .unwrap_or("General Notes");
                let format = args
                    .get("format")
                    .and_then(|v| v.as_str())
                    .unwrap_or("outline");

                let template = match format {
                    "journal" => format!(
                        "# {topic}\n\n## Date: {}\n\n## Thoughts\n\n\n## Key Points\n\n\n## Reflection\n\n",
                        chrono::Utc::now().format("%Y-%m-%d")
                    ),
                    "meeting" => format!(
                        "# Meeting Notes: {topic}\n\n## Date: {}\n## Attendees\n\n\n## Agenda\n\n\n## Discussion\n\n\n## Action Items\n\n\n## Next Steps\n\n",
                        chrono::Utc::now().format("%Y-%m-%d %H:%M UTC")
                    ),
                    _ => format!(
                        "# {topic}\n\n## Overview\n\n\n## Main Points\n\n- \n- \n- \n\n## Details\n\n\n## Conclusion\n\n"
                    ),
                };

                Ok(PromptContent {
                    messages: vec![PromptMessage {
                        role: "user".to_string(),
                        content: format!(
                            "Here's a template for your note about '{topic}':\n\n{template}"
                        ),
                    }],
                })
            }
            _ => Err(anyhow::anyhow!("Prompt not found: {}", name)),
        }
    }
}
