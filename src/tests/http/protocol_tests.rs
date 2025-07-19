//! Protocol compliance and error case tests for MCP HTTP

#[cfg(test)]
mod tests {
    use crate::http::{create_mcp_routes, McpHttpConfig};
    use warp::test::request;
    use serde_json::json;

    #[tokio::test]
    async fn test_jsonrpc_id_preserved() {
        let config = McpHttpConfig::default();
        let routes = create_mcp_routes(config);
        
        let resp = request()
            .method("POST")
            .path("/mcp/v1/messages")
            .json(&json!({
                "jsonrpc": "2.0",
                "id": 42,
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
        let body: serde_json::Value = serde_json::from_slice(resp.body()).unwrap();
        assert_eq!(body["id"], 42);
    }

    #[tokio::test]
    async fn test_notification_no_response() {
        let config = McpHttpConfig::default();
        let routes = create_mcp_routes(config);
        
        // Notification has no id field
        let resp = request()
            .method("POST")
            .path("/mcp/v1/messages")
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
        // Response should still have result structure
        let body: serde_json::Value = serde_json::from_slice(resp.body()).unwrap();
        assert!(body["result"].is_object());
    }

    #[tokio::test]
    async fn test_error_response_format() {
        let config = McpHttpConfig::default();
        let routes = create_mcp_routes(config);
        
        // Call tools/list without initialization
        let resp = request()
            .method("POST")
            .path("/mcp/v1/messages")
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
}