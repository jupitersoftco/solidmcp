# TODO-001: Memory Leak Fix - Session Cleanup and Bounded Storage

**Status**: pending
**Priority**: critical
**Created**: 2025-07-30
**Updated**: 2025-07-30
**Assignee**: development-team
**Due Date**: 2025-08-06
**Tags**: memory-leak, performance, critical, security
**Estimated Effort**: 2-3 days

## Description

The `McpProtocolEngine` in `src/shared.rs` has an unbounded session storage using `Arc<Mutex<HashMap<String, McpProtocolHandlerImpl>>>` that grows indefinitely. This creates a memory leak as sessions are never cleaned up, leading to potential DoS attacks and memory exhaustion.

## Root Cause Analysis

1. Sessions are created but never removed from the HashMap
2. No TTL (time-to-live) mechanism for inactive sessions
3. No maximum session limit enforcement
4. HTTP sessions rely on cookies but cleanup is not implemented

## Acceptance Criteria

- [ ] Implement session TTL with configurable timeout (default: 1 hour)
- [ ] Add maximum session limit with LRU eviction (default: 1000 sessions)
- [ ] Create session cleanup background task
- [ ] Add session activity tracking for proper TTL management
- [ ] Implement graceful session termination with client notification
- [ ] Add metrics for session count and cleanup operations
- [ ] Create comprehensive tests for session lifecycle management
- [ ] Verify no memory growth under sustained load

## Technical Implementation

### Files to Modify
- `src/shared.rs` - Add session management logic
- `src/http/session.rs` - Implement HTTP session cleanup
- `src/websocket.rs` - Add WebSocket session cleanup
- `src/framework.rs` - Add session configuration options

### Implementation Steps
1. Create `SessionManager` struct with TTL and capacity limits
2. Replace raw HashMap with managed session storage
3. Implement background cleanup task with tokio::spawn
4. Add session activity tracking
5. Update session creation/access to refresh TTL
6. Add configuration options to McpServerBuilder

## Dependencies
- Blocks: TODO-005 (God Object Refactoring)
- Related: TODO-003 (Security Vulnerabilities)

## Risk Assessment
- **High Impact**: Memory exhaustion can crash the server
- **Medium Complexity**: Requires careful thread-safe implementation
- **Low Risk**: Well-understood problem with established patterns

## Testing Strategy
- Unit tests for SessionManager logic
- Integration tests for session cleanup under load
- Memory leak detection tests with valgrind/similar tools
- Stress tests with thousands of sessions

## Progress Notes
- 2025-07-30: Task created and analyzed