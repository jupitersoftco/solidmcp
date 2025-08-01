//! Core framework handler implementation.
//!
//! This module provides the `FrameworkHandler` struct which bridges the high-level
//! framework API with the low-level MCP protocol, automatically routing requests
//! to registered tools and providers.

use crate::handler::{
    McpContext, McpHandler, PromptContent, PromptInfo, ResourceContent, ResourceInfo,
    ToolDefinition,
};
use crate::error::{McpError, McpResult};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

use super::{notification::NotificationCtx, registry::ToolRegistry};

/// Framework handler that automatically routes MCP requests to registered tools and providers.
///
/// This is the core implementation that bridges the high-level framework API with the
/// low-level MCP protocol. It maintains the application context and routing table,
/// and handles all the protocol-level details automatically.
///
/// # Type Parameters
/// - `C`: The application context type (shared across all handlers)
pub struct FrameworkHandler<C> {
    pub(super) context: Arc<C>,
    pub(super) registry: ToolRegistry<C>,
    pub(super) server_name: String,
    pub(super) server_version: String,
}

impl<C: Send + Sync + 'static> FrameworkHandler<C> {
    /// Create a new framework handler with the specified context and server information.
    ///
    /// # Parameters
    /// - `context`: Application-specific context that will be shared with all tools
    /// - `server_name`: Name of your MCP server (used in protocol handshake)
    /// - `server_version`: Version of your MCP server (used in protocol handshake)
    ///
    /// # Returns
    /// A new `FrameworkHandler` ready for tool registration
    ///
    /// # Examples
    /// ```rust
    /// struct MyAppContext {
    ///     database_url: String,
    ///     api_key: String,
    /// }
    ///
    /// let context = MyAppContext {
    ///     database_url: "postgresql://localhost/mydb".to_string(),
    ///     api_key: "secret".to_string(),
    /// };
    ///
    /// let handler = FrameworkHandler::new(context, "my-mcp-server", "1.0.0");
    /// ```
    pub fn new(context: C, server_name: &str, server_version: &str) -> Self {
        Self {
            context: Arc::new(context),
            registry: ToolRegistry::new(),
            server_name: server_name.to_string(),
            server_version: server_version.to_string(),
        }
    }

    /// Get a mutable reference to the tool registry for registering functionality.
    ///
    /// This allows you to register tools, resources, and prompts after creating
    /// the handler. Generally, you'll use `McpServerBuilder` instead of calling
    /// this directly.
    ///
    /// # Returns
    /// Mutable reference to the internal `ToolRegistry`
    pub fn registry_mut(&mut self) -> &mut ToolRegistry<C> {
        &mut self.registry
    }

    /// Get a shared reference to the application context.
    ///
    /// This provides access to the application context that will be passed to
    /// all tool handlers. Useful for accessing shared state or configuration.
    ///
    /// # Returns
    /// Shared reference to the application context
    pub fn context(&self) -> &Arc<C> {
        &self.context
    }
}

#[async_trait]
impl<C: Send + Sync + 'static> McpHandler for FrameworkHandler<C> {
    async fn initialize(&self, _params: Value, _context: &McpContext) -> McpResult<Value> {
        Ok(serde_json::json!({
            "protocolVersion": "2025-06-18",
            "capabilities": {
                "tools": {},
                "resources": {},
                "prompts": {}
            },
            "serverInfo": {
                "name": self.server_name,
                "version": self.server_version
            }
        }))
    }

    async fn list_tools(&self, _context: &McpContext) -> McpResult<Vec<ToolDefinition>> {
        Ok(self
            .registry
            .tools
            .values()
            .map(|(def, _)| def.clone())
            .collect())
    }

    async fn call_tool(&self, name: &str, arguments: Value, context: &McpContext) -> McpResult<Value> {
        if let Some((_, tool_fn)) = self.registry.tools.get(name) {
            let notification_ctx = NotificationCtx::from_mcp(context);
            tool_fn(arguments, self.context.clone(), notification_ctx).await
        } else {
            Err(McpError::UnknownTool(name.to_string()))
        }
    }

    async fn list_resources(&self, _context: &McpContext) -> McpResult<Vec<ResourceInfo>> {
        let mut all_resources = Vec::new();
        for provider in &self.registry.resources {
            let mut resources = provider.list_resources(self.context.clone()).await?;
            all_resources.append(&mut resources);
        }
        Ok(all_resources)
    }

    async fn read_resource(&self, uri: &str, _context: &McpContext) -> McpResult<ResourceContent> {
        for provider in &self.registry.resources {
            if let Ok(content) = provider.read_resource(uri, self.context.clone()).await {
                return Ok(content);
            }
        }
        Err(McpError::UnknownResource(uri.to_string()))
    }

    async fn list_prompts(&self, _context: &McpContext) -> McpResult<Vec<PromptInfo>> {
        let mut all_prompts = Vec::new();
        for provider in &self.registry.prompts {
            let mut prompts = provider.list_prompts(self.context.clone()).await?;
            all_prompts.append(&mut prompts);
        }
        Ok(all_prompts)
    }

    async fn get_prompt(
        &self,
        name: &str,
        arguments: Option<Value>,
        _context: &McpContext,
    ) -> McpResult<PromptContent> {
        for provider in &self.registry.prompts {
            if let Ok(content) = provider
                .get_prompt(name, arguments.clone(), self.context.clone())
                .await
            {
                return Ok(content);
            }
        }
        Err(McpError::UnknownPrompt(name.to_string()))
    }
}