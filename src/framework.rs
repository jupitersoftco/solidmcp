//! Simplified Framework API for Building MCP Servers
//!
//! This provides a clean, minimal API for end developers to build MCP servers
//! with automatic tool discovery, routing, and schema generation.

use {
    crate::{
        core::McpServer,
        handler::{
            LogLevel, McpContext, McpHandler, McpNotification, PromptContent, PromptInfo,
            ResourceContent, ResourceInfo, ToolDefinition,
        },
    },
    anyhow::Result,
    async_trait::async_trait,
    schemars::JsonSchema,
    serde::de::DeserializeOwned,
    serde_json::Value,
    std::{collections::HashMap, future::Future, pin::Pin, sync::Arc},
};

/// A tool function that can be called by the MCP client
pub type ToolFunction<C> = Box<
    dyn Fn(Value, Arc<C>, &McpContext) -> Pin<Box<dyn Future<Output = Result<Value>> + Send>>
        + Send
        + Sync,
>;

/// Tool registration information
pub struct ToolRegistry<C> {
    tools: HashMap<String, (ToolDefinition, ToolFunction<C>)>,
    resources: Vec<Box<dyn ResourceProvider<C>>>,
    prompts: Vec<Box<dyn PromptProvider<C>>>,
}

impl<C> Default for ToolRegistry<C> {
    fn default() -> Self {
        Self {
            tools: HashMap::new(),
            resources: Vec::new(),
            prompts: Vec::new(),
        }
    }
}

impl<C: Send + Sync + 'static> ToolRegistry<C> {
    /// Create a new tool registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a tool function with automatic schema generation
    pub fn register_tool<I, O, F, Fut>(&mut self, name: &str, description: &str, handler: F)
    where
        I: JsonSchema + DeserializeOwned + Send + 'static,
        O: JsonSchema + serde::Serialize + Send + 'static,
        F: Fn(I, Arc<C>, &McpContext) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<O>> + Send + 'static,
    {
        let tool_def = ToolDefinition::from_schema::<I>(name, description);
        let handler = Arc::new(handler);

        let wrapper: ToolFunction<C> = Box::new(move |args, context, mcp_context| {
            let handler = Arc::clone(&handler);
            // Extract the data we need from mcp_context instead of trying to clone it
            let session_id = mcp_context.session_id.clone();
            let protocol_version = mcp_context.protocol_version.clone();
            let client_info = mcp_context.client_info.clone();
            let notification_sender = mcp_context.notification_sender.clone();

            Box::pin(async move {
                // Parse and validate input
                let input: I = serde_json::from_value(args)?;

                // Reconstruct a context for the handler
                let reconstructed_context = McpContext {
                    session_id,
                    protocol_version,
                    client_info,
                    notification_sender,
                };

                // Call the handler
                let output = handler(input, context, &reconstructed_context).await?;

                // Serialize output
                Ok(serde_json::to_value(output)?)
            })
        });

        self.tools.insert(name.to_string(), (tool_def, wrapper));
    }

    /// Register a resource provider
    pub fn register_resource_provider(&mut self, provider: Box<dyn ResourceProvider<C>>) {
        self.resources.push(provider);
    }

    /// Register a prompt provider  
    pub fn register_prompt_provider(&mut self, provider: Box<dyn PromptProvider<C>>) {
        self.prompts.push(provider);
    }
}

/// Trait for providing resources dynamically
#[async_trait]
pub trait ResourceProvider<C>: Send + Sync {
    async fn list_resources(&self, context: Arc<C>) -> Result<Vec<ResourceInfo>>;
    async fn read_resource(&self, uri: &str, context: Arc<C>) -> Result<ResourceContent>;
}

/// Trait for providing prompts dynamically
#[async_trait]
pub trait PromptProvider<C>: Send + Sync {
    async fn list_prompts(&self, context: Arc<C>) -> Result<Vec<PromptInfo>>;
    async fn get_prompt(
        &self,
        name: &str,
        arguments: Option<Value>,
        context: Arc<C>,
    ) -> Result<PromptContent>;
}

/// Framework handler that automatically routes tools and handles initialization
pub struct FrameworkHandler<C> {
    context: Arc<C>,
    registry: ToolRegistry<C>,
    server_name: String,
    server_version: String,
}

impl<C: Send + Sync + 'static> FrameworkHandler<C> {
    /// Create a new framework handler with custom context
    pub fn new(context: C, server_name: &str, server_version: &str) -> Self {
        Self {
            context: Arc::new(context),
            registry: ToolRegistry::new(),
            server_name: server_name.to_string(),
            server_version: server_version.to_string(),
        }
    }

    /// Get a mutable reference to the tool registry for registration
    pub fn registry_mut(&mut self) -> &mut ToolRegistry<C> {
        &mut self.registry
    }

    /// Get the context
    pub fn context(&self) -> &Arc<C> {
        &self.context
    }
}

