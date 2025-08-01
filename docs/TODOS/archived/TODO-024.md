# TODO-024: Add Framework Layer Unit Tests

**Status**: ✅ COMPLETED (2025-08-01)  
**Priority**: 🔴 HIGH  
**Effort**: 8 hours  
**Dependencies**: TODO-023 (need clean architecture first)  
**Category**: Testing, Quality

## 📋 Description

The entire framework layer (`src/framework/*`) has ZERO tests. This is the high-level API users interact with most. Add comprehensive unit tests for the builder pattern and registry system.

## 🎯 Acceptance Criteria

- [x] 90%+ code coverage for framework modules ✅
- [x] Builder pattern thoroughly tested ✅
- [x] Registry operations tested ✅
- [x] Error cases covered ✅
- [x] Type safety verified ✅
- [x] Examples in tests serve as documentation ✅

## 📊 Current State

```rust
// ZERO TESTS for:
// src/framework/builder.rs
// src/framework/handler.rs  
// src/framework/registry.rs
// src/framework/providers.rs
// src/framework/notification.rs

// This is the PRIMARY user API!
```

## 🔧 Implementation

### 1. Test the Builder Pattern

Create `src/framework/builder/tests.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ToolResponse, json};
    
    #[tokio::test]
    async fn test_builder_basic() {
        let server = McpServerBuilder::new()
            .with_name("test-server")
            .with_version("1.0.0")
            .build();
        
        assert_eq!(server.name(), "test-server");
        assert_eq!(server.version(), "1.0.0");
    }
    
    #[tokio::test]
    async fn test_builder_with_tool() {
        let server = McpServerBuilder::new()
            .with_tool(
                "echo",
                "Echo input back",
                json!({
                    "type": "object",
                    "properties": {
                        "message": {"type": "string"}
                    },
                    "required": ["message"]
                }),
                |params| async move {
                    let msg = params["message"].as_str().unwrap();
                    ToolResponse::success(json!({"echo": msg}))
                }
            )
            .build();
        
        // Verify tool is registered
        let tools = server.list_tools().await.unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "echo");
        
        // Test tool execution
        let result = server.call_tool("echo", json!({"message": "hello"})).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_builder_with_typed_tool() {
        #[derive(serde::Deserialize, JsonSchema)]
        struct EchoParams {
            message: String,
        }
        
        #[derive(serde::Serialize)]
        struct EchoResponse {
            echo: String,
        }
        
        let server = McpServerBuilder::new()
            .with_typed_tool(
                "typed_echo",
                "Type-safe echo",
                |params: EchoParams| async move {
                    Ok(TypedResponse::new(EchoResponse {
                        echo: params.message,
                    }))
                }
            )
            .build();
        
        let result = server.call_tool("typed_echo", json!({"message": "test"})).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_builder_with_resource_provider() {
        let server = McpServerBuilder::new()
            .with_resource_provider(
                "file://",
                "File system resources",
                |uri| async move {
                    if uri == "file:///test.txt" {
                        Ok(json!({
                            "content": "test content",
                            "mime_type": "text/plain"
                        }))
                    } else {
                        Err(McpError::UnknownResource(uri.to_string()))
                    }
                }
            )
            .build();
        
        let resources = server.list_resources().await.unwrap();
        assert!(!resources.is_empty());
        
        let content = server.read_resource("file:///test.txt").await;
        assert!(content.is_ok());
    }
    
    #[tokio::test]
    async fn test_builder_with_context() {
        #[derive(Clone)]
        struct AppContext {
            db_url: String,
        }
        
        let context = AppContext {
            db_url: "postgres://localhost".into(),
        };
        
        let server = McpServerBuilder::with_context(context.clone())
            .with_tool(
                "get_db",
                "Get database URL",
                json!({}),
                |_params| async move {
                    // Access context here
                    ToolResponse::success(json!({
                        "db_url": context.db_url
                    }))
                }
            )
            .build();
        
        let result = server.call_tool("get_db", json!({})).await.unwrap();
        assert_eq!(result["db_url"], "postgres://localhost");
    }
}
```

### 2. Test the Registry System

