//! HTTP Progress Notification Handling
//!
//! Functions for handling progress notifications in HTTP requests.

use serde_json::Value;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, warn};

/// Handler for progress notifications
pub struct ProgressHandler {
    sender: mpsc::UnboundedSender<Value>,
}

impl ProgressHandler {
    /// Create a new progress handler
    pub fn new() -> Self {
        let (sender, _receiver) = mpsc::unbounded_channel();
        Self { sender }
    }
    
    /// Get the sender for passing to processing functions
    pub fn sender(&self) -> mpsc::UnboundedSender<Value> {
        self.sender.clone()
    }
    
}


#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    
    #[tokio::test]
    async fn test_progress_handler_basic() {
        let mut handler = ProgressHandler::new();
        let sender = handler.sender();
        
        // Send some notifications
        sender.send(json!({"progress": 0.5})).unwrap();
        sender.send(json!({"progress": 1.0})).unwrap();
        
        // Drop sender to close channel
        drop(sender);
        
        // Collect notifications
        let notifications = handler.handle_with_timeout(Duration::from_secs(1)).await;
        
        assert_eq!(notifications.len(), 2);
        assert_eq!(notifications[0]["progress"], 0.5);
        assert_eq!(notifications[1]["progress"], 1.0);
    }
    
    #[tokio::test]
    async fn test_progress_handler_timeout() {
        let handler = ProgressHandler::new();
        let _sender = handler.sender(); // Keep sender alive
        
        // Should timeout after 100ms
        let notifications = handler.handle_with_timeout(Duration::from_millis(100)).await;
        
        assert_eq!(notifications.len(), 0);
    }
    
    #[test]
    fn test_progress_handler_try_receive() {
        let mut handler = ProgressHandler::new();
        let sender = handler.sender();
        
        // Send notifications
        sender.send(json!({"progress": 0.25})).unwrap();
        sender.send(json!({"progress": 0.75})).unwrap();
        
        // Try receive all
        let notifications = handler.try_receive_all();
        
        assert_eq!(notifications.len(), 2);
    }
    
    #[test]
    fn test_has_progress_token() {
        let with_token = json!({
            "params": {
                "_meta": {
                    "progressToken": "abc123"
                }
            }
        });
        
        let without_token = json!({
            "params": {
                "tool": "test"
            }
        });
        
        assert!(has_progress_token(&with_token));
        assert!(!has_progress_token(&without_token));
    }
    
    #[test]
    fn test_extract_progress_token() {
        let message = json!({
            "params": {
                "_meta": {
                    "progressToken": "xyz789"
                }
            }
        });
        
        let token = extract_progress_token(&message);
        assert_eq!(token, Some(json!("xyz789")));
        
        let no_token = json!({"params": {}});
        assert_eq!(extract_progress_token(&no_token), None);
    }
}