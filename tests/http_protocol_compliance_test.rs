//! HTTP Protocol Compliance Test
//!
//! Tests that HTTP responses follow RFC 7230 requirements:
//! - MUST NOT send both Content-Length and Transfer-Encoding headers
//! - Transfer-Encoding takes precedence over Content-Length

use reqwest::header::{CONTENT_LENGTH, TRANSFER_ENCODING};
use serde_json::json;
use std::time::Duration;

mod mcp_test_helpers;
use mcp_test_helpers::with_mcp_test_server;

#[tokio::test]
async fn test_http_headers_no_dual_encoding() {
    // Test that server never sends both Content-Length and Transfer-Encoding
    with_mcp_test_server("http_headers_compliance", |server| async move {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()?;

        // Test 1: Initialize request (should not have both headers)
        let init_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {"name": "test", "version": "1.0"}
            }
        });

        let response = client
            .post(&server.http_url())
            .json(&init_request)
            .send()
            .await?;

        let has_content_length = response.headers().contains_key(CONTENT_LENGTH);
        let has_transfer_encoding = response.headers().contains_key(TRANSFER_ENCODING);

        assert!(
            !(has_content_length && has_transfer_encoding),
            "HTTP protocol violation: Response has both Content-Length and Transfer-Encoding headers"
        );

        // Test 2: Tools call with progress token (high risk for dual headers)
        let progress_request = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "echo",
                "arguments": {"message": "test"},
                "_meta": {
                    "progressToken": "test-token-123"
                }
            }
        });

        let response = client
            .post(&server.http_url())
            .json(&progress_request)
            .send()
            .await?;

        let has_content_length = response.headers().contains_key(CONTENT_LENGTH);
        let has_transfer_encoding = response.headers().contains_key(TRANSFER_ENCODING);

        assert!(
            !(has_content_length && has_transfer_encoding),
            "HTTP protocol violation in progress response: Has both Content-Length and Transfer-Encoding"
        );

        // Verify exactly one is present
        assert!(
            has_content_length || has_transfer_encoding,
            "Response must have either Content-Length or Transfer-Encoding"
        );

        Ok(())
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn test_chunked_encoding_for_progress_tokens() {
    // Test that progress token requests use chunked encoding
    with_mcp_test_server("chunked_encoding_progress", |server| async move {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()?;

        // Initialize first
        let init_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {"name": "test", "version": "1.0"}
            }
        });

        let init_response = client
            .post(&server.http_url())
            .json(&init_request)
            .send()
            .await?;

        // Ensure initialization succeeded
        assert_eq!(init_response.status(), 200);

        // Request with progress token
        let progress_request = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "echo",
                "arguments": {"message": "test"},
                "_meta": {
                    "progressToken": "test-progress-123"
                }
            }
        });

        let response = client
            .post(&server.http_url())
            .json(&progress_request)
            .send()
            .await?;

        // Debug headers
        println!("Response headers: {:?}", response.headers());

        // Should use chunked encoding for progress
        let transfer_encoding = response
            .headers()
            .get(TRANSFER_ENCODING)
            .and_then(|v| v.to_str().ok());

        assert_eq!(
            transfer_encoding,
            Some("chunked"),
            "Progress token requests should use chunked encoding"
        );

        // Should NOT have Content-Length
        assert!(
            !response.headers().contains_key(CONTENT_LENGTH),
            "Chunked responses must not have Content-Length header"
        );

        Ok(())
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn test_regular_requests_use_content_length() {
    // Test that regular requests without progress tokens use Content-Length
    with_mcp_test_server("content_length_regular", |server| async move {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()?;

        // Initialize first
        let init_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {"name": "test", "version": "1.0"}
            }
        });

        let response = client
            .post(&server.http_url())
            .json(&init_request)
            .send()
            .await?;

        // Regular responses should have Content-Length
        assert!(
            response.headers().contains_key(CONTENT_LENGTH),
            "Regular responses should have Content-Length header"
        );

        // Should NOT have Transfer-Encoding
        assert!(
            !response.headers().contains_key(TRANSFER_ENCODING),
            "Regular responses should not have Transfer-Encoding header"
        );

        Ok(())
    })
    .await
    .unwrap();
}
