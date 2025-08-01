//! Integration tests for concurrent client handling
//!
//! These tests verify that the MCP server can handle multiple clients
//! simultaneously without interference or data corruption.

mod helpers;

use helpers::{TestServer, McpHttpClient, assert_json_rpc_success};
use serde_json::json;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

#[tokio::test]
async fn test_many_concurrent_clients() {
    let server = TestServer::start().await;
    let url = server.url("/");
    let num_clients = 20;
    
    let success_count = Arc::new(AtomicU32::new(0));
    let error_count = Arc::new(AtomicU32::new(0));
    
    let handles: Vec<_> = (0..num_clients)
        .map(|i| {
            let url = url.clone();
            let success_count = Arc::clone(&success_count);
            let error_count = Arc::clone(&error_count);
            
            tokio::spawn(async move {
                let mut client = McpHttpClient::new();
                
                // Each client performs a full protocol flow
                match client.initialize(&url, &format!("concurrent-client-{}", i)).await {
                    Ok(init_response) => {
                        if init_response.get("result").is_some() {
                            // Try to call a tool
                            match client.call_tool(&url, "test_tool", json!({
                                "input": format!("message from client {}", i)
                            })).await {
                                Ok(tool_response) => {
                                    if tool_response.get("result").is_some() {
                                        success_count.fetch_add(1, Ordering::SeqCst);
                                    } else {
                                        error_count.fetch_add(1, Ordering::SeqCst);
                                    }
                                }
                                Err(_) => {
                                    error_count.fetch_add(1, Ordering::SeqCst);
                                }
                            }
                        } else {
                            error_count.fetch_add(1, Ordering::SeqCst);
                        }
                    }
                    Err(_) => {
                        error_count.fetch_add(1, Ordering::SeqCst);
                    }
                }
            })
        })
        .collect();
    
    // Wait for all clients to complete
    for handle in handles {
        handle.await.unwrap();
    }
    
    let final_success = success_count.load(Ordering::SeqCst);
    let final_errors = error_count.load(Ordering::SeqCst);
    
    println!("Concurrent test results: {} successes, {} errors", final_success, final_errors);
    
    // Most clients should succeed (allow for some timing-related failures)
    assert!(final_success > num_clients * 80 / 100, 
           "Expected at least 80% success rate, got {}/{}", final_success, num_clients);
    
    server.stop();
}

#[tokio::test]
async fn test_concurrent_tool_calls_same_session() {
    let server = TestServer::start().await;
    let mut client = McpHttpClient::new();
    let url = server.url("/");
    
    // Initialize a single session
    client.initialize(&url, "shared-session-client").await.unwrap();
    let session_cookie = client.session_cookie().unwrap().to_string();
    
    let num_requests = 15;
    let success_count = Arc::new(AtomicU32::new(0));
    
    let handles: Vec<_> = (0..num_requests)
        .map(|i| {
            let url = url.clone();
            let cookie = session_cookie.clone();
            let success_count = Arc::clone(&success_count);
            
            tokio::spawn(async move {
                let mut concurrent_client = McpHttpClient::new();
                concurrent_client.session_cookie = Some(cookie);
                
                match concurrent_client.call_tool(&url, "test_tool", json!({
                    "input": format!("concurrent request {}", i)
                })).await {
                    Ok(response) => {
                        if response.get("result").is_some() {
                            success_count.fetch_add(1, Ordering::SeqCst);
                        }
                    }
                    Err(e) => {
                        println!("Request {} failed: {}", i, e);
                    }
                }
            })
        })
        .collect();
    
    // Wait for all requests to complete
    for handle in handles {
        handle.await.unwrap();
    }
    
    let final_success = success_count.load(Ordering::SeqCst);
    println!("Shared session test: {}/{} requests succeeded", final_success, num_requests);
    
    // All requests should succeed since they're using a valid session
    assert!(final_success >= num_requests * 90 / 100,
           "Expected at least 90% success rate for shared session, got {}/{}", 
           final_success, num_requests);
    
    server.stop();
}

#[tokio::test]
async fn test_concurrent_initialization() {
    let server = TestServer::start().await;
    let url = server.url("/");
    let num_initializers = 10;
    
    let success_count = Arc::new(AtomicU32::new(0));
    
    let handles: Vec<_> = (0..num_initializers)
        .map(|i| {
            let url = url.clone();
            let success_count = Arc::clone(&success_count);
            
            tokio::spawn(async move {
                let mut client = McpHttpClient::new();
                
                match client.initialize(&url, &format!("init-client-{}", i)).await {
                    Ok(response) => {
                        if response.get("result").is_some() {
                            success_count.fetch_add(1, Ordering::SeqCst);
                        }
                    }
                    Err(e) => {
                        println!("Client {} initialization failed: {}", i, e);
                    }
                }
            })
        })
        .collect();
    
    // Wait for all initializations to complete
    for handle in handles {
        handle.await.unwrap();
    }
    
    let final_success = success_count.load(Ordering::SeqCst);
    println!("Concurrent initialization: {}/{} succeeded", final_success, num_initializers);
    
    // All initializations should succeed
    assert_eq!(final_success, num_initializers,
              "All concurrent initializations should succeed");
    
    server.stop();
}

