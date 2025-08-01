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

// For compatibility with existing code that uses anyhow::Error
impl From<anyhow::Error> for McpError {
    fn from(err: anyhow::Error) -> Self {
        McpError::Internal(err.to_string())
    }
}