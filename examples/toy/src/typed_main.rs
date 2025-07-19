//! Main entry point for the typed notes server using schemars

use {
    anyhow::Result,
    solidmcp::McpServer,
    std::{env, path::PathBuf, sync::Arc},
    toy_notes_server::TypedNotesHandler,
    tracing::{info, warn},
};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    // Get notes directory from environment or use default
    let notes_dir = env::var("NOTES_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let mut dir = env::temp_dir();
            dir.push("toy_notes_typed");
            dir
        });

    info!("📁 Notes directory: {}", notes_dir.display());

    // Create typed handler
    let handler = TypedNotesHandler::new(notes_dir);

    // Load existing notes
    if let Err(e) = handler.load_notes().await {
        warn!("Failed to load existing notes: {}", e);
    }

    // Create MCP server with our handler
    let mut server = McpServer::with_handler(Arc::new(handler)).await?;

    // Get port from environment or use default
    let port = env::var("PORT")
        .unwrap_or_else(|_| "3001".to_string())
        .parse::<u16>()
        .unwrap_or(3001);

    info!("🚀 Starting typed notes server on port {}", port);
    info!("📋 Available tools:");
    info!("   • add_note - Save a note (strongly typed with AddNoteInput schema)");
    info!("   • list_notes - List all notes (strongly typed with ListNotesInput schema)");
    info!("   • read_note - Read a note (strongly typed with ReadNoteInput schema)");
    info!("   • delete_note - Delete a note (strongly typed with DeleteNoteInput schema)");
    info!("   • send_notification - Send notification (strongly typed with SendNotificationInput schema)");
    info!("📚 Available resources: notes:// URIs for each note");
    info!("📝 Available prompts: create_note_template");
    info!("");
    info!("💡 This server demonstrates compile-time type safety using schemars JsonSchema");
    info!("   All tool inputs and outputs are validated against their schemas automatically!");

    // Start the server
    server.start(port).await?;

    Ok(())
}
