//! MCP Protocol Implementation
//!
//! Concrete implementation of the MCP protocol handler trait.

use {
    super::protocol::McpProtocol,
    super::tools::McpTools,
    crate::error::{McpError, McpResult},
    serde_json::{json, Value},
    std::sync::atomic::{AtomicBool, Ordering},
    tokio::sync::RwLock,
    tracing::{debug, error, info},
};

// McpError is now imported from crate::error

pub struct McpProtocolHandlerImpl {
    protocol: McpProtocol,
    pub initialized: AtomicBool,
    pub client_info: RwLock<Option<Value>>,
    pub protocol_version: RwLock<Option<String>>,
}

impl McpProtocolHandlerImpl {
    /// Create a new MCP protocol handler
    pub fn new() -> Self {
        Self {
            protocol: McpProtocol::new(),
            initialized: AtomicBool::new(false),
            client_info: RwLock::new(None),
            protocol_version: RwLock::new(None),
        }
    }

    pub fn with_initialized(client_info: Option<Value>, protocol_version: Option<String>) -> Self {
        Self {
            protocol: McpProtocol::new(),
            initialized: AtomicBool::new(true),
            client_info: RwLock::new(client_info),
            protocol_version: RwLock::new(protocol_version),
        }
    }
}

impl McpProtocolHandlerImpl {
    pub async fn handle_message(&self, message: Value) -> McpResult<Value> {
        // Validate required JSON-RPC fields
        let jsonrpc = message
            .get("jsonrpc")
            .and_then(|j| j.as_str())
            .ok_or_else(|| McpError::InvalidParams("Missing or invalid 'jsonrpc' field".to_string()))?;

        if jsonrpc != "2.0" {
            return Err(McpError::InvalidParams(format!("Invalid jsonrpc version: {jsonrpc}")));
        }

        let method = message
            .get("method")
            .and_then(|m| m.as_str())
            .ok_or_else(|| McpError::InvalidParams("Missing or invalid 'method' field".to_string()))?;
        let id = message.get("id").cloned();
        let params = message.get("params").cloned().unwrap_or(json!({}));
        debug!(
            "📥 [PROTOCOL] handle_message: method={:?}, id={:?}, params={:?}",
            method, id, params
        );
        info!(
            "🔍 [MESSAGE] Dispatching method '{}' with id {:?} (initialized: {})",
            method, id, self.initialized.load(Ordering::Relaxed)
        );

        let result = match method {
            "initialize" => {
                // Initialize method requires params field to be present
                if message.get("params").is_none() {
                    Err(McpError::InvalidParams(
                        "Missing params field for initialize method".to_string(),
                    ))
                } else {
                    self.handle_initialize(params).await
                }
            }
            "tools/list" => self.handle_tools_list().await,
            "tools/call" => self.handle_tool_call(params).await,
            "notifications/cancel" => self.handle_cancel(params).await,
            "notifications/initialized" => self.handle_initialized_notification().await,
            "notifications/message" => self.handle_logging_notification(params).await,
            _ => {
                error!("[PROTOCOL] Unknown method: {:?} (id={:?})", method, id);
                Err(McpError::UnknownMethod(method.to_string()))
            }
        };
        match result {
            Ok(success) => {
                debug!(
                    "[PROTOCOL] Success: method={:?}, id={:?}, result={:?}",
                    method, id, success
                );
                if let Some(id_value) = id {
                    Ok(self.protocol.create_success_response(id_value, success))
                } else {
                    Ok(success)
                }
            }
            Err(e) => {
                error!(
                    "[PROTOCOL] Error: method={:?}, id={:?}, error={:?}",
                    method, id, e
                );
                let error_response = e.to_json_rpc_error(id.clone());
                Ok(error_response)
            }
        }
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized.load(Ordering::Relaxed)
    }

    pub fn protocol_version(&self) -> &str {
        self.protocol.version()
    }

    pub fn create_error_response(&self, id: Value, code: i32, message: &str) -> Value {
        self.protocol.create_error_response(id, code, message)
    }
}

impl McpProtocolHandlerImpl {
    /// Handle MCP initialization
    async fn handle_initialize(&self, params: Value) -> McpResult<Value> {
        info!("🔧 [INIT] Processing MCP initialization request");
        info!(
            "   📋 Input params: {}",
            serde_json::to_string_pretty(&params).unwrap_or_else(|_| "<invalid json>".to_string())
        );
        info!("   📋 Current initialized state: {}", self.initialized.load(Ordering::Relaxed));

        // Check if already initialized
        if self.initialized.load(Ordering::Relaxed) {
            info!("⚠️  [INIT] Already initialized! Allowing re-initialization");
            // Reset state for clean re-initialization
            self.initialized.store(false, Ordering::Relaxed);
            *self.client_info.write().await = None;
            *self.protocol_version.write().await = None;
            info!("🔄 [INIT] State reset for re-initialization");
        }

        // Store client info if provided
        if let Some(client_info) = params.get("clientInfo") {
            *self.client_info.write().await = Some(client_info.clone());
            info!("📋 [INIT] Client info stored: {:?}", client_info);
        } else {
            info!("📋 [INIT] No client info provided");
        }

        // Check protocol version compatibility
        if let Some(protocol_version) = params.get("protocolVersion") {
            let client_version = protocol_version.as_str().unwrap_or("");
            info!("🔍 PROTOCOL VERSION NEGOTIATION:");
            info!("   📋 Client protocol version: {}", client_version);
            info!("   📋 Server protocol version: {}", self.protocol.version());

            // Accept both 2025-03-26 (Cursor) and 2025-06-18 (latest) versions
            let supported_versions = ["2025-03-26", "2025-06-18"];
            if !supported_versions.contains(&client_version) {
                error!(
                    "❌ Unsupported protocol version: client={}, supported={:?}",
                    client_version, supported_versions
                );
                return Err(McpError::Internal(format!(
                    "Unsupported protocol version: {client_version}. Supported versions: {supported_versions:?}"
                )));
            }

            // Store the client's protocol version
            *self.protocol_version.write().await = Some(client_version.to_string());

            info!(
                "✅ PROTOCOL VERSION NEGOTIATED: client={}, server supports both {} and {}",
                client_version, supported_versions[0], supported_versions[1]
            );
            info!("   🎯 Using client version: {}", client_version);
        }

        info!("🔧 [INIT] Setting initialized flag to true");
        self.initialized.store(true, Ordering::Relaxed);
        info!("🔧 [INIT] Initialized flag is now: {}", self.initialized.load(Ordering::Relaxed));

        // Create response with the client's protocol version
        let response = if let Some(ref client_version) = *self.protocol_version.read().await {
            // Both protocol versions should enable tools capabilities
            // The key is to indicate that tools are supported and enabled
            let capabilities = json!({
                "tools": {
                    "listChanged": false
                }
            });

            json!({
                "protocolVersion": client_version,
                "capabilities": capabilities,
                "serverInfo": {
                    "name": self.protocol.server_name(),
                    "version": self.protocol.server_version()
                }
            })
        } else {
            self.protocol.create_initialize_response()
        };

        info!("✅ MCP client initialized successfully");
        Ok(response)
    }

