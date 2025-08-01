# TODO-017: Add Emergency Health Check Endpoint

**Priority**: 🔴 CRITICAL  
**Effort**: 1-2 hours  
**Dependencies**: None  
**Category**: Operations, Monitoring

## 📋 Description

Add a basic health check endpoint that monitoring systems can use to verify the service is running. This is the minimum viable monitoring capability.

## 🎯 Acceptance Criteria

- [ ] `/health` endpoint returns 200 OK when healthy
- [ ] Health check includes basic system info
- [ ] No authentication required for health endpoint
- [ ] Response time under 100ms
- [ ] Works for both HTTP and WebSocket transports
- [ ] Includes session count in response

## 📊 Current State

```rust
// NO HEALTH CHECK EXISTS
// Monitoring systems cannot verify service status
// No way to check if service is accepting connections
```

## 🔧 Implementation

### 1. Create Health Check Handler

Create `src/health.rs`:
```rust
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use warp::{Reply, Rejection};

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: u64,
    pub version: String,
    pub session_count: usize,
    pub uptime_seconds: u64,
}

pub struct HealthChecker {
    start_time: SystemTime,
    engine: Arc<McpProtocolEngine<AppContext>>,
}

impl HealthChecker {
    pub fn new(engine: Arc<McpProtocolEngine<AppContext>>) -> Self {
        Self {
            start_time: SystemTime::now(),
            engine,
        }
    }
    
    pub async fn check_health(&self) -> Result<impl Reply, Rejection> {
        let now = SystemTime::now();
        let timestamp = now.duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let uptime = now.duration_since(self.start_time)
            .unwrap()
            .as_secs();
        
        let response = HealthResponse {
            status: "healthy".to_string(),
            timestamp,
            version: env!("CARGO_PKG_VERSION").to_string(),
            session_count: self.engine.session_count(),
            uptime_seconds: uptime,
        };
        
        Ok(warp::reply::json(&response))
    }
}
```

### 2. Add Health Route to HTTP Server

Update `src/http.rs`:
```rust
pub fn create_http_routes(
    engine: Arc<McpProtocolEngine<AppContext>>,
    health_checker: Arc<HealthChecker>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    // Health check route (no auth required)
    let health = warp::path("health")
        .and(warp::get())
        .and(with_health_checker(health_checker.clone()))
        .and_then(handle_health_check);
    
    // MCP route (existing)
    let mcp = warp::path("mcp")
        .and(warp::post())
        .and(warp::body::json())
        .and(warp::header::headers_cloned())
        .and(warp::cookie::optional("mcp_session"))
        .and(with_engine(engine))
        .and_then(handle_mcp_http);
    
    health.or(mcp)
}

async fn handle_health_check(
    health_checker: Arc<HealthChecker>,
) -> Result<impl Reply, Rejection> {
    health_checker.check_health().await
}

fn with_health_checker(
    health_checker: Arc<HealthChecker>,
) -> impl Filter<Extract = (Arc<HealthChecker>,), Error = Infallible> + Clone {
    warp::any().map(move || health_checker.clone())
}
```

### 3. Add WebSocket Health Support

Update `src/websocket.rs` to handle health check messages:
```rust
async fn handle_websocket_message(
    msg: Message,
    engine: Arc<McpProtocolEngine<AppContext>>,
    health_checker: Arc<HealthChecker>,
) -> Option<Message> {
    if let Ok(text) = msg.to_str() {
        // Check for health ping
        if text == "ping" || text == "health" {
            let health = health_checker.check_health().await.ok()?;
            return Some(Message::text(serde_json::to_string(&health).ok()?));
        }
        
        // Regular message processing
        // ... existing logic
    }
    None
}
```

### 4. Update Main Server

In `src/core.rs` or main server file:
```rust
pub struct McpServer {
    engine: Arc<McpProtocolEngine<AppContext>>,
    health_checker: Arc<HealthChecker>,
    // ... other fields
}

impl McpServer {
    pub fn new(context: AppContext) -> Self {
        let engine = Arc::new(McpProtocolEngine::new(context, None));
        let health_checker = Arc::new(HealthChecker::new(Arc::clone(&engine)));
        
        Self {
            engine,
            health_checker,
            // ...
        }
    }
    
    pub async fn start(self, addr: SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
        let routes = create_http_routes(
            Arc::clone(&self.engine),
            Arc::clone(&self.health_checker),
        );
        
        println!("Health check available at: http://{}/health", addr);
        
        warp::serve(routes).run(addr).await;
        Ok(())
    }
}
```

## 🧪 Testing

Create `tests/health_check_test.rs`:
```rust
use reqwest;

#[tokio::test]
async fn test_health_endpoint() {
    let server = start_test_server().await;
    let health_url = format!("http://{}/health", server.addr);
    
    let response = reqwest::get(&health_url).await.unwrap();
    
    assert_eq!(response.status(), 200);
    
    let health: HealthResponse = response.json().await.unwrap();
    assert_eq!(health.status, "healthy");
    assert!(health.uptime_seconds >= 0);
    assert_eq!(health.session_count, 0);
}

#[tokio::test]
async fn test_health_check_performance() {
    let server = start_test_server().await;
    let health_url = format!("http://{}/health", server.addr);
    
    let start = std::time::Instant::now();
    let response = reqwest::get(&health_url).await.unwrap();
    let duration = start.elapsed();
    
    assert!(response.status().is_success());
    assert!(duration.as_millis() < 100, "Health check took {}ms", duration.as_millis());
}

#[tokio::test]
async fn test_health_with_active_sessions() {
    let server = start_test_server().await;
    
    // Create some sessions
    for i in 0..5 {
        server.create_session(&format!("session_{}", i)).await;
    }
    
    let health_url = format!("http://{}/health", server.addr);
    let response = reqwest::get(&health_url).await.unwrap();
    let health: HealthResponse = response.json().await.unwrap();
    
    assert_eq!(health.session_count, 5);
}
```

## ✅ Verification

1. Start server and visit http://localhost:PORT/health
2. Verify JSON response with all fields
3. Test with curl: `curl http://localhost:PORT/health`
4. Add to monitoring system (Prometheus, DataDog, etc.)
5. Test WebSocket health: `wscat -c ws://localhost:PORT/mcp` then send "health"

## 📝 Notes

- Consider adding more detailed health checks later (database connectivity, etc.)
- May want to add `/metrics` endpoint for Prometheus
- Could add `/ready` endpoint for Kubernetes readiness probes
- Keep health check lightweight to avoid impacting performance