//! Session and cookie isolation tests for MCP HTTP

#[cfg(test)]
mod tests {
    use crate::http::session::SessionManager;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_session_creation() {
        let manager = SessionManager::new();
        let session_id = Uuid::new_v4().to_string();
        
        let session = manager.get_or_create_session(&session_id).await;
        assert_eq!(session.id, session_id);
        assert!(!session.is_initialized);
    }

    #[tokio::test]
    async fn test_session_isolation() {
        let manager = SessionManager::new();
        
        let session1_id = Uuid::new_v4().to_string();
        let session2_id = Uuid::new_v4().to_string();
        
        let session1 = manager.get_or_create_session(&session1_id).await;
        let session2 = manager.get_or_create_session(&session2_id).await;
        
        // Sessions should be different instances
        assert_ne!(session1.id, session2.id);
    }

    #[tokio::test]
    async fn test_session_persistence() {
        let manager = SessionManager::new();
        let session_id = Uuid::new_v4().to_string();
        
        // Create session
        {
            let mut session = manager.get_or_create_session(&session_id).await;
            session.is_initialized = true;
        }
        
        // Retrieve same session
        {
            let session = manager.get_or_create_session(&session_id).await;
            assert!(session.is_initialized);
        }
    }
}