    /// Handle tools list request
    async fn handle_tools_list(&self) -> McpResult<Value> {
        info!("🔍 [INIT CHECK] Checking initialization status for tools/list request");
        info!("   📋 Current initialized state: {}", self.initialized.load(Ordering::Relaxed));
        info!("   📋 Protocol version: {:?}", *self.protocol_version.read().await);
        info!("   📋 Client info: {:?}", *self.client_info.read().await);

        if !self.initialized.load(Ordering::Relaxed) {
            error!("❌ [INIT CHECK] Client not initialized! Rejecting tools/list request");
            error!("   📋 This means initialize() was never called or failed");
            error!(
                "   📋 Current state: initialized={}, protocol_version={:?}",
                self.initialized.load(Ordering::Relaxed), *self.protocol_version.read().await
            );
            return Err(McpError::NotInitialized);
        }

        info!("✅ [INIT CHECK] Client is initialized, proceeding with tools/list");
        info!("📋 Processing MCP tools list request");
        info!("   🎯 Using protocol version: {:?}", *self.protocol_version.read().await);
        let response = McpTools::get_tools_list_for_version(self.protocol_version.read().await.as_deref());
        let tools_count = response["tools"]
            .as_array()
            .map(|arr| arr.len())
            .unwrap_or(0);
        info!(
            "📋 Returning {} available tools for protocol version {:?}",
            tools_count, *self.protocol_version.read().await
        );
        Ok(response)
    }

    /// Handle tool calls
    async fn handle_tool_call(&self, params: Value) -> McpResult<Value> {
        if !self.initialized.load(Ordering::Relaxed) {
            return Err(McpError::NotInitialized);
        }

        let tool_name = params["name"].as_str().ok_or_else(|| {
            McpError::InvalidParams("Missing required 'name' field for tool call".to_string())
        })?;
        
        // Validate arguments is present and is an object
        let arguments = params.get("arguments").ok_or_else(|| {
            McpError::InvalidParams("Missing required 'arguments' field for tool call".to_string())
        })?;
        
        // Arguments must be an object
        if !arguments.is_object() {
            return Err(McpError::InvalidParams(
                "Arguments must be an object".to_string()
            ));
        }
        
        let arguments = arguments.clone();

        debug!(
            "🛠️  Processing tool call: {} with args: {:?}",
            tool_name, arguments
        );

        match McpTools::execute_tool(tool_name, arguments).await {
            Ok(result) => {
                info!("🛠️  Tool '{}' executed successfully", tool_name);
                Ok(result)
            }
            Err(e) => {
                let error_msg = e.to_string();
                if error_msg.contains("Unknown tool") {
                    Err(McpError::UnknownTool(tool_name.to_string()))
                } else if error_msg.contains("Missing required parameter") 
                    || error_msg.contains("cannot be empty") 
                    || error_msg.contains("Wrong type for") {
                    // Convert parameter validation errors to InvalidParams
                    Err(McpError::InvalidParams(error_msg))
                } else {
                    Err(McpError::Internal(e.to_string()))
                }
            }
        }
    }

    /// Handle cancel notifications
    async fn handle_cancel(&self, _params: Value) -> McpResult<Value> {
        info!("❌ MCP operation cancelled by client");
        Ok(json!({}))
    }

    /// Handle initialized notification
    async fn handle_initialized_notification(&self) -> McpResult<Value> {
        info!("✅ MCP client sent initialized notification");
        Ok(json!({}))
    }

    /// Handle logging notification
    async fn handle_logging_notification(&self, params: Value) -> McpResult<Value> {
        let level = params
            .get("level")
            .and_then(|l| l.as_str())
            .unwrap_or("info");
        let message = params.get("message").and_then(|m| m.as_str()).unwrap_or("");

        match level {
            "error" => error!("📝 [CLIENT LOG] {}", message),
            "warn" => info!("📝 [CLIENT LOG] WARN: {}", message),
            "info" => info!("📝 [CLIENT LOG] {}", message),
            "debug" => debug!("📝 [CLIENT LOG] {}", message),
            _ => info!("📝 [CLIENT LOG] [{}]: {}", level, message),
        }

        Ok(json!({}))
    }
}

impl Default for McpProtocolHandlerImpl {
    fn default() -> Self {
        Self::new()
    }
}
