# TODO-021: Refactor Complex HTTP Handler Function

**Priority**: 🟢 MEDIUM  
**Effort**: 6 hours  
**Dependencies**: TODO-020 (need logging to track refactoring)  
**Category**: Code Quality, Maintainability

## 📋 Description

Break down the monstrous `handle_mcp_http` function (627 lines, cyclomatic complexity 45!) into smaller, testable functions. This is blocking maintainability and debugging.

## 🎯 Acceptance Criteria

- [ ] No function longer than 50 lines
- [ ] Cyclomatic complexity under 10 per function
- [ ] Each function has single responsibility
- [ ] All existing tests pass
- [ ] New unit tests for extracted functions
- [ ] Performance not degraded

## 📊 Current State

```rust
// THE MONSTER in src/http.rs:109
pub async fn handle_mcp_http(
    message: Value,
    headers: HttpHeaders,
    cookie: Option<String>,
    engine: Arc<McpProtocolEngine<AppContext>>,
) -> Result<Response<Body>, Box<dyn std::error::Error + Send + Sync>> {
    // 627 LINES OF NESTED MADNESS!
    // - Session extraction
    // - Cookie parsing
    // - Message validation
    // - Progress handling
    // - Response building
    // - Error handling
    // ALL IN ONE FUNCTION!
}
```

## 🔧 Implementation

### 1. Extract Session Management

Create functions for session handling:
```rust
#[derive(Debug)]
struct SessionContext {
    id: String,
    is_new: bool,
    from_cookie: bool,
}

fn extract_session_context(
    method: &str,
    cookie: &Option<String>,
) -> SessionContext {
    let from_cookie = cookie.is_some();
    let id = if method == "initialize" {
        generate_session_id()
    } else {
        cookie.as_ref()
            .and_then(|c| parse_session_cookie(c))
            .unwrap_or_else(|| "http_default_session".to_string())
    };
    
    SessionContext {
        id,
        is_new: method == "initialize",
        from_cookie,
    }
}

fn parse_session_cookie(cookie: &str) -> Option<String> {
    cookie.split(';')
        .find_map(|part| {
            let trimmed = part.trim();
            if trimmed.starts_with("mcp_session=") {
                Some(trimmed.trim_start_matches("mcp_session=").to_string())
            } else {
                None
            }
        })
}
```

### 2. Extract Request Validation

```rust
#[derive(Debug)]
struct ValidatedRequest {
    message: Value,
    method: String,
    has_progress_token: bool,
}

fn validate_request(
    message: Value,
    content_type: &str,
) -> McpResult<ValidatedRequest> {
    // Validate content type
    if !content_type.contains("application/json") {
        return Err(McpError::InvalidParams(
            "Content-Type must be application/json".into()
        ));
    }
    
    // Extract method
    let method = message.get("method")
        .and_then(|m| m.as_str())
        .ok_or_else(|| McpError::InvalidParams("Missing method".into()))?
        .to_string();
    
    // Check for progress token
    let has_progress_token = message.get("params")
        .and_then(|p| p.get("_progress"))
        .is_some();
    
    Ok(ValidatedRequest {
        message,
        method,
        has_progress_token,
    })
}
```

### 3. Extract Response Building

```rust
struct ResponseBuilder {
    use_chunked: bool,
    session_id: Option<String>,
}

impl ResponseBuilder {
    fn new(use_chunked: bool) -> Self {
        Self {
            use_chunked,
            session_id: None,
        }
    }
    
    fn with_session(mut self, session_id: String) -> Self {
        self.session_id = Some(session_id);
        self
    }
    
    fn build_success(self, body: Value) -> Response<Body> {
        let mut response = Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/json");
        
        if let Some(session_id) = self.session_id {
            response = response.header(
                "Set-Cookie",
                format!("mcp_session={}; Path=/mcp; HttpOnly; SameSite=Strict", session_id)
            );
        }
        
        if self.use_chunked {
            response = response.header("Transfer-Encoding", "chunked");
        }
        
        let body_str = serde_json::to_string(&body).unwrap();
        response.body(Body::from(body_str)).unwrap()
    }
    
    fn build_error(self, error: McpError) -> Response<Body> {
        Response::builder()
            .status(StatusCode::OK) // JSON-RPC errors use 200
            .header("Content-Type", "application/json")
            .body(Body::from(serde_json::to_string(&error.to_json_rpc_error(None)).unwrap()))
            .unwrap()
    }
}
```

