# TODO-019: Implement Structured Error Types

**Priority**: 🟡 HIGH  
**Effort**: 3 hours  
**Dependencies**: None  
**Category**: Code Quality, Maintainability

## 📋 Description

Replace ALL `anyhow::Error` usage with structured error types using `thiserror`. Create a hierarchy where each submodule has its own error types that compose into the main `McpError`. This enables better error handling, clearer error messages, and proper error codes for clients.

## 🎯 Acceptance Criteria

- [ ] McpError enum covers all error cases
- [ ] JSON-RPC error codes properly mapped
- [ ] Error context preserved
- [ ] All `unwrap()` calls replaced with proper errors
- [ ] Client receives meaningful error messages

## 📊 Current State

```rust
// EVERYWHERE in the codebase:
Result<Value, Box<dyn std::error::Error + Send + Sync>>
anyhow::anyhow!("Something went wrong")
unwrap_or_else(|_| "default".to_string())
```

## 🔧 Implementation

### 1. Create Root Error Type

Create `src/error.rs`:
```rust
use thiserror::Error;
use serde_json::Value;

#[derive(Debug, Error)]
pub enum McpError {
    // Protocol Errors
    #[error("Method not found: {0}")]
    UnknownMethod(String),
    
    #[error("Invalid parameters: {0}")]
    InvalidParams(String),
    
    #[error("Not initialized")]
    NotInitialized,
    
    #[error("Already initialized")]
    AlreadyInitialized,
    
    // Resource Errors
    #[error("Tool not found: {0}")]
    UnknownTool(String),
    
    #[error("Resource not found: {0}")]
    UnknownResource(String),
    
    #[error("Prompt not found: {0}")]
    UnknownPrompt(String),
    
    // Security Errors
    #[error("Invalid path: {0}")]
    InvalidPath(String),
    
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    
    // Limit Errors
    #[error("Too many sessions (max: {0})")]
    TooManySessions(usize),
    
    #[error("Message too large: {0} bytes (max: {1})")]
    MessageTooLarge(usize, usize),
    
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    
    // IO Errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    // JSON Errors
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    
    // Internal Errors
    #[error("Internal error: {0}")]
    Internal(String),
}

impl McpError {
    /// Convert to JSON-RPC error code
    pub fn error_code(&self) -> i32 {
        match self {
            Self::UnknownMethod(_) => -32601,
            Self::InvalidParams(_) => -32602,
            Self::Json(_) => -32700,
            Self::NotInitialized => -32002,
            Self::UnknownTool(_) | Self::UnknownResource(_) | Self::UnknownPrompt(_) => -32601,
            Self::TooManySessions(_) | Self::MessageTooLarge(_, _) | Self::RateLimitExceeded => -32000,
            Self::InvalidPath(_) | Self::PermissionDenied(_) => -32003,
            _ => -32603, // Internal error
        }
    }
    
    /// Create JSON-RPC error response
    pub fn to_json_rpc_error(&self, id: Option<Value>) -> Value {
        serde_json::json!({
            "jsonrpc": "2.0",
            "error": {
                "code": self.error_code(),
                "message": self.to_string(),
            },
            "id": id,
        })
    }
}

// Result type alias for convenience
pub type McpResult<T> = Result<T, McpError>;

// Module-specific errors that compose into McpError
#[derive(Debug, Error)]
pub enum TransportError {
    #[error("WebSocket error: {0}")]
    WebSocket(String),
    
    #[error("HTTP error: {0}")]
    Http(String),
    
    #[error("Connection closed")]
    ConnectionClosed,
}

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("Invalid protocol version: {0}")]
    InvalidVersion(String),
    
    #[error("Message too large: {0} bytes")]
    MessageTooLarge(usize),
    
    #[error("Invalid message format")]
    InvalidFormat,
}

// Convert module errors to main error
impl From<TransportError> for McpError {
    fn from(err: TransportError) -> Self {
        McpError::Internal(err.to_string())
    }
}

impl From<ProtocolError> for McpError {
    fn from(err: ProtocolError) -> Self {
        match err {
            ProtocolError::MessageTooLarge(size) => McpError::MessageTooLarge(size, 2 * 1024 * 1024),
            _ => McpError::Internal(err.to_string()),
        }
    }
}
```

