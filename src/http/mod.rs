//! HTTP Transport Module
//!
//! This module contains the HTTP transport implementation for MCP,
//! including request handling, session management, and response building.

pub mod progress;
pub mod response;
pub mod session;
pub mod validation;

// Re-export commonly used types
pub use progress::{ProgressHandler, has_progress_token, extract_progress_token};
pub use response::{ResponseBuilder, create_error_response, create_chunked_response, apply_cors_headers};
pub use session::{SessionContext, extract_session_context, extract_session_id_from_cookie, generate_session_id, create_session_cookie};
pub use validation::{ValidatedRequest, validate_request, validate_message_structure, extract_content_type};