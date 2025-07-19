//! MCP Protocol Engine
//!
//! Core protocol routing and session management for MCP messages.
//! Routes JSON-RPC messages to user-provided handler implementations.

use {
    super::protocol_impl::McpProtocolHandlerImpl, super::protocol_testable::McpProtocolHandler,
    anyhow::Result, serde_json::Value, std::collections::HashMap, std::sync::Arc,
    tokio::sync::Mutex,
};

pub struct McpProtocolEngine {
    // Maintain protocol handlers per session ID for proper client isolation
    session_handlers: Arc<Mutex<HashMap<String, McpProtocolHandlerImpl>>>,
    // Handler implementation for MCP functionality
    handler: Option<Arc<dyn super::handler::McpHandler>>,
}

impl McpProtocolEngine {
    pub fn new() -> Self {
        Self {
            session_handlers: Arc::new(Mutex::new(HashMap::new())),
            handler: None,
        }
    }

    pub fn with_handler(handler: Arc<dyn super::handler::McpHandler>) -> Self {
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
        let params = message["params"].clone();

        // Get or create protocol handler for this session
        let mut sessions = self.session_handlers.lock().await;
        let session_key = session_id
            .as_ref()
            .unwrap_or(&"default".to_string())
            .clone();

        let protocol_handler = sessions
            .entry(session_key.clone())
            .or_insert_with(|| McpProtocolHandlerImpl::new());

        // If we have a custom handler, delegate to it for supported methods
        if let Some(ref custom_handler) = self.handler {
            match method {
                "initialize" => {
                    // TODO: Call custom_handler.initialize() and update protocol_handler state
                    // For now, fall back to built-in handler
                }
                "tools/list" => {
                    // TODO: Call custom_handler.list_tools()
                    // For now, fall back to built-in handler
                }
                "tools/call" => {
                    // TODO: Call custom_handler.call_tool()
                    // For now, fall back to built-in handler
                }
                // Add other methods as needed
                _ => {}
            }
        }

        // Fall back to built-in protocol handler
        protocol_handler.handle_message(message).await
    }
}
