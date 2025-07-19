//! Custom Tools MCP Server Example
//!
//! This example demonstrates how to extend the MCP server with custom tools
//! beyond the built-in echo and read_file tools. It shows the architecture
//! for adding your own tools and handling custom functionality.
//!
//! Usage:
//!   cargo run --example custom_tools
//!
//! Then connect to:
//!   WebSocket: ws://localhost:3033/mcp
//!   HTTP: http://localhost:3033/mcp

use serde_json::json;
use solidmcp::{McpServer, McpTools};
use tracing::{error, info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for logging
    tracing_subscriber::fmt().with_env_filter("debug").init();

    info!("🛠️  Starting SolidMCP Custom Tools Server Example");

    // Create a new MCP server
    let mut server = McpServer::new().await?;

    // Start the server on port 3033
    let port = 3033;
    info!("🌐 Server will be available at:");
    info!("  WebSocket: ws://localhost:{}/mcp", port);
    info!("  HTTP: http://localhost:{}/mcp", port);
    info!("📋 Built-in tools: echo, read_file");
    info!("🛠️  Custom tools: You can extend McpTools to add more!");
    info!("💡 To add custom tools, you would:");
    info!("   1. Extend the McpTools struct");
    info!("   2. Add new tool definitions to get_tools_list()");
    info!("   3. Add new cases to execute_tool()");
    info!("   4. Implement your custom tool logic");

    // Example of how you might add custom tools:
    demo_custom_tool_usage().await;

    info!("Press Ctrl+C to stop the server");

    if let Err(e) = server.start(port).await {
        error!("❌ Server error: {}", e);
        return Err(e.into());
    }

    Ok(())
}

/// Demonstrates how custom tools would work
async fn demo_custom_tool_usage() {
    info!("🧪 Demonstrating custom tool concepts...");

    // Show the built-in tools list
    let tools_list = McpTools::get_tools_list();
    let tools = tools_list["tools"].as_array().unwrap();
    info!("📋 Current tools count: {}", tools.len());

    for tool in tools {
        let name = tool["name"].as_str().unwrap();
        let description = tool["description"].as_str().unwrap();
        info!("  • {}: {}", name, description);
    }

    // Example of using the built-in echo tool
    match McpTools::execute_tool(
        "echo",
        json!({"message": "Hello from custom tools example!"}),
    )
    .await
    {
        Ok(result) => {
            info!("✅ Echo tool result: {}", result);
        }
        Err(e) => {
            warn!("⚠️  Echo tool error: {}", e);
        }
    }

    info!("💭 To add a custom tool like 'calculate', you would:");
    info!("   1. Add to tools list in get_tools_list():");
    info!("      {{\"name\": \"calculate\", \"description\": \"Perform math calculations\", ...}}");
    info!("   2. Add to execute_tool() match:");
    info!("      \"calculate\" => handle_calculate(arguments).await,");
    info!("   3. Implement handle_calculate() function");
    info!("   4. Return results in the expected MCP format");
}
