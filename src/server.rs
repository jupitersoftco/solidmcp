//! High-level MCP Server API
//!
//! This module provides a high-level API for building MCP servers with custom functionality.

use {
    crate::{
        core::McpServer,
        handler::{
            McpContext, McpHandler, McpNotification, PromptContent, PromptInfo, ResourceContent,
            ResourceInfo, ToolDefinition,
        },
    },
    anyhow::Result,
    async_trait::async_trait,
    serde_json::{json, Value},
    std::{collections::HashMap, sync::Arc},
    tokio::sync::{mpsc, RwLock},
    tracing::info,
};

/// Context provided to tools for accessing server functionality
pub struct ToolContext {
    /// Sender for notifications
    pub notification_sender: Option<mpsc::UnboundedSender<McpNotification>>,
}

/// Trait for implementing custom MCP tools
#[async_trait]
pub trait McpTool: Send + Sync {
    /// Get the tool definition for tools/list
    fn definition(&self) -> ExtendedToolDefinition;

    /// Execute the tool with given arguments
    async fn execute(&self, arguments: Value, context: &ToolContext) -> Result<Value>;
}

/// Extended tool definition for MCP (with output_schema)
#[derive(Debug, Clone)]
pub struct ExtendedToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
    pub output_schema: Value,
}

/// Trait for implementing MCP resources
#[async_trait]
pub trait McpResourceProvider: Send + Sync {
    /// List available resources
    async fn list_resources(&self) -> Result<Vec<ResourceInfo>>;

    /// Read a specific resource by URI
    async fn read_resource(&self, uri: &str) -> Result<ResourceContent>;
}

/// Trait for implementing MCP prompts
#[async_trait]
pub trait McpPromptProvider: Send + Sync {
    /// List available prompts
    async fn list_prompts(&self) -> Result<Vec<PromptInfo>>;

    /// Get a specific prompt by name
    async fn get_prompt(&self, name: &str, arguments: Option<Value>) -> Result<PromptContent>;
}

/// High-level MCP Server that can be customized with tools, resources, and prompts
pub struct McpServerBuilder {
    tools: HashMap<String, Arc<dyn McpTool>>,
    resource_provider: Option<Arc<dyn McpResourceProvider>>,
    prompt_provider: Option<Arc<dyn McpPromptProvider>>,
    capabilities: ServerCapabilities,
}

/// Server capabilities configuration
#[derive(Debug, Clone, Default)]
pub struct ServerCapabilities {
    pub experimental: HashMap<String, Value>,
}

impl Default for McpServerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl McpServerBuilder {
    /// Create a new MCP server builder
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            resource_provider: None,
            prompt_provider: None,
            capabilities: ServerCapabilities::default(),
        }
    }

    /// Add a custom tool to the server
    pub fn add_tool(mut self, tool: impl McpTool + 'static) -> Self {
        let definition = tool.definition();
        self.tools.insert(definition.name.clone(), Arc::new(tool));
        self
    }

    /// Set the resource provider
    pub fn with_resources(mut self, provider: impl McpResourceProvider + 'static) -> Self {
        self.resource_provider = Some(Arc::new(provider));
        self
    }

    /// Set the prompt provider
    pub fn with_prompts(mut self, provider: impl McpPromptProvider + 'static) -> Self {
        self.prompt_provider = Some(Arc::new(provider));
        self
    }

    /// Set experimental capabilities
    pub fn with_experimental_capability(mut self, name: String, value: Value) -> Self {
        self.capabilities.experimental.insert(name, value);
        self
    }

    /// Build and start the MCP server
    pub async fn build(self) -> Result<HighLevelMcpServer> {
        let (notification_tx, notification_rx) = mpsc::unbounded_channel();

        let custom_handler = Arc::new(CustomMcpHandler {
            tools: Arc::new(RwLock::new(self.tools)),
            resource_provider: self.resource_provider,
            prompt_provider: self.prompt_provider,
            capabilities: self.capabilities,
            notification_sender: Some(notification_tx),
        });

        Ok(HighLevelMcpServer {
            custom_handler,
            notification_receiver: notification_rx,
        })
    }
}

/// High-level MCP Server
pub struct HighLevelMcpServer {
    custom_handler: Arc<CustomMcpHandler>,
    notification_receiver: mpsc::UnboundedReceiver<McpNotification>,
}

