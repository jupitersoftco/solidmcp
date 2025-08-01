//! Toy Notes Server - SolidMCP Framework Example
//!
//! Demonstrates the new SolidMCP framework with minimal boilerplate and maximum type safety

use {
    anyhow::Result,
    schemars::JsonSchema,
    serde::{Deserialize, Serialize},
    serde_json::Value,
    solidmcp::McpServerBuilder,
    std::{env, sync::Arc},
    toy_notes_server::{NotesContext, NotesPromptProvider, NotesResourceProvider},
    tracing::info,
};

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
        .map(std::path::PathBuf::from)
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
            .with_prompt_provider(Box::new(NotesPromptProvider))
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
