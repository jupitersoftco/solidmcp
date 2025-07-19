//! HTTP-Only MCP Server Example
//!
//! This example demonstrates how to create an MCP server that focuses
//! on HTTP requests, which is useful for stateless applications and
//! simpler integration scenarios.
//!
//! Usage:
//!   cargo run --example http_server
//!
//! Then connect to:
//!   HTTP: http://localhost:3032/mcp

use solidmcp::McpServer;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for logging
    tracing_subscriber::fmt().with_env_filter("debug").init();

    info!("🌐 Starting SolidMCP HTTP-Only Server Example");

    // Create a new MCP server
    let mut server = McpServer::new().await?;

    // Start the server on port 3032
    let port = 3032;
    info!("🌐 HTTP server will be available at:");
    info!("  http://localhost:{}/mcp", port);
    info!("📋 Available tools: echo, read_file");
    info!("💡 This example focuses on HTTP requests (JSON-RPC over HTTP)");
    info!("🔗 Example curl request:");
    info!(r#"  curl -X POST http://localhost:{}/mcp \"#, port);
    info!(r#"    -H "Content-Type: application/json" \"#);
    info!(
        r#"    -d '{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"protocolVersion":"2025-06-18","capabilities":{{}},"clientInfo":{{"name":"curl-client","version":"1.0.0"}}}}}}"#
    );
    info!("Press Ctrl+C to stop the server");

    // Note: The McpServer.start() method provides both WebSocket and HTTP endpoints
    // For an HTTP-only server in production, you would want to create a custom
    // warp filter that only includes the HTTP route. For this example, we'll
    // use the standard server but emphasize HTTP usage.

    if let Err(e) = server.start(port).await {
        error!("❌ Server error: {}", e);
        return Err(e.into());
    }

    Ok(())
}
