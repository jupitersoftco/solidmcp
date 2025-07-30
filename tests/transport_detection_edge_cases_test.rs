//! Transport Detection Edge Cases Tests
//!
//! Comprehensive tests for transport detection edge cases following TDD principles

use reqwest::header::{ACCEPT, CONNECTION, UPGRADE, USER_AGENT, CONTENT_TYPE};

mod mcp_test_helpers;
use mcp_test_helpers::with_mcp_test_server;

/// Test 1: RED - Ambiguous transport headers
#[tokio::test]
async fn test_ambiguous_transport_headers() {
    // Test cases where headers might be ambiguous or conflicting
    with_mcp_test_server("ambiguous_headers_test", |server| async move {
        let test_cases = vec![
            (
                vec![
                    (UPGRADE, "websocket"),
                    (CONNECTION, "keep-alive"), // Wrong connection header
                ],
                "WebSocket upgrade without proper connection header"
            ),
            (
                vec![
                    (UPGRADE, "WEBSOCKET"), // Different case
                    (CONNECTION, "Upgrade"),
                ],
                "WebSocket with different case"
            ),
            (
                vec![
                    (UPGRADE, "websocket, http2"), // Multiple upgrades
                    (CONNECTION, "upgrade"),
                ],
                "Multiple upgrade options"
            ),
            (
                vec![
                    (ACCEPT, "text/event-stream, application/json"), // Multiple accepts
                    (CONTENT_TYPE, "application/json"),
                ],
                "Multiple accept types"
            ),
        ];

        for (headers, description) in test_cases {
            let client = reqwest::Client::new();
            let mut request = client.get(&server.http_url());
            
            for (header, value) in headers {
                request = request.header(header, value);
            }

            let response = request.send().await?;
            
            // Should handle gracefully - either upgrade or provide info
            assert!(
                response.status().is_success(),
                "Failed for: {}",
                description
            );
        }

        Ok(())
    })
    .await
    .unwrap();
}

/// Test 2: RED - Missing or malformed upgrade headers
#[tokio::test]
async fn test_malformed_upgrade_headers() {
    // Test WebSocket upgrade with malformed headers
    with_mcp_test_server("malformed_upgrade_test", |server| async move {
        let test_cases = vec![
            (
                vec![
                    (UPGRADE, ""), // Empty upgrade
                    (CONNECTION, "upgrade"),
                ],
                "Empty upgrade header"
            ),
            (
                vec![
                    (UPGRADE, "websocket"),
                    (CONNECTION, ""), // Empty connection
                ],
                "Empty connection header"
            ),
            (
                vec![
                    (UPGRADE, "web socket"), // Space in websocket
                    (CONNECTION, "upgrade"),
                ],
                "Space in websocket value"
            ),
            (
                vec![
                    (UPGRADE, "ws"), // Abbreviated
                    (CONNECTION, "upgrade"),
                ],
                "Abbreviated websocket"
            ),
        ];

        for (headers, description) in test_cases {
            let client = reqwest::Client::new();
            let mut request = client.get(&server.http_url());
            
            for (header, value) in headers {
                request = request.header(header, value);
            }

            let response = request.send().await?;
            
            // Should not crash and return appropriate response
            assert!(
                response.status() == 200 || response.status() == 426,
                "Unexpected status {} for: {}",
                response.status(),
                description
            );
        }

        Ok(())
    })
    .await
    .unwrap();
}

/// Test 3: RED - User agent detection edge cases
#[tokio::test]
async fn test_user_agent_edge_cases() {
    // Test various user agent strings and their impact on transport detection
    with_mcp_test_server("user_agent_test", |server| async move {
        let long_agent = "a".repeat(1000);
        let test_cases = vec![
            ("", "Empty user agent"),
            ("Mozilla/5.0 (compatible; Googlebot/2.1; +http://www.google.com/bot.html)", "Bot user agent"),
            ("curl/7.68.0", "cURL"),
            ("Python/3.9 aiohttp/3.8.1", "Python client"),
            ("🚀 Unicode Client 1.0 🔥", "Unicode in user agent"),
            (long_agent.as_str(), "Very long user agent"),
            ("Client\0With\0Nulls", "Null bytes in user agent"),
        ];

        for (user_agent, description) in test_cases {
            let client = reqwest::Client::new();
            let response = client
                .post(&server.http_url())
                .header(USER_AGENT, user_agent)
                .json(&serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "initialize",
                    "params": {
                        "protocolVersion": "2025-06-18",
                        "capabilities": {},
                        "clientInfo": {"name": "test", "version": "1.0"}
                    }
                }))
                .send()
                .await?;

            assert!(
                response.status().is_success(),
                "Failed for user agent: {}",
                description
            );
        }

        Ok(())
    })
    .await
    .unwrap();
}

