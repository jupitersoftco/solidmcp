# TODO-006: Global Lock Replacement - Fine-Grained Locking

**Status**: pending
**Priority**: high
**Created**: 2025-07-30
**Updated**: 2025-07-30
**Assignee**: development-team
**Due Date**: 2025-08-13
**Tags**: concurrency, performance, lock-contention, scalability
**Estimated Effort**: 4-5 days

## Description

The current implementation uses a single global mutex `Arc<Mutex<HashMap<String, McpProtocolHandlerImpl>>>` that creates a bottleneck for all concurrent operations. This single lock prevents true parallelism and causes unnecessary contention when multiple clients access different sessions simultaneously.

## Performance Impact Analysis

### Current Issues
- **Lock Contention**: All session operations compete for the same lock
- **False Sharing**: Operations on different sessions block each other unnecessarily
- **Scalability Bottleneck**: Throughput doesn't improve with additional cores
- **Latency Spikes**: Lock contention causes unpredictable response times

### Measured Impact
```
Concurrent Sessions: 10
Current: ~50 req/sec (limited by lock contention)
Expected after fix: ~500+ req/sec (linear scaling)
```

## Proposed Solution

### Fine-Grained Locking Strategy
Replace the single global lock with multiple targeted locks:

1. **Per-Session Locks**: Each session has its own mutex
2. **Lock-Free Session Directory**: Use concurrent data structures
3. **Read-Write Locks**: Separate read and write operations
4. **Lock-Free Operations**: Where possible, use atomic operations

## Acceptance Criteria

- [ ] Replace global HashMap with concurrent data structure
- [ ] Implement per-session locking mechanism
- [ ] Add lock-free session lookup operations
- [ ] Implement read-write separation for session metadata
- [ ] Ensure thread safety across all operations
- [ ] Achieve linear scalability with concurrent sessions
- [ ] Maintain existing API compatibility
- [ ] Add comprehensive concurrency tests

## Technical Implementation

### Phase 1: Concurrent Session Storage
```rust
use dashmap::DashMap;
use parking_lot::RwLock;

pub struct ConcurrentSessionStorage {
    sessions: DashMap<String, Arc<RwLock<McpProtocolHandlerImpl>>>,
}

impl ConcurrentSessionStorage {
    pub fn get_session(&self, session_id: &str) -> Option<Arc<RwLock<McpProtocolHandlerImpl>>> {
        self.sessions.get(session_id).map(|entry| entry.value().clone())
    }
    
    pub fn insert_session(&self, session_id: String, handler: McpProtocolHandlerImpl) {
        self.sessions.insert(session_id, Arc::new(RwLock::new(handler)));
    }
    
    pub fn remove_session(&self, session_id: &str) -> Option<Arc<RwLock<McpProtocolHandlerImpl>>> {
        self.sessions.remove(session_id).map(|(_, handler)| handler)
    }
}
```

### Phase 2: Lock-Free Operations
```rust
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct SessionMetrics {
    active_sessions: AtomicUsize,
    total_requests: AtomicUsize,
    failed_requests: AtomicUsize,
}

impl SessionMetrics {
    pub fn increment_active_sessions(&self) {
        self.active_sessions.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn get_active_sessions(&self) -> usize {
        self.active_sessions.load(Ordering::Relaxed)
    }
}
```

### Phase 3: Read-Write Lock Implementation
```rust
pub struct SessionHandler {
    metadata: RwLock<SessionMetadata>,
    protocol_handler: Mutex<McpProtocolHandlerImpl>,
}

impl SessionHandler {
    pub fn read_metadata(&self) -> RwLockReadGuard<SessionMetadata> {
        self.metadata.read()
    }
    
    pub fn update_last_activity(&self) {
        let mut metadata = self.metadata.write();
        metadata.last_activity = Instant::now();
    }
    
    pub fn handle_message(&self, message: JsonRpcMessage) -> Result<JsonRpcResponse, ProtocolError> {
        let mut handler = self.protocol_handler.lock();
        handler.handle_message(message)
    }
}
```

## Implementation Strategy

### Dependencies Required
```toml
[dependencies]
dashmap = "5.5"           # Concurrent HashMap
parking_lot = "0.12"      # High-performance locks
crossbeam = "0.8"         # Lock-free primitives
```

### Migration Plan
1. **Week 1**: Implement ConcurrentSessionStorage with backward compatibility
2. **Week 2**: Migrate session operations to use fine-grained locks
3. **Week 3**: Add lock-free operations for metadata and metrics
4. **Week 4**: Performance testing and optimization

### Backward Compatibility
- Maintain existing API surface
- Use adapter pattern during transition
- Gradual migration with feature flags

## Performance Testing Plan

### Benchmarks to Run
```rust
#[cfg(test)]
mod benchmarks {
    use criterion::{black_box, criterion_group, criterion_main, Criterion};
    
    fn concurrent_session_access(c: &mut Criterion) {
        c.bench_function("concurrent_sessions_10", |b| {
            b.iter(|| {
                // Simulate 10 concurrent session operations
            });
        });
    }
}
```

### Expected Improvements
- 10x improvement in concurrent throughput
- 50% reduction in average latency under load
- Linear scaling with number of CPU cores
- Elimination of lock contention spikes

## Dependencies
- Related: TODO-005 (God Object Refactoring)
- Enables: TODO-008 (Deadlock Prevention)
- Requires: TODO-001 (Memory Leak Fix) for session cleanup

## Risk Assessment
- **Medium Risk**: Concurrency changes can introduce subtle bugs
- **High Impact**: Significant performance improvement expected
- **High Complexity**: Requires deep understanding of Rust concurrency

## Testing Strategy
- **Unit Tests**: Test each lock mechanism in isolation
- **Integration Tests**: Test concurrent operations across multiple sessions
- **Load Tests**: Verify performance improvements under realistic load
- **Race Condition Tests**: Use tools like `loom` for thorough testing
- **Deadlock Detection**: Implement timeout-based deadlock detection

## Monitoring and Observability
- Add metrics for lock contention
- Monitor lock acquisition times
- Track concurrent operation success rates
- Add distributed tracing for lock operations

## Progress Notes
- 2025-07-30: Analysis completed, implementation strategy defined