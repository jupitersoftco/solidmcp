//! HTTP Response Building
//!
//! Functions for building HTTP responses with proper headers and formatting.

use crate::error::McpError;
use serde_json::{json, Value};
use tracing::debug;
use warp::http::{HeaderMap, HeaderValue, StatusCode};
use warp::reply;
use warp::Reply;

/// Builder for HTTP responses
#[derive(Debug)]
pub struct ResponseBuilder {
    use_chunked: bool,
    session_id: Option<String>,
    headers: HeaderMap,
}

impl ResponseBuilder {
    /// Create a new response builder
    pub fn new() -> Self {
        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", HeaderValue::from_static("application/json"));
        
        Self {
            use_chunked: false,
            session_id: None,
            headers,
        }
    }
    
    /// Enable chunked encoding
    pub fn with_chunked_encoding(mut self, use_chunked: bool) -> Self {
        self.use_chunked = use_chunked;
        self
    }
    
    /// Add session cookie
    pub fn with_session(mut self, session_id: String) -> Self {
        self.session_id = Some(session_id);
        self
    }
    
    /// Add custom header
    pub fn with_header(mut self, name: &str, value: &str) -> Self {
        if let Ok(header_name) = warp::http::HeaderName::from_bytes(name.as_bytes()) {
            if let Ok(header_value) = HeaderValue::from_str(value) {
                self.headers.insert(header_name, header_value);
            }
        }
        self
    }
    
    /// Build a success response
    pub fn build_success(mut self, body: Value) -> warp::reply::Response {
        let has_session = self.session_id.is_some();
        
        // Add session cookie if present
        if let Some(session_id) = self.session_id {
            let cookie_value = super::session::create_session_cookie(&session_id);
            if let Ok(header_value) = HeaderValue::from_str(&cookie_value) {
                self.headers.insert("Set-Cookie", header_value);
            }
        }
        
        // Add chunked encoding if requested
        if self.use_chunked {
            self.headers.insert("Transfer-Encoding", HeaderValue::from_static("chunked"));
        }
        
        // Serialize body
        let body_str = serde_json::to_string(&body).unwrap_or_else(|_| "{}".to_string());
        
        // Build response
        let mut response = reply::with_status(
            reply::json(&body),
            StatusCode::OK
        ).into_response();
        
        // Apply headers
        let headers_mut = response.headers_mut();
        for (key, value) in self.headers {
            if let Some(key) = key {
                headers_mut.insert(key, value);
            }
        }
        
        debug!(
            status = ?StatusCode::OK,
            chunked = self.use_chunked,
            has_session = has_session,
            body_size = body_str.len(),
            "Built success response"
        );
        
        response
    }
    
    /// Build an error response
    pub fn build_error(self, error: McpError, message_id: Option<Value>) -> warp::reply::Response {
        let error_response = error.to_json_rpc_error(message_id);
        
        debug!(
            status = ?StatusCode::OK,
            error = %error,
            "Built error response"
        );
        
        self.build_success(error_response)
    }
}

impl Default for ResponseBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a simple error response
pub fn create_error_response(
    error: McpError,
    message_id: Option<Value>,
) -> Value {
    error.to_json_rpc_error(message_id)
}

/// Create a chunked response with notifications
pub fn create_chunked_response(
    primary_response: Value,
    notifications: Vec<Value>,
) -> warp::reply::Response {
    // Combine primary response and notifications
    let mut responses = vec![primary_response];
    responses.extend(notifications);
    
    // Create multipart response
    let body = responses.into_iter()
        .map(|r| serde_json::to_string(&r).unwrap_or_default())
        .collect::<Vec<_>>()
        .join("\n");
    
    ResponseBuilder::new()
        .with_chunked_encoding(true)
        .build_success(json!({ "responses": body }))
}

/// Apply CORS headers to a response
pub fn apply_cors_headers(mut response: warp::reply::Response) -> warp::reply::Response {
    let headers = response.headers_mut();
    
    headers.insert(
        "Access-Control-Allow-Origin",
        HeaderValue::from_static("*")
    );
    headers.insert(
        "Access-Control-Allow-Methods",
        HeaderValue::from_static("GET, POST, OPTIONS")
    );
    headers.insert(
        "Access-Control-Allow-Headers",
        HeaderValue::from_static("Content-Type, Authorization")
    );
    headers.insert(
        "Access-Control-Max-Age",
        HeaderValue::from_static("86400")
    );
    
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_response_builder_basic() {
        let response = ResponseBuilder::new()
            .build_success(json!({"result": "ok"}));
        
        assert_eq!(response.status(), StatusCode::OK);
        assert!(response.headers().get("Content-Type").is_some());
    }
    
    #[test]
    fn test_response_builder_with_session() {
        let response = ResponseBuilder::new()
            .with_session("test123".to_string())
            .build_success(json!({"result": "ok"}));
        
        let cookie_header = response.headers().get("Set-Cookie");
        assert!(cookie_header.is_some());
        
        let cookie_str = cookie_header.unwrap().to_str().unwrap();
        assert!(cookie_str.contains("mcp_session=test123"));
    }
    
    #[test]
    fn test_response_builder_with_chunked() {
        let response = ResponseBuilder::new()
            .with_chunked_encoding(true)
            .build_success(json!({"result": "ok"}));
        
        let encoding_header = response.headers().get("Transfer-Encoding");
        assert!(encoding_header.is_some());
        assert_eq!(encoding_header.unwrap(), "chunked");
    }
    
    #[test]
    fn test_response_builder_error() {
        let error = McpError::InvalidParams("Test error".to_string());
        let response = ResponseBuilder::new()
            .build_error(error, Some(json!(1)));
        
        assert_eq!(response.status(), StatusCode::OK); // JSON-RPC errors use 200
    }
    
    #[test]
    fn test_create_error_response() {
        let error = McpError::InvalidParams("Invalid Request".to_string());
        let response = create_error_response(error, Some(json!(1)));
        
        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["id"], 1);
        assert_eq!(response["error"]["code"], -32602);
        assert!(response["error"]["message"].as_str().unwrap().contains("Invalid Request"));
    }
}