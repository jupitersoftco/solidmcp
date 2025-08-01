# TODO-033: Add Metrics Collection

**Priority**: 🟤 MEDIUM  
**Effort**: 4 hours  
**Dependencies**: TODO-020 (tracing), TODO-032 (metrics deps)  
**Category**: Observability

## 📋 Description

Add Prometheus-compatible metrics for monitoring system health, performance, and usage. This enables real production monitoring and alerting.

## 🎯 Acceptance Criteria

- [ ] Key metrics exposed on /metrics endpoint
- [ ] No performance impact on hot paths
- [ ] Prometheus format output
- [ ] Grafana dashboard template provided
- [ ] Metrics documented

## 📊 Current State

```rust
// NO METRICS AT ALL
// Can't monitor:
// - Request rates
// - Error rates  
// - Response times
// - Session counts
// - Resource usage
```

## 🔧 Implementation

### 1. Add Metrics Dependencies

In `Cargo.toml`:
```toml
[dependencies]
metrics = "0.21"
metrics-exporter-prometheus = "0.12"
```

### 2. Define Core Metrics

Create `src/metrics.rs`:
```rust
use metrics::{counter, gauge, histogram, describe_counter, describe_gauge, describe_histogram};
use std::time::Instant;

/// Initialize metric descriptions
pub fn init_metrics() {
    describe_counter!(
        "mcp_requests_total",
        "Total number of MCP requests by method"
    );
    
    describe_counter!(
        "mcp_errors_total", 
        "Total number of errors by type"
    );
    
    describe_histogram!(
        "mcp_request_duration_seconds",
        "Request duration in seconds by method"
    );
    
    describe_gauge!(
        "mcp_sessions_active",
        "Number of active sessions"
    );
    
    describe_gauge!(
        "mcp_memory_usage_bytes",
        "Memory usage in bytes"
    );
    
    describe_histogram!(
        "mcp_tool_execution_seconds",
        "Tool execution time in seconds"
    );
}

/// Track request metrics
pub struct RequestMetrics {
    start: Instant,
    method: String,
}

impl RequestMetrics {
    pub fn new(method: &str) -> Self {
        counter!("mcp_requests_total", "method" => method.to_string()).increment(1);
        
        Self {
            start: Instant::now(),
            method: method.to_string(),
        }
    }
    
    pub fn success(self) {
        let duration = self.start.elapsed().as_secs_f64();
        histogram!(
            "mcp_request_duration_seconds",
            "method" => self.method,
            "status" => "success"
        ).record(duration);
    }
    
    pub fn error(self, error_type: &str) {
        let duration = self.start.elapsed().as_secs_f64();
        
        counter!(
            "mcp_errors_total",
            "method" => self.method.clone(),
            "error" => error_type.to_string()
        ).increment(1);
        
        histogram!(
            "mcp_request_duration_seconds", 
            "method" => self.method,
            "status" => "error"
        ).record(duration);
    }
}

/// Track active sessions
pub fn set_active_sessions(count: usize) {
    gauge!("mcp_sessions_active").set(count as f64);
}

/// Track tool execution
pub fn track_tool_execution(tool_name: &str, duration: f64, success: bool) {
    histogram!(
        "mcp_tool_execution_seconds",
        "tool" => tool_name.to_string(),
        "status" => if success { "success" } else { "error" }
    ).record(duration);
}
```

### 3. Instrument Protocol Handler

Update `src/protocol_impl.rs`:
```rust
use crate::metrics::{RequestMetrics, track_tool_execution};

impl<C> McpProtocolHandlerImpl<C> {
    pub async fn handle_message(
        &self,
        message: Value,
        progress_sender: Option<mpsc::UnboundedSender<Value>>,
    ) -> McpResult<Value> {
        let method = message.get("method")
            .and_then(|m| m.as_str())
            .unwrap_or("unknown");
        
        let metrics = RequestMetrics::new(method);
        
        match self.handle_message_inner(message, progress_sender).await {
            Ok(response) => {
                metrics.success();
                Ok(response)
            }
            Err(e) => {
                metrics.error(&error_type(&e));
                Err(e)
            }
        }
    }
    
    async fn handle_call_tool(&self, params: ToolCallParams) -> McpResult<Value> {
        let start = Instant::now();
        let tool_name = &params.name;
        
        let result = self.call_tool_inner(params).await;
        
        let duration = start.elapsed().as_secs_f64();
        track_tool_execution(tool_name, duration, result.is_ok());
        
        result
    }
}

fn error_type(error: &McpError) -> &'static str {
    match error {
        McpError::UnknownMethod(_) => "unknown_method",
        McpError::InvalidParams(_) => "invalid_params",
        McpError::NotInitialized => "not_initialized",
        McpError::UnknownTool(_) => "unknown_tool",
        McpError::RateLimitExceeded => "rate_limit",
        _ => "internal",
    }
}
```

