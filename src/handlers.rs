//! MCP Protocol Handlers
//!
//! Handles individual MCP protocol methods with comprehensive logging
//! and error handling.

use {
    super::logging::McpDebugLogger,
    super::protocol::McpProtocol,
    super::tools::McpTools,
    anyhow::Result,
    serde_json::{json, Value},
    std::time::Instant,
    tracing::{debug, error, info},
};

pub struct McpHandlers {
    logger: McpDebugLogger,
    protocol: McpProtocol,
}

impl McpHandlers {
    pub fn new(logger: McpDebugLogger) -> Self {
        let protocol = McpProtocol::new();
        Self {
            logger,
            protocol,
        }
    }

    /// Handle MCP protocol messages with comprehensive logging
    pub async fn handle_mcp_message(&self, message: Value) -> Result<Value> {
        tracing::debug!("MCP received: {}", message);
        let start_time = Instant::now();

        let method = message["method"].as_str().unwrap_or("");
        let id = message["id"].clone();
        let params = message["params"].clone();

        // Enhanced JSON logging
        debug!(
            "{}",
            self.logger.fmt_message_parsed(method, &id.to_string())
        );
        debug!(
            "📥 MCP JSON Message: {}",
            serde_json::to_string_pretty(&message).unwrap_or_else(|_| "invalid json".to_string())
        );
        debug!("{}", self.logger.fmt_message_handling_start(method));

        let result = match method {
            "initialize" => self.handle_initialize(params).await?,
            "tools/list" => self.handle_tools_list().await?,
            "tools/call" => self.handle_tool_call(params).await?,
            "notifications/cancel" => self.handle_cancel(params).await?,
            _ => {
                error!("{}", self.logger.fmt_unknown_method(method));
                return Err(anyhow::anyhow!("Unknown method: {}", method));
            }
        };

        let duration = start_time.elapsed();
        debug!(
            "{}",
            self.logger.fmt_message_handling_success(method, duration)
        );

        let response = self.protocol.create_success_response(id, result);
        tracing::debug!("MCP sending: {}", response);
        debug!(
            "📤 MCP JSON Response: {}",
            serde_json::to_string_pretty(&response).unwrap_or_else(|_| "invalid json".to_string())
        );

        Ok(response)
    }

    /// Handle MCP initialization
    pub async fn handle_initialize(&self, _params: Value) -> Result<Value> {
        tracing::info!("🔧 Processing MCP initialization request: {}", _params);

        let response = self.protocol.create_initialize_response();
        tracing::info!("✅ MCP client initialized successfully: {}", response);
        Ok(response)
    }

    /// Handle tools list request
    pub async fn handle_tools_list(&self) -> Result<Value> {
        tracing::info!("📋 Processing MCP tools list request");

        let response = McpTools::get_tools_list();
        tracing::info!("📋 Returning tools list: {}", response);
        Ok(response)
    }

    /// Handle tool calls with detailed logging
    pub async fn handle_tool_call(&self, params: Value) -> Result<Value> {
        let tool_name = params["name"].as_str().unwrap_or("");
        let arguments = params["arguments"].clone();

        let args_str = serde_json::to_string(&arguments).unwrap_or_else(|_| "invalid".to_string());
        debug!("{}", self.logger.fmt_tool_call(tool_name, &args_str));

        let result = McpTools::execute_tool(tool_name, arguments).await?;

        debug!("🛠️  Tool '{}' executed successfully", tool_name);
        Ok(result)
    }

    /// Handle cancel notifications
    pub async fn handle_cancel(&self, _params: Value) -> Result<Value> {
        info!("❌ MCP operation cancelled by client");
        Ok(json!({}))
    }
}