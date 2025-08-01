# TODO-020: Add Structured Logging with Tracing

**Priority**: 🟡 HIGH  
**Effort**: 3 hours  
**Dependencies**: TODO-019 (need proper error types first)  
**Category**: Observability, Debugging  
**Status**: ✅ COMPLETED (2025-08-01)

## 📋 Description

Replace all `println!`, `eprintln!`, and debug logging with the `tracing` crate for structured, filterable logging that works in production.

## 🎯 Acceptance Criteria

- [x] All println! statements replaced
- [x] Structured logging with context
- [x] Log levels properly used (error, warn, info, debug, trace)
- [x] Request IDs in log context
- [x] Performance impact minimal
- [x] JSON output format available

## 📊 Current State

```rust
// EVERYWHERE in http.rs and other files:
println!("[HTTP] Received message: {:?}", message);
eprintln!("Error: {}", e);
// No way to filter, no context, no structure
```

## 🔧 Implementation

### 1. Add Tracing Dependencies

In `Cargo.toml`:
```rust
[dependencies]
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }

[dev-dependencies]
tracing-test = "0.2"
```

### 2. Initialize Tracing in Main

Update `src/main.rs` or server initialization:
```rust
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn init_tracing() {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "solidmcp=info,warp=info".into());
    
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true);
    
    // JSON format for production
    let json_layer = if std::env::var("LOG_FORMAT").as_deref() == Ok("json") {
        Some(tracing_subscriber::fmt::layer().json())
    } else {
        None
    };
    
    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .with(json_layer)
        .init();
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();
    
    tracing::info!("Starting SolidMCP server");
    // ... rest of main
}
```

### 3. Add Request ID Generation

Create `src/request_id.rs`:
```rust
use std::sync::atomic::{AtomicU64, Ordering};

static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(1);

pub fn generate_request_id() -> String {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    
    let counter = REQUEST_COUNTER.fetch_add(1, Ordering::Relaxed);
    
    format!("{:x}-{:04x}", timestamp, counter % 0x10000)
}
```

### 4. Replace Logging Throughout Codebase

Update `src/http.rs`:
```rust
use tracing::{info, debug, warn, error, instrument, Span};

#[instrument(
    skip(message, engine),
    fields(
        request_id = %generate_request_id(),
        method = tracing::field::Empty,
        session_id = tracing::field::Empty,
    )
)]
pub async fn handle_mcp_http(
    message: Value,
    headers: HttpHeaders,
    cookie: Option<String>,
    engine: Arc<McpProtocolEngine<AppContext>>,
) -> Result<Response<Body>, McpError> {
    let span = Span::current();
    
    // Extract and record method
    if let Some(method) = message.get("method").and_then(|m| m.as_str()) {
        span.record("method", &method);
    }
    
    // Extract session ID
    let session_id = extract_session_id(&cookie);
    span.record("session_id", &session_id);
    
    debug!("Processing HTTP request");
    
    match engine.process_message(&session_id, message, None).await {
        Ok(response) => {
            info!("Request completed successfully");
            Ok(create_response(response))
        }
        Err(e) => {
            error!(error = %e, "Request failed");
            Ok(create_error_response(e))
        }
    }
}
```

Update `src/shared.rs`:
```rust
use tracing::{debug, trace, instrument};

impl<C: Send + Sync + 'static> McpProtocolEngine<C> {
    #[instrument(
        skip(self, message, progress_sender),
        fields(session_id = %session_id)
    )]
    pub async fn process_message(
        &self,
        session_id: &str,
        message: Value,
        progress_sender: Option<mpsc::UnboundedSender<Value>>,
    ) -> McpResult<Value> {
        trace!(?message, "Processing message");
        
        // Check message size
        let message_size = serde_json::to_vec(&message)?.len();
        if message_size > self.limits.max_message_size {
            warn!(
                message_size = message_size,
                limit = self.limits.max_message_size,
                "Message too large"
            );
            return Err(McpError::MessageTooLarge(
                message_size,
                self.limits.max_message_size
            ));
        }
        
        debug!("Getting session handler");
        let handler = self.sessions
            .entry(session_id.to_string())
            .or_insert_with(|| {
                debug!("Creating new session");
                McpProtocolHandlerImpl::new(
                    Arc::clone(&self.context),
                    self.custom_handler.clone(),
                )
            })
            .clone();
        
        handler.handle_message(message, progress_sender).await
    }
}
```

