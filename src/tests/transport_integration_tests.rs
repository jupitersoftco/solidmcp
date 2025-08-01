//! Comprehensive integration tests for transport capability detection and HTTP enhancements

#[cfg(test)]
mod tests {
    use crate::http_handler::HttpMcpHandler;
    use crate::logging::{McpConnectionId, McpDebugLogger};
    use crate::shared::McpProtocolEngine;
    use crate::transport::{
        cors_headers, TransportCapabilities, TransportInfo, TransportNegotiation, TransportType,
    };
    use serde_json::json;
    use std::sync::Arc;
    use warp::http::{HeaderMap, HeaderValue, StatusCode};
    use warp::test::request;

    #[tokio::test]
    async fn test_cors_options_request() {
        let connection_id = McpConnectionId::new();
        let _logger = McpDebugLogger::new(connection_id);
        let shared_handler = Arc::new(McpProtocolEngine::new());
        let http_handler = HttpMcpHandler::new(shared_handler);
        let routes = http_handler.route();

        let resp = request()
            .method("OPTIONS")
            .path("/mcp")
            .header("origin", "https://example.com")
            .header("access-control-request-method", "POST")
            .header("access-control-request-headers", "content-type")
            .reply(&routes)
            .await;

        assert_eq!(resp.status(), StatusCode::OK);

        // Check CORS headers
        let headers = resp.headers();
        assert_eq!(headers.get("access-control-allow-origin").unwrap(), "*");
        assert_eq!(
            headers.get("access-control-allow-methods").unwrap(),
            "GET, POST, OPTIONS"
        );
        assert!(headers.get("access-control-allow-headers").is_some());
        assert_eq!(headers.get("access-control-max-age").unwrap(), "3600");

        // Response should contain transport capabilities
        let body: serde_json::Value = serde_json::from_slice(resp.body()).unwrap();
        assert!(body["mcp_server"].is_object());
        assert!(body["mcp_server"]["available_transports"].is_object());
    }

    #[tokio::test]
    async fn test_websocket_client_transport_detection() {
        let connection_id = McpConnectionId::new();
        let _logger = McpDebugLogger::new(connection_id);
        let shared_handler = Arc::new(McpProtocolEngine::new());
        let http_handler = HttpMcpHandler::new(shared_handler);
        let routes = http_handler.route();

        // Simulate WebSocket upgrade request headers
        let resp = request()
            .method("GET")
            .path("/mcp")
            .header("upgrade", "websocket")
            .header("connection", "upgrade")
            .header("sec-websocket-key", "x3JJHMbDL1EzLkh9GBhXDw==")
            .header("sec-websocket-version", "13")
            .header("user-agent", "Claude Desktop/1.0")
            .reply(&routes)
            .await;

        // Should return transport discovery rather than upgrading in this test context
        assert_eq!(resp.status(), StatusCode::OK);

        let body: serde_json::Value = serde_json::from_slice(resp.body()).unwrap();
        assert!(body["mcp_server"]["available_transports"]["websocket"].is_object());

        // Should indicate WebSocket preference
        let transport_info = body["mcp_server"]["available_transports"]["websocket"]
            .as_object()
            .unwrap();
        assert_eq!(transport_info["uri"].as_str().unwrap(), "ws://unknown/mcp");
    }

    #[tokio::test]
    async fn test_sse_client_fallback_to_http() {
        let connection_id = McpConnectionId::new();
        let _logger = McpDebugLogger::new(connection_id);
        let shared_handler = Arc::new(McpProtocolEngine::new());
        let http_handler = HttpMcpHandler::new(shared_handler);
        let routes = http_handler.route();

        // Simulate SSE-capable client
        let resp = request()
            .method("GET")
            .path("/mcp")
            .header("accept", "text/event-stream")
            .header("cache-control", "no-cache")
            .header("user-agent", "EventSource/1.0")
            .reply(&routes)
            .await;

        assert_eq!(resp.status(), StatusCode::OK);

        let body: serde_json::Value = serde_json::from_slice(resp.body()).unwrap();

        // Should fall back to HTTP since SSE is disabled
        assert!(body["mcp_server"]["available_transports"]["http"].is_object());
        // Should NOT advertise SSE capability
        assert!(!body["mcp_server"]["available_transports"]
            .as_object()
            .unwrap()
            .contains_key("sse"));
    }

