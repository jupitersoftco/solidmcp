//! Resource limits configuration for MCP servers
//!
//! Provides configurable limits to prevent resource exhaustion and DoS attacks.

use serde::{Deserialize, Serialize};

/// Configuration for various resource limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum number of concurrent sessions
    pub max_sessions: Option<usize>,
    
    /// Maximum message size in bytes
    pub max_message_size: usize,
    
    /// Maximum number of tools that can be registered
    pub max_tools: Option<usize>,
    
    /// Maximum number of resources that can be registered
    pub max_resources: Option<usize>,
    
    /// Maximum number of prompts that can be registered
    pub max_prompts: Option<usize>,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_sessions: Some(10_000),
            max_message_size: 2 * 1024 * 1024, // 2MB
            max_tools: Some(1_000),
            max_resources: Some(10_000),
            max_prompts: Some(1_000),
        }
    }
}

impl ResourceLimits {
    /// Create unlimited resource limits (use with caution)
    pub fn unlimited() -> Self {
        Self {
            max_sessions: None,
            max_message_size: usize::MAX,
            max_tools: None,
            max_resources: None,
            max_prompts: None,
        }
    }
    
    /// Create strict limits for testing or restricted environments
    pub fn strict() -> Self {
        Self {
            max_sessions: Some(100),
            max_message_size: 256 * 1024, // 256KB
            max_tools: Some(50),
            max_resources: Some(100),
            max_prompts: Some(50),
        }
    }
}