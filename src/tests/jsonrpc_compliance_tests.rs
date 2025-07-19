//! JSON-RPC 2.0 Compliance Tests
//!
//! Tests to ensure strict compliance with JSON-RPC 2.0 specification

#[cfg(test)]
mod tests {
    use crate::protocol_impl::McpProtocolHandlerImpl;
    use serde_json::{json, Value};

    /// Test that all responses include required JSON-RPC 2.0 fields
    #[tokio::test]
    async fn test_response_format_compliance() {
        let mut handler = McpProtocolHandlerImpl::new();

        let request = json!({
            "jsonrpc": "2.0",
            "id": 12345,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18"
            }
        });

        let response = handler.handle_message(request).await.unwrap();

        // JSON-RPC 2.0 required fields
        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["id"], 12345);

        // Must have either result or error, but not both
        let has_result = response.get("result").is_some();
        let has_error = response.get("error").is_some();
        assert!(has_result || has_error);
        assert!(!(has_result && has_error));
    }

    /// Test JSON-RPC 2.0 error object format
    #[tokio::test]
    async fn test_error_object_compliance() {
        let mut handler = McpProtocolHandlerImpl::new();

        // Send an invalid request to trigger an error
        let invalid_request = json!({
            "jsonrpc": "2.0",
            "id": "test-error",
            "method": "unknown/method",
            "params": {}
        });

        let response = handler.handle_message(invalid_request).await.unwrap();

        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["id"], "test-error");
        assert!(response.get("error").is_some());

        let error = &response["error"];

        // JSON-RPC 2.0 error object requirements
        assert!(error.get("code").is_some());
        assert!(error.get("message").is_some());

        // Code must be integer
        assert!(error["code"].is_i64());

        // Message must be string
        assert!(error["message"].is_string());

        // Data field is optional but if present should be structured
        if let Some(data) = error.get("data") {
            assert!(data.is_object() || data.is_array() || data.is_string());
        }
    }

    /// Test standard JSON-RPC 2.0 error codes
    #[tokio::test]
    async fn test_standard_error_codes() {
        let mut handler = McpProtocolHandlerImpl::new();

        // Test -32601 Method not found
        let method_not_found = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "completely/unknown/method",
            "params": {}
        });

        let response = handler.handle_message(method_not_found).await.unwrap();
        if let Some(error) = response.get("error") {
            let code = error["code"].as_i64().unwrap();
            assert_eq!(code, -32601);
        }

        // Test -32600 Invalid Request (malformed request)
        let invalid_request = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": 123, // Method should be string
            "params": {}
        });

        let response = handler.handle_message(invalid_request).await;
        // Should either fail or return -32600
        if let Ok(response) = response {
            if let Some(error) = response.get("error") {
                let code = error["code"].as_i64().unwrap();
                assert!(code == -32600 || code == -32602); // Invalid Request or Invalid params
            }
        }
    }

    /// Test notification handling (no response expected)
    #[tokio::test]
    async fn test_notification_compliance() {
        let mut handler = McpProtocolHandlerImpl::new();

        // Notifications have no ID field
        let notification = json!({
            "jsonrpc": "2.0",
            "method": "notifications/cancel",
            "params": {
                "requestId": "some-request"
            }
        });

        let response = handler.handle_message(notification).await.unwrap();

        // For notifications, response should be empty or have null ID
        if !response.as_object().unwrap().is_empty() {
            assert_eq!(response.get("id"), Some(&Value::Null));
        }
    }

    /// Test ID preservation and types
    #[tokio::test]
    async fn test_id_preservation() {
        let mut handler = McpProtocolHandlerImpl::new();

        // Test different ID types that JSON-RPC 2.0 supports
        let test_cases = vec![
            (json!(42), json!(42)),                   // Number
            (json!("string-id"), json!("string-id")), // String
            (json!(null), json!(null)),               // Null (notification)
        ];

        for (request_id, expected_id) in test_cases {
            let request = json!({
                "jsonrpc": "2.0",
                "id": request_id,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2025-06-18"
                }
            });

            let response = handler.handle_message(request).await.unwrap();
            assert_eq!(response["id"], expected_id);

            // Create new handler for each test to avoid "already initialized" error
            handler = McpProtocolHandlerImpl::new();
        }
    }

    /// Test parameter handling compliance
    #[tokio::test]
    async fn test_parameter_compliance() {
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

        // Test with structured parameters (object)
        let with_object_params = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        });

        let response = handler.handle_message(with_object_params).await.unwrap();
        assert!(response.get("result").is_some() || response.get("error").is_some());

        // Test with array parameters (less common in MCP)
        let with_array_params = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/list",
            "params": []
        });

        let response = handler.handle_message(with_array_params).await.unwrap();
        // Should handle gracefully (may convert to object or error)
        assert!(response.get("result").is_some() || response.get("error").is_some());

        // Test with no params field (should be treated as empty object)
        let without_params = json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/list"
        });

        let response = handler.handle_message(without_params).await.unwrap();
        assert!(response.get("result").is_some() || response.get("error").is_some());
    }

    /// Test version field enforcement
    #[tokio::test]
    async fn test_version_field_enforcement() {
        let mut handler = McpProtocolHandlerImpl::new();

        // Request without jsonrpc field
        let without_version = json!({
            "id": 1,
            "method": "initialize",
            "params": {}
        });

        let result = handler.handle_message(without_version).await;
        // Should either return an error response or fail with an error
        match result {
            Ok(response) => {
                assert!(response.get("error").is_some());
            }
            Err(_) => {
                // Also acceptable - malformed JSON-RPC can fail entirely
            }
        }

        // Request with wrong version
        let wrong_version = json!({
            "jsonrpc": "1.0",
            "id": 2,
            "method": "initialize",
            "params": {}
        });

        let result = handler.handle_message(wrong_version).await;
        // Should either return an error response or fail with an error
        match result {
            Ok(response) => {
                assert!(response.get("error").is_some());
            }
            Err(_) => {
                // Also acceptable - malformed JSON-RPC can fail entirely
            }
        }

        // Request with non-string version
        let invalid_version = json!({
            "jsonrpc": 2.0,
            "id": 3,
            "method": "initialize",
            "params": {}
        });

        let result = handler.handle_message(invalid_version).await;
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

    /// Test handling of extra fields (should be ignored)
    #[tokio::test]
    async fn test_extra_fields_ignored() {
        let mut handler = McpProtocolHandlerImpl::new();

        let request_with_extras = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18"
            },
            "extra_field": "should be ignored",
            "another_extra": 42,
            "nested_extra": {
                "foo": "bar"
            }
        });

        let response = handler.handle_message(request_with_extras).await.unwrap();
        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["id"], 1);
        assert!(response.get("result").is_some());

        // Response should not include extra fields
        assert!(response.get("extra_field").is_none());
        assert!(response.get("another_extra").is_none());
        assert!(response.get("nested_extra").is_none());
    }

    /// Test batch request rejection (not supported by MCP)
    #[tokio::test]
    async fn test_batch_request_rejection() {
        let mut handler = McpProtocolHandlerImpl::new();

        // JSON-RPC 2.0 batch request (array of requests)
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

        // Should fail to parse as single request or return appropriate error
        let result = handler.handle_message(batch_request).await;
        // MCP doesn't support batch requests, so this should fail
        assert!(result.is_err());
    }

    /// Test large number handling (JSON-RPC allows arbitrary precision)
    #[tokio::test]
    async fn test_large_number_handling() {
        let mut handler = McpProtocolHandlerImpl::new();

        // Test with large integer ID
        let large_id_request = json!({
            "jsonrpc": "2.0",
            "id": 9007199254740991i64, // Max safe integer in JavaScript
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18"
            }
        });

        let response = handler.handle_message(large_id_request).await.unwrap();
        assert_eq!(response["id"], 9007199254740991i64);

        // Test with floating point ID (valid in JSON-RPC)
        let mut handler2 = McpProtocolHandlerImpl::new();
        let float_id_request = json!({
            "jsonrpc": "2.0",
            "id": 123.456,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18"
            }
        });

        let response = handler2.handle_message(float_id_request).await.unwrap();
        assert_eq!(response["id"], 123.456);
    }

    /// Test Unicode and special character handling
    #[tokio::test]
    async fn test_unicode_handling() {
        let mut handler = McpProtocolHandlerImpl::new();

        // Test with Unicode in ID and method
        let unicode_request = json!({
            "jsonrpc": "2.0",
            "id": "测试-тест-🔥",
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "clientInfo": {
                    "name": "测试客户端",
                    "version": "1.0.0-α"
                }
            }
        });

        let response = handler.handle_message(unicode_request).await.unwrap();
        assert_eq!(response["id"], "测试-тест-🔥");
        assert!(response.get("result").is_some());
    }

    /// Test response must be valid JSON
    #[tokio::test]
    async fn test_response_json_validity() {
        let mut handler = McpProtocolHandlerImpl::new();

        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18"
            }
        });

        let response = handler.handle_message(request).await.unwrap();

        // Response should be serializable back to valid JSON
        let json_string = serde_json::to_string(&response).unwrap();
        let parsed_back: Value = serde_json::from_str(&json_string).unwrap();
        assert_eq!(response, parsed_back);
    }

    /// Test that response time is reasonable (performance aspect)
    #[tokio::test]
    async fn test_response_timing() {
        let mut handler = McpProtocolHandlerImpl::new();

        let start = std::time::Instant::now();

        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18"
            }
        });

        let _response = handler.handle_message(request).await.unwrap();

        let elapsed = start.elapsed();

        // Should respond quickly (less than 1 second for simple operations)
        assert!(elapsed.as_secs() < 1, "Response took too long: {elapsed:?}");
    }
}
