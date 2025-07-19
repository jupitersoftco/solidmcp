// Session and cookie helpers for MCP HTTP

use rand::{distributions::Alphanumeric, Rng};

#[allow(dead_code)]
pub(crate) fn extract_session_id_from_cookie(cookie: &Option<String>) -> Option<String> {
    cookie.as_ref().and_then(|cookie| {
        cookie.split(';').find_map(|part| {
            let part = part.trim();
            if part.starts_with("mcp_session=") {
                Some(part.trim_start_matches("mcp_session=").to_string())
            } else {
                None
            }
        })
    })
}

#[allow(dead_code)]
pub(crate) fn generate_session_id() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect()
}