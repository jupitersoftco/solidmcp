//! High-level framework for building MCP (Model Context Protocol) servers with minimal boilerplate.
//!
//! This module provides an ergonomic API for creating MCP servers with automatic schema generation,
//! type-safe tool registration, and built-in resource/prompt management. The framework handles all
//! the low-level protocol details while exposing a clean, type-safe interface.
//!
//! # Quick Start
//!
//! ```rust
//! use solidmcp::framework::{McpServerBuilder, NotificationCtx};
//! use serde::{Deserialize, Serialize};
//! use schemars::JsonSchema;
//! use anyhow::Result;
//! use std::sync::Arc;
//!
//! #[derive(JsonSchema, Deserialize)]
//! struct CalculateInput {
//!     a: f64,
//!     b: f64,
//!     operation: String,
//! }
//!
//! #[derive(JsonSchema, Serialize)]
//! struct CalculateOutput {
//!     result: f64,
//! }
//!
//! // Application context (can be any type)
//! struct AppContext {
//!     version: String,
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let context = AppContext {
//!         version: "1.0.0".to_string(),
//!     };
//!
//!     let server = McpServerBuilder::new(context, "calculator", "1.0.0")
//!         .with_tool("calculate", "Perform basic arithmetic", |input: CalculateInput, ctx: Arc<AppContext>, notif: NotificationCtx| async move {
//!             notif.info(&format!("Calculating {} {} {}", input.a, input.operation, input.b))?;
//!             
//!             let result = match input.operation.as_str() {
//!                 "add" => input.a + input.b,
//!                 "subtract" => input.a - input.b,
//!                 "multiply" => input.a * input.b,
//!                 "divide" => input.a / input.b,
//!                 _ => return Err(anyhow::anyhow!("Unknown operation")),
//!             };
//!             
//!             Ok(CalculateOutput { result })
//!         })
//!         .build()
//!         .await?;
//!
//!     server.start(3000).await
//! }
//! ```

// Module declarations
pub mod builder;
pub mod handler;
pub mod notification;
pub mod providers;
pub mod registry;

// Re-export main types
pub use builder::McpServerBuilder;
pub use notification::NotificationCtx;
pub use providers::{PromptProvider, ResourceProvider};