//! High-level framework for building MCP (Model Context Protocol) servers with minimal boilerplate.
//!
//! This module provides an ergonomic API for creating MCP servers with automatic schema generation,
//! type-safe tool registration, and built-in resource/prompt management. The framework handles all
//! the low-level protocol details while exposing a clean, type-safe interface.
//!
//! # Quick Start
//!
//! ```rust
//! use solidmcp::framework::{McpServerBuilder, NotificationCtx};
//! use serde::{Deserialize, Serialize};
//! use schemars::JsonSchema;
//! use anyhow::Result;
//! use std::sync::Arc;
//!
//! #[derive(JsonSchema, Deserialize)]
//! struct CalculateInput {
//!     a: f64,
//!     b: f64,
//!     operation: String,
//! }
//!
//! #[derive(JsonSchema, Serialize)]
//! struct CalculateOutput {
//!     result: f64,
//! }
//!
//! // Application context (can be any type)
//! struct AppContext {
//!     version: String,
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let context = AppContext {
//!         version: "1.0.0".to_string(),
//!     };
//!
//!     let server = McpServerBuilder::new(context, "calculator", "1.0.0")
//!         .with_tool("calculate", "Perform basic arithmetic", |input: CalculateInput, ctx: Arc<AppContext>, notif: NotificationCtx| async move {
//!             notif.info(&format!("Calculating {} {} {}", input.a, input.operation, input.b))?;
//!             
//!             let result = match input.operation.as_str() {
//!                 "add" => input.a + input.b,
//!                 "subtract" => input.a - input.b,
//!                 "multiply" => input.a * input.b,
//!                 "divide" => input.a / input.b,
//!                 _ => return Err(anyhow::anyhow!("Unknown operation")),
//!             };
//!             
//!             Ok(CalculateOutput { result })
//!         })
//!         .build()
//!         .await?;
//!
//!     server.start(3000).await
//! }
//! ```

use {
    crate::{
        content_types::{McpResponse, ToMcpResponse},
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
    tokio::sync::mpsc,
};

/// Ergonomic notification context that simplifies sending notifications to MCP clients.
///
/// This struct wraps the underlying notification system and provides convenient methods
/// for sending different types of notifications with minimal boilerplate. It automatically
/// handles serialization and error cases.
///
/// # Examples
///
/// ```rust
/// use solidmcp::framework::NotificationCtx;
/// use anyhow::Result;
///
/// async fn example_tool(ctx: NotificationCtx) -> Result<()> {
///     // Send different types of notifications
///     ctx.info("Processing started")?;
///     ctx.debug("Internal state: processing")?;
///     ctx.warn("This might take a while")?;
///     
///     // Send with structured data
///     ctx.log(LogLevel::Info, "Progress update", Some(serde_json::json!({
///         "progress": 50,
///         "total": 100
///     })))?;
///     
///     // Notify about resource changes
///     ctx.resources_changed()?;
///     
///     Ok(())
/// }
/// ```
#[derive(Clone)]
pub struct NotificationCtx {
    sender: Option<mpsc::UnboundedSender<McpNotification>>,
}

impl NotificationCtx {
    /// Create a new notification context from an existing MCP context.
    ///
    /// This is typically called internally by the framework when setting up tool handlers.
    /// You usually don't need to call this directly.
    ///
    /// # Parameters
    /// - `mcp`: The MCP context containing the notification sender
    ///
    /// # Returns
    /// A new `NotificationCtx` that can send notifications to the connected client
    pub fn from_mcp(mcp: &McpContext) -> Self {
        Self {
            sender: mcp.notification_sender.clone(),
        }
    }

    /// Send an informational notification to the client.
    ///
    /// This is the most common type of notification for general status updates
    /// and user-facing information.
    ///
    /// # Parameters
    /// - `message`: The message to send (can be String, &str, or anything implementing Into<String>)
    ///
    /// # Returns
    /// `Result<()>` - Ok if sent successfully, Err if serialization fails or channel is closed
    ///
    /// # Examples
    /// ```rust
    /// ctx.info("File processing completed successfully")?;
    /// ctx.info(format!("Processed {} items", count))?;
    /// ```
    pub fn info(&self, message: impl Into<String>) -> Result<()> {
        self.log(LogLevel::Info, message, None::<Value>)
    }

    /// Send a debug notification to the client.
    ///
    /// Use this for detailed diagnostic information that's primarily useful
    /// for developers or debugging purposes.
    ///
    /// # Parameters
    /// - `message`: The debug message to send
    ///
    /// # Returns
    /// `Result<()>` - Ok if sent successfully, Err if serialization fails
    pub fn debug(&self, message: impl Into<String>) -> Result<()> {
        self.log(LogLevel::Debug, message, None::<Value>)
    }

    /// Send a warning notification to the client.
    ///
    /// Use this for non-fatal issues that the user should be aware of but
    /// don't prevent the operation from completing.
    ///
    /// # Parameters
    /// - `message`: The warning message to send
    ///
    /// # Returns
    /// `Result<()>` - Ok if sent successfully, Err if serialization fails
    pub fn warn(&self, message: impl Into<String>) -> Result<()> {
        self.log(LogLevel::Warning, message, None::<Value>)
    }

    /// Send an error notification to the client.
    ///
    /// Use this for fatal errors or issues that prevent normal operation.
    /// Note that this doesn't stop execution - it just notifies the client.
    ///
    /// # Parameters
    /// - `message`: The error message to send
    ///
    /// # Returns
    /// `Result<()>` - Ok if sent successfully, Err if serialization fails
    pub fn error(&self, message: impl Into<String>) -> Result<()> {
        self.log(LogLevel::Error, message, None::<Value>)
    }

    /// Send a log notification with custom level and optional structured data.
    ///
    /// This is the most flexible notification method, allowing you to specify
    /// the log level and attach structured data to the notification.
    ///
    /// # Type Parameters
    /// - `T`: Type of the data to attach (must implement `serde::Serialize`)
    ///
    /// # Parameters
    /// - `level`: The log level for this message
    /// - `message`: The log message
    /// - `data`: Optional structured data to attach to the notification
    ///
    /// # Returns
    /// `Result<()>` - Ok if sent successfully, Err if serialization fails
    ///
    /// # Examples
    /// ```rust
    /// ctx.log(LogLevel::Info, "Operation completed", Some(json!({
    ///     "duration": 1234,
    ///     "items_processed": 42
    /// })))?;
    /// ```
    pub fn log<T>(&self, level: LogLevel, message: impl Into<String>, data: Option<T>) -> Result<()>
    where
        T: serde::Serialize,
    {
        if let Some(sender) = &self.sender {
            let data = data.map(|d| serde_json::to_value(d)).transpose()?;

            sender.send(McpNotification::LogMessage {
                level,
                logger: Some("app".to_string()),
                message: message.into(),
                data,
            })?;
        }
        Ok(())
    }

    /// Notify the client that the list of available resources has changed.
    ///
    /// This should be called whenever resources are added, removed, or modified
    /// to ensure clients can refresh their resource listings.
    ///
    /// # Returns
    /// `Result<()>` - Ok if sent successfully, Err if channel is closed
    ///
    /// # Examples
    /// ```rust
    /// // After adding a new file to your resource provider
    /// ctx.resources_changed()?;
    /// ```
    pub fn resources_changed(&self) -> Result<()> {
        if let Some(sender) = &self.sender {
            sender.send(McpNotification::ResourcesListChanged)?;
        }
        Ok(())
    }
}

/// Type alias for tool handler functions.
///
/// This represents a boxed, async function that can be called by the MCP client.
/// Tool functions receive validated input parameters, application context, and
/// a notification context for sending updates back to the client.
///
/// # Type Parameters
/// - `C`: The application context type (shared across all tools)
///
/// # Function Signature
/// - `Value`: JSON input parameters (already validated against the tool's schema)
/// - `Arc<C>`: Shared reference to application context
/// - `NotificationCtx`: Context for sending notifications
/// - Returns: `Pin<Box<dyn Future<Output = Result<Value>>>>` - Async result with JSON output
pub type ToolFunction<C> = Box<
    dyn Fn(Value, Arc<C>, NotificationCtx) -> Pin<Box<dyn Future<Output = Result<Value>> + Send>>
        + Send
        + Sync,
>;

/// Registry for managing tools, resources, and prompts within a server instance.
///
/// This struct maintains collections of registered functionality that can be
/// exposed to MCP clients. It provides type-safe registration methods with
/// automatic schema generation.
///
/// # Type Parameters
/// - `C`: The application context type shared across all registered handlers
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
    /// Create a new, empty tool registry.
    ///
    /// # Returns
    /// A new `ToolRegistry` with no registered tools, resources, or prompts
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a tool function with type-safe MCP response format.
    ///
    /// This method ensures all tool responses follow the MCP protocol format by requiring
    /// tools to return `McpResponse`. This prevents issues where raw data is returned
    /// instead of properly formatted MCP responses that clients like Claude Code expect.
    ///
    /// # Type Parameters
    /// - `I`: Input type (must implement `JsonSchema` and `DeserializeOwned`)
    /// - `F`: Handler function type
    /// - `Fut`: Future returned by the handler function
    ///
    /// # Parameters
    /// - `name`: Unique name for the tool (used by clients to invoke it)
    /// - `description`: Human-readable description of what the tool does
    /// - `handler`: Async function that returns an `McpResponse`
    ///
    /// # Examples
    /// ```rust
    /// use solidmcp::{McpResponse, McpContent, JsonSchema};
    /// use serde::Deserialize;
    /// use serde_json::json;
    ///
    /// #[derive(JsonSchema, Deserialize)]
    /// struct SearchInput {
    ///     query: String,
    ///     limit: Option<u32>,
    /// }
    ///
    /// registry.register_tool("search", "Search the knowledge base", |input: SearchInput, ctx, notif| async move {
    ///     notif.info(&format!("Searching for: {}", input.query))?;
    ///     
    ///     let results = ctx.database.search(&input.query).await?;
    ///     
    ///     // Type-safe MCP response - prevents "no results found" issues
    ///     Ok(McpResponse::with_text_and_data(
    ///         format!("Found {} results for '{}'", results.len(), input.query),
    ///         json!({
    ///             "results": results,
    ///             "query": input.query,
    ///             "total": results.len()
    ///         })
    ///     ))
    /// });
    /// ```
    pub fn register_tool<I, F, Fut>(&mut self, name: &str, description: &str, handler: F)
    where
        I: JsonSchema + DeserializeOwned + Send + 'static,
        F: Fn(I, Arc<C>, NotificationCtx) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<McpResponse>> + Send + 'static,
    {
        let tool_def = ToolDefinition::from_schema::<I>(name, description);
        let handler = Arc::new(handler);

        let wrapper: ToolFunction<C> = Box::new(move |args, context, notification_ctx| {
            let handler = Arc::clone(&handler);

            Box::pin(async move {
                // Parse and validate input
                let input: I = serde_json::from_value(args)?;

                // Call the handler with clean API
                let mcp_response = handler(input, context, notification_ctx).await?;

                // Convert McpResponse to JSON - this ensures MCP protocol compliance
                Ok(serde_json::to_value(mcp_response)?)
            })
        });

        self.tools.insert(name.to_string(), (tool_def, wrapper));
    }

    /// Register a resource provider for dynamic resource management.
    ///
    /// Resource providers allow your server to expose data and files through
    /// the MCP resource protocol. Resources are identified by URIs and can
    /// be listed and read by clients.
    ///
    /// # Parameters
    /// - `provider`: Boxed resource provider implementing the `ResourceProvider` trait
    ///
    /// # Examples
    /// ```rust
    /// struct FileSystemProvider;
    ///
    /// #[async_trait]
    /// impl ResourceProvider<AppContext> for FileSystemProvider {
    ///     async fn list_resources(&self, context: Arc<AppContext>) -> Result<Vec<ResourceInfo>> {
    ///         // Return list of available files
    ///     }
    ///     
    ///     async fn read_resource(&self, uri: &str, context: Arc<AppContext>) -> Result<ResourceContent> {
    ///         // Read and return file content
    ///     }
    /// }
    ///
    /// registry.register_resource_provider(Box::new(FileSystemProvider));
    /// ```
    pub fn register_resource_provider(&mut self, provider: Box<dyn ResourceProvider<C>>) {
        self.resources.push(provider);
    }

    /// Register a prompt provider for dynamic prompt template management.
    ///
    /// Prompt providers allow your server to expose reusable prompt templates
    /// that clients can use with AI models. Prompts can have parameters and
    /// generate contextual messages.
    ///
    /// # Parameters
    /// - `provider`: Boxed prompt provider implementing the `PromptProvider` trait
    ///
    /// # Examples
    /// ```rust
    /// struct CodeReviewProvider;
    ///
    /// #[async_trait]
    /// impl PromptProvider<AppContext> for CodeReviewProvider {
    ///     async fn list_prompts(&self, context: Arc<AppContext>) -> Result<Vec<PromptInfo>> {
    ///         // Return available prompt templates
    ///     }
    ///     
    ///     async fn get_prompt(&self, name: &str, arguments: Option<Value>, context: Arc<AppContext>) -> Result<PromptContent> {
    ///         // Generate prompt content based on template and arguments
    ///     }
    /// }
    ///
    /// registry.register_prompt_provider(Box::new(CodeReviewProvider));
    /// ```  
    pub fn register_prompt_provider(&mut self, provider: Box<dyn PromptProvider<C>>) {
        self.prompts.push(provider);
    }
}

/// Trait for providing resources dynamically to MCP clients.
///
/// Resource providers allow your server to expose data, files, or other content
/// through URI-based access. Clients can list available resources and read their
/// content on demand.
///
/// # Type Parameters
/// - `C`: The application context type
///
/// # Examples
/// ```rust
/// struct DatabaseProvider {
///     connection_pool: sqlx::Pool<sqlx::Postgres>,
/// }
///
/// #[async_trait]
/// impl ResourceProvider<AppContext> for DatabaseProvider {
///     async fn list_resources(&self, context: Arc<AppContext>) -> Result<Vec<ResourceInfo>> {
///         Ok(vec![
///             ResourceInfo {
///                 uri: "db://users".to_string(),
///                 name: "User Database".to_string(),
///                 description: Some("All registered users".to_string()),
///                 mime_type: Some("application/json".to_string()),
///             }
///         ])
///     }
///
///     async fn read_resource(&self, uri: &str, context: Arc<AppContext>) -> Result<ResourceContent> {
///         match uri {
///             "db://users" => {
///                 let users = sqlx::query!("SELECT * FROM users")
///                     .fetch_all(&self.connection_pool)
///                     .await?;
///                 Ok(ResourceContent {
///                     uri: uri.to_string(),
///                     mime_type: Some("application/json".to_string()),
///                     content: serde_json::to_string_pretty(&users)?,
///                 })
///             }
///             _ => Err(anyhow::anyhow!("Resource not found: {}", uri))
///         }
///     }
/// }
/// ```
#[async_trait]
pub trait ResourceProvider<C>: Send + Sync {
    /// List all available resources that this provider can serve.
    ///
    /// # Parameters
    /// - `context`: Shared application context
    ///
    /// # Returns
    /// `Result<Vec<ResourceInfo>>` - List of available resources or an error
    async fn list_resources(&self, context: Arc<C>) -> Result<Vec<ResourceInfo>>;

    /// Read the content of a specific resource identified by URI.
    ///
    /// # Parameters
    /// - `uri`: The unique identifier for the resource to read
    /// - `context`: Shared application context
    ///
    /// # Returns
    /// `Result<ResourceContent>` - The resource content or an error if not found
    async fn read_resource(&self, uri: &str, context: Arc<C>) -> Result<ResourceContent>;
}

/// Trait for providing dynamic prompt templates to MCP clients.
///
/// Prompt providers allow your server to expose reusable prompt templates that
/// clients can use with AI models. Prompts can have parameters and generate
/// contextual conversation messages.
///
/// # Type Parameters
/// - `C`: The application context type
///
/// # Examples
/// ```rust
/// struct TemplateProvider;
///
/// #[async_trait]
/// impl PromptProvider<AppContext> for TemplateProvider {
///     async fn list_prompts(&self, context: Arc<AppContext>) -> Result<Vec<PromptInfo>> {
///         Ok(vec![
///             PromptInfo {
///                 name: "code_review".to_string(),
///                 description: Some("Generate a code review for the given code".to_string()),
///                 arguments: vec![
///                     PromptArgument {
///                         name: "code".to_string(),
///                         description: Some("The code to review".to_string()),
///                         required: true,
///                     },
///                     PromptArgument {
///                         name: "language".to_string(),
///                         description: Some("Programming language".to_string()),
///                         required: false,
///                     },
///                 ],
///             }
///         ])
///     }
///
///     async fn get_prompt(&self, name: &str, arguments: Option<Value>, context: Arc<AppContext>) -> Result<PromptContent> {
///         match name {
///             "code_review" => {
///                 let args: serde_json::Map<String, Value> = arguments
///                     .and_then(|v| v.as_object().cloned())
///                     .unwrap_or_default();
///                 
///                 let code = args.get("code")
///                     .and_then(|v| v.as_str())
///                     .ok_or_else(|| anyhow::anyhow!("Missing required argument: code"))?;
///                 
///                 let language = args.get("language")
///                     .and_then(|v| v.as_str())
///                     .unwrap_or("unknown");
///
///                 Ok(PromptContent {
///                     messages: vec![
///                         PromptMessage {
///                             role: "user".to_string(),
///                             content: format!("Please review this {} code:\n\n{}", language, code),
///                         }
///                     ],
///                 })
///             }
///             _ => Err(anyhow::anyhow!("Prompt not found: {}", name))
///         }
///     }
/// }
/// ```
#[async_trait]
pub trait PromptProvider<C>: Send + Sync {
    /// List all available prompt templates that this provider can serve.
    ///
    /// # Parameters
    /// - `context`: Shared application context
    ///
    /// # Returns
    /// `Result<Vec<PromptInfo>>` - List of available prompts or an error
    async fn list_prompts(&self, context: Arc<C>) -> Result<Vec<PromptInfo>>;

    /// Generate prompt content for a specific template with given arguments.
    ///
    /// # Parameters
    /// - `name`: The name of the prompt template to generate
    /// - `arguments`: Optional JSON object containing template parameters
    /// - `context`: Shared application context
    ///
    /// # Returns
    /// `Result<PromptContent>` - The generated prompt messages or an error if not found
    async fn get_prompt(
        &self,
        name: &str,
        arguments: Option<Value>,
        context: Arc<C>,
    ) -> Result<PromptContent>;
}

/// Framework handler that automatically routes MCP requests to registered tools and providers.
///
/// This is the core implementation that bridges the high-level framework API with the
/// low-level MCP protocol. It maintains the application context and routing table,
/// and handles all the protocol-level details automatically.
///
/// # Type Parameters
/// - `C`: The application context type (shared across all handlers)
pub struct FrameworkHandler<C> {
    context: Arc<C>,
    registry: ToolRegistry<C>,
    server_name: String,
    server_version: String,
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
            let notification_ctx = NotificationCtx::from_mcp(context);
            tool_fn(arguments, self.context.clone(), notification_ctx).await
        } else {
            Err(anyhow::anyhow!("Tool not found: {}", name))
        }
    }

    async fn list_resources(&self, _context: &McpContext) -> Result<Vec<ResourceInfo>> {
        let mut all_resources = Vec::new();
        for provider in &self.registry.resources {
            let mut resources = provider.list_resources(self.context.clone()).await?;
            all_resources.append(&mut resources);
        }
        Ok(all_resources)
    }

    async fn read_resource(&self, uri: &str, _context: &McpContext) -> Result<ResourceContent> {
        for provider in &self.registry.resources {
            if let Ok(content) = provider.read_resource(uri, self.context.clone()).await {
                return Ok(content);
            }
        }
        Err(anyhow::anyhow!("Resource not found: {}", uri))
    }

    async fn list_prompts(&self, _context: &McpContext) -> Result<Vec<PromptInfo>> {
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

/// Convenience builder for creating MCP servers with minimal boilerplate.
///
/// This is the main entry point for building MCP servers using the framework.
/// It provides a fluent API for registering tools, resources, and prompts with
/// compile-time type safety and automatic schema generation.
///
/// # Type Parameters
/// - `C`: The application context type (can be any type you want to share across tools)
///
/// # Examples
/// ```rust
/// use solidmcp::framework::McpServerBuilder;
/// use serde::{Deserialize, Serialize};
/// use schemars::JsonSchema;
/// use std::sync::Arc;
/// use anyhow::Result;
///
/// // Define your application context
/// struct AppContext {
///     config: Config,
///     database: Database,
/// }
///
/// // Define tool input/output types with JsonSchema
/// #[derive(JsonSchema, Deserialize)]
/// struct SearchInput {
///     query: String,
///     limit: Option<u32>,
/// }
///
/// #[derive(JsonSchema, Serialize)]
/// struct SearchOutput {
///     results: Vec<String>,
///     total: u32,
/// }
///
/// #[tokio::main]
/// async fn main() -> Result<()> {
///     let context = AppContext::new().await?;
///
///     let server = McpServerBuilder::new(context, "search-server", "1.0.0")
///         .with_tool("search", "Search the database", |input: SearchInput, ctx: Arc<AppContext>, notif| async move {
///             notif.info(&format!("Searching for: {}", input.query))?;
///             
///             let results = ctx.database.search(&input.query, input.limit.unwrap_or(10)).await?;
///             
///             Ok(SearchOutput {
///                 results: results.clone(),
///                 total: results.len() as u32,
///             })
///         })
///         .with_resource_provider(Box::new(MyResourceProvider::new()))
///         .build()
///         .await?;
///
///     server.start(3000).await
/// }
/// ```
pub struct McpServerBuilder<C> {
    handler: FrameworkHandler<C>,
}

impl<C: Send + Sync + 'static> McpServerBuilder<C> {
    /// Create a new server builder with the specified context and server information.
    ///
    /// This is the starting point for building an MCP server. The context you provide
    /// here will be shared across all tools and can contain any application-specific
    /// state, configuration, or resources.
    ///
    /// # Parameters
    /// - `context`: Your application context (can be any type)
    /// - `server_name`: Name of your MCP server (shown to clients)
    /// - `server_version`: Version string for your server
    ///
    /// # Returns
    /// A new `McpServerBuilder` ready for tool registration
    ///
    /// # Examples
    /// ```rust
    /// // Simple context with just configuration
    /// struct Config {
    ///     api_endpoint: String,
    ///     timeout: Duration,
    /// }
    ///
    /// let builder = McpServerBuilder::new(
    ///     Config {
    ///         api_endpoint: "https://api.example.com".to_string(),
    ///         timeout: Duration::from_secs(30),
    ///     },
    ///     "example-server",
    ///     "1.0.0"
    /// );
    /// ```
    pub fn new(context: C, server_name: &str, server_version: &str) -> Self {
        Self {
            handler: FrameworkHandler::new(context, server_name, server_version),
        }
    }

    /// Register a tool with type-safe MCP response format.
    ///
    /// This method ensures all tools return MCP-compliant responses by requiring
    /// handlers to return `McpResponse`. This prevents the "no results found" issue
    /// that occurs when tools return raw JSON data that clients can't properly parse.
    ///
    /// # Type Parameters
    /// - `I`: Input type (must implement `JsonSchema` and `DeserializeOwned`)
    /// - `F`: Handler function type (async closure or function)
    /// - `Fut`: Future type returned by the handler
    ///
    /// # Parameters
    /// - `name`: Unique tool name (used by clients to invoke the tool)
    /// - `description`: Human-readable description of what the tool does
    /// - `handler`: Async function that returns an `McpResponse`
    ///
    /// # Returns
    /// The builder (for method chaining)
    ///
    /// # Error Handling
    /// Tool handlers should return `Result<McpResponse>` where the error type implements
    /// `Into<anyhow::Error>`. Any errors will be automatically converted to
    /// MCP protocol errors and sent to the client.
    ///
    /// # Examples
    /// ```rust
    /// use solidmcp::{McpServerBuilder, McpResponse, JsonSchema};
    /// use serde::Deserialize;
    /// use serde_json::json;
    ///
    /// #[derive(JsonSchema, Deserialize)]
    /// struct SearchInput {
    ///     query: String,
    ///     limit: Option<u32>,
    /// }
    ///
    /// let server = McpServerBuilder::new(AppContext::new(), "search-server", "1.0.0")
    ///     .with_tool("search", "Search the knowledge base", |input: SearchInput, ctx, notif| async move {
    ///         notif.info(&format!("Searching for: {}", input.query))?;
    ///         
    ///         let results = ctx.database.search(&input.query).await?;
    ///         
    ///         // Return type-safe MCP response - Claude Code can now parse this correctly
    ///         Ok(McpResponse::with_text_and_data(
    ///             format!("Found {} results for '{}'", results.len(), input.query),
    ///             json!({
    ///                 "results": results,
    ///                 "query": input.query,
    ///                 "total": results.len()
    ///             })
    ///         ))
    ///     });
    /// ```
    pub fn with_tool<I, F, Fut>(mut self, name: &str, description: &str, handler: F) -> Self
    where
        I: JsonSchema + DeserializeOwned + Send + 'static,
        F: Fn(I, Arc<C>, NotificationCtx) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<McpResponse>> + Send + 'static,
    {
        self.handler
            .registry_mut()
            .register_tool(name, description, handler);
        self
    }

    /// Add a resource provider to expose data through the MCP resource protocol.
    ///
    /// Resource providers allow clients to discover and read data from your server
    /// through URI-based access. This is useful for exposing files, database content,
    /// API responses, or any other data that clients might need.
    ///
    /// # Parameters
    /// - `provider`: A boxed resource provider implementing the `ResourceProvider` trait
    ///
    /// # Returns
    /// The builder (for method chaining)
    ///
    /// # Examples
    /// ```rust
    /// struct FileProvider {
    ///     base_path: PathBuf,
    /// }
    ///
    /// #[async_trait]
    /// impl ResourceProvider<AppContext> for FileProvider {
    ///     async fn list_resources(&self, context: Arc<AppContext>) -> Result<Vec<ResourceInfo>> {
    ///         let mut resources = Vec::new();
    ///         let entries = std::fs::read_dir(&self.base_path)?;
    ///         
    ///         for entry in entries {
    ///             let entry = entry?;
    ///             if entry.file_type()?.is_file() {
    ///                 resources.push(ResourceInfo {
    ///                     uri: format!("file://{}", entry.path().display()),
    ///                     name: entry.file_name().to_string_lossy().to_string(),
    ///                     description: Some("Local file".to_string()),
    ///                     mime_type: mime_guess::from_path(&entry.path())
    ///                         .first()
    ///                         .map(|m| m.to_string()),
    ///                 });
    ///             }
    ///         }
    ///         
    ///         Ok(resources)
    ///     }
    ///
    ///     async fn read_resource(&self, uri: &str, context: Arc<AppContext>) -> Result<ResourceContent> {
    ///         if let Some(path) = uri.strip_prefix("file://") {
    ///             let full_path = self.base_path.join(path);
    ///             let content = tokio::fs::read_to_string(&full_path).await?;
    ///             
    ///             Ok(ResourceContent {
    ///                 uri: uri.to_string(),
    ///                 mime_type: mime_guess::from_path(&full_path)
    ///                     .first()
    ///                     .map(|m| m.to_string()),
    ///                 content,
    ///             })
    ///         } else {
    ///             Err(anyhow::anyhow!("Invalid file URI: {}", uri))
    ///         }
    ///     }
    /// }
    ///
    /// let server = McpServerBuilder::new(context, "file-server", "1.0.0")
    ///     .with_resource_provider(Box::new(FileProvider {
    ///         base_path: PathBuf::from("./data"),
    ///     }));
    /// ```
    pub fn with_resource_provider(mut self, provider: Box<dyn ResourceProvider<C>>) -> Self {
        self.handler
            .registry_mut()
            .register_resource_provider(provider);
        self
    }

    /// Add a prompt provider to expose reusable prompt templates.
    ///
    /// Prompt providers allow clients to discover and use parameterized prompt
    /// templates that you define. This is useful for providing consistent prompts
    /// for AI interactions or generating contextual conversation starters.
    ///
    /// # Parameters
    /// - `provider`: A boxed prompt provider implementing the `PromptProvider` trait
    ///
    /// # Returns
    /// The builder (for method chaining)
    ///
    /// # Examples
    /// ```rust
    /// struct DocumentationProvider;
    ///
    /// #[async_trait]
    /// impl PromptProvider<AppContext> for DocumentationProvider {
    ///     async fn list_prompts(&self, context: Arc<AppContext>) -> Result<Vec<PromptInfo>> {
    ///         Ok(vec![
    ///             PromptInfo {
    ///                 name: "document_function".to_string(),
    ///                 description: Some("Generate documentation for a function".to_string()),
    ///                 arguments: vec![
    ///                     PromptArgument {
    ///                         name: "function_code".to_string(),
    ///                         description: Some("The function code to document".to_string()),
    ///                         required: true,
    ///                     },
    ///                     PromptArgument {
    ///                         name: "language".to_string(),
    ///                         description: Some("Programming language".to_string()),
    ///                         required: false,
    ///                     },
    ///                 ],
    ///             }
    ///         ])
    ///     }
    ///
    ///     async fn get_prompt(&self, name: &str, arguments: Option<Value>, context: Arc<AppContext>) -> Result<PromptContent> {
    ///         match name {
    ///             "document_function" => {
    ///                 let args = arguments.unwrap_or_default();
    ///                 let code = args.get("function_code")
    ///                     .and_then(|v| v.as_str())
    ///                     .ok_or_else(|| anyhow::anyhow!("Missing function_code argument"))?;
    ///                 
    ///                 let language = args.get("language")
    ///                     .and_then(|v| v.as_str())
    ///                     .unwrap_or("unknown");
    ///
    ///                 Ok(PromptContent {
    ///                     messages: vec![
    ///                         PromptMessage {
    ///                             role: "system".to_string(),
    ///                             content: format!("You are a documentation expert for {} code.", language),
    ///                         },
    ///                         PromptMessage {
    ///                             role: "user".to_string(),
    ///                             content: format!("Please write comprehensive documentation for this function:\n\n```{}\n{}\n```", language, code),
    ///                         },
    ///                     ],
    ///                 })
    ///             }
    ///             _ => Err(anyhow::anyhow!("Unknown prompt: {}", name))
    ///         }
    ///     }
    /// }
    ///
    /// let server = McpServerBuilder::new(context, "doc-server", "1.0.0")
    ///     .with_prompt_provider(Box::new(DocumentationProvider));
    /// ```
    pub fn with_prompt_provider(mut self, provider: Box<dyn PromptProvider<C>>) -> Self {
        self.handler
            .registry_mut()
            .register_prompt_provider(provider);
        self
    }

    /// Build the MCP server and prepare it for startup.
    ///
    /// This method finalizes the server configuration and creates an `McpServer`
    /// instance that can be started on a specific port. All registered tools,
    /// resources, and prompts are validated and prepared for serving.
    ///
    /// # Returns
    /// `Result<McpServer>` - A configured server ready to start, or an error if configuration is invalid
    ///
    /// # Errors
    /// - Configuration validation errors
    /// - Schema generation errors for registered tools
    /// - Internal setup errors
    ///
    /// # Examples
    /// ```rust
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let server = McpServerBuilder::new(MyContext::new(), "my-server", "1.0.0")
    ///         .with_tool("hello", "Say hello", |input: HelloInput, ctx, notif| async move {
    ///             Ok(HelloOutput { message: format!("Hello, {}!", input.name) })
    ///         })
    ///         .build()
    ///         .await?;
    ///
    ///     // Start the server on port 3000
    ///     server.start(3000).await?
    ///     Ok(())
    /// }
    /// ```
    pub async fn build(self) -> Result<McpServer> {
        McpServer::with_handler(Arc::new(self.handler)).await
    }
}

/// Convenience macro for registering tools with reduced boilerplate.
///
/// This macro provides a slightly more concise syntax for tool registration,
/// though using the builder methods directly is generally preferred for clarity.
///
/// # Examples
/// ```rust
/// let tool_info = mcp_tool!("calculate", "Perform arithmetic", calculate_handler);
/// ```
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
