//! MCP Handler Trait and Core Types
//!
//! This module defines the core `McpHandler` trait that servers must implement to provide
//! MCP functionality. It also includes essential types for notifications, context management,
//! and protocol data structures.
//!
//! # Overview
//!
//! The `McpHandler` trait is the main integration point for the solidmcp library. You can
//! either implement this trait directly for full control, or use the higher-level framework
//! API in the `framework` module for a more ergonomic experience.
//!
//! # Example Implementation
//!
//! ```rust
//! use solidmcp::handler::{McpHandler, McpContext, ToolDefinition};
//! use anyhow::Result;
//! use async_trait::async_trait;
//! use serde_json::{json, Value};
//!
//! struct MyHandler {
//!     // Your application state
//! }
//!
//! #[async_trait]
//! impl McpHandler for MyHandler {
//!     async fn initialize(&self, params: Value, context: &McpContext) -> Result<Value> {
//!         Ok(json!({
//!             "protocolVersion": "2025-06-18",
//!             "capabilities": {
//!                 "tools": {}
//!             },
//!             "serverInfo": {
//!                 "name": "my-server",
//!                 "version": "1.0.0"
//!             }
//!         }))
//!     }
//!
//!     async fn list_tools(&self, context: &McpContext) -> Result<Vec<ToolDefinition>> {
//!         Ok(vec![
//!             ToolDefinition {
//!                 name: "hello".to_string(),
//!                 description: "Say hello".to_string(),
//!                 input_schema: json!({
//!                     "type": "object",
//!                     "properties": {
//!                         "name": { "type": "string" }
//!                     },
//!                     "required": ["name"]
//!                 }),
//!             }
//!         ])
//!     }
//!
//!     async fn call_tool(&self, name: &str, arguments: Value, context: &McpContext) -> Result<Value> {
//!         match name {
//!             "hello" => {
//!                 let name = arguments.get("name")
//!                     .and_then(|v| v.as_str())
//!                     .unwrap_or("World");
//!                 Ok(json!({ "message": format!("Hello, {}!", name) }))
//!             }
//!             _ => Err(anyhow::anyhow!("Unknown tool: {}", name))
//!         }
//!     }
//! }
//! ```

use {
    anyhow::Result, async_trait::async_trait, schemars::JsonSchema, serde_json::Value,
    tokio::sync::mpsc,
};

/// Context provided to MCP handler methods.
///
/// This struct contains session-specific information and capabilities that are
/// available to handler methods. It includes the session ID, notification sender,
/// protocol version, and client information.
///
/// # Fields
///
/// - `session_id`: Unique identifier for this client session (used for HTTP session management)
/// - `notification_sender`: Channel for sending notifications back to the client
/// - `protocol_version`: The MCP protocol version negotiated during initialization
/// - `client_info`: Client-provided information from the initialization request
///
/// # Example Usage
///
/// ```rust
/// use solidmcp::handler::{McpContext, McpNotification, LogLevel};
///
/// async fn my_tool_handler(context: &McpContext) -> Result<Value> {
///     // Send a notification
///     if let Some(sender) = &context.notification_sender {
///         sender.send(McpNotification::LogMessage {
///             level: LogLevel::Info,
///             logger: Some("my_tool".to_string()),
///             message: "Processing started".to_string(),
///             data: None,
///         })?;
///     }
///     
///     // Check protocol version for feature compatibility
///     if context.protocol_version.as_deref() == Some("2025-06-18") {
///         // Use newer protocol features
///     }
///     
///     Ok(json!({ "status": "completed" }))
/// }
/// ```
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

/// Notification types that can be sent from server to client.
///
/// MCP supports various notification types for real-time updates. Notifications
/// are one-way messages from server to client and don't expect a response.
///
/// # Notification Types
///
/// - `ToolsListChanged`: Notify when available tools have changed
/// - `ResourcesListChanged`: Notify when available resources have changed
/// - `PromptsListChanged`: Notify when available prompts have changed
/// - `Progress`: Send progress updates for long-running operations
/// - `LogMessage`: Send log messages with different severity levels
/// - `Custom`: Send custom notifications with arbitrary method names
///
/// # Examples
///
/// ```rust
/// use solidmcp::handler::{McpNotification, LogLevel};
/// use serde_json::json;
///
/// // Send a log message
/// let notification = McpNotification::LogMessage {
///     level: LogLevel::Info,
///     logger: Some("file_processor".to_string()),
///     message: "Processing complete".to_string(),
///     data: Some(json!({
///         "files_processed": 42,
///         "duration_ms": 1234
///     })),
/// };
///
/// // Send progress update
/// let progress = McpNotification::Progress {
///     progress_token: "task-123".to_string(),
///     progress: 25.0,
///     total: Some(100.0),
/// };
///
/// // Notify about resource changes
/// let resource_update = McpNotification::ResourcesListChanged;
/// ```
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

