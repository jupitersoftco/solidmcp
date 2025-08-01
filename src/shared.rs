//! MCP Protocol Engine
//!
//! This module provides the core protocol routing and session management for MCP messages.
//! The `McpProtocolEngine` is responsible for maintaining per-session state, routing
//! JSON-RPC messages to appropriate handlers, and managing protocol version negotiation.
//!
//! # Architecture
//!
//! The engine maintains a separate protocol handler for each session, ensuring proper
//! client isolation. It routes incoming JSON-RPC messages to either user-provided
//! handlers or the built-in protocol implementation.
//!
//! # Session Management
//!
//! - WebSocket connections maintain state per connection
//! - HTTP connections use session cookies for state persistence
//! - Sessions can be re-initialized (important for reconnecting clients)
//!
//! # Example
//!
//! ```rust
//! use solidmcp::shared::McpProtocolEngine;
//! use std::sync::Arc;
//!
//! // Create engine with custom handler
//! let handler = Arc::new(MyHandler::new());
//! let engine = McpProtocolEngine::with_handler(handler);
//!
//! // Handle a message
//! let message = serde_json::json!({
//!     "jsonrpc": "2.0",
//!     "method": "initialize",
//!     "params": {},
//!     "id": 1
//! });
//!
//! let response = engine.handle_message(message, Some("session-123".to_string())).await?;
//! ```

use {
    super::protocol_impl::McpProtocolHandlerImpl,
    anyhow::Result,
    serde_json::{json, Value},
    std::collections::HashMap,
    std::sync::Arc,
    tokio::sync::Mutex,
    tracing::{debug, trace},
};

/// Core protocol engine for routing MCP messages and managing sessions.
///
/// The `McpProtocolEngine` is the central message router that maintains per-session
/// protocol handlers and routes messages to the appropriate handler implementation.
/// It supports both custom handlers and a built-in default implementation.
///
/// # Thread Safety
///
/// The engine is thread-safe and can be shared across multiple connections using
/// `Arc`. Session handlers are protected by a mutex to ensure safe concurrent access.
///
/// # Fields
///
/// - `session_handlers`: Thread-safe map of session IDs to protocol handler instances
/// - `handler`: Optional custom handler implementing the `McpHandler` trait
pub struct McpProtocolEngine {
    // Maintain protocol handlers per session ID for proper client isolation
    session_handlers: Arc<Mutex<HashMap<String, McpProtocolHandlerImpl>>>,
    // Handler implementation for MCP functionality
    handler: Option<Arc<dyn super::handler::McpHandler>>,
}

impl Default for McpProtocolEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl McpProtocolEngine {
    /// Create a new protocol engine with no custom handler.
    ///
    /// This creates an engine that uses the built-in protocol implementation
    /// for all MCP functionality. The built-in handler provides basic protocol
    /// compliance but no custom tools, resources, or prompts.
    ///
    /// # Returns
    ///
    /// A new `McpProtocolEngine` instance
    ///
    /// # Example
    ///
    /// ```rust
    /// let engine = McpProtocolEngine::new();
    /// ```
    pub fn new() -> Self {
        Self {
            session_handlers: Arc::new(Mutex::new(HashMap::new())),
            handler: None,
        }
    }

    /// Create a new protocol engine with a custom handler.
    ///
    /// This creates an engine that routes MCP protocol calls to your custom
    /// handler implementation. The handler will receive all tool calls,
    /// resource requests, and prompt requests.
    ///
    /// # Parameters
    ///
    /// - `handler`: Arc-wrapped implementation of the `McpHandler` trait
    ///
    /// # Returns
    ///
    /// A new `McpProtocolEngine` configured with the custom handler
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::sync::Arc;
    ///
    /// let handler = Arc::new(MyCustomHandler::new());
    /// let engine = McpProtocolEngine::with_handler(handler);
    /// ```
    pub fn with_handler(handler: Arc<dyn super::handler::McpHandler>) -> Self {
        debug!("Handler registered with MCP protocol engine");
        Self {
            session_handlers: Arc::new(Mutex::new(HashMap::new())),
            handler: Some(handler),
        }
    }
}