### 4. Add Session Metrics

Update `src/shared.rs`:
```rust
use crate::metrics::set_active_sessions;

impl<C: Send + Sync + 'static> McpProtocolEngine<C> {
    pub async fn process_message(
        &self,
        session_id: &str,
        message: Value,
        progress_sender: Option<mpsc::UnboundedSender<Value>>,
    ) -> McpResult<Value> {
        let result = self.process_message_inner(session_id, message, progress_sender).await;
        
        // Update session count metric
        set_active_sessions(self.session_count());
        
        result
    }
    
    pub fn remove_session(&self, session_id: &str) {
        self.sessions.remove(session_id);
        set_active_sessions(self.session_count());
    }
}
```

### 5. Add Metrics Endpoint

Update `src/http.rs`:
```rust
use metrics_exporter_prometheus::PrometheusHandle;

pub fn create_http_routes(
    engine: Arc<McpProtocolEngine<AppContext>>,
    health_checker: Arc<HealthChecker>,
    metrics_handle: PrometheusHandle,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    // Existing routes...
    
    // Metrics endpoint
    let metrics = warp::path("metrics")
        .and(warp::get())
        .map(move || {
            let metrics = metrics_handle.render();
            Response::builder()
                .header("Content-Type", "text/plain; version=0.0.4")
                .body(metrics)
                .unwrap()
        });
    
    health.or(mcp).or(metrics)
}
```

### 6. Initialize Metrics in Main

Update server initialization:
```rust
use metrics_exporter_prometheus::PrometheusBuilder;

fn main() {
    // Initialize metrics
    crate::metrics::init_metrics();
    
    let metrics_handle = PrometheusBuilder::new()
        .set_buckets(&[0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0])
        .unwrap()
        .install_recorder()
        .unwrap();
    
    // Start server with metrics
    let server = McpServerBuilder::new()
        .with_metrics(metrics_handle)
        .build();
}
```

### 7. Create Grafana Dashboard

Create `monitoring/dashboard.json`:
```json
{
  "dashboard": {
    "title": "SolidMCP Metrics",
    "panels": [
      {
        "title": "Request Rate",
        "targets": [{
          "expr": "rate(mcp_requests_total[5m])"
        }]
      },
      {
        "title": "Error Rate",
        "targets": [{
          "expr": "rate(mcp_errors_total[5m])"
        }]
      },
      {
        "title": "Response Time (p95)",
        "targets": [{
          "expr": "histogram_quantile(0.95, rate(mcp_request_duration_seconds_bucket[5m]))"
        }]
      },
      {
        "title": "Active Sessions",
        "targets": [{
          "expr": "mcp_sessions_active"
        }]
      }
    ]
  }
}
```

## 🧪 Testing

```rust
#[test]
fn test_metrics_collection() {
    init_metrics();
    
    let metrics = RequestMetrics::new("test_method");
    metrics.success();
    
    // Verify metric was recorded
    let rendered = handle.render();
    assert!(rendered.contains("mcp_requests_total"));
    assert!(rendered.contains("test_method"));
}

#[tokio::test]
async fn test_metrics_endpoint() {
    let server = start_test_server_with_metrics().await;
    
    let response = reqwest::get(&format!("http://{}/metrics", server.addr))
        .await
        .unwrap();
    
    assert_eq!(response.status(), 200);
    let body = response.text().await.unwrap();
    assert!(body.contains("# TYPE mcp_requests_total counter"));
}
```

## ✅ Verification

1. Start server and visit http://localhost:PORT/metrics
2. See Prometheus format metrics
3. Run some requests and verify counters increase
4. Import Grafana dashboard and see visualizations
5. No noticeable performance impact

## 📝 Notes

- Keep metrics lightweight on hot paths
- Use labels sparingly (high cardinality = high memory)
- Consider adding custom business metrics
- Set up alerting rules based on these metrics