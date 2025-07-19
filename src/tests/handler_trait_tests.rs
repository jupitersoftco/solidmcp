//! Handler Trait Implementation Tests
//!
//! Tests for mocked handler implementations to verify trait compliance

#[cfg(test)]
mod tests {
    use crate::handler::{McpContext, McpHandler, McpResourceProvider, McpToolProvider, ToolDefinition};
    use crate::shared::McpProtocolEngine;
    use anyhow::Result;
    use async_trait::async_trait;
    use serde_json::{json, Value};
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    /// Mock tool provider for testing
    struct MockToolProvider {
        call_count: AtomicUsize,
        tools: Vec<ToolDefinition>,
        call_responses: HashMap<String, Value>,
    }

    impl MockToolProvider {
        fn new() -> Self {
            Self {
                call_count: AtomicUsize::new(0),
                tools: vec![
                    ToolDefinition {
                        name: "mock_tool".to_string(),
                        description: "A mock tool for testing".to_string(),
                        input_schema: json!({
                            "type": "object",
                            "properties": {
                                "message": {
                                    "type": "string",
                                    "description": "Message to process"
                                }
                            },
                            "required": ["message"]
                        }),
                    },
                    ToolDefinition {
                        name: "failing_tool".to_string(),
                        description: "A tool that always fails".to_string(),
                        input_schema: json!({
                            "type": "object",
                            "properties": {}
                        }),
                    },
                ],
                call_responses: HashMap::from([
                    ("mock_tool".to_string(), json!({"success": true, "processed": "message"})),
                    ("failing_tool".to_string(), json!({"error": "This tool always fails"})),
                ]),
            }
        }
    }

    #[async_trait]
    impl McpToolProvider for MockToolProvider {
        async fn list_tools(&self, _context: &McpContext) -> Result<Vec<ToolDefinition>> {
            Ok(self.tools.clone())
        }

        async fn call_tool(&self, name: &str, arguments: Value, _context: &McpContext) -> Result<Value> {
            self.call_count.fetch_add(1, Ordering::Relaxed);
            
            if name == "failing_tool" {
                return Err(anyhow::anyhow!("Tool execution failed"));
            }

            if let Some(response) = self.call_responses.get(name) {
                let mut result = response.clone();
                if let Some(obj) = result.as_object_mut() {
                    obj.insert("arguments".to_string(), arguments);
                }
                Ok(result)
            } else {
                Err(anyhow::anyhow!("Tool '{}' not found", name))
            }
        }
    }

    /// Mock resource provider for testing
    struct MockResourceProvider {
        resources: Vec<crate::handler::ResourceInfo>,
    }

    impl MockResourceProvider {
        fn new() -> Self {
            Self {
                resources: vec![
                    crate::handler::ResourceInfo {
                        uri: "mock://resource1".to_string(),
                        name: "resource1".to_string(),
                        description: Some("First mock resource".to_string()),
                        mime_type: Some("text/plain".to_string()),
                    },
                    crate::handler::ResourceInfo {
                        uri: "mock://resource2".to_string(),
                        name: "resource2".to_string(),
                        description: Some("Second mock resource".to_string()),
                        mime_type: Some("application/json".to_string()),
                    },
                ],
            }
        }
    }

    #[async_trait]
    impl McpResourceProvider for MockResourceProvider {
        async fn list_resources(&self) -> Result<Vec<crate::handler::ResourceInfo>> {
            Ok(self.resources.clone())
        }

        async fn read_resource(&self, uri: &str) -> Result<crate::handler::ResourceContent> {
            match uri {
                "mock://resource1" => Ok(crate::handler::ResourceContent {
                    uri: uri.to_string(),
                    mime_type: Some("text/plain".to_string()),
                    content: "Hello, this is resource 1 content".to_string(),
                }),
                "mock://resource2" => Ok(crate::handler::ResourceContent {
                    uri: uri.to_string(),
                    mime_type: Some("application/json".to_string()),
                    content: r#"{"message": "This is resource 2", "type": "json"}"#.to_string(),
                }),
                _ => Err(anyhow::anyhow!("Resource not found: {}", uri)),
            }
        }
    }

    /// Combined mock handler implementing all traits
    struct MockHandler {
        tool_provider: MockToolProvider,
        resource_provider: MockResourceProvider,
        initialized: Arc<tokio::sync::Mutex<bool>>,
    }

    impl MockHandler {
        fn new() -> Self {
            Self {
                tool_provider: MockToolProvider::new(),
                resource_provider: MockResourceProvider::new(),
                initialized: Arc::new(tokio::sync::Mutex::new(false)),
            }
        }
    }

    #[async_trait]
    impl McpHandler for MockHandler {
        async fn initialize(&self, _params: Value, _context: &McpContext) -> Result<Value> {
            let mut initialized = self.initialized.lock().await;
            if *initialized {
                return Err(anyhow::anyhow!("Already initialized"));
            }
            *initialized = true;

            Ok(json!({
                "protocolVersion": "2025-06-18",
                "capabilities": {
                    "tools": {},
                    "resources": {}
                },
                "serverInfo": {
                    "name": "mock-server",
                    "version": "1.0.0"
                }
            }))
        }

