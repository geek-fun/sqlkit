//! Tauri command handlers.
//!
//! This module contains all Tauri commands organized by domain:
//! - `server`: Server connection testing
//! - `connection`: Connection lifecycle management
//! - `query`: SQL query execution
//! - `browse`: Database metadata browsing
//! - `store`: Key-value store management
//! - `converter`: Data conversion utilities for JSON serialization
//! - `helpers`: Shared utilities to reduce code duplication

pub mod browse;
pub mod connection;
pub mod converter;
pub mod helpers;
pub mod query;
pub mod server;
pub mod store;

// Re-export all command functions for convenience
pub use browse::*;
pub use connection::*;
pub use query::*;
pub use server::*;
pub use store::*;
