//! Transport capability detection and negotiation for SolidMCP
//!
//! This module provides intelligent transport detection based on client headers,
//! allowing graceful fallback between WebSocket, HTTP, and other transports.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use tracing::{debug, info, warn};
use warp::http::{HeaderMap, HeaderValue};
use warp::Filter;

/// Transport capability detection based on client headers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportCapabilities {
    pub supports_websocket: bool,
    pub supports_sse: bool,
    pub supports_http_only: bool,
    pub client_info: Option<String>,
    pub protocol_version: Option<String>,
}

impl TransportCapabilities {
    /// Detect transport capabilities from HTTP headers
    pub fn from_headers(headers: &HeaderMap<HeaderValue>) -> Self {
        let _accept = headers
            .get("accept")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        let connection = headers
            .get("connection")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        let upgrade = headers
            .get("upgrade")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        let user_agent = headers
            .get("user-agent")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        // Extract protocol version if provided
        let protocol_version = headers
            .get("x-mcp-protocol-version")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        // Detect capabilities based on headers
        let supports_websocket = upgrade.to_lowercase().contains("websocket")
            && connection.to_lowercase().contains("upgrade");

        // TODO: SSE support - currently disabled until properly implemented
        // Client detection: let supports_sse = accept.contains("text/event-stream");
        let supports_sse = false; // Force disable SSE advertising until implementation complete

        let supports_http_only = !supports_websocket && !supports_sse;

        debug!(
            "Transport capabilities detected: ws={}, sse={}, http={}, client={:?}, protocol={:?}",
            supports_websocket, supports_sse, supports_http_only, user_agent, protocol_version
        );

        Self {
            supports_websocket,
            supports_sse,
            supports_http_only,
            client_info: user_agent,
            protocol_version,
        }
    }

    /// Check if the client supports a specific transport
    pub fn supports(&self, transport: &TransportType) -> bool {
        match transport {
            TransportType::WebSocket => self.supports_websocket,
            TransportType::ServerSentEvents => self.supports_sse,
            TransportType::HttpOnly => self.supports_http_only,
        }
    }

    /// Get the preferred transport for this client
    pub fn preferred_transport(&self) -> TransportType {
        if self.supports_websocket {
            TransportType::WebSocket
        } else if self.supports_sse {
            TransportType::ServerSentEvents
        } else {
            TransportType::HttpOnly
        }
    }
}

/// Available transport types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransportType {
    WebSocket,
    ServerSentEvents,
    HttpOnly,
}

impl std::fmt::Display for TransportType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransportType::WebSocket => write!(f, "websocket"),
            TransportType::ServerSentEvents => write!(f, "sse"),
            TransportType::HttpOnly => write!(f, "http"),
        }
    }
}

/// Information about available transports for client discovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportInfo {
    pub server_name: String,
    pub server_version: String,
    pub available_transports: HashMap<String, TransportEndpoint>,
    pub client_capabilities: TransportCapabilities,
    pub instructions: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportEndpoint {
    pub endpoint: String,
    pub method: String,
    pub description: String,
}

impl TransportInfo {
    /// Create transport information for a client
    pub fn new(
        capabilities: &TransportCapabilities,
        server_name: &str,
        server_version: &str,
        base_endpoint: &str,
    ) -> Self {
        let mut available_transports = HashMap::new();
        let mut instructions = HashMap::new();

        // WebSocket transport
        available_transports.insert(
            "websocket".to_string(),
            TransportEndpoint {
                endpoint: base_endpoint.to_string(),
                method: "GET with Upgrade: websocket".to_string(),
                description: "Full-duplex communication for real-time MCP messaging".to_string(),
            },
        );

        instructions.insert(
            "websocket".to_string(),
            "Send GET request with 'Upgrade: websocket' and 'Connection: upgrade' headers"
                .to_string(),
        );

        // HTTP transport
        available_transports.insert(
            "http".to_string(),
            TransportEndpoint {
                endpoint: base_endpoint.to_string(),
                method: "POST with Content-Type: application/json".to_string(),
                description: "Request-response based JSON-RPC 2.0 messaging".to_string(),
            },
        );

        instructions.insert(
            "http".to_string(),
            "Send POST request with JSON-RPC 2.0 message body".to_string(),
        );

        Self {
            server_name: server_name.to_string(),
            server_version: server_version.to_string(),
            available_transports,
            client_capabilities: capabilities.clone(),
            instructions,
        }
    }

