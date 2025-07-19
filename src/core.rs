//! MCP Server Core
//!
//! Core server struct and basic functionality for the Model Context Protocol server.

use {
    super::handlers::McpHandlers,
    super::http::HttpMcpHandler,
    super::logging::McpDebugLogger,
    super::protocol::McpProtocol,
    super::shared::McpProtocolEngine,
    anyhow::{Context, Result},
    std::sync::Arc,
    tracing::info,
    warp::Filter,
};

pub struct McpServer {
    protocol: McpProtocol,
    protocol_engine: Arc<McpProtocolEngine>,
}

impl McpServer {
    /// Create a new MCP server instance
    pub async fn new() -> Result<Self> {
        let protocol = McpProtocol::new();
        let protocol_engine = Arc::new(McpProtocolEngine::new());

        info!("🚀 Initializing MCP Server");
        Ok(Self {
            protocol,
            protocol_engine,
        })
    }

    /// Create a new MCP server instance with a custom handler
    pub async fn with_handler(handler: Arc<dyn super::handler::McpHandler>) -> Result<Self> {
        let protocol = McpProtocol::new();
        let protocol_engine = Arc::new(McpProtocolEngine::with_handler(handler));

        info!("🚀 Initializing MCP Server with custom handler");
        Ok(Self {
            protocol,
            protocol_engine,
        })
    }

    /// Start the MCP server (WebSocket + HTTP)
    pub async fn start(&mut self, port: u16) -> Result<()> {
        info!("🚀 Starting MCP Server on port {}", port);

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

        println!("🌐 MCP Server listening on ws://{addr}/mcp and http://{addr}/mcp");
        println!("📡 Available endpoints:");
        println!("  WS  /mcp (WebSocket upgrade)");
        println!("  POST /mcp (HTTP JSON-RPC)");

        use tokio_stream::wrappers::TcpListenerStream;
        warp::serve(routes)
            .run_incoming(TcpListenerStream::new(listener))
            .await;

        Ok(())
    }

    /// Get a new handler instance for processing messages
    pub fn create_handler(&self) -> McpHandlers {
        let logger = McpDebugLogger::new(super::logging::McpConnectionId::new());
        McpHandlers::new(logger)
    }

    /// Get the protocol instance
    pub fn protocol(&self) -> &McpProtocol {
        &self.protocol
    }
}
