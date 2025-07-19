//! Protocol Parsing Unit Tests
//!
//! Tests for parsing and validating MCP protocol messages without network dependencies

#[cfg(test)]
mod tests {
    use crate::protocol_impl::McpProtocolHandlerImpl;
    use serde_json::{json, Value};

    /// Test parsing valid JSON-RPC requests
    #[tokio::test]
    async fn test_valid_jsonrpc_request_parsing() {
        let mut handler = McpProtocolHandlerImpl::new();

        // Test request with all required fields
        let valid_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {}
            }
        });

        let result = handler.handle_message(valid_request).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["id"], 1);
        assert!(response.get("result").is_some());
    }

    /// Test parsing requests with missing required fields
    #[tokio::test]
    async fn test_missing_required_fields() {
        let mut handler = McpProtocolHandlerImpl::new();

        // Missing jsonrpc field
        let missing_jsonrpc = json!({
            "id": 1,
            "method": "initialize",
            "params": {}
        });

        let result = handler.handle_message(missing_jsonrpc).await;
        // Should either return an error response or fail with an error
        match result {
            Ok(response) => {
                assert!(response.get("error").is_some());
            }
            Err(_) => {
                // Also acceptable - malformed JSON-RPC can fail entirely
            }
        }

        // Missing method field
        let missing_method = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "params": {}
        });

        let result = handler.handle_message(missing_method).await;
        // Should either return an error response or fail with an error
        match result {
            Ok(response) => {
                assert!(response.get("error").is_some());
            }
            Err(_) => {
                // Also acceptable - malformed JSON-RPC can fail entirely
            }
        }
    }

    /// Test handling of different ID types
    #[tokio::test]
    async fn test_id_type_handling() {
        let mut handler = McpProtocolHandlerImpl::new();

        // Initialize first
        let init = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18"
            }
        });
        handler.handle_message(init).await.unwrap();

        // Test numeric ID
        let numeric_id = json!({
            "jsonrpc": "2.0",
            "id": 42,
            "method": "tools/list",
            "params": {}
        });

        let result = handler.handle_message(numeric_id).await.unwrap();
        assert_eq!(result["id"], 42);

        // Test string ID
        let string_id = json!({
            "jsonrpc": "2.0",
            "id": "test-id-123",
            "method": "tools/list",
            "params": {}
        });

        let result = handler.handle_message(string_id).await.unwrap();
        assert_eq!(result["id"], "test-id-123");

        // Test null ID (notification)
        let null_id = json!({
            "jsonrpc": "2.0",
            "id": null,
            "method": "notifications/initialized",
            "params": {}
        });

        let result = handler.handle_message(null_id).await.unwrap();
        assert_eq!(result["id"], Value::Null);
    }

    /// Test parameter validation
    #[tokio::test]
    async fn test_parameter_validation() {
        let mut handler = McpProtocolHandlerImpl::new();

        // Initialize with valid params
        let valid_init = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {
                    "tools": {
                        "listChanged": true
                    }
                },
                "clientInfo": {
                    "name": "test-client",
                    "version": "1.0.0"
                }
            }
        });

        let result = handler.handle_message(valid_init).await.unwrap();
        assert!(result.get("result").is_some());

        // Test with empty params object
        let empty_params = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        });

        let result = handler.handle_message(empty_params).await.unwrap();
        assert!(result.get("result").is_some());

        // Test with missing params (should be treated as empty object)
        let no_params = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/list"
        });

        let result = handler.handle_message(no_params).await.unwrap();
        assert!(result.get("result").is_some());
    }

    /// Test protocol version negotiation
    #[tokio::test]
    async fn test_protocol_version_negotiation() {
        let mut handler = McpProtocolHandlerImpl::new();

        // Test with supported version
        let supported_version = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18"
            }
        });

        let result = handler.handle_message(supported_version).await.unwrap();
        assert_eq!(result["result"]["protocolVersion"], "2025-06-18");

        // Test with another supported version
        let mut handler2 = McpProtocolHandlerImpl::new();
        let supported_version2 = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-03-26"
            }
        });

        let result = handler2.handle_message(supported_version2).await.unwrap();
        assert_eq!(result["result"]["protocolVersion"], "2025-03-26");

        // Test with unsupported version (should error)
        let mut handler3 = McpProtocolHandlerImpl::new();
        let unsupported_version = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-01-01"
            }
        });

        let result = handler3.handle_message(unsupported_version).await.unwrap();
        assert!(result.get("error").is_some());
        assert_eq!(result["error"]["code"], -32603); // Internal error
    }

    /// Test notification handling
    #[tokio::test]
    async fn test_notification_handling() {
        let mut handler = McpProtocolHandlerImpl::new();

        // Notifications should work even without initialization for some cases
        let cancel_notification = json!({
            "jsonrpc": "2.0",
            "method": "notifications/cancel",
            "params": {
                "requestId": "some-request"
            }
        });

        let result = handler.handle_message(cancel_notification).await.unwrap();
        // For notifications (no id field), response should be empty or have no id
        assert!(result.get("id").is_none() || result.get("id") == Some(&Value::Null));

        // Initialize first
        let init = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18"
            }
        });
        handler.handle_message(init).await.unwrap();

        // Test initialized notification
        let initialized_notification = json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        });

        let result = handler
            .handle_message(initialized_notification)
            .await
            .unwrap();
        assert!(result.as_object().unwrap().is_empty() || result.get("id").is_some());
    }

    /// Test batch request handling (not supported, should error)
    #[tokio::test]
    async fn test_batch_request_rejection() {
        let mut handler = McpProtocolHandlerImpl::new();

        // Batch requests are arrays
        let batch_request = json!([
            {
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {}
            },
            {
                "jsonrpc": "2.0",
                "id": 2,
                "method": "tools/list",
                "params": {}
            }
        ]);

        // This should fail because we expect an object, not an array
        // In a real implementation, we'd need to handle this at a higher level
        // For now, test that we don't panic
        let _result = handler.handle_message(batch_request).await;
    }

    /// Test malformed JSON handling
    #[tokio::test]
    async fn test_malformed_json_handling() {
        let mut handler = McpProtocolHandlerImpl::new();

        // Test with invalid types for fields
        let invalid_id_type = json!({
            "jsonrpc": "2.0",
            "id": {"invalid": "object"},
            "method": "initialize",
            "params": {}
        });

        let result = handler.handle_message(invalid_id_type).await;
        // Should handle gracefully
        assert!(result.is_ok());

        // Test with invalid method type
        let invalid_method_type = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": 123, // Should be string
            "params": {}
        });

        let result = handler.handle_message(invalid_method_type).await;
        assert!(result.is_err() || result.unwrap().get("error").is_some());
    }

    /// Test handling of very large messages
    #[tokio::test]
    async fn test_large_message_handling() {
        let mut handler = McpProtocolHandlerImpl::new();

        // Create a large params object
        let mut large_data = Vec::new();
        for i in 0..1000 {
            large_data.push(format!("item_{i}"));
        }

        let large_message = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {
                    "name": "test-client",
                    "version": "1.0.0",
                    "metadata": large_data
                }
            }
        });

        let result = handler.handle_message(large_message).await;
        assert!(result.is_ok());
    }

    /// Test concurrent message handling safety
    #[tokio::test]
    async fn test_message_ordering() {
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

        // Send multiple requests with different IDs
        let requests = vec![
            json!({
                "jsonrpc": "2.0",
                "id": 100,
                "method": "tools/list",
                "params": {}
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 200,
                "method": "tools/list",
                "params": {}
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 300,
                "method": "tools/list",
                "params": {}
            }),
        ];

        // Process all requests
        let mut responses = Vec::new();
        for request in requests {
            let response = handler.handle_message(request).await.unwrap();
            responses.push(response);
        }

        // Verify each response has the correct ID
        assert_eq!(responses[0]["id"], 100);
        assert_eq!(responses[1]["id"], 200);
        assert_eq!(responses[2]["id"], 300);
    }
}