impl McpProtocolEngine {
    /// Handle an MCP message and return the response.
    ///
    /// This is the core message routing logic that works for both WebSocket and HTTP
    /// transports. It maintains initialization state per session/client and routes
    /// messages to the appropriate handler based on the JSON-RPC method.
    ///
    /// # Parameters
    ///
    /// - `message`: The JSON-RPC message to process
    /// - `session_id`: Optional session identifier for maintaining state
    ///
    /// # Returns
    ///
    /// A JSON-RPC response message
    ///
    /// # Message Routing
    ///
    /// The engine routes messages based on the method field:
    /// - `initialize`: Protocol handshake (always handled by protocol implementation)
    /// - `tools/*`: Routed to custom handler if available
    /// - `resources/*`: Routed to custom handler if available
    /// - `prompts/*`: Routed to custom handler if available
    /// - Others: Handled by the protocol implementation
    ///
    /// # Error Handling
    ///
    /// Returns JSON-RPC error responses for:
    /// - Malformed messages (-32700 Parse error)
    /// - Invalid requests (-32600 Invalid Request)
    /// - Unknown methods (-32601 Method not found)
    /// - Handler errors (-32603 Internal error)
    ///
    /// # Example
    ///
    /// ```rust
    /// let message = json!({
    ///     "jsonrpc": "2.0",
    ///     "method": "tools/list",
    ///     "id": 1
    /// });
    ///
    /// let response = engine.handle_message(message, Some("session-123".to_string())).await?;
    /// assert_eq!(response["jsonrpc"], "2.0");
    /// assert_eq!(response["id"], 1);
    /// ```
    pub async fn handle_message(
        &self,
        message: Value,
        session_id: Option<String>, // Session ID for client isolation
    ) -> Result<Value> {
        let method = message["method"].as_str().unwrap_or("");
        trace!(
            "Processing MCP method: {} (session: {:?})",
            method,
            session_id
        );

        // Get or create protocol handler for this session
        let mut sessions = self.session_handlers.lock().await;
        let session_key = session_id
            .as_ref()
            .unwrap_or(&"default".to_string())
            .clone();

        let protocol_handler = sessions.entry(session_key.clone()).or_insert_with(|| {
            trace!("Creating new protocol handler for session: {}", session_key);
            McpProtocolHandlerImpl::new()
        });

        // If we have a custom handler, delegate to it for supported methods
        if let Some(ref custom_handler) = self.handler {
            trace!("Delegating method '{}' to custom handler", method);

            let context = super::handler::McpContext {
                session_id: session_id.clone(),
                notification_sender: None, // TODO: Add notification support
                protocol_version: Some("2025-06-18".to_string()),
                client_info: None,
            };

            match method {
                "initialize" => {
                    let params = message
                        .get("params")
                        .unwrap_or(&serde_json::Value::Null)
                        .clone();

                    // Get the result from the custom handler's initialize method
                    // Note: We can't mutate the handler through the trait, but the CustomMcpHandler
                    // in server.rs returns a static response anyway

                    // Check if already initialized
                    if protocol_handler.initialized {
                        // For HTTP clients that may reconnect, allow re-initialization
                        // This is especially important for MCP clients like Cursor that may
                        // create multiple connections or reconnect frequently
                        debug!("Session {} already initialized, allowing re-initialization for HTTP client", session_key);

                        // Create a fresh protocol handler to ensure clean state
                        *protocol_handler = McpProtocolHandlerImpl::new();

                        debug!(
                            "Created fresh protocol handler for session {} re-initialization",
                            session_key
                        );
                    }

                    match custom_handler.initialize(params, &context).await {
                        Ok(result) => {
                            // Mark session as initialized in the protocol handler
                            protocol_handler.initialized = true;
                            if let Some(client_version) = message
                                .get("params")
                                .and_then(|p| p.get("protocolVersion"))
                                .and_then(|v| v.as_str())
                            {
                                protocol_handler.protocol_version =
                                    Some(client_version.to_string());
                            }

                            let response = serde_json::json!({
                                "jsonrpc": "2.0",
                                "id": message.get("id"),
                                "result": result
                            });
                            return Ok(response);
                        }
                        Err(e) => {
                            return Err(anyhow::anyhow!("Initialize error: {}", e));
                        }
                    }
                }
                "tools/list" => match custom_handler.list_tools(&context).await {
                    Ok(tools) => {
                        let tool_list: Vec<serde_json::Value> = tools
                            .into_iter()
                            .map(|t| {
                                serde_json::json!({
                                    "name": t.name,
                                    "description": t.description,
                                    "inputSchema": t.input_schema,
                                    "outputSchema": t.output_schema,
                                })
                            })
                            .collect();

                        let response = serde_json::json!({
                            "jsonrpc": "2.0",
                            "id": message.get("id"),
                            "result": {
                                "tools": tool_list
                            }
                        });
                        return Ok(response);
                    }
                    Err(e) => {
                        return Err(anyhow::anyhow!("Tools list error: {}", e));
                    }
                },
                "tools/call" => {
                    let params = message.get("params").unwrap_or(&serde_json::Value::Null);
                    if let (Some(name), Some(arguments)) = (
                        params.get("name").and_then(|n| n.as_str()),
                        params.get("arguments"),
                    ) {
                        match custom_handler
                            .call_tool(name, arguments.clone(), &context)
                            .await
                        {
                            Ok(result) => {
                                let response = serde_json::json!({
                                    "jsonrpc": "2.0",
                                    "id": message.get("id"),
                                    "result": result
                                });
                                return Ok(response);
                            }
                            Err(e) => {
                                return Err(anyhow::anyhow!("Tool call error: {}", e));
                            }
                        }
                    }
                }
                "resources/list" => match custom_handler.list_resources(&context).await {
                    Ok(resources) => {
                        let resource_list: Vec<serde_json::Value> = resources
                            .into_iter()
                            .map(|r| {
                                let mut resource = serde_json::json!({
                                    "uri": r.uri,
                                    "name": r.name,
                                });
                                if let Some(desc) = r.description {
                                    resource["description"] = serde_json::Value::String(desc);
                                }
                                if let Some(mime) = r.mime_type {
                                    resource["mimeType"] = serde_json::Value::String(mime);
                                }
                                resource
                            })
                            .collect();

                        let response = serde_json::json!({
                            "jsonrpc": "2.0",
                            "id": message.get("id"),
                            "result": {
                                "resources": resource_list
                            }
                        });
                        return Ok(response);
                    }
                    Err(e) => {
                        return Err(anyhow::anyhow!("Resources list error: {}", e));
                    }
                },
                "resources/read" => {
                    let params = message.get("params").unwrap_or(&serde_json::Value::Null);
                    if let Some(uri) = params.get("uri").and_then(|u| u.as_str()) {
                        match custom_handler.read_resource(uri, &context).await {
                            Ok(content) => {
                                let response = serde_json::json!({
                                    "jsonrpc": "2.0",
                                    "id": message.get("id"),
                                    "result": {
                                        "contents": [
                                            {
                                                "uri": content.uri,
                                                "mimeType": content.mime_type,
                                                "text": content.content,
                                            }
                                        ]
                                    }
                                });
                                return Ok(response);
                            }
                            Err(e) => {
                                return Err(anyhow::anyhow!("Resource read error: {}", e));
                            }
                        }
                    }
                }
                "prompts/list" => match custom_handler.list_prompts(&context).await {
                    Ok(prompts) => {
                        let prompt_list: Vec<serde_json::Value> = prompts
                            .into_iter()
                            .map(|p| {
                                let mut prompt = serde_json::json!({
                                    "name": p.name,
                                });
                                if let Some(desc) = p.description {
                                    prompt["description"] = serde_json::Value::String(desc);
                                }
                                if !p.arguments.is_empty() {
                                    let args: Vec<serde_json::Value> = p
                                        .arguments
                                        .into_iter()
                                        .map(|a| {
                                            let mut arg = serde_json::json!({
                                                "name": a.name,
                                                "required": a.required,
                                            });
                                            if let Some(desc) = a.description {
                                                arg["description"] =
                                                    serde_json::Value::String(desc);
                                            }
                                            arg
                                        })
                                        .collect();
                                    prompt["arguments"] = serde_json::Value::Array(args);
                                }
                                prompt
                            })
                            .collect();

                        let response = serde_json::json!({
                            "jsonrpc": "2.0",
                            "id": message.get("id"),
                            "result": {
                                "prompts": prompt_list
                            }
                        });
                        return Ok(response);
                    }
                    Err(e) => {
                        return Err(anyhow::anyhow!("Prompts list error: {}", e));
                    }
                },
                "prompts/get" => {
                    let params = message.get("params").unwrap_or(&serde_json::Value::Null);
                    if let Some(name) = params.get("name").and_then(|n| n.as_str()) {
                        let arguments = params.get("arguments").cloned();
                        match custom_handler.get_prompt(name, arguments, &context).await {
                            Ok(content) => {
                                let messages: Vec<serde_json::Value> = content
                                    .messages
                                    .into_iter()
                                    .map(|m| {
                                        serde_json::json!({
                                            "role": m.role,
                                            "content": {
                                                "type": "text",
                                                "text": m.content,
                                            }
                                        })
                                    })
                                    .collect();

                                let response = serde_json::json!({
                                    "jsonrpc": "2.0",
                                    "id": message.get("id"),
                                    "result": {
                                        "messages": messages
                                    }
                                });
                                return Ok(response);
                            }
                            Err(e) => {
                                return Err(anyhow::anyhow!("Prompt get error: {}", e));
                            }
                        }
                    }
                }
                _ => {
                    // Fall back to built-in handler for unknown methods
                }
            }
        } else {
            trace!("No custom handler registered, using built-in protocol handler");
        }

        // Fall back to built-in protocol handler
        protocol_handler.handle_message(message).await
    }

    /// Create an error response following JSON-RPC 2.0 format
    fn _create_error_response(&self, id: Option<Value>, code: i32, message: &str) -> Result<Value> {
        Ok(json!({
            "jsonrpc": "2.0",
            "id": id.unwrap_or(Value::Null),
            "error": {
                "code": code,
                "message": message
            }
        }))
    }
}
