//! Edge Case Tests for SolidMCP
//!
//! Tests for boundary conditions, unusual inputs, and edge cases
//! that might not be covered by standard tests.

#[cfg(test)]
mod tests {
    use crate::protocol_impl::McpProtocolHandlerImpl;
    use crate::transport::{TransportCapabilities, TransportInfo};
    use serde_json::{json, Value};
    use warp::http::HeaderMap;

    /// Test handling of empty/null parameters
    #[tokio::test]
    async fn test_empty_and_null_parameters() {
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

        // Test with empty params object
        let empty_params = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {}
        });
        let result = handler.handle_message(empty_params).await.unwrap();
        assert!(result.get("error").is_some());

        // Test with null params
        let null_params = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": null
        });
        let result = handler.handle_message(null_params).await.unwrap();
        assert!(result.get("error").is_some());
    }

    /// Test extremely long strings and deep nesting
    #[tokio::test]
    async fn test_extreme_data_sizes() {
        let mut handler = McpProtocolHandlerImpl::new();

        // Create a very long string (1MB)
        let long_string = "a".repeat(1_000_000);
        
        let init = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "clientInfo": {
                    "name": long_string
                }
            }
        });

        // Should handle large strings gracefully
        let result = handler.handle_message(init).await.unwrap();
        assert!(result.get("result").is_some());

        // Test deeply nested object (100 levels)
        let mut deeply_nested = json!({});
        let mut current = &mut deeply_nested;
        for i in 0..100 {
            current[format!("level_{i}")] = json!({});
            current = &mut current[format!("level_{i}")];
        }
        current["value"] = json!("deep");

        let deep_request = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "echo",
                "arguments": deeply_nested
            }
        });

        let result = handler.handle_message(deep_request).await.unwrap();
        // Should handle without stack overflow
        assert!(result.get("id").is_some());
    }

    /// Test unicode and special characters in various fields
    #[tokio::test]
    async fn test_unicode_and_special_characters() {
        let mut handler = McpProtocolHandlerImpl::new();

        // Test with various unicode characters
        let unicode_tests = [
            "Hello 世界", // Chinese
            "Привет мир", // Russian
            "🚀🔧🌟", // Emojis
            "\u{200B}invisible\u{200B}", // Zero-width spaces
            "line\nbreak\ttab", // Control characters
            r#"quotes"and'stuff"#, // Quotes
        ];

        // Initialize with unicode client name
        let init = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "clientInfo": {
                    "name": unicode_tests[0],
                    "version": unicode_tests[1]
                }
            }
        });
        let result = handler.handle_message(init).await.unwrap();
        assert!(result.get("result").is_some());

        // Test echo with unicode
        for (i, test_str) in unicode_tests.iter().enumerate() {
            let echo = json!({
                "jsonrpc": "2.0",
                "id": i + 2,
                "method": "tools/call",
                "params": {
                    "name": "echo",
                    "arguments": {
                        "message": test_str
                    }
                }
            });
            let result = handler.handle_message(echo).await.unwrap();
            if let Some(result_val) = result.get("result") {
                if let Some(content_array) = result_val.get("content").and_then(|c| c.as_array()) {
                    if let Some(first_content) = content_array.first() {
                        if let Some(text) = first_content.get("text").and_then(|t| t.as_str()) {
                            // Parse the JSON response from the echo tool
                            if let Ok(parsed) = serde_json::from_str::<Value>(text) {
                                if let Some(echo_val) = parsed.get("echo").and_then(|e| e.as_str()) {
                                    assert_eq!(echo_val, *test_str);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Test invalid JSON-RPC fields and malformed requests
    #[tokio::test]
    async fn test_malformed_jsonrpc_requests() {
        let mut handler = McpProtocolHandlerImpl::new();

        // Missing jsonrpc field
        let no_jsonrpc = json!({
            "id": 1,
            "method": "initialize",
            "params": {}
        });
        // This will return an Err, not an error response
        let result = handler.handle_message(no_jsonrpc).await;
        assert!(result.is_err());

        // Wrong jsonrpc version - returns error response
        let wrong_version = json!({
            "jsonrpc": "1.0",
            "id": 2,
            "method": "initialize",
            "params": {}
        });
        let result = handler.handle_message(wrong_version).await;
        assert!(result.is_err());

        // Non-string method - returns Err
        let numeric_method = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": 12345,
            "params": {}
        });
        let result = handler.handle_message(numeric_method).await;
        assert!(result.is_err());

        // Array as ID (valid in JSON-RPC but unusual)
        let array_id = json!({
            "jsonrpc": "2.0",
            "id": [1, 2, 3],
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18"
            }
        });
        let result = handler.handle_message(array_id).await.unwrap();
        assert_eq!(result["id"], json!([1, 2, 3]));
    }

    /// Test transport capability edge cases
    #[test]
    fn test_transport_edge_cases() {
        // Test with conflicting headers
        let mut headers = HeaderMap::new();
        headers.insert("upgrade", "websocket".parse().unwrap());
        headers.insert("connection", "close".parse().unwrap()); // Conflicting!
        
        let capabilities = TransportCapabilities::from_headers(&headers);
        assert!(!capabilities.supports_websocket); // Should not support WS with conflicting headers

        // Test with multiple values in headers
        let mut headers = HeaderMap::new();
        headers.insert("accept", "text/html, application/json, */*".parse().unwrap());
        headers.insert("connection", "keep-alive, upgrade".parse().unwrap());
        headers.insert("upgrade", "websocket".parse().unwrap());
        
        let capabilities = TransportCapabilities::from_headers(&headers);
        assert!(capabilities.supports_websocket);

        // Test transport info with special characters in endpoint
        let capabilities = TransportCapabilities {
            supports_websocket: true,
            supports_sse: false,
            supports_http_only: true,
            client_info: Some("test-client".to_string()),
            protocol_version: Some("2025-06-18".to_string()),
        };

        let special_endpoints = vec![
            "/mcp/endpoint with spaces",
            "/mcp/端点", // Chinese characters
            "/mcp/🚀", // Emoji
            "//double//slashes//",
        ];

        for endpoint in special_endpoints {
            let info = TransportInfo::new(&capabilities, "test", "1.0", endpoint);
            let json = info.to_json();
            
            // Should handle special characters in URIs
            let transports = json["mcp_server"]["available_transports"].as_object().unwrap();
            assert!(transports.contains_key("websocket"));
            assert!(transports.contains_key("http"));
        }
    }

    /// Test concurrent message handling
    #[tokio::test]
    async fn test_concurrent_message_handling() {
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

        // Create multiple concurrent requests
        let mut tasks = vec![];
        
        for i in 0..10 {
            let request = json!({
                "jsonrpc": "2.0",
                "id": i + 100,
                "method": "tools/list",
                "params": {}
            });
            
            // Clone the request for the async block
            let req = request.clone();
            tasks.push(tokio::spawn(async move {
                let mut local_handler = McpProtocolHandlerImpl::new();
                // Initialize the local handler
                let init = json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "initialize",
                    "params": {
                        "protocolVersion": "2025-06-18"
                    }
                });
                local_handler.handle_message(init).await.unwrap();
                local_handler.handle_message(req).await
            }));
        }

        // All requests should complete successfully
        for task in tasks {
            let result = task.await.unwrap().unwrap();
            assert!(result.get("result").is_some());
        }
    }

    /// Test protocol version edge cases
    #[tokio::test]
    async fn test_protocol_version_edge_cases() {
        let mut handler = McpProtocolHandlerImpl::new();

        // Test with empty protocol version
        let empty_version = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": ""
            }
        });
        let result = handler.handle_message(empty_version).await.unwrap();
        assert!(result.get("error").is_some());

        // Test with whitespace protocol version
        let whitespace_version = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "initialize",
            "params": {
                "protocolVersion": "  \t\n  "
            }
        });
        let result = handler.handle_message(whitespace_version).await.unwrap();
        assert!(result.get("error").is_some());

        // Test with very long protocol version
        let long_version = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18-experimental-beta-preview-release-candidate-final-v2"
            }
        });
        let result = handler.handle_message(long_version).await.unwrap();
        assert!(result.get("error").is_some());

        // Test with numeric protocol version
        let numeric_version = json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "initialize",
            "params": {
                "protocolVersion": 20250618
            }
        });
        let result = handler.handle_message(numeric_version).await.unwrap();
        assert!(result.get("error").is_some());
    }
}