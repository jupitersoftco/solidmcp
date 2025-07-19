//! Session and cookie isolation tests for MCP HTTP

#[cfg(test)]
mod tests {
    use crate::http::session::{extract_session_id_from_cookie, generate_session_id};

    #[test]
    fn test_session_id_generation() {
        let session_id = generate_session_id();
        assert_eq!(session_id.len(), 32);
        assert!(session_id.chars().all(|c| c.is_alphanumeric()));
        
        // Test uniqueness
        let session_id2 = generate_session_id();
        assert_ne!(session_id, session_id2);
    }

    #[test]
    fn test_extract_session_id_from_cookie() {
        // Test with valid cookie
        let cookie = Some("mcp_session=abc123def456; other=value".to_string());
        let session_id = extract_session_id_from_cookie(&cookie);
        assert_eq!(session_id, Some("abc123def456".to_string()));
        
        // Test with session cookie only
        let cookie = Some("mcp_session=xyz789".to_string());
        let session_id = extract_session_id_from_cookie(&cookie);
        assert_eq!(session_id, Some("xyz789".to_string()));
        
        // Test with no session cookie
        let cookie = Some("other=value; another=test".to_string());
        let session_id = extract_session_id_from_cookie(&cookie);
        assert_eq!(session_id, None);
        
        // Test with None cookie
        let session_id = extract_session_id_from_cookie(&None);
        assert_eq!(session_id, None);
    }

    #[test]
    fn test_session_cookie_with_spaces() {
        let cookie = Some("  mcp_session=test123  ; other=value".to_string());
        let session_id = extract_session_id_from_cookie(&cookie);
        assert_eq!(session_id, Some("test123".to_string()));
    }

    #[test]
    fn test_multiple_cookies_with_session() {
        let cookie = Some("first=1; mcp_session=found; last=3".to_string());
        let session_id = extract_session_id_from_cookie(&cookie);
        assert_eq!(session_id, Some("found".to_string()));
    }
}