    /// Convert to JSON for responses
    pub fn to_json(&self) -> Value {
        // Transform available_transports to match expected format
        let mut transports = serde_json::Map::new();
        
        for (transport_type, endpoint) in &self.available_transports {
            let uri = match transport_type.as_str() {
                "websocket" => {
                    // Convert http:// to ws:// for WebSocket URIs
                    if endpoint.endpoint.starts_with("http://") {
                        endpoint.endpoint.replace("http://", "ws://")
                    } else if endpoint.endpoint.starts_with("https://") {
                        endpoint.endpoint.replace("https://", "wss://")
                    } else {
                        format!("ws://{}", endpoint.endpoint.trim_start_matches('/'))
                    }
                },
                _ => {
                    // For HTTP, ensure proper protocol prefix
                    if endpoint.endpoint.starts_with("http://") || endpoint.endpoint.starts_with("https://") {
                        endpoint.endpoint.clone()
                    } else {
                        format!("http://{}", endpoint.endpoint.trim_start_matches('/'))
                    }
                }
            };
            
            transports.insert(transport_type.clone(), json!({
                "type": transport_type,
                "uri": uri,
                "method": if transport_type == "http" { "POST" } else { endpoint.method.clone() },
                "description": endpoint.description
            }));
        }
        
        json!({
            "mcp_server": {
                "name": self.server_name,
                "version": self.server_version,
                "available_transports": transports,
                "client_capabilities": self.client_capabilities,
                "instructions": self.instructions,
                "protocol": "JSON-RPC 2.0",
                "mcp_protocol_version": "2025-06-18"
            }
        })
    }
}

/// Transport negotiation result
#[derive(Debug, Clone)]
pub enum TransportNegotiation {
    WebSocketUpgrade,
    HttpJsonRpc,
    InfoResponse(TransportInfo),
    UnsupportedTransport {
        error: String,
        supported: Vec<TransportType>,
    },
}

impl TransportNegotiation {
    /// Negotiate transport based on request method and capabilities
    pub fn negotiate(
        method: &str,
        capabilities: &TransportCapabilities,
        has_body: bool,
        server_name: &str,
        server_version: &str,
        endpoint: &str,
    ) -> Self {
        match method.to_uppercase().as_str() {
            "GET" => {
                if capabilities.supports_websocket {
                    info!("Negotiated WebSocket transport");
                    TransportNegotiation::WebSocketUpgrade
                } else {
                    info!("Providing transport information for GET request");
                    let info =
                        TransportInfo::new(capabilities, server_name, server_version, endpoint);
                    TransportNegotiation::InfoResponse(info)
                }
            }
            "POST" => {
                if has_body {
                    info!("Negotiated HTTP JSON-RPC transport");
                    TransportNegotiation::HttpJsonRpc
                } else {
                    warn!("POST request without body");
                    TransportNegotiation::UnsupportedTransport {
                        error: "POST requests must include a JSON-RPC message body".to_string(),
                        supported: vec![TransportType::WebSocket, TransportType::HttpOnly],
                    }
                }
            }
            "OPTIONS" => {
                info!("Providing CORS and capability information");
                let info = TransportInfo::new(capabilities, server_name, server_version, endpoint);
                TransportNegotiation::InfoResponse(info)
            }
            _ => {
                warn!("Unsupported HTTP method: {}", method);
                TransportNegotiation::UnsupportedTransport {
                    error: format!("Unsupported HTTP method: {}", method),
                    supported: vec![TransportType::WebSocket, TransportType::HttpOnly],
                }
            }
        }
    }
}

/// Warp filter for extracting transport capabilities from headers
pub fn transport_capabilities(
) -> impl warp::Filter<Extract = (TransportCapabilities,), Error = std::convert::Infallible> + Clone
{
    warp::header::headers_cloned()
        .map(|headers: HeaderMap<HeaderValue>| TransportCapabilities::from_headers(&headers))
}

