//! MCP Handler Trait
//!
//! Core trait that users must implement to provide MCP functionality.
//! This is the main integration point for the solidmcp library.

use {anyhow::Result, async_trait::async_trait, serde_json::Value, tokio::sync::mpsc};

/// Context provided to MCP handler methods
#[derive(Clone)]
pub struct McpContext {
    /// Session ID for this client connection
    pub session_id: Option<String>,
    /// Sender for notifications (if supported)
    pub notification_sender: Option<mpsc::UnboundedSender<McpNotification>>,
    /// Protocol version negotiated with client
    pub protocol_version: Option<String>,
    /// Client information from initialization
    pub client_info: Option<Value>,
}

/// Notification types that can be sent from server to client
#[derive(Debug, Clone)]
pub enum McpNotification {
    /// Tools have changed
    ToolsListChanged,
    /// Resources have changed
    ResourcesListChanged,
    /// Prompts have changed
    PromptsListChanged,
    /// Progress notification
    Progress {
        progress_token: String,
        progress: f64,
        total: Option<f64>,
    },
    /// Log message
    LogMessage {
        level: LogLevel,
        logger: Option<String>,
        message: String,
        data: Option<Value>,
    },
    /// Custom notification
    Custom {
        method: String,
        params: Option<Value>,
    },
}

/// Log levels for log message notifications
#[derive(Debug, Clone)]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
}

/// Tool definition for MCP tools/list response
#[derive(Debug, Clone)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

/// Resource information for MCP resources/list response
#[derive(Debug, Clone)]
pub struct ResourceInfo {
    pub uri: String,
    pub name: String,
    pub description: Option<String>,
    pub mime_type: Option<String>,
}

/// Resource content for MCP resources/read response
#[derive(Debug, Clone)]
pub struct ResourceContent {
    pub uri: String,
    pub mime_type: Option<String>,
    pub content: String,
}

/// Prompt information for MCP prompts/list response
#[derive(Debug, Clone)]
pub struct PromptInfo {
    pub name: String,
    pub description: Option<String>,
    pub arguments: Vec<PromptArgument>,
}

/// Prompt argument definition
#[derive(Debug, Clone)]
pub struct PromptArgument {
    pub name: String,
    pub description: Option<String>,
    pub required: bool,
}

/// Prompt content for MCP prompts/get response
#[derive(Debug, Clone)]
pub struct PromptContent {
    pub messages: Vec<PromptMessage>,
}

/// Prompt message
#[derive(Debug, Clone)]
pub struct PromptMessage {
    pub role: String,
    pub content: String,
}

/// Core trait that users must implement to provide MCP functionality
#[async_trait]
pub trait McpHandler: Send + Sync {
    /// Initialize the handler with client information
    /// Called when a client sends an initialize request
    async fn initialize(&mut self, params: Value, context: &McpContext) -> Result<Value> {
        // Default implementation returns basic capabilities
        Ok(serde_json::json!({
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "serverInfo": {
                "name": "solidmcp-server",
                "version": "0.1.0"
            }
        }))
    }

    /// List available tools
    /// Called when a client sends a tools/list request
    async fn list_tools(&self, context: &McpContext) -> Result<Vec<ToolDefinition>>;

    /// Execute a tool
    /// Called when a client sends a tools/call request
    async fn call_tool(&self, name: &str, arguments: Value, context: &McpContext) -> Result<Value>;

    /// List available resources
    /// Called when a client sends a resources/list request
    async fn list_resources(&self, context: &McpContext) -> Result<Vec<ResourceInfo>> {
        // Default implementation - no resources
        Ok(vec![])
    }

    /// Read a resource
    /// Called when a client sends a resources/read request
    async fn read_resource(&self, uri: &str, context: &McpContext) -> Result<ResourceContent> {
        Err(anyhow::anyhow!("Resource not found: {}", uri))
    }

    /// List available prompts
    /// Called when a client sends a prompts/list request
    async fn list_prompts(&self, context: &McpContext) -> Result<Vec<PromptInfo>> {
        // Default implementation - no prompts
        Ok(vec![])
    }

    /// Get a prompt
    /// Called when a client sends a prompts/get request
    async fn get_prompt(
        &self,
        name: &str,
        arguments: Option<Value>,
        context: &McpContext,
    ) -> Result<PromptContent> {
        Err(anyhow::anyhow!("Prompt not found: {}", name))
    }

    /// Handle notification cancellation
    /// Called when a client sends a notifications/cancel request
    async fn cancel_notification(&self, params: Value, context: &McpContext) -> Result<Value> {
        // Default implementation - acknowledge cancellation
        Ok(serde_json::json!({}))
    }

    /// Handle initialized notification
    /// Called when a client sends a notifications/initialized notification
    async fn handle_initialized(&self, context: &McpContext) -> Result<()> {
        // Default implementation - do nothing
        Ok(())
    }
}