/// Test 4: RED - Mixed transport signals
#[tokio::test]
async fn test_mixed_transport_signals() {
    // Test requests that send mixed signals about desired transport
    with_mcp_test_server("mixed_signals_test", |server| async move {
        // POST with WebSocket headers (conflicting signals)
        let client = reqwest::Client::new();
        let response = client
            .post(&server.http_url())
            .header(UPGRADE, "websocket")
            .header(CONNECTION, "upgrade")
            .header(CONTENT_TYPE, "application/json")
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "test"
            }))
            .send()
            .await?;

        // Should prefer POST body over WebSocket headers
        assert_eq!(response.status(), 200, "Should handle POST with WS headers");

        // GET with JSON accept (should provide info, not error)
        let response = client
            .get(&server.http_url())
            .header(ACCEPT, "application/json")
            .send()
            .await?;

        assert_eq!(response.status(), 200, "Should provide info for GET");
        let body: serde_json::Value = response.json().await?;
        assert!(body.get("mcp_server").is_some(), "Should return server info");

        Ok(())
    })
    .await
    .unwrap();
}

/// Test 5: RED - Custom protocol version headers
#[tokio::test]
async fn test_custom_protocol_headers() {
    // Test custom MCP protocol version headers
    with_mcp_test_server("custom_protocol_headers_test", |server| async move {
        let test_cases = vec![
            ("2025-06-18", "Valid version"),
            ("2025-03-26", "Another valid version"),
            ("", "Empty version"),
            ("invalid-version", "Invalid format"),
            ("2099-99-99", "Future version"),
            ("v2.0", "Prefixed version"),
        ];

        for (version, description) in test_cases {
            let client = reqwest::Client::new();
            let response = client
                .post(&server.http_url())
                .header("x-mcp-protocol-version", version)
                .json(&serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "initialize",
                    "params": {
                        "protocolVersion": "2025-06-18",
                        "capabilities": {},
                        "clientInfo": {"name": "test", "version": "1.0"}
                    }
                }))
                .send()
                .await?;

            assert!(
                response.status().is_success(),
                "Failed for version header: {}",
                description
            );
        }

        Ok(())
    })
    .await
    .unwrap();
}

/// Test 6: RED - OPTIONS method handling
#[tokio::test]
async fn test_options_method_handling() {
    // Test OPTIONS method for CORS and capability discovery
    with_mcp_test_server("options_method_test", |server| async move {
        let client = reqwest::Client::new();
        
        // Basic OPTIONS request
        let response = client
            .request(reqwest::Method::OPTIONS, &server.http_url())
            .send()
            .await?;

        assert_eq!(response.status(), 200);
        
        // Check CORS headers
        let headers = response.headers();
        assert!(headers.contains_key("access-control-allow-origin"));
        assert!(headers.contains_key("access-control-allow-methods"));
        assert!(headers.contains_key("access-control-allow-headers"));

        // OPTIONS with WebSocket intent
        let response = client
            .request(reqwest::Method::OPTIONS, &server.http_url())
            .header(UPGRADE, "websocket")
            .header(CONNECTION, "upgrade")
            .send()
            .await?;

        assert_eq!(response.status(), 200);
        let body: serde_json::Value = response.json().await?;
        assert!(body.get("mcp_server").is_some());

        Ok(())
    })
    .await
    .unwrap();
}

/// Test 7: RED - Unsupported HTTP methods
#[tokio::test]
async fn test_unsupported_http_methods() {
    // Test handling of unsupported HTTP methods
    with_mcp_test_server("unsupported_methods_test", |server| async move {
        let methods = vec![
            reqwest::Method::PUT,
            reqwest::Method::DELETE,
            reqwest::Method::PATCH,
            reqwest::Method::HEAD,
            reqwest::Method::TRACE,
            reqwest::Method::CONNECT,
        ];

        let client = reqwest::Client::new();

        for method in methods {
            let response = client
                .request(method.clone(), &server.http_url())
                .send()
                .await?;

            // Should either return 405 Method Not Allowed or 200 with error info
            assert!(
                response.status() == 405 || response.status() == 200,
                "Unexpected status {} for method {}",
                response.status(),
                method
            );

            if response.status() == 200 {
                // If 200, should contain error information
                let body: serde_json::Value = response.json().await?;
                assert!(
                    body.get("error").is_some() || body.get("mcp_server").is_some(),
                    "Should have error or info for unsupported method"
                );
            }
        }

        Ok(())
    })
    .await
    .unwrap();
}

/// Test 8: RED - Header size limits
#[tokio::test]
async fn test_header_size_limits() {
    // Test extremely large headers
    with_mcp_test_server("header_size_test", |server| async move {
        // Create a very large header value (8KB)
        let large_value = "x".repeat(8192);
        
        let client = reqwest::Client::new();
        let response = client
            .post(&server.http_url())
            .header("x-custom-large-header", large_value)
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "test"
            }))
            .send()
            .await;

        // Should either accept or reject gracefully
        match response {
            Ok(resp) => {
                assert!(
                    resp.status().is_success() || resp.status() == 431,
                    "Should either process or reject with 431"
                );
            }
            Err(_) => {
                // Connection error is acceptable for oversized headers
                assert!(true, "Connection rejected oversized headers");
            }
        }

        Ok(())
    })
    .await
    .unwrap();
}