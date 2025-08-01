//! MCP Protocol Implementation and Optimizations
//!
//! This module contains the MCP protocol implementation and optimized message processing components.

pub mod message;

pub use message::{
    RawMessage, ParsedMessage, InitializeParams, ToolCallParams, 
    ResourceReadParams, PromptGetParams, NotificationMessage,
    get_message_schema,
};

// Re-export the main protocol struct
pub use self::protocol::McpProtocol;

mod protocol {
    //! MCP Protocol Implementation
    //!
    //! Handles MCP protocol versioning, capabilities, and server information.

    use {
        serde_json::{json, Value},
        tracing::info,
    };

    pub struct McpProtocol {
        version: String,
        server_name: String,
        server_version: String,
    }

    impl McpProtocol {
        pub fn new() -> Self {
            Self {
                version: "2025-06-18".to_string(),
                server_name: "mcp-server".to_string(),
                server_version: env!("CARGO_PKG_VERSION").to_string(),
            }
        }

        /// Get protocol version
        pub fn version(&self) -> &str {
            &self.version
        }

        /// Get server name
        pub fn server_name(&self) -> &str {
            &self.server_name
        }

        /// Get server version
        pub fn server_version(&self) -> &str {
            &self.server_version
        }

        /// Create initialization response
        pub fn create_initialize_response(&self) -> Value {
            info!("🔧 MCP client initializing");
            json!({
                "protocolVersion": self.version,
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": self.server_name,
                    "version": self.server_version
                }
            })
        }

        /// Create error response
        pub fn create_error_response(&self, id: Value, code: i32, message: &str) -> Value {
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {
                    "code": code,
                    "message": message
                }
            })
        }

        /// Create success response
        pub fn create_success_response(&self, id: Value, result: Value) -> Value {
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": result
            })
        }
    }

    impl Default for McpProtocol {
        fn default() -> Self {
            Self::new()
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use serde_json::json;

        #[test]
        fn test_initialize_response() {
            let proto = McpProtocol::new();
            let resp = proto.create_initialize_response();
            assert_eq!(resp["protocolVersion"], proto.version());
            assert_eq!(resp["serverInfo"]["name"], proto.server_name());
            assert_eq!(resp["serverInfo"]["version"], proto.server_version());
        }

        #[test]
        fn test_error_response() {
            let proto = McpProtocol::new();
            let err = proto.create_error_response(json!(42), -1, "fail");
            assert_eq!(err["id"], 42);
            assert_eq!(err["error"]["code"], -1);
            assert_eq!(err["error"]["message"], "fail");
        }

        #[test]
        fn test_success_response() {
            let proto = McpProtocol::new();
            let ok = proto.create_success_response(json!(7), json!({"foo": 1}));
            assert_eq!(ok["id"], 7);
            assert_eq!(ok["result"]["foo"], 1);
        }
    }
}