/// Log levels for log message notifications.
///
/// These levels follow standard logging conventions and help clients
/// filter and display messages appropriately.
///
/// # Log Level Hierarchy (from least to most severe)
///
/// - `Debug`: Detailed information for debugging purposes
/// - `Info`: General informational messages
/// - `Warning`: Warning messages for potentially problematic situations
/// - `Error`: Error messages for failures that don't stop execution
///
/// # Example
///
/// ```rust
/// use solidmcp::handler::{LogLevel, McpNotification};
///
/// fn get_log_level(severity: u8) -> LogLevel {
///     match severity {
///         0..=3 => LogLevel::Debug,
///         4..=6 => LogLevel::Info,
///         7..=8 => LogLevel::Warning,
///         _ => LogLevel::Error,
///     }
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
}

/// Tool definition for MCP tools/list response.
///
/// This struct represents a tool that can be called by MCP clients. It includes
/// the tool's name, description, and JSON Schema for input validation. This is
/// a type-erased version that allows storing multiple tools with different input
/// types in the same collection.
///
/// # Fields
///
/// - `name`: Unique identifier for the tool
/// - `description`: Human-readable description of what the tool does
/// - `input_schema`: JSON Schema defining the expected input parameters
///
/// # Example
///
/// ```rust
/// use solidmcp::handler::ToolDefinition;
/// use serde_json::json;
///
/// let tool = ToolDefinition {
///     name: "calculate".to_string(),
///     description: "Perform arithmetic operations".to_string(),
///     input_schema: json!({
///         "type": "object",
///         "properties": {
///             "a": { "type": "number" },
///             "b": { "type": "number" },
///             "operation": {
///                 "type": "string",
///                 "enum": ["add", "subtract", "multiply", "divide"]
///             }
///         },
///         "required": ["a", "b", "operation"]
///     }),
///     output_schema: json!({
///         "type": "object",
///         "properties": {
///             "result": { "type": "number" }
///         },
///         "required": ["result"]
///     }),
/// };
/// ```
#[derive(Debug, Clone)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
    pub output_schema: Value,
}

