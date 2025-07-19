//! MCP Protocol Implementation
//!
//! Concrete implementation of the MCP protocol handler trait.

use {
    super::protocol::McpProtocol,
    super::protocol_testable::McpProtocolHandler,
    super::tools::McpTools,
    anyhow::Result,
    serde_json::{json, Value},
    thiserror::Error,
    tracing::{debug, error, info},
};

#[derive(Debug, Error)]
pub enum McpError {
    #[error("Unknown method: {0}")]
    UnknownMethod(String),
    #[error("Unknown tool: {0}")]
    UnknownTool(String),
    #[error("Client not initialized")]
    NotInitialized,
    #[error("Internal error: {0}")]
    Internal(String),
}

pub struct McpProtocolHandlerImpl {
    protocol: McpProtocol,
    initialized: bool,
    client_info: Option<Value>,
    protocol_version: Option<String>,
}

impl McpProtocolHandlerImpl {
    /// Create a new MCP protocol handler
    pub fn new() -> Self {
        Self {
            protocol: McpProtocol::new(),
            initialized: false,
            client_info: None,
            protocol_version: None,
        }
    }

    pub fn with_initialized(client_info: Option<Value>, protocol_version: Option<String>) -> Self {
        Self {
            protocol: McpProtocol::new(),
            initialized: true,
            client_info,
            protocol_version,
        }
    }
}

#[async_trait::async_trait]
impl McpProtocolHandler for McpProtocolHandlerImpl {
    async fn handle_message(&mut self, message: Value) -> Result<Value> {
        // Validate required fields
        let method = message
            .get("method")
            .and_then(|m| m.as_str())
            .ok_or_else(|| McpError::Internal("Missing or invalid 'method' field".to_string()))?;
        let id = message.get("id").cloned();
        let params = message.get("params").cloned().unwrap_or(json!({}));
        debug!(
            "📥 [PROTOCOL] handle_message: method={:?}, id={:?}, params={:?}",
            method, id, params
        );
        info!(
            "🔍 [MESSAGE] Dispatching method '{}' with id {:?} (initialized: {})",
            method, id, self.initialized
        );

        let result = match method {
            "initialize" => self.handle_initialize(params).await,
            "tools/list" => self.handle_tools_list().await,
            "tools/call" => self.handle_tool_call(params).await,
            "notifications/cancel" => self.handle_cancel(params).await,
            "notifications/initialized" => self.handle_initialized_notification().await,
            _ => {
                error!("[PROTOCOL] Unknown method: {:?} (id={:?})", method, id);
                Err(McpError::UnknownMethod(method.to_string()).into())
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
                let (code, msg): (i32, &str) = if let Some(mcp_err) = e.downcast_ref::<McpError>() {
                    match mcp_err {
                        McpError::UnknownMethod(_) | McpError::UnknownTool(_) => {
                            (-32601, "Method not found")
                        }
                        McpError::NotInitialized => (-32002, "Not initialized"),
                        McpError::Internal(msg) => (-32603, msg.as_str()),
                    }
                } else {
                    (-32603, "Internal error")
                };
                if let Some(id_value) = id {
                    Ok(self.protocol.create_error_response(id_value, code, msg))
                } else {
                    Ok(json!({
                        "jsonrpc": "2.0",
                        "error": {
                            "code": code,
                            "message": msg
                        }
                    }))
                }
            }
        }
    }

    fn is_initialized(&self) -> bool {
        self.initialized
    }

    fn protocol_version(&self) -> &str {
        self.protocol.version()
    }

    fn create_error_response(&self, id: Value, code: i32, message: &str) -> Value {
        self.protocol.create_error_response(id, code, message)
    }
}

impl McpProtocolHandlerImpl {
    /// Handle MCP initialization
    async fn handle_initialize(&mut self, params: Value) -> Result<Value> {
        info!("🔧 [INIT] Processing MCP initialization request");
        info!(
            "   📋 Input params: {}",
            serde_json::to_string_pretty(&params).unwrap()
        );
        info!("   📋 Current initialized state: {}", self.initialized);

        // Store client info if provided
        if let Some(client_info) = params.get("clientInfo") {
            self.client_info = Some(client_info.clone());
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
                    "Unsupported protocol version: {}. Supported versions: {:?}",
                    client_version, supported_versions
                ))
                .into());
            }

            // Store the client's protocol version
            self.protocol_version = Some(client_version.to_string());

            info!(
                "✅ PROTOCOL VERSION NEGOTIATED: client={}, server supports both {} and {}",
                client_version, supported_versions[0], supported_versions[1]
            );
            info!("   🎯 Using client version: {}", client_version);
        }

        info!("🔧 [INIT] Setting initialized flag to true");
        self.initialized = true;
        info!("🔧 [INIT] Initialized flag is now: {}", self.initialized);

        // Create response with the client's protocol version
        let response = if let Some(ref client_version) = self.protocol_version {
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
    async fn handle_tools_list(&mut self) -> Result<Value> {
        info!("🔍 [INIT CHECK] Checking initialization status for tools/list request");
        info!("   📋 Current initialized state: {}", self.initialized);
        info!("   📋 Protocol version: {:?}", self.protocol_version);
        info!("   📋 Client info: {:?}", self.client_info);

        if !self.initialized {
            error!("❌ [INIT CHECK] Client not initialized! Rejecting tools/list request");
            error!("   📋 This means initialize() was never called or failed");
            error!(
                "   📋 Current state: initialized={}, protocol_version={:?}",
                self.initialized, self.protocol_version
            );
            return Err(McpError::NotInitialized.into());
        }

        info!("✅ [INIT CHECK] Client is initialized, proceeding with tools/list");
        info!("📋 Processing MCP tools list request");
        info!("   🎯 Using protocol version: {:?}", self.protocol_version);
        let response = McpTools::get_tools_list_for_version(self.protocol_version.as_deref());
        info!(
            "📋 Returning {} available tools for protocol version {:?}",
            response["tools"].as_array().unwrap().len(),
            self.protocol_version
        );
        Ok(response)
    }

    /// Handle tool calls
    async fn handle_tool_call(&mut self, params: Value) -> Result<Value> {
        if !self.initialized {
            return Err(McpError::NotInitialized.into());
        }

        let tool_name = params["name"].as_str().unwrap_or("");
        let arguments = params["arguments"].clone();

        debug!(
            "🛠️  Processing tool call: {} with args: {:?}",
            tool_name, arguments
        );

        let result = McpTools::execute_tool(tool_name, arguments).await?;
        info!("🛠️  Tool '{}' executed successfully", tool_name);
        Ok(result)
    }

    /// Handle cancel notifications
    async fn handle_cancel(&mut self, _params: Value) -> Result<Value> {
        info!("❌ MCP operation cancelled by client");
        Ok(json!({}))
    }

    /// Handle initialized notification
    async fn handle_initialized_notification(&mut self) -> Result<Value> {
        info!("✅ MCP client sent initialized notification");
        Ok(json!({}))
    }
}

impl Default for McpProtocolHandlerImpl {
    fn default() -> Self {
        Self::new()
    }
}