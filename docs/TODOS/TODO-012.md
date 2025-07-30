# TODO-012: Resource Leak Prevention - Proper Connection Cleanup

**Status**: pending
**Priority**: medium
**Created**: 2025-07-30
**Updated**: 2025-07-30
**Assignee**: development-team
**Due Date**: 2025-08-22
**Tags**: resource-leaks, memory-management, connection-cleanup, reliability
**Estimated Effort**: 3-4 days

## Description

The codebase has potential resource leaks in WebSocket and HTTP connection handling. Connections may not be properly cleaned up when clients disconnect unexpectedly, leading to resource exhaustion over time. This includes file descriptors, memory allocations, and associated session state.

## Identified Resource Leak Scenarios

### 1. WebSocket Connection Leaks
```rust
// Current: No guaranteed cleanup on disconnect
async fn handle_websocket(socket: WebSocket) {
    while let Some(msg) = socket.recv().await {
        // Process message
        // What happens if client disconnects abruptly?
        // Resources may not be cleaned up properly
    }
    // Cleanup code may never be reached
}
```

### 2. HTTP Session Resource Leaks
- Session data remains in memory after client stops making requests
- Associated file handles and network resources not released
- Background tasks continue running for inactive sessions
- Metrics and monitoring data accumulates without bounds

### 3. File Descriptor Leaks
- Open sockets not properly closed on connection errors
- File handles in resource providers not released
- Temporary files not cleaned up on errors
- Log file handles accumulating

### 4. Memory Leaks in Async Context
- Tokio tasks not properly cancelled on disconnect
- Circular references preventing garbage collection
- Event listeners not unregistered
- Buffer allocations not released

## Impact Analysis

### Resource Consumption Growth
```
Time: 1 hour  -> File Descriptors: 50,   Memory: 10MB
Time: 6 hours -> File Descriptors: 300,  Memory: 60MB
Time: 24 hours -> File Descriptors: 1200, Memory: 240MB
Time: 1 week  -> File Descriptors: 8400, Memory: 1.7GB
```

### System Limits
- **File Descriptors**: OS limit typically 1024-65536
- **Memory**: Depends on available system memory
- **Network Connections**: Limited by OS and network stack
- **Process Resources**: CPU time, thread count

## Acceptance Criteria

- [ ] Implement proper WebSocket connection cleanup with Drop trait
- [ ] Add HTTP session cleanup with timeout-based expiration
- [ ] Create resource tracking and monitoring system
- [ ] Implement graceful shutdown for all connection types
- [ ] Add connection leak detection in tests
- [ ] Ensure all async tasks are properly cancelled on disconnect
- [ ] Add comprehensive resource cleanup tests
- [ ] Implement circuit breaker for resource exhaustion scenarios

## Technical Implementation

### Phase 1: Connection Lifecycle Management
```rust
// src/connection/lifecycle.rs
pub struct ConnectionManager {
    active_connections: Arc<DashMap<ConnectionId, ConnectionHandle>>,
    cleanup_interval: Duration,
    max_connections: usize,
}

pub struct ConnectionHandle {
    id: ConnectionId,
    connection_type: ConnectionType,
    created_at: Instant,
    last_activity: Arc<AtomicInstant>,
    cleanup_tasks: Vec<tokio::task::JoinHandle<()>>,
}

impl Drop for ConnectionHandle {
    fn drop(&mut self) {
        // Ensure all resources are cleaned up
        for task in self.cleanup_tasks.drain(..) {
            task.abort();
        }
        info!("Connection {} cleaned up", self.id);
    }
}

impl ConnectionManager {
    pub async fn start_cleanup_task(&self) {
        let connections = self.active_connections.clone();
        let interval = self.cleanup_interval;
        
        tokio::spawn(async move {
            let mut cleanup_timer = tokio::time::interval(interval);
            loop {
                cleanup_timer.tick().await;
                Self::cleanup_inactive_connections(&connections).await;
            }
        });
    }
    
    async fn cleanup_inactive_connections(connections: &DashMap<ConnectionId, ConnectionHandle>) {
        let now = Instant::now();
        let timeout = Duration::from_secs(300); // 5 minutes
        
        let inactive_connections: Vec<ConnectionId> = connections
            .iter()
            .filter_map(|entry| {
                let handle = entry.value();
                let last_activity = handle.last_activity.load(Ordering::Acquire);
                if now.duration_since(last_activity) > timeout {
                    Some(*entry.key())
                } else {
                    None
                }
            })
            .collect();
        
        for connection_id in inactive_connections {
            if let Some((_, handle)) = connections.remove(&connection_id) {
                info!("Cleaned up inactive connection: {}", connection_id);
                drop(handle); // Explicit cleanup
            }
        }
    }
}
```

