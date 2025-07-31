# SolidMCP Troubleshooting Guide

This guide helps you diagnose and fix common issues when using SolidMCP.

## Table of Contents

1. [Connection Issues](#connection-issues)
2. [Transport Negotiation Problems](#transport-negotiation-problems)
3. [Session Management Issues](#session-management-issues)
4. [Tool Execution Errors](#tool-execution-errors)
5. [Protocol Version Conflicts](#protocol-version-conflicts)
6. [Performance Issues](#performance-issues)
7. [Common Error Messages](#common-error-messages)
8. [Debugging Tips](#debugging-tips)

## Connection Issues

### Problem: Server fails to start

**Symptom:**
```
Error: Could not bind to 127.0.0.1:3000: Address already in use
```

**Solution:**
1. Check if another process is using the port:
   ```bash
   lsof -i :3000  # macOS/Linux
   netstat -ano | findstr :3000  # Windows
   ```

2. Either kill the process or use a different port:
   ```rust
   server.start(3001).await?;  // Use different port
   ```

### Problem: Client can't connect to server

**Symptoms:**
- Connection refused errors
- Timeout errors
- WebSocket upgrade failures

**Solutions:**

1. **Verify server is running:**
   ```bash
   curl http://localhost:3000/health
   # Should return: OK
   ```

2. **Check firewall settings:**
   - Ensure port is not blocked by firewall
   - For remote connections, ensure server binds to `0.0.0.0` not `127.0.0.1`

3. **Verify transport headers:**
   ```bash
   # Test WebSocket
   curl -i -N -H "Connection: Upgrade" -H "Upgrade: websocket" \
        -H "Sec-WebSocket-Version: 13" -H "Sec-WebSocket-Key: x3JJHMbDL1EzLkh9GBhXDw==" \
        http://localhost:3000/mcp

   # Test HTTP
   curl -X POST http://localhost:3000/mcp \
        -H "Content-Type: application/json" \
        -d '{"jsonrpc":"2.0","method":"initialize","params":{},"id":1}'
   ```

## Transport Negotiation Problems

### Problem: Wrong transport type selected

**Symptom:**
Client expects WebSocket but gets HTTP response, or vice versa.

**Solution:**

1. **Ensure correct headers are sent:**

   For WebSocket clients:
   ```
   Upgrade: websocket
   Connection: Upgrade
   ```

   For HTTP clients:
   ```
   Content-Type: application/json
   Accept: application/json
   ```

   For SSE clients:
   ```
   Accept: text/event-stream
   ```

2. **Enable debug logging:**
   ```bash
   RUST_LOG=solidmcp=debug cargo run
   ```

3. **Check transport detection in logs:**
   ```
   DEBUG solidmcp::transport: Negotiating transport for POST /mcp
   DEBUG solidmcp::transport: Selected transport: Http
   ```

### Problem: SSE not working properly

**Symptom:**
Streaming responses not received by client.

**Solution:**

1. Ensure client accepts SSE:
   ```bash
   curl -N -H "Accept: text/event-stream" \
        -H "Content-Type: application/json" \
        -d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"long_task"},"id":1}' \
        http://localhost:3000/mcp
   ```

2. Check for proxy/reverse proxy buffering:
   ```nginx
   # Nginx configuration
   proxy_buffering off;
   proxy_cache off;
   ```

## Session Management Issues

### Problem: Session state lost between requests

**Symptom:**
"Not initialized" errors on subsequent HTTP requests.

**Solution:**

1. **Ensure cookies are preserved:**
   ```bash
   # Save cookies
   curl -c cookies.txt -X POST http://localhost:3000/mcp \
        -H "Content-Type: application/json" \
        -d '{"jsonrpc":"2.0","method":"initialize","params":{},"id":1}'
   
   # Use cookies in subsequent requests
   curl -b cookies.txt -X POST http://localhost:3000/mcp \
        -H "Content-Type: application/json" \
        -d '{"jsonrpc":"2.0","method":"tools/list","id":2}'
   ```

2. **For programmatic clients, maintain cookie jar:**
   ```rust
   let client = reqwest::Client::builder()
       .cookie_store(true)
       .build()?;
   ```

### Problem: Multiple initialization attempts

**Symptom:**
"Already initialized" errors when reconnecting.

**Solution:**

SolidMCP now supports re-initialization. If you still see issues:

1. Clear session cookies
2. Use a new session ID
3. Restart the server if necessary

## Tool Execution Errors

### Problem: Tool not found

**Error:**
```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32601,
    "message": "Tool not found: my_tool"
  },
  "id": 1
}
```

**Solution:**

1. **Verify tool registration:**
   ```rust
   // Ensure tool is registered
   .with_tool("my_tool", "Description", handler)
   ```

2. **Check tool name spelling:**
   - Tool names are case-sensitive
   - No spaces allowed in tool names

3. **List available tools:**
   ```bash
   curl -X POST http://localhost:3000/mcp \
        -H "Content-Type: application/json" \
        -d '{"jsonrpc":"2.0","method":"tools/list","id":1}'
   ```

### Problem: Invalid tool arguments

**Error:**
```json
{
  "error": {
    "code": -32602,
    "message": "Invalid params: missing field `name`"
  }
}
```

**Solution:**

1. **Check schema requirements:**
   ```rust
   #[derive(JsonSchema, Deserialize)]
   struct MyInput {
       name: String,        // Required
       count: Option<u32>,  // Optional
   }
   ```

2. **Validate JSON structure:**
   ```json
   {
     "jsonrpc": "2.0",
     "method": "tools/call",
     "params": {
       "name": "my_tool",
       "arguments": {
         "name": "value"  // Ensure all required fields present
       }
     },
     "id": 1
   }
   ```

## Protocol Version Conflicts

### Problem: Protocol version mismatch

**Error:**
```
Unsupported protocol version: 2024-11-05
```

**Solution:**

1. **Check supported versions:**
   SolidMCP supports:
   - `2025-06-18` (recommended)
   - `2025-03-26` (legacy)

2. **Update client or server:**
   ```json
   {
     "jsonrpc": "2.0",
     "method": "initialize",
     "params": {
       "protocolVersion": "2025-06-18"
     },
     "id": 1
   }
   ```

## Performance Issues

### Problem: Slow response times

**Symptoms:**
- High latency on tool calls
- Timeouts on large responses

**Solutions:**

1. **Enable progress notifications:**
   ```rust
   async fn long_task(ctx: NotificationCtx) -> Result<Output> {
       for i in 0..100 {
           ctx.log(LogLevel::Info, "Progress", Some(json!({
               "progress": i,
               "total": 100
           })))?;
           // Do work...
       }
       Ok(output)
   }
   ```

2. **Use streaming for large responses:**
   - HTTP clients with SSE support will automatically stream
   - WebSocket clients receive real-time updates

3. **Profile your handlers:**
   ```rust
   use std::time::Instant;
   
   let start = Instant::now();
   let result = expensive_operation().await?;
   debug!("Operation took: {:?}", start.elapsed());
   ```

### Problem: High memory usage

**Solution:**

1. **Stream large resources:**
   ```rust
   // Instead of loading entire file
   let content = tokio::fs::read_to_string(path).await?;
   
   // Consider chunked reading for large files
   use tokio::io::{AsyncReadExt, BufReader};
   let file = tokio::fs::File::open(path).await?;
   let mut reader = BufReader::new(file);
   let mut buffer = vec![0; 8192];
   ```

2. **Limit concurrent operations:**
   ```rust
   use tokio::sync::Semaphore;
   
   static PERMITS: Semaphore = Semaphore::const_new(10);
   
   async fn rate_limited_operation() -> Result<()> {
       let _permit = PERMITS.acquire().await?;
       // Do work...
   }
   ```

## Common Error Messages

### JSON-RPC Errors

| Code | Message | Meaning | Solution |
|------|---------|---------|----------|
| -32700 | Parse error | Invalid JSON | Check JSON syntax |
| -32600 | Invalid Request | Missing required fields | Include jsonrpc, method, id |
| -32601 | Method not found | Unknown method | Check method name spelling |
| -32602 | Invalid params | Wrong parameter structure | Verify parameter schema |
| -32603 | Internal error | Handler error | Check server logs |

### Example Error Response

```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32603,
    "message": "Internal error",
    "data": {
      "details": "Database connection failed: timeout"
    }
  },
  "id": 1
}
```

## Debugging Tips

### Enable Debug Logging

```bash
# Full debug output
RUST_LOG=debug cargo run

# Module-specific debugging
RUST_LOG=solidmcp::transport=debug cargo run
RUST_LOG=solidmcp::handler=debug cargo run
RUST_LOG=solidmcp::protocol_impl=debug cargo run
```

### Use Request Inspection

```rust
// In your handler
async fn call_tool(
    &self,
    name: &str,
    arguments: Value,
    context: &McpContext,
) -> Result<Value> {
    debug!("Tool called: {} with args: {}", name, arguments);
    debug!("Session ID: {:?}", context.session_id);
    debug!("Protocol version: {:?}", context.protocol_version);
    
    // Handler logic...
}
```

### Test with curl

```bash
# Test initialize
curl -X POST http://localhost:3000/mcp \
     -H "Content-Type: application/json" \
     -d @- << EOF
{
  "jsonrpc": "2.0",
  "method": "initialize",
  "params": {
    "protocolVersion": "2025-06-18",
    "capabilities": {}
  },
  "id": 1
}
EOF

# Test with verbose output
curl -v -X POST http://localhost:3000/mcp \
     -H "Content-Type: application/json" \
     -d '{"jsonrpc":"2.0","method":"tools/list","id":2}'
```

### Common Debugging Patterns

1. **Check initialization first:**
   ```rust
   if !protocol_handler.initialized {
       return Err(anyhow::anyhow!("Not initialized"));
   }
   ```

2. **Add context to errors:**
   ```rust
   database.query(sql)
       .await
       .context("Failed to execute database query")?;
   ```

3. **Use structured logging:**
   ```rust
   info!(
       session_id = ?context.session_id,
       tool_name = name,
       "Executing tool"
   );
   ```

### Getting Help

If you're still experiencing issues:

1. Check the [GitHub Issues](https://github.com/johnblat/solidmcp/issues)
2. Enable debug logging and collect logs
3. Create a minimal reproduction example
4. Include your Rust version: `rustc --version`
5. Include your SolidMCP version from `Cargo.toml`

## Quick Fixes Checklist

- [ ] Server running on correct port?
- [ ] Client sending correct headers?
- [ ] Session cookies preserved (HTTP)?
- [ ] All required tool parameters provided?
- [ ] Using supported protocol version?
- [ ] No JSON syntax errors?
- [ ] Handler errors properly handled?
- [ ] Sufficient logging enabled?