//! MCP Server Example
//!
//! Example application showing how to run the MCP server.

use anyhow::Result;
use solidmcp::{McpServer, logging};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize structured logging with tracing
    logging::init_tracing();

    // Create and start the MCP server
    let mut server = McpServer::new().await?;

    // Start server on port 3000
    let port = 3000;
    logging::log_server_startup(port);
    server.start(port).await?;

    Ok(())
}