### Phase 2: WebSocket Resource Management
```rust
// src/websocket/connection.rs
pub struct WebSocketConnection {
    socket: WebSocket,
    session_id: SessionId,
    connection_id: ConnectionId,
    _cleanup_guard: ConnectionCleanupGuard,
}

pub struct ConnectionCleanupGuard {
    connection_id: ConnectionId,
    session_manager: Arc<SessionManager>,
    connection_manager: Arc<ConnectionManager>,
}

impl Drop for ConnectionCleanupGuard {
    fn drop(&mut self) {
        // Clean up session state
        self.session_manager.remove_session(&self.connection_id.to_session_id());
        
        // Remove from connection tracking
        self.connection_manager.remove_connection(&self.connection_id);
        
        info!("WebSocket connection {} resources cleaned up", self.connection_id);
    }
}

impl WebSocketConnection {
    pub fn new(socket: WebSocket, session_id: SessionId, managers: (Arc<SessionManager>, Arc<ConnectionManager>)) -> Self {
        let connection_id = ConnectionId::new();
        let cleanup_guard = ConnectionCleanupGuard {
            connection_id,
            session_manager: managers.0,
            connection_manager: managers.1,
        };
        
        Self {
            socket,
            session_id,
            connection_id,
            _cleanup_guard: cleanup_guard,
        }
    }
    
    pub async fn handle_messages(mut self) -> Result<(), WebSocketError> {
        // Set up cancellation token for graceful shutdown
        let cancellation_token = CancellationToken::new();
        let token_clone = cancellation_token.clone();
        
        // Spawn heartbeat task
        let heartbeat_task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        // Send heartbeat
                        if let Err(_) = self.socket.send(Message::Ping(vec![])).await {
                            break;
                        }
                    }
                    _ = token_clone.cancelled() => {
                        break;
                    }
                }
            }
        });
        
        // Main message loop with proper error handling
        let result = async {
            while let Some(msg) = self.socket.recv().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        self.handle_text_message(text).await?;
                    }
                    Ok(Message::Binary(_)) => {
                        return Err(WebSocketError::UnsupportedMessageType);
                    }
                    Ok(Message::Close(_)) => {
                        info!("WebSocket connection closed gracefully");
                        break;
                    }
                    Ok(Message::Ping(data)) => {
                        self.socket.send(Message::Pong(data)).await?;
                    }
                    Ok(Message::Pong(_)) => {
                        // Update last activity
                        self._cleanup_guard.connection_manager
                            .update_activity(&self.connection_id);
                    }
                    Err(e) => {
                        warn!("WebSocket error: {}", e);
                        break;
                    }
                }
            }
            Ok(())
        }.await;
        
        // Cancel heartbeat task
        cancellation_token.cancel();
        heartbeat_task.await.ok();
        
        result
    }
}
```

### Phase 3: HTTP Session Cleanup
```rust
// src/http/session_cleanup.rs
pub struct HttpSessionCleanup {
    session_store: Arc<DashMap<SessionId, HttpSession>>,
    cleanup_interval: Duration,
    session_timeout: Duration,
}

#[derive(Debug)]
pub struct HttpSession {
    id: SessionId,
    created_at: Instant,
    last_access: Arc<AtomicInstant>,
    protocol_handler: Arc<Mutex<McpProtocolHandlerImpl>>,
    resource_handles: Vec<ResourceHandle>,
}

impl Drop for HttpSession {
    fn drop(&mut self) {
        // Clean up all associated resources
        for handle in self.resource_handles.drain(..) {
            handle.cleanup();
        }
        info!("HTTP session {} cleaned up", self.id);
    }
}

impl HttpSessionCleanup {
    pub fn start_cleanup_task(&self) -> tokio::task::JoinHandle<()> {
        let session_store = self.session_store.clone();
        let interval = self.cleanup_interval;
        let timeout = self.session_timeout;
        
        tokio::spawn(async move {
            let mut cleanup_timer = tokio::time::interval(interval);
            loop {
                cleanup_timer.tick().await;
                
                let now = Instant::now();
                let expired_sessions: Vec<SessionId> = session_store
                    .iter()
                    .filter_map(|entry| {
                        let session = entry.value();
                        let last_access = session.last_access.load(Ordering::Acquire);
                        if now.duration_since(last_access) > timeout {
                            Some(session.id.clone())
                        } else {
                            None
                        }
                    })
                    .collect();
                
                for session_id in expired_sessions {
                    if let Some((_, session)) = session_store.remove(&session_id) {
                        info!("Expired HTTP session cleaned up: {}", session_id);
                        drop(session); // Explicit cleanup
                    }
                }
            }
        })
    }
}
```

