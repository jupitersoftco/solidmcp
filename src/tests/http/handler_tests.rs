//! Basic handler tests for MCP HTTP

#[cfg(test)]
mod tests {
    use crate::http::HttpMcpHandler;
    use crate::logging::{McpConnectionId, McpDebugLogger};
    use crate::shared::SharedMcpHandler;
    use serde_json::json;
    use std::sync::Arc;
    use warp::test::request;

    #[tokio::test]
    async fn test_http_endpoint_exists() {
        let connection_id = McpConnectionId::new();
        let _logger = McpDebugLogger::new(connection_id);
        let shared_handler = Arc::new(SharedMcpHandler::new());
        let http_handler = HttpMcpHandler::new(shared_handler);
        let routes = http_handler.route();

        let resp = request()
            .method("POST")
            .path("/mcp")
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

        assert_eq!(resp.status(), 200);
    }

    #[tokio::test]
    async fn test_invalid_json_returns_error() {
        let connection_id = McpConnectionId::new();
        let _logger = McpDebugLogger::new(connection_id);
        let shared_handler = Arc::new(SharedMcpHandler::new());
        let http_handler = HttpMcpHandler::new(shared_handler);
        let routes = http_handler.route();

        let resp = request()
            .method("POST")
            .path("/mcp")
            .body("invalid json")
            .header("content-type", "application/json")
            .reply(&routes)
            .await;

        // Warp will reject invalid JSON before it reaches our handler
        assert_ne!(resp.status(), 200);
    }

    #[tokio::test]
    async fn test_get_method_not_allowed() {
        let connection_id = McpConnectionId::new();
        let _logger = McpDebugLogger::new(connection_id);
        let shared_handler = Arc::new(SharedMcpHandler::new());
        let http_handler = HttpMcpHandler::new(shared_handler);
        let routes = http_handler.route();

        let resp = request().method("GET").path("/mcp").reply(&routes).await;

        assert_eq!(resp.status(), 405); // Method Not Allowed
    }
}
