# Fix 1: HTTP Protocol Violation (Content-Length + Transfer-Encoding)

## Problem

The HTTP/1.1 specification (RFC 7230) explicitly forbids sending both `Content-Length` and `Transfer-Encoding` headers in the same response. This violation causes client timeouts and crashes, particularly with strict HTTP clients like Cursor.

## Evidence

1. **RFC 7230 Section 3.3.3**: "A sender MUST NOT send a Content-Length header field in any message that contains a Transfer-Encoding header field."

2. **Client behavior**: Cursor MCP client was experiencing timeouts when receiving responses with both headers.

3. **Code analysis**: The previous implementation always set `use_chunked = true`, but the response creation logic had multiple conflicting branches.

## Solution

### Changes Made

1. **Dynamic chunked encoding decision** (src/http.rs:294-319):
   - Only use chunked encoding when progress tokens are present
   - Clear logic: `let use_chunked = has_progress_token;`

2. **Simplified response creation** (src/http.rs:564-600):
   - Two clear branches: chunked OR content-length, never both
   - Explicit comments warning about the protocol requirement

3. **Fixed test helper** (tests/mcp_test_helpers.rs:59):
   - Added `/mcp` path to HTTP URL to match server endpoint

### Test Coverage

Created comprehensive tests in `tests/http_protocol_compliance_test.rs`:
- `test_http_headers_no_dual_encoding`: Verifies no dual headers
- `test_chunked_encoding_for_progress_tokens`: Ensures progress tokens use chunked
- `test_regular_requests_use_content_length`: Ensures regular requests use content-length

## Why This Approach Works

1. **Protocol Compliance**: Strictly follows HTTP/1.1 specification
2. **Backward Compatible**: Regular requests still use Content-Length
3. **Progress Support**: Progress tokens get proper streaming support
4. **Clear Semantics**: Code clearly shows the mutual exclusivity

## Impact

- Fixes Cursor client timeouts
- Prevents HTTP protocol violations
- Improves compatibility with strict HTTP clients
- Maintains streaming capability for progress notifications