        async fn list_tools(&self, context: &McpContext) -> Result<Vec<ToolDefinition>> {
            self.tool_provider.list_tools(context).await
        }

        async fn call_tool(&self, name: &str, arguments: Value, context: &McpContext) -> Result<Value> {
            self.tool_provider.call_tool(name, arguments, context).await
        }
    }

    #[async_trait]
    impl McpResourceProvider for MockHandler {
        async fn list_resources(&self) -> Result<Vec<crate::handler::ResourceInfo>> {
            self.resource_provider.list_resources().await
        }

        async fn read_resource(&self, uri: &str) -> Result<crate::handler::ResourceContent> {
            self.resource_provider.read_resource(uri).await
        }
    }

    /// Test tool provider trait implementation
    #[tokio::test]
    async fn test_tool_provider_implementation() {
        let provider = MockToolProvider::new();
        let context = McpContext {
            session_id: Some("test-session".to_string()),
        };

        // Test list_tools
        let tools = provider.list_tools(&context).await.unwrap();
        assert_eq!(tools.len(), 2);
        assert_eq!(tools[0].name, "mock_tool");
        assert_eq!(tools[1].name, "failing_tool");

        // Test successful tool call
        let args = json!({"message": "hello world"});
        let result = provider.call_tool("mock_tool", args.clone(), &context).await.unwrap();
        assert_eq!(result["success"], true);
        assert_eq!(result["arguments"], args);

        // Test failing tool call
        let result = provider.call_tool("failing_tool", json!({}), &context).await;
        assert!(result.is_err());

        // Test unknown tool call
        let result = provider.call_tool("unknown_tool", json!({}), &context).await;
        assert!(result.is_err());

        // Verify call count
        assert_eq!(provider.call_count.load(Ordering::Relaxed), 2); // 2 successful calls
    }

    /// Test resource provider trait implementation
    #[tokio::test]
    async fn test_resource_provider_implementation() {
        let provider = MockResourceProvider::new();

        // Test list_resources
        let resources = provider.list_resources().await.unwrap();
        assert_eq!(resources.len(), 2);
        assert_eq!(resources[0].uri, "mock://resource1");
        assert_eq!(resources[1].uri, "mock://resource2");

        // Test read valid resource
        let content = provider.read_resource("mock://resource1").await.unwrap();
        assert_eq!(content.uri, "mock://resource1");
        assert_eq!(content.mime_type, Some("text/plain".to_string()));
        assert!(content.content.contains("resource 1"));

        // Test read JSON resource
        let content = provider.read_resource("mock://resource2").await.unwrap();
        assert_eq!(content.mime_type, Some("application/json".to_string()));
        assert!(content.content.contains("json"));

        // Test read non-existent resource
        let result = provider.read_resource("mock://nonexistent").await;
        assert!(result.is_err());
    }