Create `src/framework/registry/tests.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_tool_registry() {
        let mut registry = ToolRegistry::new();
        
        // Register a tool
        registry.register(
            "test_tool",
            "Test tool",
            json!({"type": "object"}),
            Box::new(|params| Box::pin(async move {
                ToolResponse::success(params)
            }))
        );
        
        // Verify registration
        assert!(registry.get("test_tool").is_some());
        assert!(registry.get("nonexistent").is_none());
        
        let tools = registry.list();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "test_tool");
    }
    
    #[test]
    fn test_registry_duplicate_names() {
        let mut registry = ToolRegistry::new();
        
        registry.register("tool", "First", json!({}), create_mock_handler());
        registry.register("tool", "Second", json!({}), create_mock_handler());
        
        // Should replace
        let tools = registry.list();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].description, "Second");
    }
    
    #[tokio::test]
    async fn test_registry_execution() {
        let mut registry = ToolRegistry::new();
        
        registry.register(
            "add",
            "Add two numbers",
            json!({
                "type": "object",
                "properties": {
                    "a": {"type": "number"},
                    "b": {"type": "number"}
                }
            }),
            Box::new(|params| Box::pin(async move {
                let a = params["a"].as_f64().unwrap();
                let b = params["b"].as_f64().unwrap();
                ToolResponse::success(json!({"sum": a + b}))
            }))
        );
        
        let handler = registry.get("add").unwrap();
        let result = handler(json!({"a": 5, "b": 3})).await;
        
        assert_eq!(result.to_value().unwrap()["sum"], 8.0);
    }
}
```

### 3. Test Error Handling

Create `src/framework/error_tests.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_unknown_tool_error() {
        let server = McpServerBuilder::new().build();
        
        let result = server.call_tool("nonexistent", json!({})).await;
        assert!(matches!(result, Err(McpError::UnknownTool(_))));
    }
    
    #[tokio::test] 
    async fn test_invalid_params_error() {
        let server = McpServerBuilder::new()
            .with_tool(
                "strict_tool",
                "Requires specific params",
                json!({
                    "type": "object",
                    "properties": {
                        "required_field": {"type": "string"}
                    },
                    "required": ["required_field"]
                }),
                |params| async move {
                    if params.get("required_field").is_none() {
                        return ToolResponse::error("Missing required_field");
                    }
                    ToolResponse::success(json!({"ok": true}))
                }
            )
            .build();
        
        let result = server.call_tool("strict_tool", json!({})).await;
        // Should fail validation or return error
        assert!(result.is_err() || result.unwrap().is_error);
    }
}
```

### 4. Test Notification System

Create `src/framework/notification/tests.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;
    
    #[tokio::test]
    async fn test_notification_sending() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let sender = NotificationSender::new(tx);
        
        // Send notification
        sender.send("test_event", json!({"data": "value"})).await;
        
        // Verify received
        let notification = rx.recv().await.unwrap();
        assert_eq!(notification["method"], "test_event");
        assert_eq!(notification["params"]["data"], "value");
    }
    
    #[tokio::test]
    async fn test_progress_notifications() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let sender = NotificationSender::new(tx);
        
        // Send progress
        sender.send_progress(50, "Half way there").await;
        
        let notification = rx.recv().await.unwrap();
        assert_eq!(notification["method"], "progress");
        assert_eq!(notification["params"]["progress"], 50);
    }
}
```

### 5. Integration Test

Create `tests/framework_integration_test.rs`:
```rust
use solidmcp::{McpServerBuilder, ToolResponse, json};

#[tokio::test]
async fn test_full_framework_flow() {
    // Build a complete server
    let server = McpServerBuilder::new()
        .with_name("integration-test")
        .with_version("1.0.0")
        .with_tool("greet", "Greet someone", json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            }
        }), |params| async move {
            let name = params["name"].as_str().unwrap_or("World");
            ToolResponse::success(json!({
                "greeting": format!("Hello, {}!", name)
            }))
        })
        .with_resource_provider("test://", "Test resources", |uri| async move {
            Ok(json!({"content": format!("Resource: {}", uri)}))
        })
        .build();
    
    // Start server
    let addr = "127.0.0.1:0"; // Random port
    let handle = tokio::spawn(async move {
        server.start(addr).await
    });
    
    // Would test with client here...
    
    handle.abort(); // Clean shutdown
}
```

## 🧪 Testing

Run all framework tests:
```bash
cargo test framework --lib
cargo test framework_integration
```

Coverage report:
```bash
cargo tarpaulin -p solidmcp --lib -- framework
```

## ✅ Verification

1. All framework modules have test files
2. Coverage > 90% for framework/
3. All edge cases tested
4. Examples in tests are clear
5. No flaky tests

## 📝 Notes

- Tests serve as living documentation
- Cover both success and error paths
- Test concurrency where applicable
- Keep tests focused and fast