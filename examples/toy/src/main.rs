//! Toy Notes Server - SolidMCP Framework Example
//!
//! Demonstrates the new SolidMCP framework with minimal boilerplate and maximum type safety

use {
    anyhow::Result,
    async_trait::async_trait,
    schemars::JsonSchema,
    serde::{Deserialize, Serialize},
    serde_json::Value,
    solidmcp::{
        framework::{McpServerBuilder, ResourceProvider},
        handler::{ResourceContent, ResourceInfo},
    },
    std::{collections::HashMap, env, fs, path::PathBuf, sync::Arc},
    tokio::sync::RwLock,
    tracing::info,
};

/// Custom context for our notes server
#[derive(Debug)]
pub struct NotesContext {
    notes_dir: PathBuf,
    notes: RwLock<HashMap<String, String>>,
}

impl NotesContext {
    pub fn new(notes_dir: PathBuf) -> Self {
        Self {
            notes_dir,
            notes: RwLock::new(HashMap::new()),
        }
    }

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

    async fn save_note(&self, name: &str, content: &str) -> Result<()> {
        let file_path = self.notes_dir.join(format!("{}.md", name));
        fs::write(&file_path, content)?;
        self.notes
            .write()
            .await
            .insert(name.to_string(), content.to_string());
        Ok(())
    }

    async fn get_note(&self, name: &str) -> Option<String> {
        self.notes.read().await.get(name).cloned()
    }

    async fn list_notes(&self) -> Vec<String> {
        self.notes.read().await.keys().cloned().collect()
    }

    async fn delete_note(&self, name: &str) -> Result<()> {
        let file_path = self.notes_dir.join(format!("{}.md", name));
        if file_path.exists() {
            fs::remove_file(&file_path)?;
        }
        self.notes.write().await.remove(name);
        Ok(())
    }
}

/// Resource provider for notes - exposes notes as MCP resources
pub struct NotesResourceProvider;

#[async_trait]
impl ResourceProvider<NotesContext> for NotesResourceProvider {
    async fn list_resources(&self, context: Arc<NotesContext>) -> Result<Vec<ResourceInfo>> {
        let notes = context.list_notes().await;
        let mut resources = Vec::new();

        for note_name in notes {
            resources.push(ResourceInfo {
                uri: format!("note://{}", note_name),
                name: note_name.clone(),
                description: Some(format!("Markdown note: {}", note_name)),
                mime_type: Some("text/markdown".to_string()),
            });
        }

        Ok(resources)
    }

    async fn read_resource(
        &self,
        uri: &str,
        context: Arc<NotesContext>,
    ) -> Result<ResourceContent> {
        if let Some(note_name) = uri.strip_prefix("note://") {
            if let Some(content) = context.get_note(note_name).await {
                return Ok(ResourceContent {
                    uri: uri.to_string(),
                    mime_type: Some("text/markdown".to_string()),
                    content,
                });
            }
        }
        Err(anyhow::anyhow!("Resource not found: {}", uri))
    }
}

// Input/Output schemas - much simpler, just the data structures
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct AddNote {
    /// The name of the note
    pub name: String,
    /// The content in markdown format
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct NoteResult {
    pub message: String,
    pub timestamp: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ListNotes {
    // Empty - no input needed
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct NotesList {
    pub notes: Vec<String>,
    pub count: usize,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ReadNote {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct NoteContent {
    pub name: String,
    pub content: String,
    pub size: usize,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DeleteNote {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SendNotification {
    #[schemars(regex(pattern = r"^(debug|info|warning|error)$"))]
    pub level: String,
    pub message: String,
    pub data: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct NotificationResult {
    pub success: bool,
    pub sent_at: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    // Setup notes directory
    let notes_dir = env::var("NOTES_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let mut dir = env::temp_dir();
            dir.push("toy_notes_minimal");
            dir
        });

    info!("📁 Notes directory: {}", notes_dir.display());

    // Create context and load existing notes
    let context = NotesContext::new(notes_dir);
    context.load_notes().await?;

    // Build MCP server with minimal boilerplate - this is the key improvement!
    let mut server =
        McpServerBuilder::new(context, env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
            .with_tool(
                "add_note",
                "Save a note to the notes directory",
                |input: AddNote, ctx: Arc<NotesContext>, notify| async move {
                    ctx.save_note(&input.name, &input.content).await?;

                    // Clean notification API - no boilerplate!
                    notify.info(&format!("Note '{}' saved", input.name))?;

                    Ok(NoteResult {
                        message: format!("Note '{}' saved successfully", input.name),
                        timestamp: chrono::Utc::now().to_rfc3339(),
                    })
                },
            )
            .with_tool(
                "list_notes",
                "List all available notes",
                |_input: ListNotes, ctx: Arc<NotesContext>, _notify| async move {
                    let notes = ctx.list_notes().await;
                    Ok(NotesList {
                        count: notes.len(),
                        notes,
                    })
                },
            )
            .with_tool(
                "read_note",
                "Read the content of a specific note",
                |input: ReadNote, ctx: Arc<NotesContext>, _notify| async move {
                    if let Some(content) = ctx.get_note(&input.name).await {
                        Ok(NoteContent {
                            name: input.name,
                            size: content.len(),
                            content,
                        })
                    } else {
                        Err(anyhow::anyhow!("Note not found: {}", input.name))
                    }
                },
            )
            .with_tool(
                "delete_note",
                "Delete a note from the notes directory",
                |input: DeleteNote, ctx: Arc<NotesContext>, notify| async move {
                    if ctx.get_note(&input.name).await.is_none() {
                        return Err(anyhow::anyhow!("Note not found: {}", input.name));
                    }

                    ctx.delete_note(&input.name).await?;

                    // Clean notification API - no boilerplate!
                    notify.info(&format!("Note '{}' deleted", input.name))?;

                    Ok(NoteResult {
                        message: format!("Note '{}' deleted successfully", input.name),
                        timestamp: chrono::Utc::now().to_rfc3339(),
                    })
                },
            )
            .with_tool(
                "send_notification",
                "Send a notification to the client",
                |input: SendNotification, _ctx: Arc<NotesContext>, notify| async move {
                    // Clean notification API with level matching
                    match input.level.as_str() {
                        "debug" => notify.debug(&input.message)?,
                        "info" => notify.info(&input.message)?,
                        "warning" => notify.warn(&input.message)?,
                        "error" => notify.error(&input.message)?,
                        _ => return Err(anyhow::anyhow!("Invalid log level: {}", input.level)),
                    }

                    Ok(NotificationResult {
                        success: true,
                        sent_at: chrono::Utc::now().to_rfc3339(),
                    })
                },
            )
            .with_resource_provider(Box::new(NotesResourceProvider))
            .build()
            .await?;

    // Get port and start server
    let port = env::var("PORT")
        .unwrap_or_else(|_| "3002".to_string())
        .parse::<u16>()
        .unwrap_or(3002);

    info!(
        "🚀 Starting {} v{} on port {}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        port
    );
    info!("💡 This server demonstrates the new minimal SolidMCP framework API");
    info!("   - Automatic tool registration and routing");
    info!("   - Compile-time schema generation with schemars");
    info!("   - Generic context support");
    info!("   - Zero boilerplate initialization");
    info!("   - Type-safe tool handlers with automatic serialization");
    info!("   - Resource providers for exposing notes as MCP resources");

    server.start(port).await?;
    Ok(())
}