/// CORS headers for MCP endpoints
pub fn cors_headers() -> HeaderMap<HeaderValue> {
    let mut headers = HeaderMap::new();
    headers.insert("access-control-allow-origin", HeaderValue::from_static("*"));
    headers.insert(
        "access-control-allow-methods",
        HeaderValue::from_static("GET, POST, OPTIONS"),
    );
    headers.insert(
        "access-control-allow-headers",
        HeaderValue::from_static("content-type, upgrade, connection, x-mcp-protocol-version"),
    );
    headers.insert(
        "access-control-expose-headers",
        HeaderValue::from_static("x-mcp-protocol-version"),
    );
    headers.insert(
        "access-control-max-age",
        HeaderValue::from_static("3600"),
    );
    headers
}

#[cfg(test)]
mod tests {
    use super::*;
    use warp::http::{HeaderMap, HeaderValue};

    #[test]
    fn test_websocket_capability_detection() {
        let mut headers = HeaderMap::new();
        headers.insert("upgrade", HeaderValue::from_static("websocket"));
        headers.insert("connection", HeaderValue::from_static("upgrade"));
        headers.insert("user-agent", HeaderValue::from_static("test-client/1.0"));

        let capabilities = TransportCapabilities::from_headers(&headers);

        assert!(capabilities.supports_websocket);
        assert!(!capabilities.supports_sse);
        assert!(!capabilities.supports_http_only);
        assert_eq!(
            capabilities.client_info,
            Some("test-client/1.0".to_string())
        );
        assert_eq!(capabilities.preferred_transport(), TransportType::WebSocket);
    }

    #[test]
    fn test_http_only_capability_detection() {
        let mut headers = HeaderMap::new();
        headers.insert("content-type", HeaderValue::from_static("application/json"));
        headers.insert("user-agent", HeaderValue::from_static("curl/7.68.0"));

        let capabilities = TransportCapabilities::from_headers(&headers);

        assert!(!capabilities.supports_websocket);
        assert!(!capabilities.supports_sse);
        assert!(capabilities.supports_http_only);
        assert_eq!(capabilities.preferred_transport(), TransportType::HttpOnly);
    }

    #[test]
    fn test_sse_capability_detection_disabled() {
        let mut headers = HeaderMap::new();
        headers.insert("accept", HeaderValue::from_static("text/event-stream"));

        let capabilities = TransportCapabilities::from_headers(&headers);

        // SSE is currently disabled, so should fall back to HTTP-only
        assert!(!capabilities.supports_websocket);
        assert!(!capabilities.supports_sse); // TODO: Will be true when SSE is implemented
        assert!(capabilities.supports_http_only);
        assert_eq!(
            capabilities.preferred_transport(),
            TransportType::HttpOnly // Falls back to HTTP when SSE unavailable
        );
    }

    #[test]
    fn test_transport_negotiation() {
        let capabilities = TransportCapabilities {
            supports_websocket: true,
            supports_sse: false,
            supports_http_only: true,
            client_info: Some("test".to_string()),
            protocol_version: None,
        };

        // GET with WebSocket capability should upgrade
        let result = TransportNegotiation::negotiate(
            "GET",
            &capabilities,
            false,
            "test-server",
            "1.0",
            "/mcp",
        );
        matches!(result, TransportNegotiation::WebSocketUpgrade);

        // POST with body should use HTTP
        let result = TransportNegotiation::negotiate(
            "POST",
            &capabilities,
            true,
            "test-server",
            "1.0",
            "/mcp",
        );
        matches!(result, TransportNegotiation::HttpJsonRpc);
    }

    #[test]
    fn test_transport_info_creation() {
        let capabilities = TransportCapabilities {
            supports_websocket: true,
            supports_sse: false,
            supports_http_only: true,
            client_info: Some("test-client".to_string()),
            protocol_version: Some("2025-06-18".to_string()),
        };

        let info = TransportInfo::new(&capabilities, "test-server", "1.0.0", "/mcp");

        assert_eq!(info.server_name, "test-server");
        assert_eq!(info.server_version, "1.0.0");
        assert!(info.available_transports.contains_key("websocket"));
        assert!(info.available_transports.contains_key("http"));

        let json = info.to_json();
        assert!(json["mcp_server"]["available_transports"]["websocket"].is_object());
    }
}
