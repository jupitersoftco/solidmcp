//! MCP Protocol Trait
//!
//! Defines the trait for MCP protocol handling that can be easily mocked and tested.

use {anyhow::Result, serde_json::Value};

/// Trait for MCP protocol handling
/// This allows for easy mocking and pure unit testing
#[async_trait::async_trait]
pub trait McpProtocolHandler: Send + Sync {
    /// Handle an MCP message and return the response
    async fn handle_message(&mut self, message: Value) -> Result<Value>;

    /// Check if the client is initialized
    fn is_initialized(&self) -> bool;

    /// Get protocol version
    fn protocol_version(&self) -> &str;

    /// Create an error response
    fn create_error_response(&self, id: Value, code: i32, message: &str) -> Value;
}
