//! HTTP Session Management
//!
//! Functions for extracting and managing HTTP session identifiers.

use rand::{distr::Alphanumeric, Rng};
use tracing::debug;

/// Context about the current session
#[derive(Debug, Clone)]
pub struct SessionContext {
    pub id: String,
    pub is_new: bool,
    pub from_cookie: bool,
}

/// Extract session context from request
pub fn extract_session_context(
    method: &str,
    cookie: &Option<String>,
) -> SessionContext {
    let session_id = extract_session_id_from_cookie(cookie);
    
    // For HTTP clients that don't handle cookies properly, we need a fallback
    // Use a consistent session ID for the duration of the server process
    let effective_session_id = if method == "initialize" || session_id.is_none() {
        // Always use a consistent session for initialize requests
        // or when clients don't send cookies
        if method != "initialize" && session_id.is_none() {
            debug!(
                method = %method,
                fallback_session = "http_default_session",
                "No session cookie found - using default HTTP session"
            );
        }
        Some("http_default_session".to_string())
    } else {
        session_id.clone()
    };
    
    SessionContext {
        id: effective_session_id.unwrap_or_else(|| "http_default_session".to_string()),
        is_new: method == "initialize",
        from_cookie: session_id.is_some(),
    }
}

/// Extract session ID from cookie header
pub fn extract_session_id_from_cookie(cookie: &Option<String>) -> Option<String> {
    cookie.as_ref().and_then(|cookie_str| {
        cookie_str
            .split(';')
            .find_map(|part| {
                let trimmed = part.trim();
                if trimmed.starts_with("mcp_session=") {
                    Some(trimmed.trim_start_matches("mcp_session=").to_string())
                } else {
                    None
                }
            })
    })
}

/// Generate a new session ID
pub fn generate_session_id() -> String {
    rand::rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect()
}

/// Create a session cookie header value
pub fn create_session_cookie(session_id: &str) -> String {
    format!(
        "mcp_session={}; Path=/mcp; HttpOnly; SameSite=Strict; Max-Age=86400",
        session_id
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_extract_session_id_from_cookie() {
        // Test valid cookie
        let cookie = Some("mcp_session=abc123; Path=/".to_string());
        assert_eq!(
            extract_session_id_from_cookie(&cookie),
            Some("abc123".to_string())
        );
        
        // Test cookie with multiple values
        let cookie = Some("other=value; mcp_session=xyz789; another=val".to_string());
        assert_eq!(
            extract_session_id_from_cookie(&cookie),
            Some("xyz789".to_string())
        );
        
        // Test no session cookie
        let cookie = Some("other=value; another=val".to_string());
        assert_eq!(extract_session_id_from_cookie(&cookie), None);
        
        // Test None cookie
        assert_eq!(extract_session_id_from_cookie(&None), None);
    }
    
    #[test]
    fn test_extract_session_context_initialize() {
        let cookie = Some("mcp_session=existing".to_string());
        let context = extract_session_context("initialize", &cookie);
        
        assert_eq!(context.id, "http_default_session");
        assert!(context.is_new);
        assert!(context.from_cookie);
    }
    
    #[test]
    fn test_extract_session_context_no_cookie() {
        let context = extract_session_context("tools/list", &None);
        
        assert_eq!(context.id, "http_default_session");
        assert!(!context.is_new);
        assert!(!context.from_cookie);
    }
    
    #[test]
    fn test_extract_session_context_with_cookie() {
        let cookie = Some("mcp_session=session123".to_string());
        let context = extract_session_context("tools/list", &cookie);
        
        assert_eq!(context.id, "session123");
        assert!(!context.is_new);
        assert!(context.from_cookie);
    }
    
    #[test]
    fn test_generate_session_id() {
        let id1 = generate_session_id();
        let id2 = generate_session_id();
        
        // Should be 32 characters
        assert_eq!(id1.len(), 32);
        assert_eq!(id2.len(), 32);
        
        // Should be different
        assert_ne!(id1, id2);
        
        // Should be alphanumeric
        assert!(id1.chars().all(|c| c.is_ascii_alphanumeric()));
    }
    
    #[test]
    fn test_create_session_cookie() {
        let cookie = create_session_cookie("test123");
        assert!(cookie.contains("mcp_session=test123"));
        assert!(cookie.contains("HttpOnly"));
        assert!(cookie.contains("SameSite=Strict"));
        assert!(cookie.contains("Max-Age=86400"));
    }
}
