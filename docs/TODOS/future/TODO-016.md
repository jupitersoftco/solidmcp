# TODO-016: Add Resource Limits and DoS Protection

**Priority**: 🔴 CRITICAL  
**Effort**: 4 hours  
**Dependencies**: TODO-015 (security foundation)  
**Category**: Security, Scalability

## 📋 Description

Implement resource limits to prevent DoS attacks through resource exhaustion. Currently, the system has no limits on sessions, message sizes, or concurrent operations.

## 🎯 Acceptance Criteria

- [ ] Session count limits enforced
- [ ] Message size limits implemented
- [ ] Connection rate limiting added
- [ ] Memory usage caps in place
- [ ] Graceful rejection with proper error codes
- [ ] Metrics for rejected requests

## 📊 Current State

```rust
// VULNERABLE: Unlimited sessions
sessions: Arc<Mutex<HashMap<String, McpProtocolHandlerImpl>>>

// VULNERABLE: No message size validation
let request: JsonRpcRequest = serde_json::from_str(&message)?;

// VULNERABLE: No connection limits
// Anyone can create unlimited connections
```

## 🔧 Implementation

### 1. Create Resource Limits Configuration

Create `src/config/limits.rs`:
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub max_sessions: usize,
    pub max_message_size: usize,
    pub max_connections_per_ip: usize,
    pub connection_rate_per_minute: usize,
    pub max_memory_per_session: usize,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_sessions: 10_000,
            max_message_size: 2 * 1024 * 1024, // 2MB
            max_connections_per_ip: 100,
            connection_rate_per_minute: 60,
            max_memory_per_session: 10 * 1024 * 1024, // 10MB
        }
    }
}
```

### 2. Implement Session Limiter

Update `src/shared.rs`:
```rust
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct McpProtocolEngine<C> {
    sessions: Arc<Mutex<HashMap<String, McpProtocolHandlerImpl<C>>>>,
    custom_handler: Option<Arc<dyn McpHandler<C>>>,
    context: Arc<C>,
    limits: ResourceLimits,
    active_sessions: Arc<AtomicUsize>,
}

impl<C: Send + Sync + 'static> McpProtocolEngine<C> {
    pub async fn process_message(
        &self,
        session_id: &str,
        message: Value,
        progress_sender: Option<mpsc::UnboundedSender<Value>>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        // Check message size
        let message_size = serde_json::to_vec(&message)?.len();
        if message_size > self.limits.max_message_size {
            return Err(McpError::MessageTooLarge(message_size).into());
        }
        
        // Get or create session with limit check
        let handler = {
            let mut sessions = self.sessions.lock()
                .map_err(|e| format!("Session lock error: {}", e))?;
            
            if !sessions.contains_key(session_id) {
                // Check session limit
                let current_sessions = self.active_sessions.load(Ordering::Relaxed);
                if current_sessions >= self.limits.max_sessions {
                    return Err(McpError::TooManySessions.into());
                }
                
                // Create new session
                self.active_sessions.fetch_add(1, Ordering::Relaxed);
                sessions.insert(
                    session_id.to_string(),
                    McpProtocolHandlerImpl::new(
                        Arc::clone(&self.context),
                        self.custom_handler.clone(),
                    )
                );
            }
            
            sessions.get(session_id).unwrap().clone()
        };
        
        handler.handle_message(message, progress_sender).await
    }
    
    pub fn remove_session(&self, session_id: &str) {
        if let Ok(mut sessions) = self.sessions.lock() {
            if sessions.remove(session_id).is_some() {
                self.active_sessions.fetch_sub(1, Ordering::Relaxed);
            }
        }
    }
}
```

### 3. Add Connection Rate Limiting

Create `src/middleware/rate_limit.rs`:
```rust
use std::collections::HashMap;
use std::net::IpAddr;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

pub struct RateLimiter {
    limits: ResourceLimits,
    connections: Arc<Mutex<HashMap<IpAddr, ConnectionInfo>>>,
}