#[async_trait]
impl<C: Send + Sync + 'static> McpHandler for FrameworkHandler<C> {
    async fn initialize(&self, _params: Value, _context: &McpContext) -> Result<Value> {
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

    async fn list_tools(&self, _context: &McpContext) -> Result<Vec<ToolDefinition>> {
        Ok(self
            .registry
            .tools
            .values()
            .map(|(def, _)| def.clone())
            .collect())
    }

    async fn call_tool(&self, name: &str, arguments: Value, context: &McpContext) -> Result<Value> {
        if let Some((_, tool_fn)) = self.registry.tools.get(name) {
            tool_fn(arguments, self.context.clone(), context).await
        } else {
            Err(anyhow::anyhow!("Tool not found: {}", name))
        }
    }

    async fn list_resources(&self, context: &McpContext) -> Result<Vec<ResourceInfo>> {
        let mut all_resources = Vec::new();
        for provider in &self.registry.resources {
            let mut resources = provider.list_resources(self.context.clone()).await?;
            all_resources.append(&mut resources);
        }
        Ok(all_resources)
    }

    async fn read_resource(&self, uri: &str, context: &McpContext) -> Result<ResourceContent> {
        for provider in &self.registry.resources {
            if let Ok(content) = provider.read_resource(uri, self.context.clone()).await {
                return Ok(content);
            }
        }
        Err(anyhow::anyhow!("Resource not found: {}", uri))
    }

    async fn list_prompts(&self, context: &McpContext) -> Result<Vec<PromptInfo>> {
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
        context: &McpContext,
    ) -> Result<PromptContent> {
        for provider in &self.registry.prompts {
            if let Ok(content) = provider
                .get_prompt(name, arguments.clone(), self.context.clone())
                .await
            {
                return Ok(content);
            }
        }
        Err(anyhow::anyhow!("Prompt not found: {}", name))
    }
}

/// Convenience builder for creating MCP servers with minimal boilerplate
pub struct McpServerBuilder<C> {
    handler: FrameworkHandler<C>,
}

impl<C: Send + Sync + 'static> McpServerBuilder<C> {
    /// Create a new server builder with custom context
    pub fn new(context: C, server_name: &str, server_version: &str) -> Self {
        Self {
            handler: FrameworkHandler::new(context, server_name, server_version),
        }
    }

    /// Register a tool with automatic schema generation and routing
    pub fn with_tool<I, O, F, Fut>(mut self, name: &str, description: &str, handler: F) -> Self
    where
        I: JsonSchema + DeserializeOwned + Send + 'static,
        O: JsonSchema + serde::Serialize + Send + 'static,
        F: Fn(I, Arc<C>, &McpContext) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<O>> + Send + 'static,
    {
        self.handler
            .registry_mut()
            .register_tool(name, description, handler);
        self
    }

    /// Add a resource provider
    pub fn with_resource_provider(mut self, provider: Box<dyn ResourceProvider<C>>) -> Self {
        self.handler
            .registry_mut()
            .register_resource_provider(provider);
        self
    }

    /// Add a prompt provider
    pub fn with_prompt_provider(mut self, provider: Box<dyn PromptProvider<C>>) -> Self {
        self.handler
            .registry_mut()
            .register_prompt_provider(provider);
        self
    }

    /// Build the MCP server
    pub async fn build(self) -> Result<McpServer> {
        McpServer::with_handler(Arc::new(self.handler)).await
    }
}

/// Convenience macro for registering tools with less boilerplate
#[macro_export]
macro_rules! mcp_tool {
    ($name:expr, $desc:expr, $handler:expr) => {
        ($name, $desc, $handler)
    };
}

/// Helper for sending notifications easily
pub fn send_notification(
    context: &McpContext,
    level: LogLevel,
    message: &str,
    data: Option<Value>,
) -> Result<()> {
    if let Some(sender) = &context.notification_sender {
        sender.send(McpNotification::LogMessage {
            level,
            logger: Some("app".to_string()),
            message: message.to_string(),
            data,
        })?;
    }
    Ok(())
}

/// Helper for sending resource change notifications
pub fn notify_resources_changed(context: &McpContext) -> Result<()> {
    if let Some(sender) = &context.notification_sender {
        sender.send(McpNotification::ResourcesListChanged)?;
    }
    Ok(())
}
