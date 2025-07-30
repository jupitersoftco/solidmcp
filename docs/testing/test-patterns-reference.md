# SolidMCP Test Patterns Reference

## Overview

This reference documents reusable test patterns established during TDD implementation of SolidMCP's test suite. These patterns ensure consistency and reduce boilerplate across tests.

## Core Test Helpers

### Server Setup Pattern

```rust
use mcp_test_helpers::TestServerBuilder;

// Basic server
let (server, port) = TestServerBuilder::new()
    .build()
    .await;

// Server with tools
let (server, port) = TestServerBuilder::new()
    .with_tool("echo", "Echo tool", |msg: String| async move {
        Ok(json!({ "echoed": msg }))
    })
    .with_tool("math", "Math tool", |args: MathArgs| async move {
        Ok(json!({ "result": args.a + args.b }))
    })
    .build()
    .await;

// Server with context
struct AppContext {
    database: Database,
}

let (server, port) = TestServerBuilder::new()
    .with_context(AppContext { database })
    .build()
    .await;
```

### HTTP Client Setup

```rust
let client = reqwest::Client::builder()
    .timeout(Duration::from_secs(5))
    .build()
    .unwrap();
```

### Session Management Patterns

```rust
// Create new session
async fn create_session(client: &Client, port: u16) -> String {
    let response = client
        .post(&format!("http://localhost:{}/", port))
        .json(&json!({
            "jsonrpc": "2.0",
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {
                    "name": "test-client",
                    "version": "1.0.0"
                }
            },
            "id": 1
        }))
        .send()
        .await
        .unwrap();
    
    extract_session_cookie(&response)
}

// Extract cookie from response
fn extract_session_cookie(response: &Response) -> String {
    response
        .headers()
        .get("set-cookie")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string()
}

// Initialize existing session
async fn initialize_session(
    client: &Client,
    port: u16,
    session_cookie: &str,
    client_name: &str
) -> Value {
    let response = client
        .post(&format!("http://localhost:{}/", port))
        .header("Cookie", session_cookie)
        .json(&init_request(client_name))
        .send()
        .await
        .unwrap();
    
    response.json().await.unwrap()
}
```

## Common Request Patterns

### Initialize Request

```rust
fn init_request(client_name: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {
                "name": client_name,
                "version": "1.0.0"
            }
        },
        "id": 1
    })
}
```

### Tool Call Request

```rust
fn tool_call_request(tool_name: &str, arguments: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": tool_name,
            "arguments": arguments
        },
        "id": 2
    })
}

// Helper function
async fn call_tool(
    client: &Client,
    port: u16,
    session_cookie: &str,
    tool_name: &str,
    arguments: Value
) -> Value {
    let response = client
        .post(&format!("http://localhost:{}/", port))
        .header("Cookie", session_cookie)
        .json(&tool_call_request(tool_name, arguments))
        .send()
        .await
        .unwrap();
    
    response.json().await.unwrap()
}
```

### List Tools Request

```rust
fn list_tools_request() -> Value {
    json!({
        "jsonrpc": "2.0",
        "method": "tools/list",
        "params": {},
        "id": 3
    })
}
```

## Error Validation Patterns

### JSON-RPC Error Validation

```rust
fn assert_json_rpc_error(response: &Value, expected_code: i32) {
    assert!(response["error"].is_object());
    let code = response["error"]["code"].as_i64().unwrap();
    assert_eq!(code, expected_code as i64);
    assert!(response["error"]["message"].is_string());
}

// Common error codes
const PARSE_ERROR: i32 = -32700;
const INVALID_REQUEST: i32 = -32600;
const METHOD_NOT_FOUND: i32 = -32601;
const INVALID_PARAMS: i32 = -32602;
const INTERNAL_ERROR: i32 = -32603;
```

### HTTP Error Validation

```rust
async fn assert_http_error(response: Response, expected_status: u16) {
    assert_eq!(response.status(), expected_status);
    
    // For JSON-RPC over HTTP, errors often return 200 with error in body
    if response.status() == 200 {
        let body: Value = response.json().await.unwrap();
        assert!(body["error"].is_object());
    }
}
```