impl ToolDefinition {
    /// Create a new tool definition from a JsonSchema type.
    ///
    /// This method automatically generates a JSON Schema from a Rust type that
    /// implements the `JsonSchema` trait. This is the preferred way to create
    /// tool definitions when using the framework API.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Type that implements `schemars::JsonSchema`
    ///
    /// # Parameters
    ///
    /// - `name`: The unique name for this tool
    /// - `description`: Human-readable description of the tool's purpose
    ///
    /// # Returns
    ///
    /// A new `ToolDefinition` with the schema automatically generated from type `T`
    ///
    /// # Example
    ///
    /// ```rust
    /// use solidmcp::handler::ToolDefinition;
    /// use schemars::JsonSchema;
    /// use serde::Deserialize;
    ///
    /// #[derive(JsonSchema, Deserialize)]
    /// struct SearchInput {
    ///     query: String,
    ///     #[serde(default)]
    ///     limit: u32,
    /// }
    ///
    /// let tool = ToolDefinition::from_schema::<SearchInput>(
    ///     "search",
    ///     "Search for items matching a query"
    /// );
    /// ```
    pub fn from_schema<T: JsonSchema>(
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        let schema = schemars::schema_for!(T);
        let input_schema = serde_json::to_value(schema).unwrap_or_else(|_| {
            serde_json::json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            })
        });

        Self {
            name: name.into(),
            description: description.into(),
            input_schema,
            output_schema: serde_json::json!({
                "type": "object",
                "properties": {},
                "additionalProperties": true
            }),
        }
    }

    /// Create a new tool definition with both input and output schemas from JsonSchema types.
    ///
    /// This method automatically generates JSON Schemas from Rust types that
    /// implement the `JsonSchema` trait for both input and output validation.
    ///
    /// # Type Parameters
    ///
    /// - `I`: Input type that implements `schemars::JsonSchema`
    /// - `O`: Output type that implements `schemars::JsonSchema`
    ///
    /// # Parameters
    ///
    /// - `name`: The unique name for this tool
    /// - `description`: Human-readable description of the tool's purpose
    ///
    /// # Returns
    ///
    /// A new `ToolDefinition` with schemas automatically generated from types `I` and `O`
    ///
    /// # Example
    ///
    /// ```rust
    /// use solidmcp::handler::ToolDefinition;
    /// use schemars::JsonSchema;
    /// use serde::{Deserialize, Serialize};
    ///
    /// #[derive(JsonSchema, Deserialize)]
    /// struct SearchInput {
    ///     query: String,
    ///     limit: u32,
    /// }
    ///
    /// #[derive(JsonSchema, Serialize)]
    /// struct SearchOutput {
    ///     results: Vec<String>,
    ///     total_count: u32,
    /// }
    ///
    /// let tool = ToolDefinition::from_schemas::<SearchInput, SearchOutput>(
    ///     "search",
    ///     "Search for items matching a query"
    /// );
    /// ```
    pub fn from_schemas<I: JsonSchema, O: JsonSchema>(
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        let input_schema = schemars::schema_for!(I);
        let output_schema = schemars::schema_for!(O);
        
        let input_json = serde_json::to_value(input_schema).unwrap_or_else(|_| {
            serde_json::json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            })
        });
        
        let output_json = serde_json::to_value(output_schema).unwrap_or_else(|_| {
            serde_json::json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            })
        });

        Self {
            name: name.into(),
            description: description.into(),
            input_schema: input_json,
            output_schema: output_json,
        }
    }

    /// Convert to the JSON format expected by MCP protocol.
    ///
    /// This method serializes the tool definition into the format required
    /// by the MCP protocol for the tools/list response.
    ///
    /// # Returns
    ///
    /// A JSON value containing the tool's name, description, and input schema
    ///
    /// # Example
    ///
    /// ```rust
    /// let json = tool.to_json();
    /// assert_eq!(json["name"], "calculate");
    /// assert_eq!(json["description"], "Perform arithmetic operations");
    /// assert!(json["input_schema"].is_object());
    /// ```
    pub fn to_json(&self) -> Value {
        serde_json::json!({
            "name": self.name,
            "description": self.description,
            "input_schema": self.input_schema,
            "output_schema": self.output_schema
        })
    }
}

/// Typed tool definition helper that provides compile-time type safety.
///
/// This struct provides a type-safe wrapper around tool definitions, ensuring
/// that the JSON schema is correctly generated from the input type. Use this
/// when you want to maintain type information before converting to the
/// type-erased `ToolDefinition`.
///
/// # Type Parameters
///
/// - `T`: The input type that implements `JsonSchema`
///
/// # Example
///
/// ```rust
/// use solidmcp::handler::TypedToolDefinition;
/// use schemars::JsonSchema;
/// use serde::Deserialize;
///
/// #[derive(JsonSchema, Deserialize)]
/// struct CalculateInput {
///     a: f64,
///     b: f64,
///     operation: String,
/// }
///
/// let typed_tool = TypedToolDefinition::<CalculateInput>::new(
///     "calculate",
///     "Perform arithmetic operations"
/// );
///
/// // Convert to ToolDefinition for storage
/// let tool = typed_tool.to_tool_definition();
/// ```
#[derive(Debug, Clone)]
pub struct TypedToolDefinition<T: JsonSchema> {
    pub name: String,
    pub description: String,
    pub input_schema: std::marker::PhantomData<T>,
}

impl<T: JsonSchema> TypedToolDefinition<T> {
    /// Create a new typed tool definition.
    ///
    /// # Parameters
    ///
    /// - `name`: The unique name for this tool
    /// - `description`: Human-readable description of the tool's purpose
    ///
    /// # Returns
    ///
    /// A new `TypedToolDefinition` maintaining type information for input type `T`
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            input_schema: std::marker::PhantomData,
        }
    }

    /// Convert to a type-erased ToolDefinition for collections.
    ///
    /// This method creates a `ToolDefinition` with the JSON schema generated
    /// from the type parameter `T`. Use this when you need to store multiple
    /// tools with different input types in the same collection.
    ///
    /// # Returns
    ///
    /// A `ToolDefinition` with the schema generated from type `T`
    pub fn to_tool_definition(&self) -> ToolDefinition {
        ToolDefinition::from_schema::<T>(self.name.clone(), self.description.clone())
    }

    /// Get the JSON schema for this tool's input.
    ///
    /// This method generates and returns the JSON Schema for the input type `T`.
    /// Useful for debugging or when you need direct access to the schema.
    ///
    /// # Returns
    ///
    /// A JSON value containing the generated schema for type `T`
    pub fn get_input_schema(&self) -> Value {
        let schema = schemars::schema_for!(T);
        serde_json::to_value(schema).unwrap_or_else(|_| {
            serde_json::json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            })
        })
    }
}