### 2. Update Function Signatures

Update `src/handler.rs`:
```rust
use crate::error::{McpError, McpResult};

#[async_trait]
pub trait McpHandler<C>: Send + Sync {
    async fn initialize(
        &self,
        params: Value,
        context: Arc<C>,
    ) -> McpResult<InitializeResult> {
        Err(McpError::UnknownMethod("initialize".into()))
    }
    
    async fn list_tools(&self, context: Arc<C>) -> McpResult<Vec<ToolDefinition>> {
        Ok(vec![])
    }
    
    async fn call_tool(
        &self,
        tool_name: &str,
        params: Value,
        context: Arc<C>,
    ) -> McpResult<Value> {
        Err(McpError::UnknownTool(tool_name.to_string()))
    }
}
```

### 3. Replace Error Creation

Update throughout codebase:
```rust
// BEFORE:
let method = message["method"].as_str().unwrap_or("");

// AFTER:
let method = message["method"]
    .as_str()
    .ok_or_else(|| McpError::InvalidParams("Missing method field".into()))?;

// BEFORE:
anyhow::anyhow!("Tool not found: {}", tool_name)

// AFTER:
McpError::UnknownTool(tool_name.to_string())

// BEFORE:
.map_err(|e| format!("Session lock error: {}", e))?

// AFTER:
.map_err(|e| McpError::Internal(format!("Session lock poisoned: {}", e)))?
```

### 4. Update Error Handling in Protocol

In `src/protocol_impl.rs`:
```rust
impl<C> McpProtocolHandlerImpl<C> {
    pub async fn handle_message(
        &self,
        message: Value,
        progress_sender: Option<mpsc::UnboundedSender<Value>>,
    ) -> McpResult<Value> {
        let request = serde_json::from_value::<JsonRpcRequest>(message)?;
        
        match request.method.as_str() {
            "initialize" => {
                if self.initialized.load(Ordering::Relaxed) {
                    return Err(McpError::AlreadyInitialized);
                }
                self.handle_initialize(request.params).await
            }
            "tools/list" => {
                if !self.initialized.load(Ordering::Relaxed) {
                    return Err(McpError::NotInitialized);
                }
                self.handle_list_tools().await
            }
            method => Err(McpError::UnknownMethod(method.to_string())),
        }
    }
}
```

## 🧪 Testing

Create `tests/error_handling_test.rs`:
```rust
#[test]
fn test_error_codes() {
    assert_eq!(McpError::UnknownMethod("test".into()).error_code(), -32601);
    assert_eq!(McpError::InvalidParams("test".into()).error_code(), -32602);
    assert_eq!(McpError::NotInitialized.error_code(), -32002);
}

#[test]
fn test_json_rpc_error_format() {
    let error = McpError::UnknownTool("my_tool".into());
    let json_error = error.to_json_rpc_error(Some(json!(1)));
    
    assert_eq!(json_error["jsonrpc"], "2.0");
    assert_eq!(json_error["error"]["code"], -32601);
    assert_eq!(json_error["error"]["message"], "Tool not found: my_tool");
    assert_eq!(json_error["id"], 1);
}

#[tokio::test]
async fn test_error_propagation() {
    let engine = create_test_engine();
    
    let result = engine.process_message(
        "test",
        json!({
            "jsonrpc": "2.0",
            "method": "unknown_method",
            "id": 1
        }),
        None
    ).await;
    
    assert!(matches!(result, Err(McpError::UnknownMethod(_))));
}
```

## ✅ Verification

1. All functions return `McpResult<T>` instead of boxed errors
2. Error messages are clear and actionable
3. JSON-RPC error codes match spec
4. No more `unwrap()` in production code
5. Error context preserved through call stack

## 📝 Notes

- Consider adding error source chain for debugging
- May want to add error telemetry later
- Keep error messages user-friendly
- Document error codes in API docs