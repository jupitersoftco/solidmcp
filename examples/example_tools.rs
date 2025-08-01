//! Example Tools Implementation
//!
//! This example demonstrates how to implement custom tools using the SolidMCP framework.
//! It includes the classic "echo" and "read_file" tools that were previously built into the library.

use {
    anyhow::Result,
    serde_json::{json, Value},
    solidmcp::{McpServerBuilder, ToolResponse, JsonSchema},
    std::{env, path::Path},
    tokio::fs,
    tracing::info,
};

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
async fn echo_tool(input: EchoInput) -> Result<ToolResponse> {
    info!("📢 Echo tool called with message: {}", input.message);
    
    // Create structured response with both human-readable content and structured data
    let response = json!({
        "echo": input.message,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "tool": "echo"
    });
    
    Ok(ToolResponse::success(format!("Echo: {}", input.message)))
}

/// Read file tool implementation with basic security
async fn read_file_tool(input: ReadFileInput) -> Result<ToolResponse> {
    info!("📖 Read file tool called for: {}", input.file_path);
    
    // Basic path validation (in production, use more robust validation)
    let path = Path::new(&input.file_path);
    
    // Prevent path traversal attacks
    if input.file_path.contains("..") {
        return Ok(ToolResponse::error("Path traversal not allowed"));
    }
    
    // Only allow reading from current directory and subdirectories
    if path.is_absolute() {
        return Ok(ToolResponse::error("Absolute paths not allowed"));
    }
    
    match fs::read_to_string(&input.file_path).await {
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
    let server = McpServerBuilder::new()
        .with_tool("echo", "Echo back the input message", echo_tool)
        .with_tool("read_file", "Read contents of a file", read_file_tool)
        .build();
    
    // Get port from environment or use default
    let port = env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse::<u16>()
        .unwrap_or(3000);
    
    info!("🌐 Server starting on port {}", port);
    info!("📋 Available tools: echo, read_file");
    
    server.start(port).await
}