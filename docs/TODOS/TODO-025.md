# TODO-025: Add Integration Tests for Protocol Flows

**Priority**: 🔴 HIGH  
**Effort**: 6 hours  
**Dependencies**: TODO-024 (framework tests first)  
**Category**: Testing, Quality

## 📋 Description

Add comprehensive integration tests that verify full protocol flows work correctly across both HTTP and WebSocket transports. Test the actual MCP protocol, not just individual functions.

## 🎯 Acceptance Criteria

- [ ] Full initialize → tools/list → tools/call flow tested
- [ ] Both HTTP and WebSocket transports tested
- [ ] Session persistence verified
- [ ] Error scenarios tested
- [ ] Concurrent client handling verified
- [ ] Progress notifications tested

## 📊 Current State

```rust
// Some transport tests exist but no full protocol flow tests
// No tests for session lifecycle
// No tests for concurrent clients
```

## 🔧 Implementation

### 1. Create Test Helpers

Create `tests/helpers/mod.rs`:
```rust
use solidmcp::{McpServerBuilder, McpServer};
use tokio::net::TcpListener;
use std::net::SocketAddr;

pub struct TestServer {
    pub addr: SocketAddr,
    pub shutdown_tx: tokio::sync::oneshot::Sender<()>,
}

impl TestServer {
    pub async fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        
        let server = McpServerBuilder::new()
            .with_tool("test_tool", "Test tool", json!({
                "type": "object",
                "properties": {
                    "input": {"type": "string"}
                }
            }), |params| async move {
                let input = params["input"].as_str().unwrap_or("none");
                ToolResponse::success(json!({
                    "output": format!("Processed: {}", input)
                }))
            })
            .build();
        
        tokio::spawn(async move {
            server.start_with_shutdown(listener, shutdown_rx).await
        });
        
        // Wait for server to be ready
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        TestServer { addr, shutdown_tx }
    }
    
    pub fn url(&self, path: &str) -> String {
        format!("http://{}{}", self.addr, path)
    }
    
    pub fn ws_url(&self) -> String {
        format!("ws://{}/mcp", self.addr)
    }
}

pub struct McpClient {
    client: reqwest::Client,
    session_cookie: Option<String>,
}

impl McpClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            session_cookie: None,
        }
    }
    
    pub async fn call(&mut self, url: &str, request: Value) -> Result<Value, String> {
        let mut req = self.client
            .post(url)
            .json(&request);
        
        if let Some(cookie) = &self.session_cookie {
            req = req.header("Cookie", cookie);
        }
        
        let response = req.send().await.map_err(|e| e.to_string())?;
        
        // Save session cookie
        if let Some(set_cookie) = response.headers().get("set-cookie") {
            self.session_cookie = Some(set_cookie.to_str().unwrap().to_string());
        }
        
        response.json().await.map_err(|e| e.to_string())
    }
}
```

### 2. Test Full Protocol Flow

Create `tests/protocol_flow_test.rs`:
```rust
use crate::helpers::{TestServer, McpClient};

#[tokio::test]
async fn test_full_protocol_flow_http() {
    let server = TestServer::start().await;
    let mut client = McpClient::new();
    
    // 1. Initialize
    let init_response = client.call(&server.url("/mcp"), json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
        }
    })).await.unwrap();
    
    assert_eq!(init_response["result"]["protocolVersion"], "2025-06-18");
    assert!(client.session_cookie.is_some());
    
    // 2. List tools
    let tools_response = client.call(&server.url("/mcp"), json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list"
    })).await.unwrap();
    
    let tools = tools_response["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0]["name"], "test_tool");
    
    // 3. Call tool
    let call_response = client.call(&server.url("/mcp"), json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "test_tool",
            "arguments": {
                "input": "hello"
            }
        }
    })).await.unwrap();
    
    assert_eq!(
        call_response["result"]["content"][0]["text"],
        json!({"output": "Processed: hello"}).to_string()
    );
}

#[tokio::test]
async fn test_full_protocol_flow_websocket() {
    use tokio_tungstenite::{connect_async, tungstenite::Message};
    
    let server = TestServer::start().await;
    let (mut ws_stream, _) = connect_async(server.ws_url()).await.unwrap();
    
    // 1. Initialize
    ws_stream.send(Message::Text(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {
                "name": "test-ws-client",
                "version": "1.0.0"
            }
        }
    }).to_string())).await.unwrap();
    
    let msg = ws_stream.next().await.unwrap().unwrap();
    let response: Value = serde_json::from_str(msg.to_text().unwrap()).unwrap();
    assert_eq!(response["result"]["protocolVersion"], "2025-06-18");
    
    // Continue with tools/list and tools/call...
}
```

