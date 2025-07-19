//! Toy Notes Server - MCP Protocol Demonstration
//!
//! A simple note-taking server that demonstrates MCP protocol features including notifications.

mod server;

use anyhow::Result;
use std::env;
use std::path::PathBuf;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    info!("🚀 Starting Toy Notes MCP Server");
    info!("⚠️  Note: This is currently using the built-in echo and read_file tools");
    info!(
        "📝 Full note-taking functionality will be available once the high-level API is integrated"
    );

    // Get notes directory from environment or use default
    let notes_dir = env::var("NOTES_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let mut path = env::current_dir().unwrap();
            path.push("notes");
            path
        });

    info!("📁 Notes directory: {}", notes_dir.display());

    // Create notes directory if it doesn't exist
    tokio::fs::create_dir_all(&notes_dir).await?;

    // Create our custom toy server with notification support
    let server = server::create_toy_server(notes_dir).await?;

    // Get port from environment or use default
    let port = env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);

    info!("🌐 Starting server on port {}", port);
    info!("📡 Connect via:");
    info!("   WebSocket: ws://localhost:{}/mcp", port);
    info!("   HTTP: http://localhost:{}/mcp", port);
    info!("");
    info!("Available tools:");
    info!("  - list_notes: List all available notes");
    info!("  - add_note: Add or update a note");
    info!("  - add_notification: Send a notification to the client");
    info!("");
    info!("To test the server:");
    info!("  1. Connect with an MCP client");
    info!("  2. Call 'initialize' to start a session");
    info!("  3. Call 'tools/list' to see available tools");
    info!("  4. Call 'tools/call' with tool name and arguments");
    info!("");
    info!("Example: Add a note and receive a notification");
    info!("  tools/call with:");
    info!("    name: 'add_note'");
    info!("    arguments: {{\"name\": \"test\", \"content\": \"Hello, World!\"}}");
    info!("");
    info!("⚠️  Note: This is a demonstration of the notification architecture.");
    info!("    Full bidirectional notification support requires WebSocket integration.");

    // For this demonstration, we'll use the standard server start method
    // In a full implementation, notifications would be dispatched through WebSocket connections
    server.start(port).await?;

    // Run a simple demonstration of the notification flow
    // simple_server::demo_notification_flow().await;

    Ok(())
}