    #[tokio::test]
    async fn test_curl_http_client_detection() {
        let connection_id = McpConnectionId::new();
        let _logger = McpDebugLogger::new(connection_id);
        let shared_handler = Arc::new(McpProtocolEngine::new());
        let http_handler = HttpMcpHandler::new(shared_handler);
        let routes = http_handler.route();

        // Simulate curl client
        let resp = request()
            .method("GET")
            .path("/mcp")
            .header("user-agent", "curl/7.68.0")
            .header("accept", "*/*")
            .reply(&routes)
            .await;

        assert_eq!(resp.status(), StatusCode::OK);

        let body: serde_json::Value = serde_json::from_slice(resp.body()).unwrap();

        // Should provide HTTP transport information
        let http_transport = &body["mcp_server"]["available_transports"]["http"];
        assert!(http_transport.is_object());
        assert_eq!(
            http_transport["uri"].as_str().unwrap(),
            "http://unknown/mcp"
        );
        assert_eq!(http_transport["method"].as_str().unwrap(), "POST");
    }

    #[tokio::test]
    async fn test_post_request_with_cors_headers() {
        let connection_id = McpConnectionId::new();
        let _logger = McpDebugLogger::new(connection_id);
        let shared_handler = Arc::new(McpProtocolEngine::new());
        let http_handler = HttpMcpHandler::new(shared_handler);
        let routes = http_handler.route();

        let resp = request()
            .method("POST")
            .path("/mcp")
            .header("origin", "https://web-mcp-client.com")
            .json(&json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2025-06-18",
                    "capabilities": {},
                    "clientInfo": {
                        "name": "web-client",
                        "version": "1.0.0"
                    }
                }
            }))
            .reply(&routes)
            .await;

        assert_eq!(resp.status(), StatusCode::OK);

        // Verify CORS headers are present in POST responses
        let headers = resp.headers();
        assert_eq!(headers.get("access-control-allow-origin").unwrap(), "*");

        // Verify MCP response
        let body: serde_json::Value = serde_json::from_slice(resp.body()).unwrap();
        assert_eq!(body["jsonrpc"], "2.0");
        assert_eq!(body["id"], 1);
        assert!(body["result"].is_object());
    }

    #[test]
    fn test_transport_capability_edge_cases() {
        // Test empty headers
        let empty_headers = HeaderMap::new();
        let capabilities = TransportCapabilities::from_headers(&empty_headers);
        assert!(!capabilities.supports_websocket);
        assert!(!capabilities.supports_sse);
        assert!(capabilities.supports_http_only);
        assert_eq!(capabilities.preferred_transport(), TransportType::HttpOnly);

        // Test case-insensitive header handling
        let mut headers = HeaderMap::new();
        headers.insert("UPGRADE", HeaderValue::from_static("WEBSOCKET"));
        headers.insert("CONNECTION", HeaderValue::from_static("UPGRADE"));
        let capabilities = TransportCapabilities::from_headers(&headers);
        assert!(capabilities.supports_websocket);

        // Test partial WebSocket headers (missing connection upgrade)
        let mut headers = HeaderMap::new();
        headers.insert("upgrade", HeaderValue::from_static("websocket"));
        let capabilities = TransportCapabilities::from_headers(&headers);
        assert!(!capabilities.supports_websocket); // Requires both headers

        // Test various user agent strings
        let mut headers = HeaderMap::new();
        headers.insert(
            "user-agent",
            HeaderValue::from_static("Mozilla/5.0 (compatible; MCP Client)"),
        );
        let capabilities = TransportCapabilities::from_headers(&headers);
        assert_eq!(
            capabilities.client_info,
            Some("Mozilla/5.0 (compatible; MCP Client)".to_string())
        );
    }

    #[test]
    fn test_transport_negotiation_scenarios() {
        // WebSocket-capable client making GET request
        let ws_capabilities = TransportCapabilities {
            supports_websocket: true,
            supports_sse: false,
            supports_http_only: true,
            client_info: Some("ws-client".to_string()),
            protocol_version: None,
        };

        let result =
            TransportNegotiation::negotiate("GET", &ws_capabilities, false, "test", "1.0", "/mcp");
        assert!(matches!(result, TransportNegotiation::WebSocketUpgrade));

        // HTTP-only client making POST request
        let http_capabilities = TransportCapabilities {
            supports_websocket: false,
            supports_sse: false,
            supports_http_only: true,
            client_info: Some("http-client".to_string()),
            protocol_version: None,
        };

        let result = TransportNegotiation::negotiate(
            "POST",
            &http_capabilities,
            true,
            "test",
            "1.0",
            "/mcp",
        );
        assert!(matches!(result, TransportNegotiation::HttpJsonRpc));

        // OPTIONS request should return info response
        let result = TransportNegotiation::negotiate(
            "OPTIONS",
            &ws_capabilities,
            false,
            "test",
            "1.0",
            "/mcp",
        );
        assert!(matches!(result, TransportNegotiation::InfoResponse(_)));
    }

    #[test]
    fn test_transport_info_serialization() {
        let capabilities = TransportCapabilities {
            supports_websocket: true,
            supports_sse: false,
            supports_http_only: true,
            client_info: Some("test-client".to_string()),
            protocol_version: Some("2025-06-18".to_string()),
        };

        let info = TransportInfo::new(&capabilities, "test-server", "1.0.0", "/mcp");
        let json = info.to_json();

        // Verify structure
        assert_eq!(json["mcp_server"]["name"], "test-server");
        assert_eq!(json["mcp_server"]["version"], "1.0.0");
        assert!(json["mcp_server"]["available_transports"]["websocket"].is_object());
        assert!(json["mcp_server"]["available_transports"]["http"].is_object());

        // Verify no SSE transport is advertised
        assert!(!json["mcp_server"]["available_transports"]
            .as_object()
            .unwrap()
            .contains_key("sse"));

        // Verify WebSocket transport details
        let ws_transport = &json["mcp_server"]["available_transports"]["websocket"];
        assert_eq!(ws_transport["type"], "websocket");
        assert!(ws_transport["uri"].as_str().unwrap().starts_with("ws://"));

        // Verify HTTP transport details
        let http_transport = &json["mcp_server"]["available_transports"]["http"];
        assert_eq!(http_transport["type"], "http");
        assert_eq!(http_transport["method"], "POST");
        assert!(http_transport["uri"]
            .as_str()
            .unwrap()
            .starts_with("http://"));
    }

    #[test]
    fn test_cors_headers_generation() {
        let headers = cors_headers();

        assert_eq!(headers.get("access-control-allow-origin").unwrap(), "*");
        assert_eq!(
            headers.get("access-control-allow-methods").unwrap(),
            "GET, POST, OPTIONS"
        );
        assert!(headers
            .get("access-control-allow-headers")
            .unwrap()
            .to_str()
            .unwrap()
            .contains("content-type"));
        assert_eq!(headers.get("access-control-max-age").unwrap(), "3600");
    }

    #[tokio::test]
    async fn test_malformed_websocket_upgrade_handling() {
        let connection_id = McpConnectionId::new();
        let _logger = McpDebugLogger::new(connection_id);
        let shared_handler = Arc::new(McpProtocolEngine::new());
        let http_handler = HttpMcpHandler::new(shared_handler);
        let routes = http_handler.route();

        // Malformed WebSocket upgrade (missing required headers)
        let resp = request()
            .method("GET")
            .path("/mcp")
            .header("upgrade", "websocket")
            // Missing connection: upgrade header
            .header("user-agent", "broken-ws-client")
            .reply(&routes)
            .await;

        // Should still return transport discovery, not attempt upgrade
        assert_eq!(resp.status(), StatusCode::OK);

        let body: serde_json::Value = serde_json::from_slice(resp.body()).unwrap();
        assert!(body["mcp_server"]["available_transports"]["websocket"].is_object());
    }

    #[tokio::test]
    async fn test_multiple_origin_cors_handling() {
        let connection_id = McpConnectionId::new();
        let _logger = McpDebugLogger::new(connection_id);
        let shared_handler = Arc::new(McpProtocolEngine::new());
        let http_handler = HttpMcpHandler::new(shared_handler);
        let routes = http_handler.route();

        // Test various origins
        for origin in &[
            "https://example.com",
            "http://localhost:3000",
            "https://web-mcp.app",
        ] {
            let resp = request()
                .method("OPTIONS")
                .path("/mcp")
                .header("origin", *origin)
                .reply(&routes)
                .await;

            assert_eq!(resp.status(), StatusCode::OK);

            // Should accept all origins with *
            let headers = resp.headers();
            assert_eq!(headers.get("access-control-allow-origin").unwrap(), "*");
        }
    }

    #[tokio::test]
    async fn test_client_info_extraction_and_logging() {
        let connection_id = McpConnectionId::new();
        let _logger = McpDebugLogger::new(connection_id);
        let shared_handler = Arc::new(McpProtocolEngine::new());
        let http_handler = HttpMcpHandler::new(shared_handler);
        let routes = http_handler.route();

        // Test with detailed user agent
        let resp = request()
            .method("GET")
            .path("/mcp")
            .header("user-agent", "Cursor/0.42.3 (Claude Desktop; macOS 14.0)")
            .header("x-mcp-protocol-version", "2025-06-18")
            .reply(&routes)
            .await;

        assert_eq!(resp.status(), StatusCode::OK);

        let body: serde_json::Value = serde_json::from_slice(resp.body()).unwrap();

        // Transport info should be returned regardless of client
        assert!(body["mcp_server"]["available_transports"].is_object());

        // The client info is used internally for capability detection
        // (we can't easily test the internal logging here, but the request should succeed)
    }
}
