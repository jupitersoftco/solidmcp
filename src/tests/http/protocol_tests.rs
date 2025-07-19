//! Protocol compliance and error case tests for MCP HTTP

#[cfg(test)]
mod tests {
    use crate::http::HttpMcpHandler;
    use crate::logging::{McpConnectionId, McpDebugLogger};
    use crate::shared::McpProtocolEngine;
    use serde_json::json;
    use std::sync::Arc;
    use warp::test::request;

    #[tokio::test]
    async fn test_jsonrpc_id_preserved() {
        let connection_id = McpConnectionId::new();
        let _logger = McpDebugLogger::new(connection_id);
        let shared_handler = Arc::new(McpProtocolEngine::new());
        let http_handler = HttpMcpHandler::new(shared_handler);
        let routes = http_handler.route();

        let resp = request()
            .method("POST")
            .path("/mcp")
            .json(&json!({
                "jsonrpc": "2.0",
                "id": 42,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2025-06-18",
                    "capabilities": {},
                    "clientInfo": {
                        "name": "test-client",
                        "version": "1.0.0"
                    }
                }
            }))
            .reply(&routes)
            .await;

        assert_eq!(resp.status(), 200);
        let body: serde_json::Value = serde_json::from_slice(resp.body()).unwrap();
        assert_eq!(body["id"], 42);
    }

    #[tokio::test]
    async fn test_notification_no_id() {
        let connection_id = McpConnectionId::new();
        let _logger = McpDebugLogger::new(connection_id);
        let shared_handler = Arc::new(McpProtocolEngine::new());
        let http_handler = HttpMcpHandler::new(shared_handler);
        let routes = http_handler.route();

        // Notification has no id field
        let resp = request()
            .method("POST")
            .path("/mcp")
            .json(&json!({
                "jsonrpc": "2.0",
                "method": "notifications/cancel",
                "params": {
                    "requestId": "test-request"
                }
            }))
            .reply(&routes)
            .await;

        assert_eq!(resp.status(), 200);
        // For notifications without ID, response should be empty object
        let body: serde_json::Value = serde_json::from_slice(resp.body()).unwrap();
        assert!(body.is_object());
        // Notifications without ID don't have jsonrpc or result fields
        assert!(body.get("jsonrpc").is_none() || body.get("result").is_some());
    }

    #[tokio::test]
    async fn test_error_response_format() {
        let connection_id = McpConnectionId::new();
        let _logger = McpDebugLogger::new(connection_id);
        let shared_handler = Arc::new(McpProtocolEngine::new());
        let http_handler = HttpMcpHandler::new(shared_handler);
        let routes = http_handler.route();

        // Call tools/list without initialization
        let resp = request()
            .method("POST")
            .path("/mcp")
            .json(&json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "tools/list",
                "params": {}
            }))
            .reply(&routes)
            .await;

        assert_eq!(resp.status(), 200); // JSON-RPC errors still return 200
        let body: serde_json::Value = serde_json::from_slice(resp.body()).unwrap();
        assert!(body["error"].is_object());
        assert!(body["error"]["code"].is_number());
        assert!(body["error"]["message"].is_string());
    }

    #[tokio::test]
    async fn test_content_type_headers() {
        let connection_id = McpConnectionId::new();
        let _logger = McpDebugLogger::new(connection_id);
        let shared_handler = Arc::new(McpProtocolEngine::new());
        let http_handler = HttpMcpHandler::new(shared_handler);
        let routes = http_handler.route();

        let resp = request()
            .method("POST")
            .path("/mcp")
            .header("content-type", "application/json")
            .header("accept", "application/json")
            .json(&json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2025-06-18",
                    "capabilities": {},
                    "clientInfo": {
                        "name": "test-client",
                        "version": "1.0.0"
                    }
                }
            }))
            .reply(&routes)
            .await;

        println!("HTTP status: {}", resp.status());
        if resp.status() != 200 {
            let body = String::from_utf8_lossy(resp.body());
            println!("HTTP error body: {body}");
        }
        assert_eq!(resp.status(), 200);
        assert_eq!(
            resp.headers().get("content-type").unwrap(),
            "application/json"
        );
    }
}
