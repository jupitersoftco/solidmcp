//! WebSocket-Only MCP Server Example
//!
//! This example demonstrates how to create an MCP server that only
//! supports WebSocket connections, which is useful for applications
//! that need real-time bidirectional communication.
//!
//! Usage:
//!   cargo run --example websocket_server
//!
//! Then connect to:
//!   WebSocket: ws://localhost:3031/mcp

use solidmcp::McpServer;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for logging
    tracing_subscriber::fmt().with_env_filter("debug").init();

    info!("🔌 Starting SolidMCP WebSocket-Only Server Example");

    // Create a new MCP server
    let mut server = McpServer::new().await?;

    // Start the server on port 3031
    let port = 3031;
    info!("🌐 WebSocket server will be available at:");
    info!("  ws://localhost:{}/mcp", port);
    info!("📋 Available tools: echo, read_file");
    info!("💡 This example focuses on WebSocket connections only");
    info!("Press Ctrl+C to stop the server");

    // Note: The McpServer.start() method provides both WebSocket and HTTP endpoints
    // For a WebSocket-only server in production, you would want to create a custom
    // warp filter that only includes the WebSocket route. For this example, we'll
    // use the standard server but emphasize WebSocket usage.

    if let Err(e) = server.start(port).await {
        error!("❌ Server error: {}", e);
        return Err(e.into());
    }

    Ok(())
}