### 5. Add Logging Macros for Common Patterns

Create `src/logging_utils.rs`:
```rust
#[macro_export]
macro_rules! log_error_with_context {
    ($err:expr, $msg:expr) => {
        tracing::error!(
            error = %$err,
            error_type = std::any::type_name_of_val(&$err),
            $msg
        )
    };
}

#[macro_export]
macro_rules! log_tool_execution {
    ($tool_name:expr, $duration:expr, $result:expr) => {
        match $result {
            Ok(_) => tracing::info!(
                tool = $tool_name,
                duration_ms = $duration.as_millis(),
                "Tool executed successfully"
            ),
            Err(ref e) => tracing::error!(
                tool = $tool_name,
                duration_ms = $duration.as_millis(),
                error = %e,
                "Tool execution failed"
            ),
        }
    };
}
```

## 🧪 Testing

Create `tests/logging_test.rs`:
```rust
use tracing_test::traced_test;

#[traced_test]
#[tokio::test]
async fn test_request_logging() {
    let engine = create_test_engine();
    
    let result = engine.process_message(
        "test-session",
        json!({"method": "initialize"}),
        None
    ).await;
    
    // Verify logs were generated
    assert!(logs_contain("Processing message"));
    assert!(logs_contain("test-session"));
}

#[traced_test]
#[test]
fn test_error_logging() {
    let error = McpError::UnknownMethod("test".into());
    log_error_with_context!(error, "Test error");
    
    assert!(logs_contain("Test error"));
    assert!(logs_contain("UnknownMethod"));
}
```

## ✅ Verification

1. Set `RUST_LOG=solidmcp=debug` and verify detailed logs
2. Set `LOG_FORMAT=json` and verify JSON output
3. Check that sensitive data isn't logged
4. Verify performance impact is minimal
5. Ensure all println! removed from codebase

## 📝 Notes

- Use appropriate log levels (error for failures, warn for issues, info for operations, debug for details)
- Don't log sensitive data (passwords, tokens, PII)
- Consider adding OpenTelemetry support later
- Keep structured fields consistent across modules

## ✅ Completion Summary (2025-08-01)

Successfully implemented structured logging with the tracing crate:

1. **Created comprehensive logging module** (`src/logging.rs`):
   - Proper tracing initialization with environment filters
   - Support for JSON format (via LOG_FORMAT=json)
   - Request ID generation for tracking
   - Connection ID tracking
   - Structured logging helper functions
   - Span creation for request lifecycle

2. **Replaced all println!/eprintln! statements**:
   - Updated `src/main.rs` to use structured logging
   - Updated `src/core.rs` to use log functions
   - Preserved debug output in tests for debugging

3. **Added structured fields throughout**:
   - HTTP handler logs with request_id, method, session_id
   - WebSocket handler logs with connection_id
   - Protocol handler logs with structured context
   - All log messages include relevant fields

4. **Implemented span tracking**:
   - Request spans in shared.rs with instrumentation
   - Connection spans in websocket.rs
   - Automatic span propagation through async calls

5. **Environment-based configuration**:
   - RUST_LOG environment variable support
   - JSON format via LOG_FORMAT=json
   - Default configuration: "solidmcp=info,warp=info"

6. **Backward compatibility**:
   - Deprecated McpDebugLogger wrapper maintained
   - Existing logging calls automatically use new system
   - No breaking changes to public API

All 137 library tests pass with the new logging implementation.