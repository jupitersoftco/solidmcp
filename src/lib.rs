//! SolidMCP - A Rust framework for building MCP (Model Context Protocol) servers
//! 
//! SolidMCP provides a type-safe, ergonomic API for creating Model Context Protocol
//! servers with support for tools, resources, and prompts.
//! 
//! # Quick Start
//! 
//! ```rust,no_run
//! use solidmcp::{McpServerBuilder, ToolResponse};
//! use serde_json::json;
//! 
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let server = McpServerBuilder::new()
//!         .with_tool("hello", "Say hello", |_params| async {
//!             Ok(ToolResponse::success("Hello, world!"))
//!         })
//!         .build();
//!     
//!     server.start(3000).await
//! }
//! ```
//! 
//! # Features
//! 
//! - **Type-safe tools** with automatic JSON schema generation
//! - **WebSocket and HTTP** transport support
//! - **Resource providers** for exposing data
//! - **Prompt templates** for structured interactions
//! - **Async/await** throughout

// Internal modules (not exposed)
mod content_types;
mod error;
mod framework;
mod handler;
mod http;
mod http_handler;
mod logging;
mod protocol;
mod protocol_impl;
mod response;
mod server;
mod shared;
mod tool_response;
mod transport;
mod typed_response;
mod types;
mod validation;
mod websocket;

// Test modules
#[cfg(test)]
mod tests;

// === PUBLIC API ===
// Keep this minimal and stable!

// Core server type
pub use crate::server::McpServer;

// Framework API (preferred)
pub use crate::framework::{McpServerBuilder, PromptProvider, ResourceProvider};

// Handler trait and context
pub use crate::handler::{McpHandler, McpContext};

// Type definitions
pub use crate::types::{
    PromptArgument, PromptContent, PromptDefinition,
    PromptInfo, PromptMessage, ResourceContent, ResourceDefinition, ResourceInfo,
    ToolDefinition, 
};

// Re-export from handler module (will be moved to types in future)
pub use crate::handler::{
    LogLevel, McpNotification, TypedToolDefinition,
};

// Response types
pub use crate::response::{ToolContent, ToolResponse, TypedResponse};

// Error types
pub use crate::error::{McpError, McpResult};

// Re-export commonly used dependencies
pub use schemars::JsonSchema;
pub use serde_json::{json, Value};

// Legacy exports for backward compatibility
// These will be removed in v1.0
#[doc(hidden)]
pub use crate::protocol::McpProtocol;
#[doc(hidden)]
pub use crate::websocket::handle_mcp_ws_main as handle_mcp_ws;

// Framework handler is internal but needed by examples
#[doc(hidden)]
pub use crate::framework::FrameworkHandler;