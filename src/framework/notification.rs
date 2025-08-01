//! Notification handling for the MCP framework.
//!
//! This module provides the `NotificationCtx` struct which offers an ergonomic
//! interface for sending notifications to MCP clients, including log messages,
//! progress updates, and resource change notifications.

use crate::handler::{LogLevel, McpContext, McpNotification};
use anyhow::Result;
use serde_json::Value;
use tokio::sync::mpsc;

/// Ergonomic notification context that simplifies sending notifications to MCP clients.
///
/// This struct wraps the underlying notification system and provides convenient methods
/// for sending different types of notifications with minimal boilerplate. It automatically
/// handles serialization and error cases.
///
/// # Examples
///
/// ```rust
/// use solidmcp::framework::NotificationCtx;
/// use anyhow::Result;
///
/// async fn example_tool(ctx: NotificationCtx) -> Result<()> {
///     // Send different types of notifications
///     ctx.info("Processing started")?;
///     ctx.debug("Internal state: processing")?;
///     ctx.warn("This might take a while")?;
///     
///     // Send with structured data
///     ctx.log(LogLevel::Info, "Progress update", Some(serde_json::json!({
///         "progress": 50,
///         "total": 100
///     })))?;
///     
///     // Notify about resource changes
///     ctx.resources_changed()?;
///     
///     Ok(())
/// }
/// ```
#[derive(Clone)]
pub struct NotificationCtx {
    sender: Option<mpsc::UnboundedSender<McpNotification>>,
}

impl NotificationCtx {
    /// Create a new notification context from an existing MCP context.
    ///
    /// This is typically called internally by the framework when setting up tool handlers.
    /// You usually don't need to call this directly.
    ///
    /// # Parameters
    /// - `mcp`: The MCP context containing the notification sender
    ///
    /// # Returns
    /// A new `NotificationCtx` that can send notifications to the connected client
    pub fn from_mcp(mcp: &McpContext) -> Self {
        Self {
            sender: mcp.notification_sender.clone(),
        }
    }

    /// Send an informational notification to the client.
    ///
    /// This is the most common type of notification for general status updates
    /// and user-facing information.
    ///
    /// # Parameters
    /// - `message`: The message to send (can be String, &str, or anything implementing Into<String>)
    ///
    /// # Returns
    /// `Result<()>` - Ok if sent successfully, Err if serialization fails or channel is closed
    ///
    /// # Examples
    /// ```rust
    /// ctx.info("File processing completed successfully")?;
    /// ctx.info(format!("Processed {} items", count))?;
    /// ```
    pub fn info(&self, message: impl Into<String>) -> Result<()> {
        self.log(LogLevel::Info, message, None::<Value>)
    }

    /// Send a debug notification to the client.
    ///
    /// Use this for detailed diagnostic information that's primarily useful
    /// for developers or debugging purposes.
    ///
    /// # Parameters
    /// - `message`: The debug message to send
    ///
    /// # Returns
    /// `Result<()>` - Ok if sent successfully, Err if serialization fails
    pub fn debug(&self, message: impl Into<String>) -> Result<()> {
        self.log(LogLevel::Debug, message, None::<Value>)
    }

    /// Send a warning notification to the client.
    ///
    /// Use this for non-fatal issues that the user should be aware of but
    /// don't prevent the operation from completing.
    ///
    /// # Parameters
    /// - `message`: The warning message to send
    ///
    /// # Returns
    /// `Result<()>` - Ok if sent successfully, Err if serialization fails
    pub fn warn(&self, message: impl Into<String>) -> Result<()> {
        self.log(LogLevel::Warning, message, None::<Value>)
    }

    /// Send an error notification to the client.
    ///
    /// Use this for fatal errors or issues that prevent normal operation.
    /// Note that this doesn't stop execution - it just notifies the client.
    ///
    /// # Parameters
    /// - `message`: The error message to send
    ///
    /// # Returns
    /// `Result<()>` - Ok if sent successfully, Err if serialization fails
    pub fn error(&self, message: impl Into<String>) -> Result<()> {
        self.log(LogLevel::Error, message, None::<Value>)
    }

    /// Send a log notification with custom level and optional structured data.
    ///
    /// This is the most flexible notification method, allowing you to specify
    /// the log level and attach structured data to the notification.
    ///
    /// # Type Parameters
    /// - `T`: Type of the data to attach (must implement `serde::Serialize`)
    ///
    /// # Parameters
    /// - `level`: The log level for this message
    /// - `message`: The log message
    /// - `data`: Optional structured data to attach to the notification
    ///
    /// # Returns
    /// `Result<()>` - Ok if sent successfully, Err if serialization fails
    ///
    /// # Examples
    /// ```rust
    /// ctx.log(LogLevel::Info, "Operation completed", Some(json!({
    ///     "duration": 1234,
    ///     "items_processed": 42
    /// })))?;
    /// ```
    pub fn log<T>(&self, level: LogLevel, message: impl Into<String>, data: Option<T>) -> Result<()>
    where
        T: serde::Serialize,
    {
        if let Some(sender) = &self.sender {
            let data = data.map(|d| serde_json::to_value(d)).transpose()?;

            sender.send(McpNotification::LogMessage {
                level,
                logger: Some("app".to_string()),
                message: message.into(),
                data,
            })?;
        }
        Ok(())
    }