### Phase 4: Resource Tracking and Monitoring
```rust
// src/monitoring/resource_tracker.rs
pub struct ResourceTracker {
    active_connections: AtomicUsize,
    active_sessions: AtomicUsize,
    open_file_descriptors: AtomicUsize,
    memory_usage: AtomicUsize,
    peak_connections: AtomicUsize,
}

impl ResourceTracker {
    pub fn track_connection_created(&self) {
        let count = self.active_connections.fetch_add(1, Ordering::Relaxed) + 1;
        let peak = self.peak_connections.load(Ordering::Relaxed);
        if count > peak {
            self.peak_connections.store(count, Ordering::Relaxed);
        }
        
        if count > MAX_CONNECTIONS {
            warn!("High connection count: {}", count);
        }
    }
    
    pub fn track_connection_closed(&self) {
        self.active_connections.fetch_sub(1, Ordering::Relaxed);
    }
    
    pub fn get_metrics(&self) -> ResourceMetrics {
        ResourceMetrics {
            active_connections: self.active_connections.load(Ordering::Relaxed),
            active_sessions: self.active_sessions.load(Ordering::Relaxed),
            open_file_descriptors: self.open_file_descriptors.load(Ordering::Relaxed),
            memory_usage: self.memory_usage.load(Ordering::Relaxed),
            peak_connections: self.peak_connections.load(Ordering::Relaxed),
        }
    }
    
    pub fn check_resource_limits(&self) -> Result<(), ResourceExhaustionError> {
        let metrics = self.get_metrics();
        
        if metrics.active_connections > MAX_CONNECTIONS {
            return Err(ResourceExhaustionError::TooManyConnections(metrics.active_connections));
        }
        
        if metrics.open_file_descriptors > MAX_FILE_DESCRIPTORS {
            return Err(ResourceExhaustionError::TooManyFileDescriptors(metrics.open_file_descriptors));
        }
        
        Ok(())
    }
}
```

## Testing Strategy

### Resource Leak Detection Tests
```rust
#[cfg(test)]
mod resource_leak_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_websocket_connection_cleanup() {
        let tracker = ResourceTracker::new();
        let initial_connections = tracker.active_connections.load(Ordering::Relaxed);
        
        // Create and drop connection
        {
            let connection = create_test_websocket_connection(&tracker).await;
            assert_eq!(tracker.active_connections.load(Ordering::Relaxed), initial_connections + 1);
        } // Connection dropped here
        
        // Give time for cleanup
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Verify cleanup
        assert_eq!(tracker.active_connections.load(Ordering::Relaxed), initial_connections);
    }
    
    #[tokio::test]
    async fn test_session_timeout_cleanup() {
        let session_cleanup = HttpSessionCleanup::new(Duration::from_millis(100));
        let session_id = session_cleanup.create_session().await;
        
        // Wait for timeout
        tokio::time::sleep(Duration::from_millis(200)).await;
        
        // Session should be cleaned up
        assert!(session_cleanup.get_session(&session_id).is_none());
    }
    
    #[tokio::test]
    async fn test_resource_limit_enforcement() {
        let tracker = ResourceTracker::new();
        
        // Create connections up to limit
        let mut connections = Vec::new();
        for _ in 0..MAX_CONNECTIONS {
            connections.push(create_test_connection(&tracker).await);
        }
        
        // Next connection should fail
        assert!(tracker.check_resource_limits().is_err());
    }
}
```

### Memory Leak Detection
```rust
// Use tools like valgrind or AddressSanitizer for memory leak detection
#[cfg(test)]
mod memory_tests {
    #[test]
    #[ignore] // Run manually with memory debugging tools
    fn test_no_memory_leaks_under_load() {
        // Create many connections and let them drop
        // Monitor memory usage to ensure no leaks
    }
}
```

## Implementation Timeline

### Phase 1 (Days 1-2): Connection Management
- Implement ConnectionManager and ConnectionHandle
- Add Drop trait implementations for proper cleanup
- Create connection tracking system

### Phase 2 (Days 2-3): Transport-Specific Cleanup
- Implement WebSocket connection cleanup
- Add HTTP session cleanup mechanisms
- Ensure proper resource release in all code paths

### Phase 3 (Days 3-4): Monitoring and Testing
- Add resource tracking and monitoring
- Implement comprehensive leak detection tests
- Add circuit breakers for resource exhaustion

## Expected Benefits

### System Reliability
- **No Resource Exhaustion**: Proper cleanup prevents resource leaks
- **Predictable Performance**: Consistent resource usage over time
- **Better Scalability**: Can handle more concurrent connections safely
- **Graceful Degradation**: Proper handling of resource limits

### Operational Benefits
- **Monitoring**: Clear visibility into resource usage
- **Debugging**: Better tools for diagnosing resource issues
- **Maintenance**: No need to restart servers due to resource leaks

## Dependencies
- Related: TODO-010 (Tight Coupling Reduction) - better separation of concerns
- Requires: Good understanding of Rust ownership and RAII patterns
- Enables: Better system reliability and scalability

## Risk Assessment
- **Low Risk**: Resource cleanup typically improves system stability
- **High Impact**: Prevents system failures due to resource exhaustion
- **Medium Complexity**: Requires careful understanding of async Rust patterns

## Progress Notes
- 2025-07-30: Resource leak analysis completed, cleanup strategy designed