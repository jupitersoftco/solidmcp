//! MCP Server Core
//!
//! Core server struct and basic functionality for the Model Context Protocol server.

use {
    super::http_handler::HttpMcpHandler,
    super::logging::McpDebugLogger,
    super::protocol::McpProtocol,
    super::shared::McpProtocolEngine,
    anyhow::{Context, Result},
    std::sync::Arc,
    tracing::debug,
    warp::Filter,
};

pub struct McpServer {
    protocol: McpProtocol,
    protocol_engine: Arc<McpProtocolEngine>,
}

impl McpServer {
    /// Create a new MCP server instance with the default built-in handler.
    ///
    /// This creates a server with a basic handler that provides minimal MCP
    /// functionality. For most use cases, you'll want to use `with_handler()`
    /// or the framework API instead.
    ///
    /// # Returns
    ///
    /// A new `McpServer` instance ready to be started
    ///
    /// # Errors
    ///
    /// Currently this function doesn't fail, but returns Result for future compatibility
    ///
    /// # Example
    ///
    /// ```rust
    /// let mut server = McpServer::new().await?;
    /// server.start(3000).await?;
    /// ```
    pub async fn new() -> Result<Self> {
        let protocol = McpProtocol::new();
        let protocol_engine = Arc::new(McpProtocolEngine::new());

        debug!("🚀 Initializing MCP Server");
        Ok(Self {
            protocol,
            protocol_engine,
        })
    }

    /// Create a new MCP server instance with a custom handler.
    ///
    /// This is the primary way to create a server with your own functionality.
    /// The handler you provide will receive all MCP protocol calls and can
    /// implement tools, resources, and prompts.
    ///
    /// # Parameters
    ///
    /// - `handler`: An Arc-wrapped implementation of the `McpHandler` trait
    ///
    /// # Returns
    ///
    /// A new `McpServer` instance configured with your handler
    ///
    /// # Errors
    ///
    /// Currently this function doesn't fail, but returns Result for future compatibility
    ///
    /// # Example
    ///
    /// ```rust
    /// use solidmcp::{McpServer, handler::McpHandler};
    /// use std::sync::Arc;
    /// use async_trait::async_trait;
    /// use anyhow::Result;
    /// use serde_json::Value;
    ///
    /// struct MyHandler;
    ///
    /// #[async_trait]
    /// impl McpHandler for MyHandler {
    ///     async fn list_tools(&self, context: &McpContext) -> Result<Vec<ToolDefinition>> {
    ///         // Return your tools
    ///         Ok(vec![])
    ///     }
    ///     
    ///     async fn call_tool(&self, name: &str, arguments: Value, context: &McpContext) -> Result<Value> {
    ///         // Handle tool calls
    ///         Err(anyhow::anyhow!("No tools available"))
    ///     }
    /// }
    ///
    /// let handler = Arc::new(MyHandler);
    /// let mut server = McpServer::with_handler(handler).await?;
    /// ```
    pub async fn with_handler(handler: Arc<dyn super::handler::McpHandler>) -> Result<Self> {
        let protocol = McpProtocol::new();
        let protocol_engine = Arc::new(McpProtocolEngine::with_handler(handler));

        debug!("🚀 Initializing MCP Server with custom handler");
        Ok(Self {
            protocol,
            protocol_engine,
        })
    }

    /// Start the MCP server on the specified port.
    ///
    /// This method starts the server listening on the given port for both
    /// WebSocket and HTTP connections. The server automatically detects the
    /// transport type based on request headers and handles each accordingly.
    ///
    /// # Parameters
    ///
    /// - `port`: The TCP port to listen on (e.g., 3000, 8080)
    ///
    /// # Returns
    ///
    /// This method runs indefinitely until the server is shut down
    ///
    /// # Errors
    ///
    /// - Port binding errors if the port is already in use
    /// - Network errors during operation
    ///
    /// # Transport Support
    ///
    /// The server supports:
    /// - **WebSocket**: For clients that send `Upgrade: websocket` header
    /// - **HTTP**: For clients that send `Content-Type: application/json`
    /// - **SSE**: For HTTP clients that accept `text/event-stream`
    ///
    /// # Example
    ///
    /// ```rust
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let mut server = McpServer::new().await?;
    ///     
    ///     // Start server on port 3000
    ///     println!("Server running on http://localhost:3000");
    ///     server.start(3000).await?;
    ///     
    ///     Ok(())
    /// }
    /// ```
    pub async fn start(&mut self, port: u16) -> Result<()> {
        debug!("🚀 Starting MCP Server on port {}", port);

        // Create HTTP handler
        let http_handler = HttpMcpHandler::new(self.protocol_engine.clone());

        // Combine WebSocket and HTTP routes on the same /mcp path
        let ws_route = super::websocket::create_ws_handler(self.protocol_engine.clone());

        let http_route = http_handler.route();

        // Add health check endpoint
        let health_route = warp::path!("health")
            .and(warp::get())
            .map(|| warp::reply::with_status("OK", warp::http::StatusCode::OK));

        // Combine routes - warp will handle content negotiation
        let routes = ws_route.or(http_route).or(health_route);

        let addr = format!("127.0.0.1:{port}")
            .parse::<std::net::SocketAddr>()
            .context("Invalid address")?;

        // Try to bind the port first
        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|e| anyhow::anyhow!("Could not bind to {}: {}", addr, e))?;

        crate::logging::log_server_ready(&format!("ws://{addr}/mcp and http://{addr}/mcp"));
        tracing::info!(
            endpoints = ?vec!["WS /mcp (WebSocket upgrade)", "POST /mcp (HTTP JSON-RPC)"],
            "Available endpoints"
        );

        use tokio_stream::wrappers::TcpListenerStream;
        warp::serve(routes)
            .run_incoming(TcpListenerStream::new(listener))
            .await;

        Ok(())
    }


    /// Get the protocol instance.
    ///
    /// Provides access to the underlying protocol configuration and utilities.
    /// This is primarily for internal use.
    ///
    /// # Returns
    ///
    /// A reference to the `McpProtocol` instance
    pub fn protocol(&self) -> &McpProtocol {
        &self.protocol
    }
}
