//! Simple trait-based response system that leverages Rust's type inference
//!
//! This module provides a minimal trait that allows any type to be used as
//! a tool response, automatically handling the conversion to MCP format.

use crate::content_types::McpResponse;
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::Value;

/// Trait for converting any type into an MCP response.
/// 
/// This trait is automatically implemented for all types that implement
/// `Serialize + JsonSchema`, so developers don't need to implement it manually.
pub trait IntoToolResponse {
    /// Convert this type into an MCP response
    fn into_tool_response(self) -> McpResponse;
}

/// Automatic implementation for any type that can be serialized and has a schema
impl<T> IntoToolResponse for T
where
    T: Serialize + JsonSchema,
{
    fn into_tool_response(self) -> McpResponse {
        // Get the type name for a nice summary
        let type_name = std::any::type_name::<T>()
            .split("::")
            .last()
            .unwrap_or("Response");
        
        // Serialize the data
        let data = serde_json::to_value(&self).unwrap_or(Value::Null);
        
        // Create a simple text summary
        let summary = if let Some(obj) = data.as_object() {
            if let Some(first_field) = obj.iter().next() {
                format!("{}: {}", type_name, first_field.0)
            } else {
                format!("{} (empty)", type_name)
            }
        } else {
            format!("{}", type_name)
        };
        
        McpResponse::with_text_and_data(summary, data)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use schemars::JsonSchema;
    use serde::Serialize;
    
    #[derive(Serialize, JsonSchema)]
    struct TestOutput {
        message: String,
        count: usize,
    }
    
    #[test]
    fn test_auto_conversion() {
        let output = TestOutput {
            message: "Hello".to_string(),
            count: 42,
        };
        
        let response = output.into_tool_response();
        assert!(response.content.len() > 0);
        assert!(response.data.is_some());
    }
}