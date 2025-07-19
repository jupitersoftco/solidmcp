//! Notification Implementation Tests
//!
//! Tests that verify the notification system is correctly implemented
//! even though clients may not display them

use anyhow::Result;
use anyhow::Result;
use serde_json::json;
use solidmcp::McpTool;
use solidmcp::{LogLevel, McpNotification, ToolContext};
use tokio::sync::mpsc;
use toy_notes_server::server::{AddNotificationTool, NotesStorage};

/// Test that the add_notification tool properly sends notifications through the context
#[tokio::test]
async fn test_notification_tool_sends_to_context() -> Result<()> {
    // Create a channel to capture notifications
    let (tx, mut rx) = mpsc::unbounded_channel::<McpNotification>();

    // Create the tool
    let tool = AddNotificationTool {};

    // Create a context with our notification sender
    let context = ToolContext {
        notification_sender: Some(tx),
    };

    // Execute the tool
    let args = json!({
        "level": "info",
        "message": "Test notification",
        "data": {
            "test": true
        }
    });

    let result = tool.execute(args, &context).await?;

    // Verify success response
    assert_eq!(result["success"], true);

    // Check that we received the notification
    let notification = rx.recv().await.expect("Should receive notification");

    match notification {
        McpNotification::LogMessage {
            level,
            message,
            data,
            ..
        } => {
            assert_eq!(level, LogLevel::Info);
            assert_eq!(message, "Test notification");
            assert_eq!(data, Some(json!({"test": true})));
        }
        _ => panic!("Expected LogMessage notification"),
    }

    println!("✅ Notification tool correctly sends notifications!");
    Ok(())
}

/// Test that add_note tool sends both log and resource changed notifications
#[tokio::test]
async fn test_add_note_sends_notifications() -> Result<()> {
    use toy_notes_server::server::AddNoteTool;

    // Create a channel to capture notifications
    let (tx, mut rx) = mpsc::unbounded_channel::<McpNotification>();

    // Create storage
    let temp_dir = tempfile::TempDir::new()?;
    let storage = NotesStorage::new(temp_dir.path().to_path_buf());

    // Create the tool
    let tool = AddNoteTool::new(storage);

    // Create a context with our notification sender
    let context = ToolContext {
        notification_sender: Some(tx),
    };

    // Execute the tool
    let args = json!({
        "name": "test-note",
        "content": "Test content"
    });

    let result = tool.execute(args, &context).await?;

    // Verify success response
    assert!(result["message"]
        .as_str()
        .unwrap()
        .contains("saved successfully"));

    // Should receive two notifications
    let notification1 = rx.recv().await.expect("Should receive first notification");
    let notification2 = rx.recv().await.expect("Should receive second notification");

    // One should be LogMessage, one should be ResourcesListChanged
    let mut has_log = false;
    let mut has_resources_changed = false;

    for notification in [notification1, notification2] {
        match notification {
            McpNotification::LogMessage { level, message, .. } => {
                assert_eq!(level, LogLevel::Info);
                assert!(message.contains("test-note"));
                assert!(message.contains("saved"));
                has_log = true;
            }
            McpNotification::ResourcesListChanged => {
                has_resources_changed = true;
            }
            _ => panic!("Unexpected notification type"),
        }
    }

    assert!(has_log, "Should have received log notification");
    assert!(
        has_resources_changed,
        "Should have received resources changed notification"
    );

    println!("✅ Add note correctly sends both notifications!");
    Ok(())
}

/// Test notification level parsing
#[tokio::test]
async fn test_notification_level_parsing() -> Result<()> {
    let tool = AddNotificationTool {};

    // Test valid levels
    let valid_levels = [
        ("debug", LogLevel::Debug),
        ("info", LogLevel::Info),
        ("warning", LogLevel::Warning),
        ("error", LogLevel::Error),
    ];

    for (level_str, expected_level) in valid_levels {
        let (tx, mut rx) = mpsc::unbounded_channel::<McpNotification>();
        let context = ToolContext {
            notification_sender: Some(tx),
        };

        let args = json!({
            "level": level_str,
            "message": format!("Test {} level", level_str),
            "data": null
        });

        let result = tool.execute(args, &context).await?;
        assert_eq!(result["success"], true);

        let notification = rx.recv().await.expect("Should receive notification");
        match notification {
            McpNotification::LogMessage { level, .. } => {
                assert_eq!(level, expected_level);
            }
            _ => panic!("Expected LogMessage notification"),
        }
    }

    // Test invalid level
    let (tx, _rx) = mpsc::unbounded_channel::<McpNotification>();
    let context = ToolContext {
        notification_sender: Some(tx),
    };

    let args = json!({
        "level": "invalid",
        "message": "This should fail",
        "data": null
    });

    let result = tool.execute(args, &context).await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Invalid log level"));

    println!("✅ Notification level parsing works correctly!");
    Ok(())
}

/// Test that notifications work without a sender (graceful degradation)
#[tokio::test]
async fn test_notification_without_sender() -> Result<()> {
    let tool = AddNotificationTool {};

    // Create a context without notification sender
    let context = ToolContext {
        notification_sender: None,
    };

    let args = json!({
        "level": "info",
        "message": "Test without sender",
        "data": null
    });

    let result = tool.execute(args, &context).await;

    // Should get an error when no sender is available
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Notification sender not available"));

    println!("✅ Notification tool handles missing sender correctly!");
    Ok(())
}
