//! MCP Server Core
//!
//! Core server struct and basic functionality for the Model Context Protocol server.

use {
    super::handlers::McpHandlers,
    super::http::HttpMcpHandler,
    super::logging::McpDebugLogger,
    super::protocol::McpProtocol,
    super::shared::SharedMcpHandler,
    anyhow::{Context, Result},
    std::sync::Arc,
    tokio::sync::Mutex,
    tracing::info,
    warp::Filter,
};

pub struct McpServer {
    protocol: McpProtocol,
    shared_handler: Arc<SharedMcpHandler>,
}

impl McpServer {
    /// Create a new MCP server instance
    pub async fn new() -> Result<Self> {
        let protocol = McpProtocol::new();
        let shared_handler = Arc::new(SharedMcpHandler::new());

        info!("🚀 Initializing MCP Server");
        Ok(Self {
            protocol,
            shared_handler,
        })
    }

    /// Start the MCP server (WebSocket + HTTP)
    pub async fn start(&mut self, port: u16) -> Result<()> {
        info!("🚀 Starting MCP Server on port {}", port);

        // Create HTTP handler
        let http_handler = HttpMcpHandler::new(self.shared_handler.clone());

        // Combine WebSocket and HTTP routes on the same /mcp path
        let ws_route = warp::path!("mcp")
            .and(warp::ws())
            .and_then(super::websocket::handle_mcp_ws_main);

        let http_route = http_handler.route();

        // Combine routes - warp will handle content negotiation
        let routes = ws_route.or(http_route);

        let addr = format!("127.0.0.1:{}", port)
            .parse::<std::net::SocketAddr>()
            .context("Invalid address")?;

        // Try to bind the port first
        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|e| anyhow::anyhow!("Could not bind to {}: {}", addr, e))?;

        println!(
            "🌐 MCP Server listening on ws://{}/mcp and http://{}/mcp",
            addr, addr
        );
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