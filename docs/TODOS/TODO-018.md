# TODO-018: Replace Global Session Mutex with DashMap

**Priority**: 🟡 HIGH  
**Effort**: 4 hours  
**Dependencies**: TODO-016 (need limits in place first)  
**Category**: Performance, Scalability

## 📋 Description

Replace the global `Arc<Mutex<HashMap<String, Session>>>` with DashMap for lock-free concurrent access. This single mutex is a massive bottleneck causing all sessions to wait on each other.

## 🎯 Acceptance Criteria

- [ ] DashMap replaces Mutex<HashMap> for sessions
- [ ] No more global lock contention
- [ ] All existing tests pass
- [ ] Concurrent session operations verified
- [ ] No race conditions introduced

## 📊 Current State

```rust
// THE BOTTLENECK in src/shared.rs
sessions: Arc<Mutex<HashMap<String, McpProtocolHandlerImpl<C>>>>

// Every operation locks ALL sessions:
let mut sessions = self.sessions.lock().map_err(...)?; // BLOCKS EVERYTHING!
```

## 🔧 Implementation

### 1. Add DashMap Dependency

In `Cargo.toml`:
```toml
[dependencies]
dashmap = "5.5"
```

### 2. Update McpProtocolEngine

Replace the mutex with DashMap in `src/shared.rs`:
```rust
use dashmap::DashMap;

pub struct McpProtocolEngine<C> {
    /// Lock-free concurrent session storage
    sessions: Arc<DashMap<String, McpProtocolHandlerImpl<C>>>,
    custom_handler: Option<Arc<dyn McpHandler<C>>>,
    context: Arc<C>,
    limits: ResourceLimits,
}

impl<C: Send + Sync + 'static> McpProtocolEngine<C> {
    pub fn new(context: C, custom_handler: Option<Arc<dyn McpHandler<C>>>) -> Self {
        Self {
            sessions: Arc::new(DashMap::new()),
            custom_handler,
            context: Arc::new(context),
            limits: ResourceLimits::default(),
        }
    }

    pub async fn process_message(
        &self,
        session_id: &str,
        message: Value,
        progress_sender: Option<mpsc::UnboundedSender<Value>>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        // No more lock! Get or create session atomically
        let handler = self.sessions
            .entry(session_id.to_string())
            .or_insert_with(|| {
                McpProtocolHandlerImpl::new(
                    Arc::clone(&self.context),
                    self.custom_handler.clone(),
                )
            })
            .clone();

        // Process without holding any locks
        handler.handle_message(message, progress_sender).await
    }

    pub fn get_session_info(&self, session_id: &str) -> Option<SessionInfo> {
        // Direct access, no lock needed
        self.sessions.get(session_id).map(|handler| SessionInfo {
            initialized: handler.is_initialized(),
            client_info: handler.get_client_info().cloned(),
        })
    }

    pub fn remove_session(&self, session_id: &str) {
        // Atomic removal
        self.sessions.remove(session_id);
    }

    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    pub fn clear_inactive_sessions(&self, inactive_threshold: Duration) {
        // Can iterate while others access - no global lock!
        self.sessions.retain(|_, handler| {
            handler.last_activity() > SystemTime::now() - inactive_threshold
        });
    }
}
```

### 3. Update Session Handler for Thread Safety

Ensure `McpProtocolHandlerImpl` tracks activity in `src/protocol_impl.rs`:
```rust
pub struct McpProtocolHandlerImpl<C> {
    initialized: AtomicBool,
    client_info: RwLock<Option<ClientInfo>>,
    protocol_version: RwLock<Option<String>>,
    context: Arc<C>,
    custom_handler: Option<Arc<dyn McpHandler<C>>>,
    last_activity: AtomicU64, // Track for cleanup
}

impl<C> McpProtocolHandlerImpl<C> {
    pub fn new(context: Arc<C>, custom_handler: Option<Arc<dyn McpHandler<C>>>) -> Self {
        Self {
            initialized: AtomicBool::new(false),
            client_info: RwLock::new(None),
            protocol_version: RwLock::new(None),
            context,
            custom_handler,
            last_activity: AtomicU64::new(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            ),
        }
    }

    pub async fn handle_message(&self, message: Value, ...) -> Result<Value, ...> {
        // Update activity timestamp
        self.last_activity.store(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            Ordering::Relaxed
        );
        
        // ... rest of message handling
    }

    pub fn last_activity(&self) -> SystemTime {
        let secs = self.last_activity.load(Ordering::Relaxed);
        UNIX_EPOCH + Duration::from_secs(secs)
    }
}
```

## 🧪 Testing

Create `tests/concurrent_sessions_test.rs`:
```rust
#[tokio::test]
async fn test_concurrent_session_access() {
    let engine = create_test_engine();
    let num_sessions = 100;
    let operations_per_session = 50;
    
    // Spawn concurrent tasks
    let handles: Vec<_> = (0..num_sessions)
        .map(|i| {
            let engine_clone = Arc::clone(&engine);
            tokio::spawn(async move {
                let session_id = format!("session_{}", i);
                
                for j in 0..operations_per_session {
                    let result = engine_clone.process_message(
                        &session_id,
                        json!({
                            "jsonrpc": "2.0",
                            "id": j,
                            "method": "test",
                            "params": {}
                        }),
                        None
                    ).await;
                    
                    assert!(result.is_ok());
                }
            })
        })
        .collect();
    
    // Wait for all to complete
    for handle in handles {
        handle.await.unwrap();
    }
    
    // Verify all sessions created
    assert_eq!(engine.session_count(), num_sessions);
}

#[tokio::test]
async fn test_no_lock_contention() {
    let engine = create_test_engine();
    let start = Instant::now();
    
    // Parallel operations that would block with mutex
    let futures: Vec<_> = (0..1000)
        .map(|i| {
            let engine = Arc::clone(&engine);
            async move {
                engine.get_session_info(&format!("session_{}", i))
            }
        })
        .collect();
    
    futures::future::join_all(futures).await;
    
    let duration = start.elapsed();
    // Should be fast - under 100ms for 1000 operations
    assert!(duration.as_millis() < 100);
}
```

## ✅ Verification

1. Run concurrent session tests
2. Benchmark before/after performance
3. Verify no deadlocks under load
4. Test session cleanup works correctly
5. Monitor CPU usage during concurrent access

## 📝 Notes

- DashMap handles sharding internally for better performance
- No need for read/write locks - all operations are atomic
- Consider adding metrics for lock contention (before/after)
- May want to add session pooling later for even better performance