/// Resource information for MCP resources/list response.
///
/// This struct represents metadata about a resource that can be accessed
/// through the MCP protocol. Resources are identified by URIs and can
/// represent files, database entries, API responses, or any other data.
///
/// # Fields
///
/// - `uri`: Unique identifier for the resource (e.g., "file:///path/to/file")
/// - `name`: Human-readable name for the resource
/// - `description`: Optional description of the resource's contents
/// - `mime_type`: Optional MIME type hint (e.g., "text/plain", "application/json")
///
/// # Example
///
/// ```rust
/// use solidmcp::handler::ResourceInfo;
///
/// let resource = ResourceInfo {
///     uri: "file:///data/users.json".to_string(),
///     name: "User Database".to_string(),
///     description: Some("JSON file containing user records".to_string()),
///     mime_type: Some("application/json".to_string()),
/// };
/// ```
#[derive(Debug, Clone)]
pub struct ResourceInfo {
    pub uri: String,
    pub name: String,
    pub description: Option<String>,
    pub mime_type: Option<String>,
}

/// Resource content for MCP resources/read response.
///
/// This struct contains the actual content of a resource when read through
/// the MCP protocol. It includes the resource URI, optional MIME type,
/// and the content as a string.
///
/// # Fields
///
/// - `uri`: The URI of the resource being returned
/// - `mime_type`: Optional MIME type of the content
/// - `content`: The actual content of the resource as a string
///
/// # Example
///
/// ```rust
/// use solidmcp::handler::ResourceContent;
///
/// let content = ResourceContent {
///     uri: "db://users/123".to_string(),
///     mime_type: Some("application/json".to_string()),
///     content: r#"{"id": 123, "name": "Alice", "email": "alice@example.com"}"#.to_string(),
/// };
/// ```
#[derive(Debug, Clone)]
pub struct ResourceContent {
    pub uri: String,
    pub mime_type: Option<String>,
    pub content: String,
}

/// Prompt information for MCP prompts/list response.
///
/// This struct represents a prompt template that can be used by MCP clients.
/// Prompts are parameterized templates that generate conversation messages
/// for AI models based on provided arguments.
///
/// # Fields
///
/// - `name`: Unique identifier for the prompt
/// - `description`: Optional human-readable description
/// - `arguments`: List of arguments that can be passed to the prompt
///
/// # Example
///
/// ```rust
/// use solidmcp::handler::{PromptInfo, PromptArgument};
///
/// let prompt = PromptInfo {
///     name: "code_review".to_string(),
///     description: Some("Generate a code review for the provided code".to_string()),
///     arguments: vec![
///         PromptArgument {
///             name: "code".to_string(),
///             description: Some("The code to review".to_string()),
///             required: true,
///         },
///         PromptArgument {
///             name: "focus_areas".to_string(),
///             description: Some("Specific areas to focus on".to_string()),
///             required: false,
///         },
///     ],
/// };
/// ```
#[derive(Debug, Clone)]
pub struct PromptInfo {
    pub name: String,
    pub description: Option<String>,
    pub arguments: Vec<PromptArgument>,
}

/// Prompt argument definition.
///
/// Represents a single argument that can be passed to a prompt template.
/// Arguments can be required or optional and include descriptions to help
/// users understand their purpose.
///
/// # Fields
///
/// - `name`: The argument name (used as key when passing values)
/// - `description`: Optional description of what this argument is for
/// - `required`: Whether this argument must be provided
///
/// # Example
///
/// ```rust
/// use solidmcp::handler::PromptArgument;
///
/// let arg = PromptArgument {
///     name: "language".to_string(),
///     description: Some("The programming language of the code".to_string()),
///     required: false,
/// };
/// ```
#[derive(Debug, Clone)]
pub struct PromptArgument {
    pub name: String,
    pub description: Option<String>,
    pub required: bool,
}