    /// Test combined handler implementation
    #[tokio::test]
    async fn test_combined_handler_implementation() {
        let handler = Arc::new(MockHandler::new());
        let engine = McpProtocolEngine::with_handler(handler.clone());

        let session_id = "test-session".to_string();

        // Test initialization
        let init_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18"
            }
        });

        let result = engine.handle_message(init_request, Some(session_id.clone())).await.unwrap();
        assert_eq!(result["result"]["protocolVersion"], "2025-06-18");
        assert!(result["result"]["capabilities"]["tools"].is_object());
        assert!(result["result"]["capabilities"]["resources"].is_object());

        // Test double initialization (should fail)
        let init_request2 = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18"
            }
        });

        let result = engine.handle_message(init_request2, Some(session_id.clone())).await.unwrap();
        assert!(result["error"].is_object());

        // Test tools/list
        let tools_request = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/list",
            "params": {}
        });

        let result = engine.handle_message(tools_request, Some(session_id.clone())).await.unwrap();
        let tools = &result["result"]["tools"];
        assert!(tools.is_array());
        assert_eq!(tools.as_array().unwrap().len(), 2);

        // Test tools/call
        let call_request = json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": {
                "name": "mock_tool",
                "arguments": {
                    "message": "test message"
                }
            }
        });

        let result = engine.handle_message(call_request, Some(session_id.clone())).await.unwrap();
        assert_eq!(result["result"]["success"], true);

        // Test resources/list
        let resources_request = json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "resources/list",
            "params": {}
        });

        let result = engine.handle_message(resources_request, Some(session_id.clone())).await.unwrap();
        let resources = &result["result"]["resources"];
        assert!(resources.is_array());
        assert_eq!(resources.as_array().unwrap().len(), 2);

        // Test resources/read
        let read_request = json!({
            "jsonrpc": "2.0",
            "id": 6,
            "method": "resources/read",
            "params": {
                "uri": "mock://resource1"
            }
        });

        let result = engine.handle_message(read_request, Some(session_id)).await.unwrap();
        assert_eq!(result["result"]["uri"], "mock://resource1");
        assert!(result["result"]["content"].as_str().unwrap().contains("resource 1"));
    }

    /// Test handler error propagation
    #[tokio::test]
    async fn test_handler_error_propagation() {
        let handler = Arc::new(MockHandler::new());
        let engine = McpProtocolEngine::with_handler(handler);

        let session_id = "error-test-session".to_string();

        // Initialize first
        let init_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18"
            }
        });

        engine.handle_message(init_request, Some(session_id.clone())).await.unwrap();

        // Test tool call that fails
        let failing_call = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "failing_tool",
                "arguments": {}
            }
        });

        let result = engine.handle_message(failing_call, Some(session_id.clone())).await.unwrap();
        assert!(result["error"].is_object());
        assert!(result["error"]["message"].as_str().unwrap().contains("failed"));

        // Test calling unknown tool
        let unknown_call = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "unknown_tool",
                "arguments": {}
            }
        });

        let result = engine.handle_message(unknown_call, Some(session_id.clone())).await.unwrap();
        assert!(result["error"].is_object());

        // Test reading unknown resource
        let unknown_read = json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "resources/read",
            "params": {
                "uri": "mock://unknown"
            }
        });

        let result = engine.handle_message(unknown_read, Some(session_id)).await.unwrap();
        assert!(result["error"].is_object());
    }

    /// Test concurrent handler access
    #[tokio::test]
    async fn test_concurrent_handler_access() {
        let handler = Arc::new(MockHandler::new());
        let engine = Arc::new(McpProtocolEngine::with_handler(handler.clone()));

        // Initialize session
        let init_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18"
            }
        });

        engine.handle_message(init_request, Some("concurrent-session".to_string())).await.unwrap();

        // Make concurrent requests
        let mut handles = vec![];
        for i in 0..20 {
            let engine = engine.clone();
            let session_id = "concurrent-session".to_string();

            let handle = tokio::spawn(async move {
                let request = json!({
                    "jsonrpc": "2.0",
                    "id": i + 2,
                    "method": "tools/call",
                    "params": {
                        "name": "mock_tool",
                        "arguments": {
                            "message": format!("concurrent message {}", i)
                        }
                    }
                });

                engine.handle_message(request, Some(session_id)).await
            });

            handles.push(handle);
        }

        // Wait for all requests to complete
        for handle in handles {
            let result = handle.await.unwrap().unwrap();
            assert_eq!(result["result"]["success"], true);
        }

        // Verify all calls were recorded
        assert_eq!(handler.tool_provider.call_count.load(Ordering::Relaxed), 20);
    }

    /// Test context propagation
    #[tokio::test]
    async fn test_context_propagation() {
        struct ContextAwareHandler {
            contexts_seen: Arc<tokio::sync::Mutex<Vec<McpContext>>>,
        }

        #[async_trait]
        impl McpHandler for ContextAwareHandler {
            async fn initialize(&self, _params: Value, context: &McpContext) -> Result<Value> {
                let mut contexts = self.contexts_seen.lock().await;
                contexts.push(context.clone());

                Ok(json!({
                    "protocolVersion": "2025-06-18",
                    "capabilities": {"tools": {}},
                    "serverInfo": {"name": "context-aware", "version": "1.0.0"}
                }))
            }

            async fn list_tools(&self, context: &McpContext) -> Result<Vec<ToolDefinition>> {
                let mut contexts = self.contexts_seen.lock().await;
                contexts.push(context.clone());

                Ok(vec![ToolDefinition {
                    name: "context_tool".to_string(),
                    description: "Tests context".to_string(),
                    input_schema: json!({"type": "object"}),
                }])
            }

            async fn call_tool(&self, _name: &str, _arguments: Value, context: &McpContext) -> Result<Value> {
                let mut contexts = self.contexts_seen.lock().await;
                contexts.push(context.clone());

                Ok(json!({"context_session": context.session_id}))
            }
        }

        let handler = Arc::new(ContextAwareHandler {
            contexts_seen: Arc::new(tokio::sync::Mutex::new(Vec::new())),
        });
        let engine = McpProtocolEngine::with_handler(handler.clone());

        let session_id = "context-test".to_string();

        // Initialize
        let init = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {"protocolVersion": "2025-06-18"}
        });
        engine.handle_message(init, Some(session_id.clone())).await.unwrap();

        // List tools
        let list_tools = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        });
        engine.handle_message(list_tools, Some(session_id.clone())).await.unwrap();

        // Call tool
        let call_tool = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {"name": "context_tool", "arguments": {}}
        });
        let result = engine.handle_message(call_tool, Some(session_id.clone())).await.unwrap();
        assert_eq!(result["result"]["context_session"], session_id);

        // Verify all contexts had the correct session ID
        let contexts = handler.contexts_seen.lock().await;
        assert_eq!(contexts.len(), 3); // init, list_tools, call_tool
        for context in contexts.iter() {
            assert_eq!(context.session_id, Some(session_id.clone()));
        }
    }
}