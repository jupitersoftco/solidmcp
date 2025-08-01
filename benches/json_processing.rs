//! JSON Processing Pipeline Benchmarks
//!
//! These benchmarks compare the old multi-pass JSON parsing approach
//! with the new optimized zero-copy parsing implementation.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use serde_json::{json, Value};
use solidmcp::{
    protocol::{RawMessage, ParsedMessage},
    protocol_impl::McpProtocolHandlerImpl,
};

/// Sample MCP messages for benchmarking
const INITIALIZE_MESSAGE: &str = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"benchmark","version":"1.0.0"}}}"#;

const TOOLS_LIST_MESSAGE: &str = r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#;

const TOOLS_CALL_MESSAGE: &str = r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"echo","arguments":{"message":"hello world","data":{"nested":{"deeply":{"value":42}}}}}}"#;

const COMPLEX_MESSAGE: &str = r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"complex_tool","arguments":{"array":[1,2,3,4,5],"object":{"key1":"value1","key2":{"nested":"data"}},"string":"This is a longer string to test parsing performance","number":123.456,"boolean":true,"null":null}}}"#;

fn benchmark_json_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("json_parsing");
    
    // Test different message types
    let messages = [
        ("initialize", INITIALIZE_MESSAGE),
        ("tools_list", TOOLS_LIST_MESSAGE), 
        ("tools_call", TOOLS_CALL_MESSAGE),
        ("complex", COMPLEX_MESSAGE),
    ];
    
    for (name, message) in &messages {
        let message_bytes = message.as_bytes();
        
        // Benchmark old parsing approach (multiple passes)
        group.bench_function(&format!("old_parsing_{}", name), |b| {
            b.iter(|| {
                // This simulates the old approach: parse -> extract -> parse again
                let parsed: Value = serde_json::from_str(black_box(message)).unwrap();
                let _jsonrpc = parsed["jsonrpc"].as_str().unwrap();
                let _method = parsed["method"].as_str().unwrap();
                let _id = parsed["id"].clone();
                
                // Second parsing pass for params
                if let Some(params) = parsed.get("params") {
                    let _params_copy: Value = serde_json::from_value(params.clone()).unwrap();
                }
            })
        });
        
        // Benchmark new parsing approach (single pass)
        group.bench_function(&format!("new_parsing_{}", name), |b| {
            b.iter(|| {
                let raw = RawMessage::from_slice(black_box(message_bytes)).unwrap();
                let _parsed = raw.parse_params().unwrap();
            })
        });
    }
    
    group.finish();
}

fn benchmark_message_handling(c: &mut Criterion) {
    let mut group = c.benchmark_group("message_handling");
    
    let handler = McpProtocolHandlerImpl::new();
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    // Benchmark old message handling
    group.bench_function("old_handle_message", |b| {
        b.to_async(&rt).iter(|| async {
            let message: Value = serde_json::from_str(black_box(TOOLS_LIST_MESSAGE)).unwrap();
            let _result = handler.handle_message(message).await;
        })
    });
    
    // Benchmark new message handling  
    group.bench_function("new_handle_message_bytes", |b| {
        b.to_async(&rt).iter(|| async {
            let message_bytes = black_box(TOOLS_LIST_MESSAGE.as_bytes());
            let _result = handler.handle_message_bytes(message_bytes).await;
        })
    });
    
    group.finish();
}

fn benchmark_schema_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("schema_validation");
    
    // Test parameter parsing and validation
    let params_json = r#"{"name":"test_tool","arguments":{"input":"hello","number":42,"array":[1,2,3]}}"#;
    
    group.bench_function("old_params_parsing", |b| {
        b.iter(|| {
            let value: Value = serde_json::from_str(black_box(params_json)).unwrap();
            let _name = value["name"].as_str().unwrap();
            let _args = value["arguments"].clone();
        })
    });
    
    group.bench_function("new_params_parsing", |b| {
        b.iter(|| {
            let parsed: solidmcp::protocol::ToolCallParams = 
                serde_json::from_str(black_box(params_json)).unwrap();
            let _name = &parsed.name;
            let _args = &parsed.arguments;
        })
    });
    
    group.finish();
}

fn benchmark_response_building(c: &mut Criterion) {
    let mut group = c.benchmark_group("response_building");
    
    let result_data = json!({
        "content": [{
            "type": "text", 
            "text": "This is a response with some data"
        }],
        "isError": false
    });
    
    // Old approach: multiple JSON operations
    group.bench_function("old_response", |b| {
        b.iter(|| {
            let response = json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": result_data.clone()
            });
            let _serialized = serde_json::to_string(&response).unwrap();
        })
    });
    
    // New approach: pre-allocated buffer (simulated)
    group.bench_function("new_response", |b| {
        b.iter(|| {
            let mut buffer = Vec::with_capacity(256);
            buffer.extend_from_slice(b"{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":");
            serde_json::to_writer(&mut buffer, black_box(&result_data)).unwrap();
            buffer.push(b'}');
        })
    });
    
    group.finish();
}

criterion_group!(
    benches,
    benchmark_json_parsing,
    benchmark_message_handling,
    benchmark_schema_validation,
    benchmark_response_building
);
criterion_main!(benches);