impl HighLevelMcpServer {
    /// Add a tool to the server after construction
    pub async fn add_tool(&self, tool: impl McpTool + 'static) -> Result<()> {
        let definition = tool.definition();
        self.custom_handler
            .tools
            .write()
            .await
            .insert(definition.name.clone(), Arc::new(tool));
        Ok(())
    }

    /// Get a notification sender for tools that need to send notifications
    pub fn notification_sender(&self) -> Option<mpsc::UnboundedSender<McpNotification>> {
        self.custom_handler.notification_sender.clone()
    }

    /// Start the server on the specified port
    pub async fn start(self, port: u16) -> Result<()> {
        // Create core server with our custom handler
        let mut core_server = McpServer::with_handler(self.custom_handler).await?;

        info!("🚀 Starting MCP server with custom tools on port {}", port);
        core_server.start(port).await
    }
}

/// Custom handler that implements the MCP protocol with user-provided tools, resources, and prompts
struct CustomMcpHandler {
    tools: Arc<RwLock<HashMap<String, Arc<dyn McpTool>>>>,
    resource_provider: Option<Arc<dyn McpResourceProvider>>,
    prompt_provider: Option<Arc<dyn McpPromptProvider>>,
    capabilities: ServerCapabilities,
    notification_sender: Option<mpsc::UnboundedSender<McpNotification>>,
}

impl CustomMcpHandler {
    /// Send a notification
    pub fn send_notification(&self, notification: McpNotification) -> Result<()> {
        if let Some(sender) = &self.notification_sender {
            sender.send(notification)?;
        }
        Ok(())
    }

    /// Get capabilities based on what's configured
    fn get_capabilities(&self) -> Value {
        let mut capabilities = json!({});

        // Add tools capability if we have tools
        // Note: We use blocking_read() here which is safe because this is called
        // during initialization before any async operations begin
        let tools_count = self.tools.blocking_read().len();
        if tools_count > 0 {
            capabilities["tools"] = json!({
                "listChanged": false
            });
        }

        // Add resources capability if we have a resource provider
        if self.resource_provider.is_some() {
            capabilities["resources"] = json!({
                "listChanged": false
            });
        }

        // Add prompts capability if we have a prompt provider
        if self.prompt_provider.is_some() {
            capabilities["prompts"] = json!({
                "listChanged": false
            });
        }

        // Add experimental capabilities
        if !self.capabilities.experimental.is_empty() {
            capabilities["experimental"] = json!(self.capabilities.experimental);
        }

        capabilities
    }

    /// Handle tools/list request
    async fn handle_tools_list(&self) -> Result<Value> {
        let tools = self.tools.read().await;
        let tool_definitions: Vec<Value> = tools
            .values()
            .map(|tool| {
                let def = tool.definition();
                json!({
                    "name": def.name,
                    "description": def.description,
                    "inputSchema": def.input_schema,
                })
            })
            .collect();

        Ok(json!({
            "tools": tool_definitions
        }))
    }

    /// Handle tools/call request
    async fn handle_tool_call(&self, name: &str, arguments: Value) -> Result<Value> {
        let tools = self.tools.read().await;

        if let Some(tool) = tools.get(name) {
            let context = ToolContext {
                notification_sender: self.notification_sender.clone(),
            };
            let result = tool.execute(arguments, &context).await?;
            Ok(json!({
                "content": [
                    {
                        "type": "text",
                        "text": serde_json::to_string(&result)?
                    }
                ]
            }))
        } else {
            Err(anyhow::anyhow!("Unknown tool: {}", name))
        }
    }

    /// Handle resources/list request
    async fn handle_resources_list(&self) -> Result<Value> {
        if let Some(provider) = &self.resource_provider {
            let resources = provider.list_resources().await?;
            let resource_list: Vec<Value> = resources
                .into_iter()
                .map(|r| {
                    let mut resource = json!({
                        "uri": r.uri,
                        "name": r.name,
                    });
                    if let Some(desc) = r.description {
                        resource["description"] = json!(desc);
                    }
                    if let Some(mime) = r.mime_type {
                        resource["mimeType"] = json!(mime);
                    }
                    resource
                })
                .collect();

            Ok(json!({
                "resources": resource_list
            }))
        } else {
            Err(anyhow::anyhow!("Resources not supported"))
        }
    }

    /// Handle resources/read request
    async fn handle_resource_read(&self, uri: &str) -> Result<Value> {
        if let Some(provider) = &self.resource_provider {
            let content = provider.read_resource(uri).await?;
            Ok(json!({
                "contents": [
                    {
                        "uri": content.uri,
                        "mimeType": content.mime_type,
                        "text": content.content,
                    }
                ]
            }))
        } else {
            Err(anyhow::anyhow!("Resources not supported"))
        }
    }

