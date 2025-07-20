//! Session Management Unit Tests
//!
//! Tests for session creation, isolation, and lifecycle

#[cfg(test)]
mod tests {
    use crate::handler::{McpContext, McpHandler, ToolDefinition};
    use crate::shared::McpProtocolEngine;
    use anyhow::Result;
    use async_trait::async_trait;
    use serde_json::{json, Value};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    /// Mock handler for testing session behavior
    struct MockHandler {
        call_count: AtomicUsize,
        session_ids_seen: Arc<tokio::sync::Mutex<Vec<Option<String>>>>,
    }

    #[async_trait]
    impl McpHandler for MockHandler {
        async fn initialize(&self, _params: Value, context: &McpContext) -> Result<Value> {
            // Track session IDs
            let mut sessions = self.session_ids_seen.lock().await;
            sessions.push(context.session_id.clone());

            Ok(json!({
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "serverInfo": {
                    "name": "mock-server",
                    "version": "1.0.0"
                }
            }))
        }

        async fn list_tools(&self, _context: &McpContext) -> Result<Vec<ToolDefinition>> {
            self.call_count.fetch_add(1, Ordering::Relaxed);
            Ok(vec![])
        }

        async fn call_tool(
            &self,
            _name: &str,
            _arguments: Value,
            _context: &McpContext,
        ) -> Result<Value> {
            Ok(json!({"result": "ok"}))
        }
    }

