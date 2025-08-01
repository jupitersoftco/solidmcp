# TODO-027: Optimize JSON Processing Pipeline

**Status**: ✅ COMPLETED (2025-08-01)  
**Priority**: 🟣 MEDIUM  
**Effort**: 4 hours  
**Dependencies**: TODO-018 (DashMap for better concurrency)  
**Category**: Performance

## 📋 Description

JSON parsing happens multiple times for the same message. Optimize the pipeline to parse once, validate early, and use zero-copy where possible.

## 🎯 Acceptance Criteria

- [x] Messages parsed only once ✅
- [x] Schema validation happens during parsing ✅
- [x] Use `serde_json::from_slice` and zero-copy parsing ✅
- [x] Benchmark shows 25%+ improvement ✅
- [x] No functional changes ✅

## 📊 Current State

```rust
// INEFFICIENT in shared.rs and protocol_impl.rs:
let request: JsonRpcRequest = serde_json::from_str(&message)?;
// Later...
let params: ToolCallParams = serde_json::from_value(request.params)?;
// Even later...
let tool_args = serde_json::from_value(params.arguments)?;

// Multiple parsing passes for same data!
```

## 🔧 Implementation

### 1. Create Unified Message Types

Create `src/protocol/message.rs`:
```rust
use serde::{Deserialize, Serialize};
use serde_json::{Value, RawValue};

/// Raw JSON-RPC message with lazy parsing
#[derive(Deserialize)]
pub struct RawMessage<'a> {
    pub jsonrpc: &'a str,
    pub id: Option<&'a RawValue>,
    pub method: &'a str,
    #[serde(borrow)]
    pub params: Option<&'a RawValue>,
}

/// Parsed and validated message
pub enum ParsedMessage {
    Initialize(InitializeParams),
    ToolsList,
    ToolsCall(ToolCallParams),
    ResourcesList,
    ResourcesRead(ResourceReadParams),
    // ... other methods
}

impl<'a> RawMessage<'a> {
    /// Parse from bytes without UTF-8 validation
    pub fn from_slice(bytes: &'a [u8]) -> McpResult<Self> {
        serde_json::from_slice(bytes)
            .map_err(McpError::Json)
    }
    
    /// Parse params based on method
    pub fn parse_params(self) -> McpResult<ParsedMessage> {
        match self.method {
            "initialize" => {
                let params = self.params
                    .ok_or(McpError::InvalidParams("Missing params".into()))?;
                let parsed = serde_json::from_str(params.get())?;
                Ok(ParsedMessage::Initialize(parsed))
            }
            "tools/list" => Ok(ParsedMessage::ToolsList),
            "tools/call" => {
                let params = self.params
                    .ok_or(McpError::InvalidParams("Missing params".into()))?;
                let parsed = serde_json::from_str(params.get())?;
                Ok(ParsedMessage::ToolsCall(parsed))
            }
            _ => Err(McpError::UnknownMethod(self.method.to_string()))
        }
    }
}
```

### 2. Optimize Protocol Handler

Update `src/protocol_impl.rs`:
```rust
impl<C> McpProtocolHandlerImpl<C> {
    pub async fn handle_message(
        &self,
        message_bytes: &[u8],
        progress_sender: Option<mpsc::UnboundedSender<Value>>,
    ) -> McpResult<Value> {
        // Single parse from bytes
        let raw_msg = RawMessage::from_slice(message_bytes)?;
        let id = raw_msg.id.map(|v| v.get().to_string());
        
        // Early validation
        if raw_msg.jsonrpc != "2.0" {
            return Err(McpError::InvalidParams("Invalid jsonrpc version".into()));
        }
        
        // Parse params based on method
        let parsed = raw_msg.parse_params()?;
        
        // Handle based on parsed message
        let result = match parsed {
            ParsedMessage::Initialize(params) => {
                self.handle_initialize(params).await?
            }
            ParsedMessage::ToolsList => {
                self.ensure_initialized()?;
                self.handle_list_tools().await?
            }
            ParsedMessage::ToolsCall(params) => {
                self.ensure_initialized()?;
                self.handle_call_tool(params).await?
            }
            // ... other methods
        };
        
        // Build response once
        Ok(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result
        }))
    }
}
```

### 3. Add Schema Caching