## Concurrent Testing Patterns

### Parallel Operations

```rust
use futures::future::join_all;

#[tokio::test]
async fn test_concurrent_operations() {
    let (server, port) = TestServerBuilder::new().build().await;
    let client = Client::new();
    
    // Spawn concurrent tasks
    let handles: Vec<_> = (0..10)
        .map(|i| {
            let client = client.clone();
            tokio::spawn(async move {
                let session = create_session(&client, port).await;
                initialize_session(&client, port, &session, &format!("client-{}", i)).await
            })
        })
        .collect();
    
    // Wait for all to complete
    let results = join_all(handles).await;
    
    // Verify all succeeded
    for result in results {
        assert!(result.is_ok());
    }
}
```

### Race Condition Testing

```rust
use std::sync::Arc;
use tokio::sync::Barrier;

#[tokio::test]
async fn test_race_conditions() {
    let (server, port) = TestServerBuilder::new().build().await;
    let barrier = Arc::new(Barrier::new(5));
    
    let handles: Vec<_> = (0..5)
        .map(|_| {
            let barrier = barrier.clone();
            tokio::spawn(async move {
                // Synchronize start
                barrier.wait().await;
                
                // Perform operation
                // ...
            })
        })
        .collect();
    
    join_all(handles).await;
}
```

## Edge Case Testing Patterns

### Malformed Input Testing

```rust
#[tokio::test]
async fn test_malformed_inputs() {
    let test_cases = vec![
        ("empty body", ""),
        ("invalid json", "{not valid json}"),
        ("json array", "[1, 2, 3]"),
        ("json string", "\"just a string\""),
        ("json number", "42"),
        ("json null", "null"),
        ("truncated", "{\"jsonrpc\": \"2.0\", \"meth"),
    ];
    
    for (name, body) in test_cases {
        let response = client
            .post(&url)
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await
            .unwrap();
        
        // Should not panic, should return error
        assert!(
            response.status() == 400 || 
            response.json::<Value>().await.unwrap()["error"].is_object(),
            "Failed for case: {}", name
        );
    }
}
```

### Large Payload Testing

```rust
fn generate_large_payload(size_mb: usize) -> String {
    let size_bytes = size_mb * 1024 * 1024;
    let padding = "x".repeat(size_bytes);
    
    json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": "test",
            "arguments": { "data": padding }
        },
        "id": 1
    }).to_string()
}

#[tokio::test]
async fn test_oversized_payload() {
    let large_payload = generate_large_payload(3); // 3MB
    
    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .body(large_payload)
        .send()
        .await;
    
    // Should handle gracefully, not panic
    assert!(response.is_ok() || response.is_err());
}
```

## Transport Detection Patterns

### Header Variation Testing

```rust
#[tokio::test]
async fn test_header_variations() {
    let header_cases = vec![
        // (Accept header, Upgrade header, expected transport)
        ("application/json", None, "http"),
        ("*/*", Some("websocket"), "websocket"),
        ("application/json, text/html", None, "http"),
        ("text/html, application/json", None, "http"),
        ("", None, "http"), // Empty accept
    ];
    
    for (accept, upgrade, expected) in header_cases {
        let mut req = client.post(&url);
        
        if !accept.is_empty() {
            req = req.header("Accept", accept);
        }
        
        if let Some(upgrade_val) = upgrade {
            req = req.header("Upgrade", upgrade_val)
                     .header("Connection", "upgrade");
        }
        
        let response = req.json(&init_request("test")).send().await.unwrap();
        
        // Verify correct transport was selected
        verify_transport_response(&response, expected);
    }
}
```

## Test Data Builders

### Request Builder Pattern