    /// Handle prompts/list request
    async fn handle_prompts_list(&self) -> Result<Value> {
        if let Some(provider) = &self.prompt_provider {
            let prompts = provider.list_prompts().await?;
            let prompt_list: Vec<Value> = prompts
                .into_iter()
                .map(|p| {
                    let mut prompt = json!({
                        "name": p.name,
                    });
                    if let Some(desc) = p.description {
                        prompt["description"] = json!(desc);
                    }
                    if !p.arguments.is_empty() {
                        let args: Vec<Value> = p
                            .arguments
                            .into_iter()
                            .map(|a| {
                                let mut arg = json!({
                                    "name": a.name,
                                    "required": a.required,
                                });
                                if let Some(desc) = a.description {
                                    arg["description"] = json!(desc);
                                }
                                arg
                            })
                            .collect();
                        prompt["arguments"] = json!(args);
                    }
                    prompt
                })
                .collect();

            Ok(json!({
                "prompts": prompt_list
            }))
        } else {
            Err(anyhow::anyhow!("Prompts not supported"))
        }
    }

    /// Handle prompts/get request
    async fn handle_prompt_get(&self, name: &str, arguments: Option<Value>) -> Result<Value> {
        if let Some(provider) = &self.prompt_provider {
            let content = provider.get_prompt(name, arguments).await?;
            let messages: Vec<Value> = content
                .messages
                .into_iter()
                .map(|m| {
                    json!({
                        "role": m.role,
                        "content": {
                            "type": "text",
                            "text": m.content,
                        }
                    })
                })
                .collect();

            Ok(json!({
                "messages": messages
            }))
        } else {
            Err(anyhow::anyhow!("Prompts not supported"))
        }
    }
}

#[async_trait]
impl McpHandler for CustomMcpHandler {
    async fn initialize(&mut self, params: Value, context: &McpContext) -> Result<Value> {
        Ok(json!({
            "protocolVersion": "2025-06-18",
            "capabilities": self.get_capabilities(),
            "serverInfo": {
                "name": "toy-notes-server",
                "version": "0.1.0"
            }
        }))
    }

    async fn list_tools(&self, _context: &McpContext) -> Result<Vec<ToolDefinition>> {
        let tools = self.tools.read().await;
        let tool_definitions: Vec<ToolDefinition> = tools
            .values()
            .map(|tool| {
                let def = tool.definition();
                ToolDefinition {
                    name: def.name,
                    description: def.description,
                    input_schema: def.input_schema,
                }
            })
            .collect();
        Ok(tool_definitions)
    }

    async fn call_tool(
        &self,
        name: &str,
        arguments: Value,
        _context: &McpContext,
    ) -> Result<Value> {
        let tools = self.tools.read().await;

        if let Some(tool) = tools.get(name) {
            let context = ToolContext {
                notification_sender: self.notification_sender.clone(),
            };
            let result = tool.execute(arguments, &context).await?;
            Ok(json!({
                "content": [
                    {
                        "type": "text",
                        "text": serde_json::to_string(&result)?
                    }
                ]
            }))
        } else {
            Err(anyhow::anyhow!("Unknown tool: {}", name))
        }
    }

    async fn list_resources(&self, _context: &McpContext) -> Result<Vec<ResourceInfo>> {
        if let Some(provider) = &self.resource_provider {
            provider.list_resources().await
        } else {
            Ok(vec![])
        }
    }

    async fn read_resource(&self, uri: &str, _context: &McpContext) -> Result<ResourceContent> {
        if let Some(provider) = &self.resource_provider {
            provider.read_resource(uri).await
        } else {
            Err(anyhow::anyhow!("Resource not found: {}", uri))
        }
    }

    async fn list_prompts(&self, _context: &McpContext) -> Result<Vec<PromptInfo>> {
        if let Some(provider) = &self.prompt_provider {
            provider.list_prompts().await
        } else {
            Ok(vec![])
        }
    }

    async fn get_prompt(
        &self,
        name: &str,
        arguments: Option<Value>,
        _context: &McpContext,
    ) -> Result<PromptContent> {
        if let Some(provider) = &self.prompt_provider {
            provider.get_prompt(name, arguments).await
        } else {
            Err(anyhow::anyhow!("Prompt not found: {}", name))
        }
    }
}
