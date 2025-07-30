//! Error Handling Unit Tests
//!
//! Tests for various error conditions and error response formatting

#[cfg(test)]
mod tests {
    use crate::protocol_impl::McpProtocolHandlerImpl;
    use serde_json::{json, Value};

    /// Test error response format compliance with JSON-RPC 2.0
    #[tokio::test]
    async fn test_error_response_format() {
        let mut handler = McpProtocolHandlerImpl::new();

        // Try to call a method before initialization
        let premature_call = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/list",
            "params": {}
        });

        let result = handler.handle_message(premature_call).await.unwrap();

        // Check error response format
        assert_eq!(result["jsonrpc"], "2.0");
        assert_eq!(result["id"], 1);
        assert!(result.get("error").is_some());

        let error = &result["error"];
        assert!(error.get("code").is_some());
        assert!(error.get("message").is_some());
        assert!(error["code"].is_i64());
        assert!(error["message"].is_string());
    }

    /// Test standard JSON-RPC error codes
    #[tokio::test]
    async fn test_standard_error_codes() {
        let mut handler = McpProtocolHandlerImpl::new();

        // Test Method not found (-32601)
        let unknown_method = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "unknown/method",
            "params": {}
        });

        let result = handler.handle_message(unknown_method).await.unwrap();
        let error_code = result["error"]["code"].as_i64().unwrap();
        assert_eq!(error_code, -32601); // Method not found

        // Initialize for further tests
        let init = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18"
            }
        });
        handler.handle_message(init).await.unwrap();

        // Test Invalid params (-32602)
        let invalid_params = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                // Missing required 'name' field
                "arguments": {}
            }
        });

        let result = handler.handle_message(invalid_params).await.unwrap();
        if let Some(error) = result.get("error") {
            let error_code = error["code"].as_i64().unwrap();
            assert!(error_code == -32602 || error_code == -32603); // Invalid params or Internal error
        }
    }

    /// Test error handling for tool execution failures
    #[tokio::test]
    async fn test_tool_execution_errors() {
        let mut handler = McpProtocolHandlerImpl::new();

        // Initialize
        let init = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18"
            }
        });
        handler.handle_message(init).await.unwrap();

        // Try to call non-existent tool
        let call_unknown_tool = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "non_existent_tool",
                "arguments": {}
            }
        });

        let result = handler.handle_message(call_unknown_tool).await.unwrap();
        assert!(result.get("error").is_some());
        let error_message = result["error"]["message"].as_str().unwrap();
        assert!(
            error_message.to_lowercase().contains("unknown tool")
                || error_message.to_lowercase().contains("not found")
        );
    }

    /// Test error propagation from nested calls
    #[tokio::test]
    async fn test_nested_error_propagation() {
        let mut handler = McpProtocolHandlerImpl::new();

        // Initialize
        let init = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18"
            }
        });
        handler.handle_message(init).await.unwrap();

        // Test with echo tool with invalid arguments
        let invalid_echo = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "echo",
                "arguments": {
                    // Missing required 'message' field
                }
            }
        });

        let result = handler.handle_message(invalid_echo).await.unwrap();
        assert!(result.get("error").is_some());
    }

    /// Test handling of initialization - now allows re-initialization
    #[tokio::test]
    async fn test_initialization_errors() {
        let mut handler = McpProtocolHandlerImpl::new();

        // Try to initialize twice
        let init = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18"
            }
        });

        // First initialization should succeed
        let result1 = handler.handle_message(init.clone()).await.unwrap();
        assert!(result1.get("result").is_some());

        // Second initialization should also succeed (re-initialization is now allowed)
        let init2 = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18"
            }
        });

        let result2 = handler.handle_message(init2).await.unwrap();
        assert!(result2.get("result").is_some()); // Now expects success
        
        // Test with invalid protocol version
        let init_invalid = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "initialize",
            "params": {
                "protocolVersion": "invalid-version"
            }
        });
        
        let result3 = handler.handle_message(init_invalid).await.unwrap();
        assert!(result3.get("error").is_some());
        let error_message = result3["error"]["message"].as_str().unwrap();
        assert!(error_message.to_lowercase().contains("unsupported protocol"));
    }

    /// Test error recovery and state consistency
    #[tokio::test]
    async fn test_error_recovery() {
        let mut handler = McpProtocolHandlerImpl::new();

        // Initialize
        let init = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18"
            }
        });
        handler.handle_message(init).await.unwrap();

        // Cause an error
        let error_request = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "unknown/method",
            "params": {}
        });

        let error_result = handler.handle_message(error_request).await.unwrap();
        assert!(error_result.get("error").is_some());

        // Verify handler can still process valid requests after error
        let valid_request = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/list",
            "params": {}
        });

        let valid_result = handler.handle_message(valid_request).await.unwrap();
        assert!(valid_result.get("result").is_some());
        assert!(valid_result.get("error").is_none());
    }

    /// Test handling of panics (should be converted to errors)
    #[tokio::test]
    async fn test_panic_handling() {
        let mut handler = McpProtocolHandlerImpl::new();

        // This test would require injecting a panic-inducing condition
        // For now, we test that very malformed input doesn't crash
        let malformed = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": Value::Null, // Completely wrong type
            "params": "not an object"
        });

        // Should not panic, but return an error
        let result = handler.handle_message(malformed).await;
        assert!(result.is_err() || result.unwrap().get("error").is_some());
    }

    /// Test timeout and cancellation handling
    #[tokio::test]
    async fn test_cancellation_handling() {
        let mut handler = McpProtocolHandlerImpl::new();

        // Send a cancel notification
        let cancel = json!({
            "jsonrpc": "2.0",
            "method": "notifications/cancel",
            "params": {
                "requestId": "12345"
            }
        });

        let result = handler.handle_message(cancel).await.unwrap();
        // Cancel notifications should be processed successfully
        assert!(result.as_object().unwrap().is_empty() || result == json!({}));
    }

    /// Test resource not found errors
    #[tokio::test]
    async fn test_resource_errors() {
        let mut handler = McpProtocolHandlerImpl::new();

        // Initialize
        let init = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18"
            }
        });
        handler.handle_message(init).await.unwrap();

        // Try to read non-existent resource
        let read_missing = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "resources/read",
            "params": {
                "uri": "file:///non/existent/resource"
            }
        });

        let result = handler.handle_message(read_missing).await.unwrap();
        // Built-in handler doesn't support resources, so should error
        assert!(result.get("error").is_some());
    }

    /// Test handling of circular references or deeply nested data
    #[tokio::test]
    async fn test_deeply_nested_data() {
        let mut handler = McpProtocolHandlerImpl::new();

        // Create deeply nested params
        let mut nested = json!({"level": 0});
        for i in 1..100 {
            nested = json!({"level": i, "nested": nested});
        }

        let deep_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": nested
            }
        });

        // Should handle without stack overflow
        let result = handler.handle_message(deep_request).await;
        assert!(result.is_ok());
    }
}
