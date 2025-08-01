//! HTTP Request Validation
//!
//! Functions for validating HTTP requests and extracting request metadata.

use crate::error::{McpError, McpResult};
use serde_json::{json, Value};
use tracing::{trace, warn};

/// Validated request with extracted metadata
#[derive(Debug)]
pub struct ValidatedRequest {
    pub message: Value,
    pub method: String,
    pub message_id: Value,
    pub has_params: bool,
    pub has_meta: bool,
    pub has_progress_token: bool,
    pub request_size: usize,
}

/// Validate an HTTP request
pub fn validate_request(
    message: Value,
    content_type: &str,
) -> McpResult<ValidatedRequest> {
    // Validate content type
    if !content_type.contains("application/json") {
        return Err(McpError::InvalidParams(format!(
            "Invalid Content-Type: {}. Expected application/json",
            content_type
        )));
    }
    
    // Validate required JSON-RPC fields
    let jsonrpc = message
        .get("jsonrpc")
        .and_then(|j| j.as_str())
        .ok_or_else(|| McpError::InvalidParams("Missing or invalid 'jsonrpc' field".to_string()))?;
    
    if jsonrpc != "2.0" {
        return Err(McpError::InvalidParams(format!(
            "Invalid jsonrpc version: {}. Expected 2.0",
            jsonrpc
        )));
    }
    
    // Extract method
    let method = message
        .get("method")
        .and_then(|m| m.as_str())
        .ok_or_else(|| McpError::InvalidParams("Missing or invalid 'method' field".to_string()))?
        .to_string();
    
    // Extract message ID
    let message_id = message.get("id").cloned().unwrap_or(json!(null));
    
    // Extract metadata
    let has_params = message.get("params").is_some();
    let has_meta = message.get("params")
        .and_then(|p| p.get("_meta"))
        .is_some();
    let has_progress_token = message.get("params")
        .and_then(|p| p.get("_meta"))
        .and_then(|m| m.get("progressToken"))
        .is_some();
    
    // Calculate request size
    let request_size = serde_json::to_string(&message)
        .unwrap_or_default()
        .len();
    
    // Log analysis
    trace!(
        method = %method,
        message_id = ?message_id,
        has_params = has_params,
        has_meta = has_meta,
        has_progress_token = has_progress_token,
        request_size_bytes = request_size,
        request_size_kb = format!("{:.2}", request_size as f64 / 1024.0),
        "Message structure analysis"
    );
    
    if has_progress_token {
        let progress_token = message
            .get("params")
            .and_then(|p| p.get("_meta"))
            .and_then(|m| m.get("progressToken"));
        warn!(
            progress_token = ?progress_token,
            "Progress token detected - client expects streaming updates"
        );
    }
    
    if request_size > 10000 {
        warn!(
            request_size_bytes = request_size,
            threshold = 10000,
            "Large request detected - may cause processing issues"
        );
    }
    
    Ok(ValidatedRequest {
        message,
        method,
        message_id,
        has_params,
        has_meta,
        has_progress_token,
        request_size,
    })
}

/// Validate MCP message structure
pub fn validate_message_structure(message: &Value) -> McpResult<()> {
    use crate::validation::McpValidator;
    
    McpValidator::validate_message(message).map_err(|e| {
        McpError::InvalidParams(format!("Invalid request: {:?}", e))
    })
}

/// Extract content type from headers with fallback
pub fn extract_content_type(content_type: Option<String>) -> String {
    content_type.unwrap_or_else(|| "application/json".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_validate_request_valid() {
        let message = json!({
            "jsonrpc": "2.0",
            "method": "initialize",
            "id": 1,
            "params": {}
        });
        
        let result = validate_request(message, "application/json");
        assert!(result.is_ok());
        
        let validated = result.unwrap();
        assert_eq!(validated.method, "initialize");
        assert_eq!(validated.message_id, 1);
        assert!(validated.has_params);
        assert!(!validated.has_meta);
        assert!(!validated.has_progress_token);
    }
    
    #[test]
    fn test_validate_request_invalid_content_type() {
        let message = json!({
            "jsonrpc": "2.0",
            "method": "test"
        });
        
        let result = validate_request(message, "text/plain");
        assert!(matches!(result, Err(McpError::InvalidParams(_))));
    }
    
    #[test]
    fn test_validate_request_missing_jsonrpc() {
        let message = json!({
            "method": "test"
        });
        
        let result = validate_request(message, "application/json");
        assert!(matches!(result, Err(McpError::InvalidParams(_))));
    }
    
    #[test]
    fn test_validate_request_wrong_jsonrpc_version() {
        let message = json!({
            "jsonrpc": "1.0",
            "method": "test"
        });
        
        let result = validate_request(message, "application/json");
        assert!(matches!(result, Err(McpError::InvalidParams(_))));
        
        if let Err(McpError::InvalidParams(msg)) = result {
            assert!(msg.contains("Invalid jsonrpc version"));
        }
    }
    
    #[test]
    fn test_validate_request_missing_method() {
        let message = json!({
            "jsonrpc": "2.0",
            "id": 1
        });
        
        let result = validate_request(message, "application/json");
        assert!(matches!(result, Err(McpError::InvalidParams(_))));
    }
    
    #[test]
    fn test_validate_request_with_progress_token() {
        let message = json!({
            "jsonrpc": "2.0",
            "method": "tools/call",
            "id": 1,
            "params": {
                "_meta": {
                    "progressToken": "abc123"
                }
            }
        });
        
        let result = validate_request(message, "application/json");
        assert!(result.is_ok());
        
        let validated = result.unwrap();
        assert!(validated.has_progress_token);
        assert!(validated.has_meta);
    }
    
    #[test]
    fn test_extract_content_type() {
        assert_eq!(
            extract_content_type(Some("application/json; charset=utf-8".to_string())),
            "application/json; charset=utf-8"
        );
        
        assert_eq!(
            extract_content_type(None),
            "application/json"
        );
    }
}