```rust
struct RequestBuilder {
    method: String,
    params: Value,
    id: Option<u64>,
}

impl RequestBuilder {
    fn new(method: &str) -> Self {
        Self {
            method: method.to_string(),
            params: json!({}),
            id: Some(1),
        }
    }
    
    fn params(mut self, params: Value) -> Self {
        self.params = params;
        self
    }
    
    fn notification(mut self) -> Self {
        self.id = None;
        self
    }
    
    fn build(self) -> Value {
        let mut req = json!({
            "jsonrpc": "2.0",
            "method": self.method,
            "params": self.params,
        });
        
        if let Some(id) = self.id {
            req["id"] = json!(id);
        }
        
        req
    }
}

// Usage
let request = RequestBuilder::new("tools/call")
    .params(json!({ "name": "echo", "arguments": { "msg": "test" } }))
    .build();
```

## Assertion Helpers

### Response Validation

```rust
trait ResponseExt {
    fn assert_success(&self);
    fn assert_error(&self, code: i32);
    fn get_result(&self) -> &Value;
}

impl ResponseExt for Value {
    fn assert_success(&self) {
        assert!(self["result"].is_object() || self["result"].is_array());
        assert!(self["error"].is_null());
    }
    
    fn assert_error(&self, code: i32) {
        assert!(self["error"].is_object());
        assert_eq!(self["error"]["code"], code);
    }
    
    fn get_result(&self) -> &Value {
        self.assert_success();
        &self["result"]
    }
}
```

## Performance Testing Patterns

### Timing Assertions

```rust
use std::time::Instant;

#[tokio::test]
async fn test_performance() {
    let start = Instant::now();
    
    // Perform operation
    let response = call_tool(&client, port, &session, "compute", json!({})).await;
    
    let duration = start.elapsed();
    
    // Assert completes within time limit
    assert!(
        duration.as_millis() < 100,
        "Operation took {}ms, expected <100ms",
        duration.as_millis()
    );
}
```

### Throughput Testing

```rust
#[tokio::test]
async fn test_throughput() {
    let start = Instant::now();
    let operation_count = 1000;
    
    for i in 0..operation_count {
        call_tool(&client, port, &session, "echo", 
            json!({ "msg": format!("test-{}", i) })).await;
    }
    
    let duration = start.elapsed();
    let ops_per_second = operation_count as f64 / duration.as_secs_f64();
    
    println!("Throughput: {:.2} ops/sec", ops_per_second);
    assert!(ops_per_second > 100.0); // Minimum expected throughput
}
```

## Test Organization Best Practices

### Module Structure

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    mod session_tests {
        use super::*;
        
        #[tokio::test]
        async fn test_session_creation() { /* ... */ }
        
        #[tokio::test]
        async fn test_session_persistence() { /* ... */ }
    }
    
    mod error_handling_tests {
        use super::*;
        
        #[tokio::test]
        async fn test_malformed_request() { /* ... */ }
    }
}
```

### Test Naming Conventions

- `test_<feature>_<scenario>_<expected_outcome>`
- Examples:
  - `test_session_reinitialization_updates_client_info`
  - `test_invalid_json_returns_parse_error`
  - `test_concurrent_requests_maintain_isolation`

## Debugging Helpers

### Request/Response Logging

```rust
async fn debug_request(client: &Client, url: &str, body: Value) -> Value {
    println!("REQUEST: {}", serde_json::to_string_pretty(&body).unwrap());
    
    let response = client
        .post(url)
        .json(&body)
        .send()
        .await
        .unwrap();
    
    let status = response.status();
    let body: Value = response.json().await.unwrap();
    
    println!("RESPONSE ({}): {}", status, serde_json::to_string_pretty(&body).unwrap());
    
    body
}
```

### Test Environment Setup

```rust
fn init_test_logging() {
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .is_test(true)
        .try_init();
}

#[tokio::test]
async fn test_with_logging() {
    init_test_logging();
    // Test implementation
}
```

## Conclusion

These patterns provide a comprehensive foundation for writing consistent, maintainable tests for SolidMCP. They emphasize clarity, reusability, and proper error handling while following Rust best practices.