    /// Test session isolation between clients
    #[tokio::test]
    async fn test_session_isolation() {
        let handler = Arc::new(MockHandler {
            call_count: AtomicUsize::new(0),
            session_ids_seen: Arc::new(tokio::sync::Mutex::new(Vec::new())),
        });

        let engine = McpProtocolEngine::with_handler(handler.clone());

        // Client 1 initializes
        let init1 = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18"
            }
        });

        let result1 = engine
            .handle_message(init1, Some("session1".to_string()))
            .await
            .unwrap();
        assert!(result1["result"].is_object());

        // Client 2 initializes with different session
        let init2 = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18"
            }
        });

        let result2 = engine
            .handle_message(init2, Some("session2".to_string()))
            .await
            .unwrap();
        assert!(result2["result"].is_object());

        // Try to use tools from session1 - should work
        let tools1 = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        });

        let result = engine
            .handle_message(tools1, Some("session1".to_string()))
            .await
            .unwrap();
        assert!(result["result"].is_object());

        // Try to use tools from session2 without init - should also work (already initialized)
        let tools2 = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        });

        let result = engine
            .handle_message(tools2, Some("session2".to_string()))
            .await
            .unwrap();
        assert!(result["result"].is_object());

        // Verify both sessions were tracked
        let sessions = handler.session_ids_seen.lock().await;
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0], Some("session1".to_string()));
        assert_eq!(sessions[1], Some("session2".to_string()));
    }

    /// Test default session handling
    #[tokio::test]
    async fn test_default_session() {
        let handler = Arc::new(MockHandler {
            call_count: AtomicUsize::new(0),
            session_ids_seen: Arc::new(tokio::sync::Mutex::new(Vec::new())),
        });

        let engine = McpProtocolEngine::with_handler(handler);

        // Initialize without session ID
        let init = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18"
            }
        });

        let result = engine.handle_message(init, None).await.unwrap();
        assert!(result["result"].is_object());

        // Should be able to use tools with no session ID
        let tools = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        });

        let result = engine.handle_message(tools, None).await.unwrap();
        assert!(result["result"].is_object());
    }

    /// Test session state persistence
    #[tokio::test]
    async fn test_session_state_persistence() {
        let handler = Arc::new(MockHandler {
            call_count: AtomicUsize::new(0),
            session_ids_seen: Arc::new(tokio::sync::Mutex::new(Vec::new())),
        });

        let engine = McpProtocolEngine::with_handler(handler.clone());

        let session_id = "persistent-session";

        // Initialize
        let init = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "clientInfo": {
                    "name": "test-client",
                    "version": "1.0.0"
                }
            }
        });

        engine
            .handle_message(init, Some(session_id.to_string()))
            .await
            .unwrap();

        // Make multiple requests with same session
        for i in 2..10 {
            let request = json!({
                "jsonrpc": "2.0",
                "id": i,
                "method": "tools/list",
                "params": {}
            });

            let result = engine
                .handle_message(request, Some(session_id.to_string()))
                .await
                .unwrap();
            assert!(result["result"].is_object());
        }

        // Verify handler was called correct number of times
        assert_eq!(handler.call_count.load(Ordering::Relaxed), 8); // 8 tools/list calls
    }

    /// Test concurrent session access
    #[tokio::test]
    async fn test_concurrent_session_access() {
        let handler = Arc::new(MockHandler {
            call_count: AtomicUsize::new(0),
            session_ids_seen: Arc::new(tokio::sync::Mutex::new(Vec::new())),
        });

        let engine = Arc::new(McpProtocolEngine::with_handler(handler.clone()));

        // Initialize multiple sessions
        let mut init_handles = vec![];
        for i in 0..10 {
            let engine = engine.clone();
            let session_id = format!("session-{i}");

            let handle = tokio::spawn(async move {
                let init = json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "initialize",
                    "params": {
                        "protocolVersion": "2025-06-18"
                    }
                });

                engine.handle_message(init, Some(session_id)).await
            });

            init_handles.push(handle);
        }

        // Wait for all initializations
        for handle in init_handles {
            let result = handle.await.unwrap().unwrap();
            assert!(result["result"].is_object());
        }

        // Make concurrent requests across sessions
        let mut request_handles = vec![];
        for i in 0..10 {
            for j in 0..5 {
                let engine = engine.clone();
                let session_id = format!("session-{i}");

                let handle = tokio::spawn(async move {
                    let request = json!({
                        "jsonrpc": "2.0",
                        "id": j + 2,
                        "method": "tools/list",
                        "params": {}
                    });

                    engine.handle_message(request, Some(session_id)).await
                });

                request_handles.push(handle);
            }
        }

        // Wait for all requests
        for handle in request_handles {
            let result = handle.await.unwrap().unwrap();
            assert!(result["result"].is_object());
        }

        // Verify correct number of calls
        assert_eq!(handler.call_count.load(Ordering::Relaxed), 50); // 10 sessions * 5 requests
    }

    /// Test session cleanup (sessions should be reusable)
    #[tokio::test]
    async fn test_session_reuse() {
        let handler = Arc::new(MockHandler {
            call_count: AtomicUsize::new(0),
            session_ids_seen: Arc::new(tokio::sync::Mutex::new(Vec::new())),
        });

        let engine = McpProtocolEngine::with_handler(handler);

        let session_id = "reusable-session";

        // First use of session
        let init1 = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18"
            }
        });

        let result = engine
            .handle_message(init1, Some(session_id.to_string()))
            .await
            .unwrap();
        assert!(result["result"].is_object());

        // Try to reinitialize same session (should succeed with graceful re-initialization)
        let init2 = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18"
            }
        });

        let result = engine
            .handle_message(init2, Some(session_id.to_string()))
            .await
            .unwrap();
        assert!(result["result"].is_object()); // Should succeed with re-initialization

        // But can still use the session
        let tools = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/list",
            "params": {}
        });

        let result = engine
            .handle_message(tools, Some(session_id.to_string()))
            .await
            .unwrap();
        assert!(result["result"].is_object());
    }

    /// Test session ID validation
    #[tokio::test]
    async fn test_session_id_validation() {
        let handler = Arc::new(MockHandler {
            call_count: AtomicUsize::new(0),
            session_ids_seen: Arc::new(tokio::sync::Mutex::new(Vec::new())),
        });

        let engine = McpProtocolEngine::with_handler(handler);

        // Test with various session ID formats
        let session_ids = vec![
            "simple-id",
            "with-numbers-123",
            "with_underscores",
            "with.dots",
            "very-long-session-id-that-should-still-work-fine-even-though-its-quite-lengthy",
            "UTF8-测试-тест-🔥", // Unicode session IDs
        ];

        for session_id in session_ids {
            let init = json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2025-06-18"
                }
            });

            let result = engine
                .handle_message(init, Some(session_id.to_string()))
                .await
                .unwrap();
            assert!(
                result["result"].is_object(),
                "Failed for session ID: {session_id}"
            );
        }
    }
}
