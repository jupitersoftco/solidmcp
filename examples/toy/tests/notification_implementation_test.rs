//! Notification Implementation Tests using new framework API

use {
    anyhow::Result,
    serde_json::json,
    solidmcp::{LogLevel, McpNotification},
    tokio::sync::mpsc,
};

/// Test that notifications are properly sent through channels
#[tokio::test]
async fn test_notification_sending() -> Result<()> {
    // Create a channel to capture notifications
    let (tx, mut rx) = mpsc::unbounded_channel::<McpNotification>();

    // Test direct notification sending (what our framework tools do)
    tx.send(McpNotification::LogMessage {
        level: LogLevel::Info,
        logger: Some("test".to_string()),
        message: "Test notification from new framework".to_string(),
        data: Some(json!({"test": true})),
    })?;

    // Check that notification was sent
    let notification = rx.try_recv()?;
    match notification {
        McpNotification::LogMessage {
            level,
            logger,
            message,
            data,
        } => {
            assert_eq!(level, LogLevel::Info);
            assert_eq!(logger, Some("test".to_string()));
            assert_eq!(message, "Test notification from new framework");
            assert_eq!(data, Some(json!({"test": true})));
        }
        _ => panic!("Expected LogMessage notification"),
    }

    // Ensure no more notifications
    assert!(rx.try_recv().is_err());

    Ok(())
}

/// Test different log levels work correctly
#[tokio::test]
async fn test_notification_log_levels() -> Result<()> {
    let (tx, mut rx) = mpsc::unbounded_channel::<McpNotification>();

    // Test each log level
    let levels = vec![
        (LogLevel::Debug, "debug"),
        (LogLevel::Info, "info"),
        (LogLevel::Warning, "warning"),
        (LogLevel::Error, "error"),
    ];

    for (level, level_str) in levels {
        tx.send(McpNotification::LogMessage {
            level: level.clone(),
            logger: Some("test".to_string()),
            message: format!("Test {} message", level_str),
            data: None,
        })?;

        let notification = rx.try_recv()?;
        match notification {
            McpNotification::LogMessage {
                level: received_level,
                message,
                ..
            } => {
                assert_eq!(received_level, level);
                assert_eq!(message, format!("Test {} message", level_str));
            }
            _ => panic!("Expected LogMessage notification"),
        }
    }

    Ok(())
}

/// Test resource change notifications
#[tokio::test]
async fn test_resource_change_notification() -> Result<()> {
    let (tx, mut rx) = mpsc::unbounded_channel::<McpNotification>();

    // Send resource change notification
    tx.send(McpNotification::ResourcesListChanged)?;

    // Check that notification was sent
    let notification = rx.try_recv()?;
    match notification {
        McpNotification::ResourcesListChanged => {
            // Expected
        }
        _ => panic!("Expected ResourcesListChanged notification"),
    }

    // Ensure no more notifications
    assert!(rx.try_recv().is_err());

    Ok(())
}
