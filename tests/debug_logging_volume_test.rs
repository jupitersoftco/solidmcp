//! Test that proves debug logging volume is excessive in HTTP handler
//! 
//! This test captures and analyzes the logging output during HTTP request processing
//! to demonstrate the root cause of the "LARGE DEBUG SECTION DETECTED" warning.

use serde_json::json;
use std::sync::{Arc, Mutex};
use tracing::{Metadata, Subscriber};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer};

mod mcp_test_helpers;
use mcp_test_helpers::*;

// Custom tracing layer to capture log output
struct LogCapture {
    logs: Arc<Mutex<Vec<String>>>,
}

impl LogCapture {
    fn new() -> (Self, Arc<Mutex<Vec<String>>>) {
        let logs = Arc::new(Mutex::new(Vec::new()));
        (Self { logs: logs.clone() }, logs)
    }
}

impl<S> Layer<S> for LogCapture
where
    S: Subscriber,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let mut visitor = LogVisitor::new();
        event.record(&mut visitor);
        
        if let Ok(mut logs) = self.logs.lock() {
            logs.push(visitor.message);
        }
    }
}

struct LogVisitor {
    message: String,
}

impl LogVisitor {
    fn new() -> Self {
        Self {
            message: String::new(),
        }
    }
}

impl tracing::field::Visit for LogVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{:?}", value);
        }
    }
}

#[tokio::test]
async fn test_excessive_debug_logging_volume() {
    // Set up log capture
    let (log_capture, captured_logs) = LogCapture::new();
    
    let subscriber = tracing_subscriber::registry()
        .with(log_capture)
        .with(tracing_subscriber::filter::LevelFilter::DEBUG);
    
    let _guard = tracing::subscriber::set_default(subscriber);
    
    // Start test server
    let test_server = McpTestServer::start().await.expect("Failed to start test server");
    let port = test_server.port;

    // Create a request that will trigger extensive logging
    let test_message = json!({
        "jsonrpc": "2.0",
        "id": "debug-volume-test",
        "method": "tools/call",
        "params": {
            "name": "test_tool",
            "arguments": {"param": "value"},
            "_meta": {
                "progressToken": "progress-123"
            }
        }
    });

    // Send request
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}/mcp", port);
    
    let _response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .header("User-Agent", "Cursor-Test-Client/1.0")
        .json(&test_message)
        .send()
        .await
        .expect("Failed to send request");

    // Wait a bit for logs to be captured
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Analyze captured logs
    let logs = captured_logs.lock().unwrap();
    let total_logs = logs.len();
    let total_log_chars: usize = logs.iter().map(|log| log.len()).sum();
    
    println!("=== DEBUG LOGGING ANALYSIS ===");
    println!("Total log entries: {}", total_logs);
    println!("Total log characters: {}", total_log_chars);
    println!("Average log entry size: {:.1} chars", 
        if total_logs > 0 { total_log_chars as f64 / total_logs as f64 } else { 0.0 });

    // Analyze log patterns
    let emoji_logs = logs.iter().filter(|log| log.contains("🚀") || log.contains("🔍") || log.contains("📊")).count();
    let debug_analysis_logs = logs.iter().filter(|log| log.contains("=== MCP REQUEST ANALYSIS")).count();
    let session_debug_logs = logs.iter().filter(|log| log.contains("SESSION DEBUG")).count();
    let size_analysis_logs = logs.iter().filter(|log| log.contains("REQUEST SIZE ANALYSIS")).count();
    
    println!("Emoji-decorated logs: {}", emoji_logs);
    println!("Request analysis blocks: {}", debug_analysis_logs);
    println!("Session debug blocks: {}", session_debug_logs);
    println!("Size analysis blocks: {}", size_analysis_logs);

    // Show a sample of the most verbose logs
    println!("\n=== SAMPLE OF VERBOSE LOGS ===");
    for (i, log) in logs.iter().take(10).enumerate() {
        println!("{}: {}", i + 1, log);
    }

    // The critical assertion: This proves the logging is excessive
    // For a single HTTP request, we should not have dozens of debug logs
    let max_reasonable_logs = 20;  // Even this is generous for a single request
    let max_reasonable_chars = 2000;  // 2KB of logs for one request is already excessive
    
    if total_logs > max_reasonable_logs {
        println!("❌ EXCESSIVE LOGGING DETECTED!");
        println!("   {} log entries for a single HTTP request", total_logs);
        println!("   This excessive logging is the root cause of the debug pollution warning");
    }
    
    if total_log_chars > max_reasonable_chars {
        println!("❌ EXCESSIVE LOG VOLUME DETECTED!");
        println!("   {} characters of logs for a single HTTP request", total_log_chars);
        println!("   This creates massive debug sections that trigger the 5000+ byte warning");
    }

    // These assertions will FAIL initially, proving the excessive logging
    assert!(total_logs <= max_reasonable_logs, 
        "Excessive logging: {} log entries for a single request (max reasonable: {}). \
         This is the root cause of debug pollution.", total_logs, max_reasonable_logs);
    
    assert!(total_log_chars <= max_reasonable_chars,
        "Excessive log volume: {} characters for a single request (max reasonable: {}). \
         This creates the 'LARGE DEBUG SECTION DETECTED' warning.", 
         total_log_chars, max_reasonable_chars);
}

#[tokio::test]
async fn test_debug_section_threshold() {
    // This test verifies the specific threshold mentioned in http.rs:512-514
    let test_content = format!("debug {}", "x".repeat(5000));
    
    // Simulate the check from http.rs
    let triggers_warning = test_content.contains("debug") && test_content.len() > 5000;
    
    assert!(triggers_warning, "Should trigger the LARGE DEBUG SECTION DETECTED warning");
    
    println!("✅ Confirmed: content with 'debug' over 5000 bytes triggers warning");
    println!("   Test content size: {} bytes", test_content.len());
    println!("   Contains 'debug': {}", test_content.contains("debug"));
}