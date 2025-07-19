//! Basic MCP Server Example
//!
//! A simple example that demonstrates how to start an MCP server
//! with both WebSocket and HTTP support on a single port.
//!
//! Usage:
//!   cargo run --example basic_server
//!
//! Then connect to:
//!   WebSocket: ws://localhost:3030/mcp
//!   HTTP: http://localhost:3030/mcp

use solidmcp::McpServer;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for logging
    tracing_subscriber::fmt().with_env_filter("debug").init();

    info!("🚀 Starting SolidMCP Basic Server Example");

    // Create a new MCP server
    let mut server = McpServer::new().await?;

    // Start the server on port 3030
    let port = 3030;
    info!("🌐 Server will be available at:");
    info!("  WebSocket: ws://localhost:{}/mcp", port);
    info!("  HTTP: http://localhost:{}/mcp", port);
    info!("📋 Available tools: echo, read_file");
    info!("Press Ctrl+C to stop the server");

    // Start the server (this will block until the server is stopped)
    if let Err(e) = server.start(port).await {
        error!("❌ Server error: {}", e);
        return Err(e.into());
    }

    Ok(())
}