/// Prompt content for MCP prompts/get response.
///
/// Contains the generated messages that make up a prompt. These messages
/// are typically sent to an AI model as a conversation history.
///
/// # Fields
///
/// - `messages`: List of messages forming the prompt conversation
///
/// # Example
///
/// ```rust
/// use solidmcp::handler::{PromptContent, PromptMessage};
///
/// let content = PromptContent {
///     messages: vec![
///         PromptMessage {
///             role: "system".to_string(),
///             content: "You are a helpful code reviewer.".to_string(),
///         },
///         PromptMessage {
///             role: "user".to_string(),
///             content: "Please review this Python function for bugs.".to_string(),
///         },
///     ],
/// };
/// ```
#[derive(Debug, Clone)]
pub struct PromptContent {
    pub messages: Vec<PromptMessage>,
}

/// Prompt message.
///
/// Represents a single message in a prompt conversation. Each message has
/// a role (typically "system", "user", or "assistant") and content.
///
/// # Fields
///
/// - `role`: The role of the message sender (e.g., "system", "user", "assistant")
/// - `content`: The actual message content
///
/// # Example
///
/// ```rust
/// use solidmcp::handler::PromptMessage;
///
/// let system_message = PromptMessage {
///     role: "system".to_string(),
///     content: "You are an expert Rust programmer.".to_string(),
/// };
///
/// let user_message = PromptMessage {
///     role: "user".to_string(),
///     content: "How do I implement error handling in Rust?".to_string(),
/// };
/// ```
#[derive(Debug, Clone)]
pub struct PromptMessage {
    pub role: String,
    pub content: String,
}

