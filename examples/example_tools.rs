//! Example Tools Implementation
//!
//! This example demonstrates how to implement custom tools using the SolidMCP framework.
//! It includes the classic "echo" and "read_file" tools that were previously built into the library.

use {
    anyhow::Result,
    serde_json::json,
    solidmcp::{McpServerBuilder, ToolResponse, JsonSchema},
    std::{env, path::Path},
    tokio::fs,
    tracing::info,
};

mod utils;
use utils::path_security::validate_path;

/// Schema for the echo tool
#[derive(serde::Deserialize, JsonSchema)]
struct EchoInput {
    /// Message to echo back
    message: String,
}

/// Schema for the read file tool
#[derive(serde::Deserialize, JsonSchema)]
struct ReadFileInput {
    /// Path to the file to read
    file_path: String,
}

/// Echo tool implementation
async fn echo_tool(input: EchoInput, _ctx: std::sync::Arc<()>, _notif: Option<solidmcp::NotificationCtx>) -> Result<ToolResponse> {
    info!("📢 Echo tool called with message: {}", input.message);
    
    // Create structured response with both human-readable content and structured data
    let response = json!({
        "echo": input.message,
        "tool": "echo"
    });
    
    Ok(ToolResponse::success(format!("Echo: {}", input.message)))
}

/// Read file tool implementation with basic security
async fn read_file_tool(input: ReadFileInput, _ctx: std::sync::Arc<()>, _notif: Option<solidmcp::NotificationCtx>) -> Result<ToolResponse> {
    info!("📖 Read file tool called for: {}", input.file_path);
    
    // Security: Validate path to prevent directory traversal
    let allowed_dir = Path::new("."); // Current directory as default allowed path
    let safe_path = match validate_path(&input.file_path, allowed_dir) {
        Ok(path) => path,
        Err(e) => return Ok(ToolResponse::error(&format!("Invalid path: {}", e))),
    };
    
    match fs::read_to_string(&safe_path).await {
        Ok(content) => {
            info!("✅ Successfully read file: {}", input.file_path);
            
            // Create structured response
            let response = json!({
                "file_path": input.file_path,
                "content": content,
                "size": content.len(),
                "tool": "read_file"
            });
            
            Ok(ToolResponse::success(format!(
                "Successfully read file '{}' ({} bytes)", 
                input.file_path, 
                content.len()
            )))
        }
        Err(e) => {
            info!("❌ Failed to read file {}: {}", input.file_path, e);
            Ok(ToolResponse::error(format!("Failed to read file: {}", e)))
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::init();
    
    info!("🚀 Starting MCP server with example tools");
    
    // Build server with example tools
    let mut server = McpServerBuilder::new((), "example-tools", "1.0.0")
        .with_tool("echo", "Echo back the input message", echo_tool)
        .with_tool("read_file", "Read contents of a file", read_file_tool)
        .build()
        .await?;
    
    // Get port from environment or use default
    let port = env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse::<u16>()
        .unwrap_or(3000);
    
    info!("🌐 Server starting on port {}", port);
    info!("📋 Available tools: echo, read_file");
    
    server.start(port).await?;
    Ok(())
}