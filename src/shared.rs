//! MCP Shared Handler
//!
//! Shared MCP protocol logic that can be used by both WebSocket and HTTP transports.

use {
    super::protocol_impl::McpProtocolHandlerImpl,
    super::protocol_testable::McpProtocolHandler,
    anyhow::Result,
    serde_json::Value,
    std::collections::HashMap,
    std::sync::Arc,
    tokio::sync::Mutex,
    tracing::debug,
};

pub struct SharedMcpHandler {
    // Maintain protocol handlers per session ID for proper client isolation
    session_handlers: Arc<Mutex<HashMap<String, McpProtocolHandlerImpl>>>,
}

impl SharedMcpHandler {
    pub fn new() -> Self {
        Self {
            session_handlers: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl SharedMcpHandler {
    /// Handle an MCP message and return the response
    /// This is the core logic that works for both WebSocket and HTTP
    /// Maintains initialization state per session/client
    pub async fn handle_message(
        &self,
        message: Value,
        session_id: Option<String>, // Session ID for client isolation
    ) -> Result<Value> {
        debug!(
            "📥 Full MCP message: {}",
            serde_json::to_string_pretty(&message).unwrap_or_else(|_| "invalid json".to_string())
        );
        let method = message["method"].as_str().unwrap_or("");
        let _id = message["id"].clone();
        let params = message["params"].clone();
        let message_clone = message.clone();
        debug!(
            "📥 Processing MCP method: {} (session: {:?})",
            method, session_id
        );
        debug!("📥 Method: {}", method);
        debug!("📥 ID: {:?}", _id);
        debug!(
            "📥 Params: {}",
            serde_json::to_string_pretty(&params).unwrap_or_else(|_| "invalid json".to_string())
        );

        // Get or create protocol handler for this session
        let mut sessions = self.session_handlers.lock().await;
        let session_key = session_id
            .as_ref()
            .unwrap_or(&"default".to_string())
            .clone();

        debug!(
            "[SESSION] handle_message called with session_id: {:?} (session_key: {})",
            session_id, session_key
        );
        let protocol_handler = sessions.entry(session_key.clone()).or_insert_with(|| {
            debug!(
                "[SESSION] Creating new protocol handler for session_key: {}",
                session_key
            );
            McpProtocolHandlerImpl::new()
        });

        debug!(
            "🚦 Using protocol handler for session: {} (initialized: {})",
            session_key,
            protocol_handler.is_initialized()
        );

        // Process the message
        let result = protocol_handler.handle_message(message_clone).await;
        debug!(
            "🚦 Protocol handler returned for session: {} method: {} result: {:?}",
            session_key, method, result
        );
        result
    }

    #[cfg(test)]
    pub async fn clear_sessions(&self) {
        let mut sessions = self.session_handlers.lock().await;
        sessions.clear();
    }
}

impl Default for SharedMcpHandler {
    fn default() -> Self {
        Self::new()
    }
}