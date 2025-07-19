//! MCP Server Example
//!
//! Example application showing how to run the MCP server.

use anyhow::Result;
use solidmcp::McpServer;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("solidmcp=debug,info")
        .init();

    // Create and start the MCP server
    let mut server = McpServer::new().await?;
    
    // Start server on port 3000
    let port = 3000;
    println!("Starting MCP server on port {}", port);
    server.start(port).await?;
    
    Ok(())
}