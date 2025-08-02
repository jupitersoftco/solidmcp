//! Test helpers for integration tests
//!
//! Provides utilities for creating test servers and clients to verify
//! full protocol flows across different transports.

use solidmcp::{McpServerBuilder, McpServer};
use serde_json::{json, Value};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use std::time::Duration;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Test context for integration tests
#[derive(Debug, Clone)]
pub struct TestContext {
    pub name: String,
    pub counter: Arc<std::sync::atomic::AtomicU32>,
}

impl TestContext {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            counter: Arc::new(std::sync::atomic::AtomicU32::new(0)),
        }
    }

    pub fn increment(&self) -> u32 {
        self.counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1
    }
}

/// Input type for test tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct TestInput {
    pub input: String,
}

/// Output type for test tool
#[derive(Debug, Serialize, JsonSchema)]
pub struct TestOutput {
    pub output: String,
    pub count: u32,
}

/// A test server that can be started and stopped for integration tests
pub struct TestServer {
    pub addr: SocketAddr,
    pub handle: tokio::task::JoinHandle<anyhow::Result<()>>,
}

impl TestServer {
    /// Start a test server with a simple test tool on a dynamic port
    pub async fn start() -> Self {
        let context = TestContext::new("test-server");
        
        let server = McpServerBuilder::new(context, "test-server", "1.0.0")
            .with_tool(
                "test_tool",
                "A test tool for integration testing",
                |input: TestInput, ctx: Arc<TestContext>, _notify| async move {
                    let count = ctx.increment();
                    Ok(TestOutput {
                        output: format!("Processed: {}", input.input),
                        count,
                    })
                }
            )
            .with_tool(
                "error_tool",
                "A tool that always errors for testing",
                |_input: TestInput, _ctx: Arc<TestContext>, _notify| async move {
                    let result: Result<TestOutput, solidmcp::McpError> = 
                        Err(solidmcp::McpError::Internal("Test error".to_string()));
                    result
                }
            )
            .build()
            .await
            .expect("Failed to build test server");
        
        // Use the new dynamic port allocation
        let (handle, port) = server.start_dynamic().await
            .expect("Failed to start server on dynamic port");
        
        let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
        
        // Wait for server to be ready
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Test that server is responding on /health endpoint
        let test_client = reqwest::Client::new();
        for _ in 0..10 {
            match test_client.get(&format!("http://127.0.0.1:{}/health", port)).send().await {
                Ok(response) => {
                    if response.status() == reqwest::StatusCode::OK {
                        // Server is ready!
                        break;
                    }
                },
                Err(_) => {
                    // Server not ready yet
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    continue;
                }
            }
        }
        
        TestServer { addr, handle }
    }
    
    /// Get the HTTP URL for the server
    pub fn url(&self, path: &str) -> String {
        if path == "/" {
            format!("http://{}/mcp", self.addr)
        } else {
            format!("http://{}{}", self.addr, path)
        }
    }
    
    /// Get the WebSocket URL for the server
    pub fn ws_url(&self) -> String {
        format!("ws://{}/", self.addr)
    }
    
    /// Stop the test server
    pub fn stop(self) {
        self.handle.abort();
    }
}

/// An HTTP client that maintains session cookies for testing
pub struct McpHttpClient {
    client: reqwest::Client,
    pub session_cookie: Option<String>,
}

impl McpHttpClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            session_cookie: None,
        }
    }
    
    /// Make an MCP call over HTTP
    pub async fn call(&mut self, url: &str, request: Value) -> Result<Value, String> {
        let mut req = self.client
            .post(url)
            .header("Content-Type", "application/json")
            .json(&request);
        
        if let Some(cookie) = &self.session_cookie {
            req = req.header("Cookie", cookie);
        }
        
        let response = req.send().await.map_err(|e| e.to_string())?;
        
        // Save session cookie if present
        if let Some(set_cookie) = response.headers().get("set-cookie") {
            if let Ok(cookie_str) = set_cookie.to_str() {
                self.session_cookie = Some(cookie_str.to_string());
            }
        }
        
        // Get the response text first for debugging
        let status = response.status();
        let headers = response.headers().clone();
        let text = response.text().await.map_err(|e| format!("Failed to get response text: {}", e))?;
        
        // Try to parse as JSON
        serde_json::from_str(&text).map_err(|e| {
            format!("Failed to parse JSON (status: {}, body: '{}'): {}", status, text, e)
        })
    }
    
    /// Initialize the MCP session
    pub async fn initialize(&mut self, url: &str, client_name: &str) -> Result<Value, String> {
        self.call(url, json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {
                    "name": client_name,
                    "version": "1.0.0"
                }
            }
        })).await
    }
    
    /// List available tools
    pub async fn list_tools(&mut self, url: &str) -> Result<Value, String> {
        self.call(url, json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list"
        })).await
    }
    
    /// Call a tool
    pub async fn call_tool(&mut self, url: &str, tool_name: &str, arguments: Value) -> Result<Value, String> {
        self.call(url, json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": tool_name,
                "arguments": arguments
            }
        })).await
    }
    
    /// Get the session cookie if present
    pub fn session_cookie(&self) -> Option<&str> {
        self.session_cookie.as_deref()
    }
}

/// Helper to create a JSON-RPC request
pub fn json_rpc_request(id: u32, method: &str, params: Option<Value>) -> Value {
    let mut request = json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method
    });
    
    if let Some(params) = params {
        request["params"] = params;
    }
    
    request
}

/// Helper to validate a JSON-RPC response
pub fn assert_json_rpc_success(response: &Value, expected_id: u32) {
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], expected_id);
    assert!(response["result"].is_object() || response["result"].is_array());
    assert!(response.get("error").is_none());
}

/// Helper to validate a JSON-RPC error response
pub fn assert_json_rpc_error(response: &Value, expected_id: u32, expected_code: i32) {
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], expected_id);
    assert!(response.get("result").is_none());
    assert_eq!(response["error"]["code"], expected_code);
    assert!(response["error"]["message"].is_string());
}