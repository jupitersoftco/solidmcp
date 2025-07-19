//! Toy Notes Server Library
//!
//! This library provides the building blocks for the toy notes MCP server,
//! demonstrating how to use the solidmcp library to create custom MCP servers.

pub mod server;
pub mod typed_handler;

pub use server::{create_toy_server, NotesStorage};
pub use typed_handler::TypedNotesHandler;
