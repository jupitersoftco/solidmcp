//! HTTP Transport Module
//!
//! This module contains the HTTP transport implementation for MCP,
//! including request handling, session management, and response building.

pub mod progress;
pub mod response;
pub mod session;
pub mod validation;

// Re-export commonly used types
pub use progress::ProgressHandler;
pub use response::ResponseBuilder;
pub use session::extract_session_context;
pub use validation::{validate_request, validate_message_structure};