    /// Notify the client that the list of available resources has changed.
    ///
    /// This should be called whenever resources are added, removed, or modified
    /// to ensure clients can refresh their resource listings.
    ///
    /// # Returns
    /// `Result<()>` - Ok if sent successfully, Err if channel is closed
    ///
    /// # Examples
    /// ```rust
    /// // After adding a new file to your resource provider
    /// ctx.resources_changed()?;
    /// ```
    pub fn resources_changed(&self) -> Result<()> {
        if let Some(sender) = &self.sender {
            sender.send(McpNotification::ResourcesListChanged)?;
        }
        Ok(())
    }

    /// Notify the client that the list of available tools has changed.
    ///
    /// This should be called whenever tools are added, removed, or modified
    /// to ensure clients can refresh their tool listings.
    ///
    /// # Returns
    /// `Result<()>` - Ok if sent successfully, Err if channel is closed
    pub fn tools_changed(&self) -> Result<()> {
        if let Some(sender) = &self.sender {
            sender.send(McpNotification::ToolsListChanged)?;
        }
        Ok(())
    }

    /// Notify the client that the list of available prompts has changed.
    ///
    /// This should be called whenever prompts are added, removed, or modified
    /// to ensure clients can refresh their prompt listings.
    ///
    /// # Returns
    /// `Result<()>` - Ok if sent successfully, Err if channel is closed
    pub fn prompts_changed(&self) -> Result<()> {
        if let Some(sender) = &self.sender {
            sender.send(McpNotification::PromptsListChanged)?;
        }
        Ok(())
    }

    /// Send a progress notification for long-running operations.
    ///
    /// This allows clients to display progress bars or status updates
    /// for operations that take significant time.
    ///
    /// # Parameters
    /// - `progress_token`: Unique identifier for this progress operation
    /// - `progress`: Current progress value
    /// - `total`: Optional total value (for percentage calculations)
    ///
    /// # Returns
    /// `Result<()>` - Ok if sent successfully, Err if channel is closed
    ///
    /// # Examples
    /// ```rust
    /// // Simple progress counter
    /// ctx.progress("import-123", 25.0, Some(100.0))?;
    ///
    /// // Indeterminate progress
    /// ctx.progress("processing", 1.0, None)?;
    /// ```
    pub fn progress(
        &self,
        progress_token: impl Into<String>,
        progress: f64,
        total: Option<f64>,
    ) -> Result<()> {
        if let Some(sender) = &self.sender {
            sender.send(McpNotification::Progress {
                progress_token: progress_token.into(),
                progress,
                total,
            })?;
        }
        Ok(())
    }
}

/// Helper function to send a notification through an MCP context.
///
/// This is a convenience function that creates a temporary NotificationCtx
/// and sends a single notification. For multiple notifications, it's more
/// efficient to create a NotificationCtx and reuse it.
///
/// # Parameters
/// - `context`: The MCP context
/// - `notification`: The notification to send
///
/// # Returns
/// `Result<()>` - Ok if sent successfully, Err if channel is closed
pub fn send_notification(context: &McpContext, notification: McpNotification) -> Result<()> {
    if let Some(sender) = &context.notification_sender {
        sender.send(notification)?;
    }
    Ok(())
}

/// Convenience function to notify that resources have changed.
///
/// # Parameters
/// - `context`: The MCP context
///
/// # Returns
/// `Result<()>` - Ok if sent successfully, Err if channel is closed
pub fn notify_resources_changed(context: &McpContext) -> Result<()> {
    send_notification(context, McpNotification::ResourcesListChanged)
}

/// Convenience function to notify that tools have changed.
///
/// # Parameters
/// - `context`: The MCP context
///
/// # Returns
/// `Result<()>` - Ok if sent successfully, Err if channel is closed
pub fn notify_tools_changed(context: &McpContext) -> Result<()> {
    send_notification(context, McpNotification::ToolsListChanged)
}

/// Convenience function to notify that prompts have changed.
///
/// # Parameters
/// - `context`: The MCP context
///
/// # Returns
/// `Result<()>` - Ok if sent successfully, Err if channel is closed
pub fn notify_prompts_changed(context: &McpContext) -> Result<()> {
    send_notification(context, McpNotification::PromptsListChanged)
}