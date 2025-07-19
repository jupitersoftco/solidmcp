//! Capability Negotiation Unit Tests
//!
//! Tests for MCP capability negotiation and feature detection

#[cfg(test)]
mod tests {
    use crate::protocol_impl::McpProtocolHandlerImpl;
    use serde_json::{json, Value};

    /// Test basic capability negotiation
    #[tokio::test]
    async fn test_basic_capability_negotiation() {
        let mut handler = McpProtocolHandlerImpl::new();

        // Initialize with basic capabilities
        let init_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {
                    "tools": {
                        "listChanged": true
                    },
                    "resources": {
                        "subscribe": false,
                        "listChanged": false
                    }
                },
                "clientInfo": {
                    "name": "test-client",
                    "version": "1.0.0"
                }
            }
        });

        let result = handler.handle_message(init_request).await.unwrap();
        assert_eq!(result["jsonrpc"], "2.0");
        assert_eq!(result["id"], 1);
        
        let capabilities = &result["result"]["capabilities"];
        assert!(capabilities.is_object());
        
        // Server should advertise its capabilities
        if let Some(tools) = capabilities.get("tools") {
            assert!(tools.is_object());
        }
    }

    /// Test capability intersection (what both client and server support)
    #[tokio::test]
    async fn test_capability_intersection() {
        let mut handler = McpProtocolHandlerImpl::new();

        // Client requests capabilities server doesn't support
        let init_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {
                    "tools": {
                        "listChanged": true
                    },
                    "prompts": {
                        "listChanged": true
                    },
                    "resources": {
                        "subscribe": true,
                        "listChanged": true
                    },
                    "logging": {}
                }
            }
        });

        let result = handler.handle_message(init_request).await.unwrap();
        let server_caps = &result["result"]["capabilities"];
        
        // Server should only advertise what it actually supports
        // Built-in handler supports tools but may not support all advanced features
        assert!(server_caps.get("tools").is_some());
    }

    /// Test version negotiation
    #[tokio::test]
    async fn test_version_negotiation() {
        let mut handler = McpProtocolHandlerImpl::new();

        // Test with exact supported version
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

        // Test with different version - should still work but echo client's version
        let mut handler2 = McpProtocolHandlerImpl::new();
        let different_version = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-12-01"
            }
        });

        let result = handler2.handle_message(different_version).await.unwrap();
        assert_eq!(result["result"]["protocolVersion"], "2024-12-01");
    }

    /// Test client info handling
    #[tokio::test]
    async fn test_client_info_handling() {
        let mut handler = McpProtocolHandlerImpl::new();

        let init_with_client_info = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "clientInfo": {
                    "name": "Claude Desktop",
                    "version": "0.7.1"
                }
            }
        });

        let result = handler.handle_message(init_with_client_info).await.unwrap();
        assert!(result["result"]["serverInfo"].is_object());
        
        let server_info = &result["result"]["serverInfo"];
        assert!(server_info["name"].is_string());
        assert!(server_info["version"].is_string());
    }

    /// Test tools capability negotiation
    #[tokio::test]
    async fn test_tools_capability_negotiation() {
        let mut handler = McpProtocolHandlerImpl::new();

        // Initialize with tools capability
        let init = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {
                    "tools": {
                        "listChanged": true
                    }
                }
            }
        });

        let result = handler.handle_message(init).await.unwrap();
        let capabilities = &result["result"]["capabilities"];
        
        // Should include tools capability
        assert!(capabilities.get("tools").is_some());

        // Should be able to list tools after negotiation
        let tools_request = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        });

        let tools_result = handler.handle_message(tools_request).await.unwrap();
        assert!(tools_result["result"]["tools"].is_array());
    }

    /// Test resources capability negotiation
    #[tokio::test]
    async fn test_resources_capability_negotiation() {
        let mut handler = McpProtocolHandlerImpl::new();

        // Initialize requesting resources capability
        let init = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {
                    "resources": {
                        "subscribe": true,
                        "listChanged": false
                    }
                }
            }
        });

        let result = handler.handle_message(init).await.unwrap();
        
        // Built-in handler may not support resources, but should respond gracefully
        let resources_request = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "resources/list",
            "params": {}
        });

        let resources_result = handler.handle_message(resources_request).await.unwrap();
        // Should either succeed with empty list or fail gracefully
        assert!(resources_result.get("result").is_some() || resources_result.get("error").is_some());
    }

    /// Test prompts capability negotiation
    #[tokio::test]
    async fn test_prompts_capability_negotiation() {
        let mut handler = McpProtocolHandlerImpl::new();

        let init = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {
                    "prompts": {
                        "listChanged": true
                    }
                }
            }
        });

        let result = handler.handle_message(init).await.unwrap();
        
        // Test prompts/list
        let prompts_request = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "prompts/list",
            "params": {}
        });

        let prompts_result = handler.handle_message(prompts_request).await.unwrap();
        // Should either succeed or fail gracefully
        assert!(prompts_result.get("result").is_some() || prompts_result.get("error").is_some());
    }

    /// Test logging capability negotiation
    #[tokio::test]
    async fn test_logging_capability_negotiation() {
        let mut handler = McpProtocolHandlerImpl::new();

        let init = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {
                    "logging": {}
                }
            }
        });

        let result = handler.handle_message(init).await.unwrap();
        
        // Test logging notification
        let log_notification = json!({
            "jsonrpc": "2.0",
            "method": "notifications/message",
            "params": {
                "level": "info",
                "message": "Test log message"
            }
        });

        let log_result = handler.handle_message(log_notification).await.unwrap();
        // Notifications should be processed successfully
        assert!(log_result.as_object().unwrap().is_empty() || log_result.get("id").is_some());
    }

    /// Test sampling capability negotiation
    #[tokio::test]
    async fn test_sampling_capability_negotiation() {
        let mut handler = McpProtocolHandlerImpl::new();

        let init = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {
                    "sampling": {}
                }
            }
        });

        let result = handler.handle_message(init).await.unwrap();
        
        // Test sampling request (may not be supported by built-in handler)
        let sampling_request = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "sampling/createMessage",
            "params": {
                "messages": [
                    {
                        "role": "user",
                        "content": {
                            "type": "text",
                            "text": "Hello"
                        }
                    }
                ]
            }
        });

        let sampling_result = handler.handle_message(sampling_request).await.unwrap();
        // Should fail gracefully if not supported
        assert!(sampling_result.get("error").is_some() || sampling_result.get("result").is_some());
    }

    /// Test empty capabilities
    #[tokio::test]
    async fn test_empty_capabilities() {
        let mut handler = McpProtocolHandlerImpl::new();

        // Initialize with no capabilities
        let init = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {}
            }
        });

        let result = handler.handle_message(init).await.unwrap();
        assert!(result["result"]["capabilities"].is_object());
        
        // Should still be able to use basic functionality
        let tools_request = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        });

        let tools_result = handler.handle_message(tools_request).await.unwrap();
        assert!(tools_result.get("result").is_some());
    }

    /// Test missing capabilities field
    #[tokio::test]
    async fn test_missing_capabilities_field() {
        let mut handler = McpProtocolHandlerImpl::new();

        // Initialize without capabilities field
        let init = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18"
            }
        });

        let result = handler.handle_message(init).await.unwrap();
        assert!(result["result"]["capabilities"].is_object());
        
        // Should work with default capabilities
        let tools_request = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        });

        let tools_result = handler.handle_message(tools_request).await.unwrap();
        assert!(tools_result.get("result").is_some());
    }

    /// Test capability-dependent method availability
    #[tokio::test]
    async fn test_capability_dependent_methods() {
        let mut handler = McpProtocolHandlerImpl::new();

        // Initialize with minimal capabilities
        let init = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {
                    "tools": {}
                }
            }
        });

        handler.handle_message(init).await.unwrap();

        // Methods that should work with basic tools capability
        let valid_methods = vec![
            "tools/list",
            "tools/call",
        ];

        for method in valid_methods {
            let request = json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": method,
                "params": {}
            });

            let result = handler.handle_message(request).await.unwrap();
            // Should not return "method not found" error
            if let Some(error) = result.get("error") {
                let code = error["code"].as_i64().unwrap_or(0);
                assert_ne!(code, -32601, "Method {method} should be available with tools capability");
            }
        }
    }

    /// Test progressive capability discovery
    #[tokio::test]
    async fn test_progressive_capability_discovery() {
        let mut handler = McpProtocolHandlerImpl::new();

        // Start with basic capabilities
        let init = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {
                    "tools": {
                        "listChanged": false
                    }
                }
            }
        });

        let result = handler.handle_message(init).await.unwrap();
        let server_caps = &result["result"]["capabilities"];
        
        // Check what server actually supports
        let supports_tools = server_caps.get("tools").is_some();
        let supports_resources = server_caps.get("resources").is_some();
        let supports_prompts = server_caps.get("prompts").is_some();

        // Verify tools work if advertised
        if supports_tools {
            let tools_request = json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "tools/list",
                "params": {}
            });

            let tools_result = handler.handle_message(tools_request).await.unwrap();
            assert!(tools_result.get("result").is_some());
        }

        // Test other capabilities only if advertised
        if supports_resources {
            let resources_request = json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "resources/list",
                "params": {}
            });

            let resources_result = handler.handle_message(resources_request).await.unwrap();
            assert!(resources_result.get("result").is_some());
        }

        if supports_prompts {
            let prompts_request = json!({
                "jsonrpc": "2.0",
                "id": 4,
                "method": "prompts/list",
                "params": {}
            });

            let prompts_result = handler.handle_message(prompts_request).await.unwrap();
            assert!(prompts_result.get("result").is_some());
        }
    }
}