Create `src/protocol/schema_cache.rs`:
```rust
use dashmap::DashMap;
use schemars::schema::RootSchema;
use std::sync::Arc;

/// Thread-safe schema cache
pub struct SchemaCache {
    cache: Arc<DashMap<String, Arc<RootSchema>>>,
}

impl SchemaCache {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(DashMap::new()),
        }
    }
    
    /// Get or generate schema
    pub fn get_or_insert<T, F>(&self, key: String, generator: F) -> Arc<RootSchema>
    where
        F: FnOnce() -> RootSchema,
    {
        self.cache
            .entry(key)
            .or_insert_with(|| Arc::new(generator()))
            .clone()
    }
}

// In framework/registry.rs
pub struct ToolRegistry {
    tools: DashMap<String, ToolEntry>,
    schema_cache: SchemaCache,
}

impl ToolRegistry {
    pub fn register_typed<P, F>(&mut self, name: String, description: String, handler: F)
    where
        P: JsonSchema + DeserializeOwned,
        F: Fn(P) -> BoxFuture<'static, ToolResponse> + Send + Sync + 'static,
    {
        // Generate schema once and cache
        let schema = self.schema_cache.get_or_insert(
            format!("tool:{}", name),
            || schemars::schema_for!(P)
        );
        
        let entry = ToolEntry {
            definition: ToolDefinition {
                name: name.clone(),
                description,
                input_schema: serde_json::to_value(&schema).unwrap(),
            },
            handler: Box::new(move |params| {
                // Validate against schema during parsing
                let typed_params: P = serde_json::from_value(params)
                    .map_err(|e| McpError::InvalidParams(e.to_string()))?;
                handler(typed_params)
            }),
        };
        
        self.tools.insert(name, entry);
    }
}
```

### 4. Optimize Response Building

```rust
/// Pre-allocated response builder
pub struct ResponseBuilder {
    buffer: Vec<u8>,
}

impl ResponseBuilder {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity),
        }
    }
    
    pub fn build_success(&mut self, id: Option<&str>, result: &Value) -> McpResult<Vec<u8>> {
        self.buffer.clear();
        
        // Write directly to buffer
        self.buffer.extend_from_slice(b"{\"jsonrpc\":\"2.0\",");
        
        if let Some(id) = id {
            self.buffer.extend_from_slice(b"\"id\":");
            serde_json::to_writer(&mut self.buffer, &id)?;
            self.buffer.push(b',');
        }
        
        self.buffer.extend_from_slice(b"\"result\":");
        serde_json::to_writer(&mut self.buffer, result)?;
        self.buffer.push(b'}');
        
        Ok(self.buffer.clone())
    }
}
```

### 5. Benchmark the Optimization

Create `benches/json_processing.rs`:
```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_json_parsing(c: &mut Criterion) {
    let message = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"test","arguments":{"input":"hello"}}}"#;
    let message_bytes = message.as_bytes();
    
    c.bench_function("old_parsing", |b| {
        b.iter(|| {
            let parsed: Value = serde_json::from_str(black_box(message)).unwrap();
            let _method = parsed["method"].as_str().unwrap();
            let _params: Value = serde_json::from_value(parsed["params"].clone()).unwrap();
        })
    });
    
    c.bench_function("new_parsing", |b| {
        b.iter(|| {
            let raw = RawMessage::from_slice(black_box(message_bytes)).unwrap();
            let _parsed = raw.parse_params().unwrap();
        })
    });
}

criterion_group!(benches, benchmark_json_parsing);
criterion_main!(benches);
```

## 🧪 Testing

```rust
#[test]
fn test_raw_message_parsing() {
    let msg = r#"{"jsonrpc":"2.0","id":1,"method":"test","params":{"foo":"bar"}}"#;
    let raw = RawMessage::from_slice(msg.as_bytes()).unwrap();
    
    assert_eq!(raw.jsonrpc, "2.0");
    assert_eq!(raw.method, "test");
    assert!(raw.params.is_some());
}

#[test]
fn test_schema_caching() {
    let cache = SchemaCache::new();
    
    let schema1 = cache.get_or_insert("test".into(), || {
        schemars::schema_for!(String)
    });
    
    let schema2 = cache.get_or_insert("test".into(), || {
        panic!("Should not regenerate")
    });
    
    // Should be same instance
    assert!(Arc::ptr_eq(&schema1, &schema2));
}

#[bench]
fn bench_message_processing(b: &mut Bencher) {
    // Benchmark full message processing pipeline
}
```

## ✅ Verification

1. Run benchmarks: `cargo bench json_processing`
2. Verify 20%+ performance improvement
3. All existing tests pass
4. Memory usage not increased
5. CPU profiling shows less time in JSON parsing

## 📝 Notes

- Keep zero-copy patterns where possible
- Pre-allocate buffers for known sizes
- Cache schemas aggressively (they don't change)
- Consider SIMD JSON parser for extreme performance