/// Core trait that users must implement to provide MCP functionality.
///
/// This is the main integration point for the solidmcp library. Implement this
/// trait to define your server's behavior. All methods have default implementations
/// except for `list_tools` and `call_tool`, which must be implemented.
///
/// # Required Methods
///
/// - `list_tools`: Return the list of available tools
/// - `call_tool`: Execute a tool with given arguments
///
/// # Optional Methods
///
/// All other methods have default implementations that can be overridden:
/// - `initialize`: Customize server capabilities and info
/// - `list_resources`: Provide available resources
/// - `read_resource`: Implement resource reading
/// - `list_prompts`: Provide available prompts
/// - `get_prompt`: Generate prompt content
/// - `cancel_notification`: Handle cancellation requests
/// - `handle_initialized`: React to client initialization
///
/// # Example Implementation
///
/// ```rust
/// use solidmcp::handler::{McpHandler, McpContext, ToolDefinition};
/// use anyhow::Result;
/// use async_trait::async_trait;
/// use serde_json::{json, Value};
///
/// struct MyHandler {
///     tools: Vec<ToolDefinition>,
/// }
///
/// #[async_trait]
/// impl McpHandler for MyHandler {
///     async fn list_tools(&self, _context: &McpContext) -> Result<Vec<ToolDefinition>> {
///         Ok(self.tools.clone())
///     }
///
///     async fn call_tool(&self, name: &str, arguments: Value, context: &McpContext) -> Result<Value> {
///         match name {
///             "echo" => {
///                 let message = arguments.get("message")
///                     .and_then(|v| v.as_str())
///                     .unwrap_or("Hello");
///                 Ok(json!({ "echoed": message }))
///             }
///             _ => Err(anyhow::anyhow!("Unknown tool: {}", name))
///         }
///     }
/// }
/// ```
#[async_trait]
pub trait McpHandler: Send + Sync {
    /// Initialize the handler with client information.
    ///
    /// Called when a client sends an initialize request. This is the first
    /// method called in the MCP protocol handshake. Override this to customize
    /// server capabilities and information.
    ///
    /// # Parameters
    ///
    /// - `params`: The initialization parameters from the client
    /// - `context`: The MCP context for this session
    ///
    /// # Returns
    ///
    /// A JSON value containing:
    /// - `protocolVersion`: The MCP protocol version (e.g., "2025-06-18")
    /// - `capabilities`: Object describing server capabilities
    /// - `serverInfo`: Object with server name and version
    ///
    /// # Default Implementation
    ///
    /// Returns basic server info with no special capabilities
    async fn initialize(&self, _params: Value, _context: &McpContext) -> Result<Value> {
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

    /// List available tools.
    ///
    /// Called when a client sends a tools/list request. This method must be
    /// implemented to return all tools that your server provides.
    ///
    /// # Parameters
    ///
    /// - `context`: The MCP context for this session
    ///
    /// # Returns
    ///
    /// A vector of `ToolDefinition` structs describing available tools
    async fn list_tools(&self, context: &McpContext) -> Result<Vec<ToolDefinition>>;

    /// Execute a tool.
    ///
    /// Called when a client sends a tools/call request. This method must be
    /// implemented to handle tool execution with the provided arguments.
    ///
    /// # Parameters
    ///
    /// - `name`: The name of the tool to execute
    /// - `arguments`: JSON value containing the tool's input parameters
    /// - `context`: The MCP context for this session
    ///
    /// # Returns
    ///
    /// A JSON value containing the tool's output
    ///
    /// # Errors
    ///
    /// Return an error if:
    /// - The tool name is not recognized
    /// - The arguments are invalid
    /// - The tool execution fails
    async fn call_tool(&self, name: &str, arguments: Value, context: &McpContext) -> Result<Value>;

    /// List available resources.
    ///
    /// Called when a client sends a resources/list request. Override this
    /// to provide a list of resources that clients can read.
    ///
    /// # Parameters
    ///
    /// - `context`: The MCP context for this session
    ///
    /// # Returns
    ///
    /// A vector of `ResourceInfo` structs describing available resources
    ///
    /// # Default Implementation
    ///
    /// Returns an empty vector (no resources)
    async fn list_resources(&self, _context: &McpContext) -> Result<Vec<ResourceInfo>> {
        // Default implementation - no resources
        Ok(vec![])
    }

    /// Read a resource.
    ///
    /// Called when a client sends a resources/read request. Override this
    /// to implement resource reading logic.
    ///
    /// # Parameters
    ///
    /// - `uri`: The URI of the resource to read
    /// - `context`: The MCP context for this session
    ///
    /// # Returns
    ///
    /// A `ResourceContent` struct containing the resource data
    ///
    /// # Default Implementation
    ///
    /// Returns an error indicating the resource was not found
    async fn read_resource(&self, uri: &str, _context: &McpContext) -> Result<ResourceContent> {
        Err(anyhow::anyhow!("Resource not found: {}", uri))
    }

    /// List available prompts.
    ///
    /// Called when a client sends a prompts/list request. Override this
    /// to provide a list of prompt templates that clients can use.
    ///
    /// # Parameters
    ///
    /// - `context`: The MCP context for this session
    ///
    /// # Returns
    ///
    /// A vector of `PromptInfo` structs describing available prompts
    ///
    /// # Default Implementation
    ///
    /// Returns an empty vector (no prompts)
    async fn list_prompts(&self, _context: &McpContext) -> Result<Vec<PromptInfo>> {
        // Default implementation - no prompts
        Ok(vec![])
    }

    /// Get a prompt.
    ///
    /// Called when a client sends a prompts/get request. Override this
    /// to generate prompt content based on the template and arguments.
    ///
    /// # Parameters
    ///
    /// - `name`: The name of the prompt template
    /// - `arguments`: Optional JSON arguments for the prompt
    /// - `context`: The MCP context for this session
    ///
    /// # Returns
    ///
    /// A `PromptContent` struct containing the generated messages
    ///
    /// # Default Implementation
    ///
    /// Returns an error indicating the prompt was not found
    async fn get_prompt(
        &self,
        name: &str,
        _arguments: Option<Value>,
        _context: &McpContext,
    ) -> Result<PromptContent> {
        Err(anyhow::anyhow!("Prompt not found: {}", name))
    }

    /// Handle notification cancellation.
    ///
    /// Called when a client sends a notifications/cancel request. Override this
    /// to handle cancellation of long-running operations.
    ///
    /// # Parameters
    ///
    /// - `params`: Cancellation parameters (typically contains a progress token)
    /// - `context`: The MCP context for this session
    ///
    /// # Returns
    ///
    /// An empty JSON object on success
    ///
    /// # Default Implementation
    ///
    /// Acknowledges the cancellation without taking action
    async fn cancel_notification(&self, _params: Value, _context: &McpContext) -> Result<Value> {
        // Default implementation - acknowledge cancellation
        Ok(serde_json::json!({}))
    }

    /// Handle initialized notification.
    ///
    /// Called when a client sends a notifications/initialized notification,
    /// indicating that the client has completed its initialization. Override
    /// this to perform any post-initialization setup.
    ///
    /// # Parameters
    ///
    /// - `context`: The MCP context for this session
    ///
    /// # Returns
    ///
    /// Ok(()) on success
    ///
    /// # Default Implementation
    ///
    /// Does nothing and returns Ok(())
    async fn handle_initialized(&self, _context: &McpContext) -> Result<()> {
        // Default implementation - do nothing
        Ok(())
    }
}
