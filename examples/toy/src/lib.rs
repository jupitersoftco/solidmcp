//! Toy Notes Server Library
//!
//! This library provides the building blocks for the toy notes MCP server,
//! demonstrating how to use the solidmcp library to create custom MCP servers.

// Legacy server module removed - use new framework API in minimal_main.rs
// pub mod server;
pub mod typed_handler;

// Legacy exports removed - use new framework API
// pub use server::{create_toy_server, NotesStorage};
pub use typed_handler::TypedNotesHandler;
