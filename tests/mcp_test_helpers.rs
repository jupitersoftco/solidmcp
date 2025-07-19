//! Shared MCP Test Helpers
//!
//! Common utilities for MCP integration tests that use dynamic ports
//! and proper server lifecycle management.

use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::time::Duration;
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::Message;
use tracing::info;

/// Find an available port for testing
pub async fn find_available_port() -> Result<u16, Box<dyn std::error::Error + Send + Sync>> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();
    drop(listener);
    Ok(port)
}

/// Test server handle that manages a dynamic port server
pub struct McpTestServer {
    pub port: u16,
    pub server_handle: tokio::task::JoinHandle<()>,
}

impl McpTestServer {
    /// Start a new MCP test server on a dynamic port
    pub async fn start() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // Find an available port
        let port = find_available_port().await?;

        // Start the server on the dynamic port
        let server_handle = tokio::spawn(async move {
            // Create the MCP server
            let server = solidmcp::McpServer::new();
            
            // Start the server with both HTTP and WebSocket support
            if let Err(e) = server.start(port).await {
                eprintln!("Test server error: {}", e);
            }
        });

        // Wait a bit for the server to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        Ok(Self {
            port,
            server_handle,
        })
    }

    /// Get the WebSocket URL for this server
    pub fn ws_url(&self) -> String {
        format!("ws://127.0.0.1:{}/mcp", self.port)
    }

    /// Get the HTTP URL for this server
    pub fn http_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    /// Stop the server
    pub async fn stop(self) {
        self.server_handle.abort();
        let _ = self.server_handle.await;
    }
}

/// Helper function to receive a WebSocket message with timeout
pub async fn receive_ws_message(
    read: &mut futures_util::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    >,
    timeout_duration: Duration,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let message = timeout(timeout_duration, read.next())
        .await
        .map_err(|_| "Timeout waiting for WebSocket message")?
        .ok_or("WebSocket stream ended unexpectedly")?
        .map_err(|e| format!("WebSocket error: {}", e))?;

    match message {
        Message::Text(text) => Ok(text),
        Message::Close(_) => Err("WebSocket connection closed".into()),
        _ => Err("Unexpected message type".into()),
    }
}

/// Helper function to initialize MCP connection with a test server
pub async fn initialize_mcp_connection_with_server(
    server: &McpTestServer,
) -> Result<
    (
        futures_util::stream::SplitSink<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
            tokio_tungstenite::tungstenite::Message,
        >,
        futures_util::stream::SplitStream<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
        >,
    ),
    Box<dyn std::error::Error + Send + Sync>,
> {
    let (ws_stream, _) = tokio_tungstenite::connect_async(&server.ws_url()).await?;
    let (mut write, mut read) = ws_stream.split();

    let init_message = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {"name": "test-client", "version": "1.0.0"}
        }
    });

    write
        .send(Message::Text(serde_json::to_string(&init_message)?))
        .await?;
    let _response = receive_ws_message(&mut read, Duration::from_secs(5)).await?;

    Ok((write, read))
}

/// Helper function to run a test with a managed MCP server
pub async fn with_mcp_test_server<F, Fut, T>(
    test_name: &str,
    test_fn: F,
) -> Result<T, Box<dyn std::error::Error + Send + Sync>>
where
    F: FnOnce(McpTestServer) -> Fut,
    Fut: std::future::Future<Output = Result<T, Box<dyn std::error::Error + Send + Sync>>>,
{
    info!("🚀 Starting MCP test server for: {}", test_name);

    let server = McpTestServer::start().await?;
    info!("✅ MCP test server started on port {}", server.port);

    let result = test_fn(server).await;

    info!("🛑 Stopping MCP test server for: {}", test_name);

    result
}

/// Helper function to run a test with a managed MCP server and initialized connection
pub async fn with_mcp_connection<F, Fut, T>(
    test_name: &str,
    test_fn: F,
) -> Result<T, Box<dyn std::error::Error + Send + Sync>>
where
    F: FnOnce(
        McpTestServer,
        futures_util::stream::SplitSink<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
            tokio_tungstenite::tungstenite::Message,
        >,
        futures_util::stream::SplitStream<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
        >,
    ) -> Fut,
    Fut: std::future::Future<Output = Result<T, Box<dyn std::error::Error + Send + Sync>>>,
{
    with_mcp_test_server(test_name, |server| async move {
        let (write, read) = initialize_mcp_connection_with_server(&server).await?;
        test_fn(server, write, read).await
    })
    .await
}

/// Initialize test tracing for debugging
pub fn init_test_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("debug")
        .with_test_writer()
        .try_init();
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing::info;

    #[tokio::test]
    async fn test_mcp_test_server_lifecycle() {
        init_test_tracing();
        
        let server = McpTestServer::start().await.unwrap();

        // Verify server is running
        assert!(server.port > 0);
        assert!(!server.ws_url().is_empty());
        assert!(!server.http_url().is_empty());

        info!("✅ Server started on port {}", server.port);

        // Stop server
        server.stop().await;

        info!("✅ Server stopped successfully");
    }

    #[tokio::test]
    async fn test_with_mcp_test_server() {
        init_test_tracing();
        
        let result = with_mcp_test_server("test_lifecycle", |server| async move {
            assert!(server.port > 0);
            Ok("test_passed")
        })
        .await
        .unwrap();

        assert_eq!(result, "test_passed");
    }

    #[tokio::test]
    async fn test_with_mcp_connection() {
        init_test_tracing();
        
        let result = with_mcp_connection(
            "test_connection",
            |_server, mut write, mut read| async move {
                // Test that connection is working by sending a tools/list request
                let tools_message = json!({
                    "jsonrpc": "2.0",
                    "id": 2,
                    "method": "tools/list",
                    "params": {}
                });

                write
                    .send(Message::Text(serde_json::to_string(&tools_message)?))
                    .await?;

                let response_text = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
                let response: Value = serde_json::from_str(&response_text)?;

                assert_eq!(response["jsonrpc"], "2.0");
                assert_eq!(response["id"], 2);
                assert!(response.get("result").is_some());

                Ok("connection_test_passed")
            },
        )
        .await
        .unwrap();

        assert_eq!(result, "connection_test_passed");
    }
}