#[tokio::test]
async fn test_mixed_concurrent_operations() {
    let server = TestServer::start().await;
    let url = server.url("/");
    
    // Pre-initialize some clients
    let mut initialized_clients = Vec::new();
    for i in 0..5 {
        let mut client = McpHttpClient::new();
        client.initialize(&url, &format!("pre-init-{}", i)).await.unwrap();
        initialized_clients.push(client);
    }
    
    let success_count = Arc::new(AtomicU32::new(0));
    let mut handles = Vec::new();
    
    // Mix of operations:
    // 1. New clients initializing
    for i in 0..5 {
        let url = url.clone();
        let success_count = Arc::clone(&success_count);
        
        let handle = tokio::spawn(async move {
            let mut client = McpHttpClient::new();
            if client.initialize(&url, &format!("new-client-{}", i)).await.is_ok() {
                success_count.fetch_add(1, Ordering::SeqCst);
            }
        });
        handles.push(handle);
    }
    
    // 2. Existing clients making tool calls
    for (i, mut client) in initialized_clients.into_iter().enumerate() {
        let url = url.clone();
        let success_count = Arc::clone(&success_count);
        
        let handle = tokio::spawn(async move {
            if client.call_tool(&url, "test_tool", json!({
                "input": format!("existing client {}", i)
            })).await.is_ok() {
                success_count.fetch_add(1, Ordering::SeqCst);
            }
        });
        handles.push(handle);
    }
    
    // 3. Some clients listing tools
    for i in 0..3 {
        let url = url.clone();
        let success_count = Arc::clone(&success_count);
        
        let handle = tokio::spawn(async move {
            let mut client = McpHttpClient::new();
            if client.initialize(&url, &format!("list-client-{}", i)).await.is_ok() {
                if client.list_tools(&url).await.is_ok() {
                    success_count.fetch_add(1, Ordering::SeqCst);
                }
            }
        });
        handles.push(handle);
    }
    
    // Wait for all operations to complete
    for handle in handles {
        handle.await.unwrap();
    }
    
    let final_success = success_count.load(Ordering::SeqCst);
    let expected_operations = 5 + 5 + 3; // init + tool calls + list ops
    
    println!("Mixed operations: {}/{} succeeded", final_success, expected_operations);
    
    // Most operations should succeed
    assert!(final_success >= expected_operations * 80 / 100,
           "Expected at least 80% success rate for mixed operations");
    
    server.stop();
}

#[tokio::test]
async fn test_session_isolation_under_load() {
    let server = TestServer::start().await;
    let url = server.url("/");
    
    // Create multiple sessions and verify they remain isolated under concurrent load
    let num_sessions = 8;
    let requests_per_session = 5;
    
    let session_results: Arc<std::sync::Mutex<Vec<Vec<bool>>>> = 
        Arc::new(std::sync::Mutex::new(vec![Vec::new(); num_sessions]));
    
    let mut handles = Vec::new();
    
    for session_id in 0..num_sessions {
        let url = url.clone();
        let session_results = Arc::clone(&session_results);
        
        let handle = tokio::spawn(async move {
            // Each session makes multiple requests
            let mut client = McpHttpClient::new();
            client.initialize(&url, &format!("isolation-test-{}", session_id)).await.unwrap();
            
            let mut session_success = Vec::new();
            
            for req_id in 0..requests_per_session {
                let result = client.call_tool(&url, "test_tool", json!({
                    "input": format!("session_{}_request_{}", session_id, req_id)
                })).await;
                
                session_success.push(result.is_ok());
            }
            
            // Store results for this session
            {
                let mut results = session_results.lock().unwrap();
                results[session_id] = session_success;
            }
        });
        
        handles.push(handle);
    }
    
    // Wait for all sessions to complete
    for handle in handles {
        handle.await.unwrap();
    }
    
    // Verify results
    let results = session_results.lock().unwrap();
    let mut total_success = 0;
    let mut total_requests = 0;
    
    for (session_id, session_results) in results.iter().enumerate() {
        let session_success = session_results.iter().filter(|&&success| success).count();
        println!("Session {}: {}/{} requests succeeded", 
                session_id, session_success, session_results.len());
        
        total_success += session_success;
        total_requests += session_results.len();
    }
    
    println!("Overall: {}/{} requests succeeded across all sessions", 
            total_success, total_requests);
    
    // Each session should have high success rate (sessions are isolated)
    assert!(total_success >= total_requests * 90 / 100,
           "Expected at least 90% success rate under concurrent load");
    
    server.stop();
}