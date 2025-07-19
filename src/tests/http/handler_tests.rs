//! Basic handler tests for MCP HTTP

#[cfg(test)]
mod tests {
    use crate::http::{create_mcp_routes, McpHttpConfig};
    use warp::test::request;
    use serde_json::json;

    #[tokio::test]
    async fn test_http_endpoint_exists() {
        let config = McpHttpConfig::default();
        let routes = create_mcp_routes(config);
        
        let resp = request()
            .method("POST")
            .path("/mcp/v1/messages")
            .json(&json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2024-11-05",
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
        let config = McpHttpConfig::default();
        let routes = create_mcp_routes(config);
        
        let resp = request()
            .method("POST")
            .path("/mcp/v1/messages")
            .body("invalid json")
            .header("content-type", "application/json")
            .reply(&routes)
            .await;
        
        assert_eq!(resp.status(), 400);
    }

    #[tokio::test]
    async fn test_cors_headers() {
        let config = McpHttpConfig::default();
        let routes = create_mcp_routes(config);
        
        let resp = request()
            .method("OPTIONS")
            .path("/mcp/v1/messages")
            .header("origin", "http://localhost:3000")
            .reply(&routes)
            .await;
        
        assert_eq!(resp.status(), 200);
        assert!(resp.headers().contains_key("access-control-allow-origin"));
        assert!(resp.headers().contains_key("access-control-allow-methods"));
    }
}