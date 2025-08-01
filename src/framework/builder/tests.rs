//! Tests for the framework builder
//!
//! The builder pattern is the primary API users interact with, so comprehensive
//! testing here serves as both quality assurance and living documentation.

#[cfg(test)]
mod tests {
    use super::super::McpServerBuilder;
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};
    use std::sync::Arc;

    // Test context for our tests
    #[derive(Debug, Clone)]
    struct TestContext {
        name: String,
        counter: Arc<std::sync::atomic::AtomicU32>,
    }

    impl TestContext {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(), 
                counter: Arc::new(std::sync::atomic::AtomicU32::new(0)),
            }
        }

        fn increment(&self) -> u32 {
            self.counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1
        }
    }

    // Test tool input/output types
    #[derive(Debug, Deserialize, JsonSchema)]
    struct EchoInput {
        message: String,
    }

    #[derive(Debug, Serialize, JsonSchema)]
    struct EchoOutput {
        echo: String,
        count: u32,
    }

    #[tokio::test]
    async fn test_builder_basic_creation() {
        let context = TestContext::new("test");
        
        let builder = McpServerBuilder::new(context, "test-server", "1.0.0");
        let server = builder.build().await;
        
        assert!(server.is_ok(), "Server creation should succeed");
    }

    #[tokio::test]
    async fn test_builder_with_typed_tool() {
        let context = TestContext::new("echo-test");
        
        let server = McpServerBuilder::new(context, "test-server", "1.0.0")
            .with_tool(
                "echo",
                "Echo input back with count",
                |input: EchoInput, ctx: Arc<TestContext>, _notify| async move {
                    let count = ctx.increment();
                    Ok(EchoOutput {
                        echo: input.message,
                        count,
                    })
                }
            )
            .build()
            .await;

        assert!(server.is_ok(), "Server with tool should be created successfully");
    }

    #[tokio::test]
    async fn test_builder_multiple_tools() {
        let context = TestContext::new("multi-test");
        
        let server = McpServerBuilder::new(context, "test-server", "1.0.0")
            .with_tool(
                "echo",
                "Echo messages",
                |input: EchoInput, _ctx: Arc<TestContext>, _notify| async move {
                    Ok(EchoOutput {
                        echo: input.message,
                        count: 1,
                    })
                }
            )
            .with_tool(
                "add",
                "Add two numbers",
                |input: EchoInput, _ctx: Arc<TestContext>, _notify| async move {
                    Ok(EchoOutput {
                        echo: format!("Result: {}", input.message.len()),
                        count: 2,
                    })
                }
            )
            .build()
            .await;

        assert!(server.is_ok(), "Server with multiple tools should be created successfully");
    }

    #[tokio::test]
    async fn test_builder_context_sharing() {
        let context = TestContext::new("context-test");
        let context_clone = context.clone();
        
        let server = McpServerBuilder::new(context, "test-server", "1.0.0")
            .with_tool(
                "get_name",
                "Get the context name",
                |_input: EchoInput, ctx: Arc<TestContext>, _notify| async move {
                    Ok(EchoOutput {
                        echo: ctx.name.clone(),
                        count: ctx.increment(),
                    })
                }
            )
            .with_tool(
                "increment",
                "Increment the counter",
                |_input: EchoInput, ctx: Arc<TestContext>, _notify| async move {
                    let count = ctx.increment();
                    Ok(EchoOutput {
                        echo: "incremented".to_string(),
                        count,
                    })
                }
            )
            .build()
            .await;
            
        assert!(server.is_ok(), "Server with context-sharing tools should be created");
        // Both tools should share the same context
        assert_eq!(context_clone.name, "context-test");
    }

    #[tokio::test]
    async fn test_builder_server_info() {
        let context = TestContext::new("info-test");
        
        // Test different server names and versions
        let server1 = McpServerBuilder::new(context.clone(), "test-server-1", "1.0.0")
            .build()
            .await;
        assert!(server1.is_ok(), "Server with basic info should be created");
        
        let server2 = McpServerBuilder::new(context.clone(), "my-awesome-server", "2.1.0-alpha")
            .build()
            .await;
        assert!(server2.is_ok(), "Server with version info should be created");
        
        let server3 = McpServerBuilder::new(context, "", "")
            .build()
            .await;
        assert!(server3.is_ok(), "Server with empty info should be created");
    }

    #[tokio::test]
    async fn test_builder_chaining() {
        let context = TestContext::new("chaining-test");
        
        // Test that builder methods can be chained fluently
        let server = McpServerBuilder::new(context, "chained-server", "1.0.0")
            .with_tool(
                "tool1",
                "First tool",
                |input: EchoInput, _ctx: Arc<TestContext>, _notify| async move {
                    Ok(EchoOutput { echo: format!("tool1: {}", input.message), count: 1 })
                }
            )
            .with_tool(
                "tool2", 
                "Second tool",
                |input: EchoInput, _ctx: Arc<TestContext>, _notify| async move {
                    Ok(EchoOutput { echo: format!("tool2: {}", input.message), count: 2 })
                }
            )
            .with_tool(
                "tool3",
                "Third tool", 
                |input: EchoInput, _ctx: Arc<TestContext>, _notify| async move {
                    Ok(EchoOutput { echo: format!("tool3: {}", input.message), count: 3 })
                }
            )
            .build()
            .await;
            
        assert!(server.is_ok(), "Server with chained tool registration should be created");
    }

    #[tokio::test]
    async fn test_builder_error_handling() {
        let context = TestContext::new("error-test");
        
        let server = McpServerBuilder::new(context, "test-server", "1.0.0")
            .with_tool(
                "error_tool",
                "A tool that always errors",
                |_input: EchoInput, _ctx: Arc<TestContext>, _notify| async move {
                    Err::<String, crate::McpError>(crate::McpError::Internal("Intentional error".to_string()))
                }
            )
            .build()
            .await;
            
        assert!(server.is_ok(), "Server with error tool should still be created");
    }
}