### 3. Test Error Scenarios

Create `tests/protocol_errors_test.rs`:
```rust
#[tokio::test]
async fn test_not_initialized_error() {
    let server = TestServer::start().await;
    let mut client = McpClient::new();
    
    // Try to list tools without initializing
    let response = client.call(&server.url("/mcp"), json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list"
    })).await.unwrap();
    
    assert!(response.get("error").is_some());
    assert_eq!(response["error"]["code"], -32002);
    assert!(response["error"]["message"]
        .as_str()
        .unwrap()
        .contains("Not initialized"));
}

#[tokio::test]
async fn test_unknown_method_error() {
    let server = TestServer::start().await;
    let mut client = McpClient::new();
    
    let response = client.call(&server.url("/mcp"), json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "unknown/method"
    })).await.unwrap();
    
    assert_eq!(response["error"]["code"], -32601);
}

#[tokio::test]
async fn test_invalid_json_error() {
    let server = TestServer::start().await;
    
    let response = reqwest::Client::new()
        .post(server.url("/mcp"))
        .header("Content-Type", "application/json")
        .body("{invalid json")
        .send()
        .await
        .unwrap();
    
    let error: Value = response.json().await.unwrap();
    assert_eq!(error["error"]["code"], -32700); // Parse error
}
```

### 4. Test Session Management

Create `tests/session_management_test.rs`:
```rust
#[tokio::test]
async fn test_session_persistence() {
    let server = TestServer::start().await;
    let mut client1 = McpClient::new();
    
    // Initialize first client
    client1.call(&server.url("/mcp"), json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "clientInfo": {"name": "client1", "version": "1.0"}
        }
    })).await.unwrap();
    
    let session_cookie = client1.session_cookie.clone().unwrap();
    
    // Create second client with same session
    let mut client2 = McpClient::new();
    client2.session_cookie = Some(session_cookie);
    
    // Should be able to use without re-initializing
    let response = client2.call(&server.url("/mcp"), json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list"
    })).await.unwrap();
    
    assert!(response.get("result").is_some());
}

#[tokio::test]
async fn test_session_reinitialization() {
    let server = TestServer::start().await;
    let mut client = McpClient::new();
    
    // Initialize once
    client.call(&server.url("/mcp"), json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "clientInfo": {"name": "client", "version": "1.0"}
        }
    })).await.unwrap();
    
    // Initialize again (should work for reconnection)
    let response = client.call(&server.url("/mcp"), json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "initialize",
        "params": {
            "clientInfo": {"name": "client", "version": "2.0"}
        }
    })).await.unwrap();
    
    assert!(response.get("result").is_some());
}
```

### 5. Test Concurrent Clients

Create `tests/concurrent_clients_test.rs`:
```rust
#[tokio::test]
async fn test_concurrent_clients() {
    let server = TestServer::start().await;
    let num_clients = 50;
    
    let handles: Vec<_> = (0..num_clients)
        .map(|i| {
            let url = server.url("/mcp");
            tokio::spawn(async move {
                let mut client = McpClient::new();
                
                // Each client does full flow
                client.call(&url, json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "initialize",
                    "params": {
                        "clientInfo": {
                            "name": format!("client_{}", i),
                            "version": "1.0"
                        }
                    }
                })).await.unwrap();
                
                client.call(&url, json!({
                    "jsonrpc": "2.0",
                    "id": 2,
                    "method": "tools/call",
                    "params": {
                        "name": "test_tool",
                        "arguments": {"input": format!("client_{}", i)}
                    }
                })).await.unwrap()
            })
        })
        .collect();
    
    // All should complete successfully
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result["result"].is_object());
    }
}
```

## 🧪 Testing

```bash
# Run all integration tests
cargo test --test '*_test'

# Run specific test
cargo test protocol_flow_test

# Run with logging
RUST_LOG=debug cargo test
```

## ✅ Verification

1. All protocol flows work end-to-end
2. Both transports behave identically
3. Sessions persist correctly
4. Errors follow JSON-RPC spec
5. Concurrent clients don't interfere

## 📝 Notes

- Integration tests are slower but catch real issues
- Test actual protocol, not mocked internals
- Cover edge cases from real client usage
- Keep test server minimal but realistic