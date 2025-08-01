//! Handler Trait Implementation Tests
//!
//! Tests for mocked handler implementations to verify trait compliance

#[cfg(test)]
mod tests {
    use crate::handler::{McpContext, McpHandler, ToolDefinition};
    use crate::shared::McpProtocolEngine;
    use anyhow::Result;
    use async_trait::async_trait;
    use serde_json::{json, Value};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    /// Combined mock handler implementing the main trait
    struct MockHandler {
        call_count: AtomicUsize,
        initialized: Arc<tokio::sync::Mutex<bool>>,
    }

    impl MockHandler {
        fn new() -> Self {
            Self {
                call_count: AtomicUsize::new(0),
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
                    "tools": {}
                },
                "serverInfo": {
                    "name": "mock-server",
                    "version": "1.0.0"
                }
            }))
        }

        async fn list_tools(&self, _context: &McpContext) -> Result<Vec<ToolDefinition>> {
            self.call_count.fetch_add(1, Ordering::Relaxed);
            Ok(vec![
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
                    output_schema: json!({
                        "type": "object",
                        "properties": {
                            "success": { "type": "boolean" },
                            "processed": { "type": "string" },
                            "arguments": { "type": "object" }
                        },
                        "required": ["success", "processed", "arguments"]
                    }),
                },
                ToolDefinition {
                    name: "failing_tool".to_string(),
                    description: "A tool that always fails".to_string(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {}
                    }),
                    output_schema: json!({
                        "type": "object",
                        "properties": {
                            "error": { "type": "string" }
                        }
                    }),
                },
            ])
        }

        async fn call_tool(
            &self,
            name: &str,
            arguments: Value,
            _context: &McpContext,
        ) -> Result<Value> {
            self.call_count.fetch_add(1, Ordering::Relaxed);

            match name {
                "mock_tool" => Ok(json!({
                    "success": true,
                    "processed": "message",
                    "arguments": arguments
                })),
                "failing_tool" => Err(anyhow::anyhow!("Tool execution failed")),
                _ => Err(anyhow::anyhow!("Tool '{}' not found", name)),
            }
        }
    }

    /// Test mock handler trait implementation
    #[tokio::test]
    async fn test_mock_handler_implementation() {
        let handler = MockHandler::new();
        let context = McpContext {
            session_id: Some("test-session".to_string()),
            notification_sender: None,
            protocol_version: Some("2025-06-18".to_string()),
            client_info: None,
        };

        // Test initialization
        let init_result = handler.initialize(json!({}), &context).await.unwrap();
        assert_eq!(init_result["protocolVersion"], "2025-06-18");
        assert!(init_result["capabilities"]["tools"].is_object());

        // Test double initialization (should fail)
        let double_init = handler.initialize(json!({}), &context).await;
        assert!(double_init.is_err());

        // Test list_tools
        let tools = handler.list_tools(&context).await.unwrap();
        assert_eq!(tools.len(), 2);
        assert_eq!(tools[0].name, "mock_tool");
        assert_eq!(tools[1].name, "failing_tool");

        // Test successful tool call
        let args = json!({"message": "hello world"});
        let result = handler
            .call_tool("mock_tool", args.clone(), &context)
            .await
            .unwrap();
        assert_eq!(result["success"], true);
        assert_eq!(result["arguments"], args);

        // Test failing tool call
        let result = handler.call_tool("failing_tool", json!({}), &context).await;
        assert!(result.is_err());

        // Test unknown tool call
        let result = handler.call_tool("unknown_tool", json!({}), &context).await;
        assert!(result.is_err());

        // Verify call count (list_tools + 3 tool calls, including the failing ones)
        assert_eq!(handler.call_count.load(Ordering::Relaxed), 4);
    }

    /// Test handler with protocol engine integration
    #[tokio::test]
    async fn test_handler_with_engine_integration() {
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

        let result = engine
            .handle_message(init_request, Some(session_id.clone()))
            .await
            .unwrap();
        assert_eq!(result["result"]["protocolVersion"], "2025-06-18");
        assert!(result["result"]["capabilities"]["tools"].is_object());

        // Test double initialization (should fail)
        let init_request2 = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18"
            }
        });

        let result = engine
            .handle_message(init_request2, Some(session_id.clone()))
            .await;
        // Should get an error response or failure
        match result {
            Ok(response) => {
                // If it returns a response, it should be an error response
                assert!(response["error"].is_object());
            }
            Err(_) => {
                // Or it can return an Err directly, which is also acceptable
            }
        }

        // Test tools/list
        let tools_request = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/list",
            "params": {}
        });

        let result = engine
            .handle_message(tools_request, Some(session_id.clone()))
            .await
            .unwrap();
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

        let result = engine
            .handle_message(call_request, Some(session_id))
            .await
            .unwrap();
        assert_eq!(result["result"]["success"], true);
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

        engine
            .handle_message(init_request, Some(session_id.clone()))
            .await
            .unwrap();

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

        let result = engine
            .handle_message(failing_call, Some(session_id.clone()))
            .await;
        match result {
            Ok(response) => {
                assert!(response["error"].is_object());
                assert!(response["error"]["message"]
                    .as_str()
                    .unwrap()
                    .contains("failed"));
            }
            Err(e) => {
                assert!(e.to_string().contains("failed"));
            }
        }

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

        let result = engine.handle_message(unknown_call, Some(session_id)).await;
        match result {
            Ok(response) => {
                assert!(response["error"].is_object());
            }
            Err(_) => {
                // Also acceptable - the engine can return errors directly
            }
        }
    }
}