struct ConnectionInfo {
    count: usize,
    last_reset: Instant,
    recent_connections: Vec<Instant>,
}

impl RateLimiter {
    pub fn new(limits: ResourceLimits) -> Self {
        Self {
            limits,
            connections: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    pub async fn check_connection(&self, ip: IpAddr) -> Result<(), McpError> {
        let mut connections = self.connections.lock().await;
        let now = Instant::now();
        
        let info = connections.entry(ip).or_insert(ConnectionInfo {
            count: 0,
            last_reset: now,
            recent_connections: Vec::new(),
        });
        
        // Clean old connections
        info.recent_connections.retain(|&time| {
            now.duration_since(time) < Duration::from_secs(60)
        });
        
        // Check rate limit
        if info.recent_connections.len() >= self.limits.connection_rate_per_minute {
            return Err(McpError::RateLimitExceeded);
        }
        
        // Check total connections
        if info.count >= self.limits.max_connections_per_ip {
            return Err(McpError::TooManyConnections);
        }
        
        info.count += 1;
        info.recent_connections.push(now);
        
        Ok(())
    }
    
    pub async fn remove_connection(&self, ip: IpAddr) {
        if let Some(info) = self.connections.lock().await.get_mut(&ip) {
            info.count = info.count.saturating_sub(1);
        }
    }
}
```

### 4. Update HTTP Handler

In `src/http.rs`:
```rust
pub async fn handle_mcp_http(
    message: Value,
    headers: HttpHeaders,
    cookie: Option<String>,
    engine: Arc<McpProtocolEngine<AppContext>>,
    rate_limiter: Arc<RateLimiter>,
) -> Result<Response<Body>, Box<dyn std::error::Error + Send + Sync>> {
    // Extract IP address
    let ip = extract_ip_from_headers(&headers)?;
    
    // Check rate limit
    rate_limiter.check_connection(ip).await?;
    
    // Existing logic...
    
    // On connection close:
    defer! {
        rate_limiter.remove_connection(ip).await;
    }
}
```

## 🧪 Testing

Create `tests/resource_limits_test.rs`:
```rust
#[tokio::test]
async fn test_session_limit_enforced() {
    let limits = ResourceLimits {
        max_sessions: 2,
        ..Default::default()
    };
    let engine = create_test_engine_with_limits(limits);
    
    // Create max sessions
    for i in 0..2 {
        let result = engine.process_message(
            &format!("session_{}", i),
            json!({"method": "initialize"}),
            None
        ).await;
        assert!(result.is_ok());
    }
    
    // Try to exceed limit
    let result = engine.process_message(
        "session_3",
        json!({"method": "initialize"}),
        None
    ).await;
    
    assert!(matches!(result, Err(e) if e.to_string().contains("Too many sessions")));
}

#[tokio::test]
async fn test_message_size_limit() {
    let engine = create_test_engine();
    let large_message = json!({
        "method": "test",
        "params": {
            "data": "x".repeat(3 * 1024 * 1024) // 3MB
        }
    });
    
    let result = engine.process_message("test", large_message, None).await;
    assert!(matches!(result, Err(e) if e.to_string().contains("Message too large")));
}

#[tokio::test]
async fn test_rate_limiting() {
    let limiter = RateLimiter::new(ResourceLimits {
        connection_rate_per_minute: 2,
        ..Default::default()
    });
    
    let ip = "127.0.0.1".parse().unwrap();
    
    // First two should succeed
    assert!(limiter.check_connection(ip).await.is_ok());
    assert!(limiter.check_connection(ip).await.is_ok());
    
    // Third should fail
    assert!(matches!(
        limiter.check_connection(ip).await,
        Err(McpError::RateLimitExceeded)
    ));
}
```

## ✅ Verification

1. Run resource limit tests
2. Attempt to create excessive sessions
3. Send oversized messages
4. Test rate limiting with rapid connections
5. Monitor memory usage under load

## 📝 Notes

- Consider making limits configurable via environment variables
- Add metrics for rejected requests
- Consider implementing backpressure mechanisms
- May need to add IP allowlist for trusted sources