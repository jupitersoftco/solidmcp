//! MCP Protocol Engine
//!
//! Core protocol routing and session management for MCP messages.
//! Routes JSON-RPC messages to user-provided handler implementations.

use {
    super::protocol_impl::McpProtocolHandlerImpl,
    super::protocol_testable::McpProtocolHandler,
    anyhow::Result,
    serde_json::Value,
    std::collections::HashMap,
    std::sync::Arc,
    tokio::sync::Mutex,
    tracing::{debug, trace},
};

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
    pub fn new() -> Self {
        Self {
            session_handlers: Arc::new(Mutex::new(HashMap::new())),
            handler: None,
        }
    }

    pub fn with_handler(handler: Arc<dyn super::handler::McpHandler>) -> Self {
        debug!("Handler registered with MCP protocol engine");
        Self {
            session_handlers: Arc::new(Mutex::new(HashMap::new())),
            handler: Some(handler),
        }
    }
}

impl McpProtocolEngine {
    /// Handle an MCP message and return the response
    /// This is the core logic that works for both WebSocket and HTTP
    /// Maintains initialization state per session/client
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
                    let handler_mut = Arc::clone(custom_handler);
                    // Since we can't get a mutable reference to the trait object directly,
                    // we'll need to use interior mutability or redesign this
                    // For now, fall back to built-in handler for initialize
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
}
