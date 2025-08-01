//! Health check functionality for MCP servers
//!
//! Provides a simple health check endpoint that can be used by monitoring systems
//! to verify the service is running and get basic status information.

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Health check response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    /// Health status (always "healthy" if responding)
    pub status: String,
    
    /// Current timestamp in seconds since Unix epoch
    pub timestamp: u64,
    
    /// Server version
    pub version: String,
    
    /// Number of active sessions (if available)
    pub session_count: Option<usize>,
    
    /// Server uptime in seconds
    pub uptime_seconds: u64,
    
    /// Additional metadata
    pub metadata: Option<serde_json::Value>,
}

/// Health check provider for MCP servers
#[derive(Debug, Clone)]
pub struct HealthChecker {
    start_time: SystemTime,
    version: String,
    server_name: String,
}

impl HealthChecker {
    /// Create a new health checker
    ///
    /// # Parameters
    /// - `server_name`: Name of the MCP server
    /// - `version`: Version of the MCP server
    pub fn new(server_name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            start_time: SystemTime::now(),
            version: version.into(),
            server_name: server_name.into(),
        }
    }
    
    /// Get current health status
    ///
    /// # Parameters
    /// - `session_count`: Optional current session count
    /// - `metadata`: Optional additional metadata to include
    ///
    /// # Returns
    /// A `HealthStatus` struct with current server information
    pub fn get_status(&self, session_count: Option<usize>, metadata: Option<serde_json::Value>) -> HealthStatus {
        let now = SystemTime::now();
        let timestamp = now.duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        let uptime_seconds = now.duration_since(self.start_time)
            .unwrap_or_default()
            .as_secs();
        
        HealthStatus {
            status: "healthy".to_string(),
            timestamp,
            version: self.version.clone(),
            session_count,
            uptime_seconds,
            metadata: metadata.or_else(|| Some(serde_json::json!({
                "server_name": self.server_name,
                "protocol_version": "2025-06-18"
            }))),
        }
    }
    
    /// Get a simple JSON health response
    ///
    /// This is a convenience method that returns a JSON Value that can be
    /// easily used in HTTP responses.
    pub fn get_json_status(&self, session_count: Option<usize>) -> serde_json::Value {
        serde_json::to_value(self.get_status(session_count, None))
            .unwrap_or_else(|_| serde_json::json!({
                "status": "error",
                "message": "Failed to serialize health status"
            }))
    }
}

impl Default for HealthChecker {
    fn default() -> Self {
        Self::new("mcp-server", env!("CARGO_PKG_VERSION"))
    }
}