### 4. Extract Progress Handling

```rust
struct ProgressHandler {
    sender: mpsc::UnboundedSender<Value>,
    receiver: mpsc::UnboundedReceiver<Value>,
}

impl ProgressHandler {
    fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        Self { sender, receiver }
    }
    
    async fn handle_with_timeout(
        mut self,
        timeout: Duration,
    ) -> Vec<Value> {
        let mut notifications = Vec::new();
        
        tokio::select! {
            _ = tokio::time::sleep(timeout) => {
                warn!("Progress handler timeout");
            }
            _ = async {
                while let Some(notification) = self.receiver.recv().await {
                    notifications.push(notification);
                }
            } => {}
        }
        
        notifications
    }
}
```

### 5. Refactored Main Handler

```rust
#[instrument(skip(engine))]
pub async fn handle_mcp_http(
    message: Value,
    headers: HttpHeaders,
    cookie: Option<String>,
    engine: Arc<McpProtocolEngine<AppContext>>,
) -> Result<Response<Body>, McpError> {
    // Validate request
    let content_type = extract_content_type(&headers)?;
    let validated = validate_request(message, &content_type)?;
    
    // Extract session context
    let session = extract_session_context(&validated.method, &cookie);
    debug!(session_id = %session.id, "Processing request");
    
    // Set up progress handling if needed
    let progress_sender = if validated.has_progress_token {
        Some(ProgressHandler::new())
    } else {
        None
    };
    
    // Process the message
    let result = engine.process_message(
        &session.id,
        validated.message,
        progress_sender.as_ref().map(|p| p.sender.clone()),
    ).await;
    
    // Build response
    let response_builder = ResponseBuilder::new(validated.has_progress_token);
    
    match result {
        Ok(response_value) => {
            let mut builder = response_builder;
            if session.is_new {
                builder = builder.with_session(session.id);
            }
            
            // Handle progress notifications if any
            if let Some(progress) = progress_sender {
                let notifications = progress.handle_with_timeout(Duration::from_secs(5)).await;
                if !notifications.is_empty() {
                    return Ok(build_chunked_response(response_value, notifications));
                }
            }
            
            Ok(builder.build_success(response_value))
        }
        Err(e) => {
            error!(error = %e, "Request failed");
            Ok(response_builder.build_error(e))
        }
    }
}

fn extract_content_type(headers: &HttpHeaders) -> McpResult<String> {
    headers.get("content-type")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
        .ok_or_else(|| McpError::InvalidParams("Missing Content-Type".into()))
}
```

## 🧪 Testing

Create unit tests for each extracted function:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_session_cookie() {
        assert_eq!(
            parse_session_cookie("mcp_session=abc123; Path=/"),
            Some("abc123".to_string())
        );
        
        assert_eq!(
            parse_session_cookie("other=value; mcp_session=xyz"),
            Some("xyz".to_string())
        );
        
        assert_eq!(parse_session_cookie("no_session=here"), None);
    }
    
    #[test]
    fn test_validate_request() {
        let valid = json!({
            "jsonrpc": "2.0",
            "method": "test",
            "id": 1
        });
        
        let result = validate_request(valid, "application/json");
        assert!(result.is_ok());
        
        let invalid = json!({
            "jsonrpc": "2.0",
            "id": 1
        });
        
        let result = validate_request(invalid, "application/json");
        assert!(matches!(result, Err(McpError::InvalidParams(_))));
    }
    
    #[test]
    fn test_response_builder() {
        let response = ResponseBuilder::new(false)
            .with_session("test123".into())
            .build_success(json!({"result": "ok"}));
        
        assert_eq!(response.status(), StatusCode::OK);
        assert!(response.headers().get("Set-Cookie").is_some());
    }
}
```

## ✅ Verification

1. Run all existing HTTP tests
2. Measure function complexity with `cargo clippy`
3. Benchmark before/after performance
4. Code coverage for new functions
5. Integration tests still pass

## 📝 Notes

- Keep functions focused on single responsibility
- Use builder pattern for complex response construction
- Consider extracting more helpers as needed
- Document each function's purpose clearly