//! Tauri command handlers.
//!
//! This module contains all Tauri commands organized by domain:
//! - `server`: Server configuration management
//! - `connection`: Connection lifecycle management
//! - `query`: SQL query execution
//! - `browse`: Database metadata browsing

pub mod browse;
pub mod connection;
pub mod query;
pub mod server;

// Re-export all command functions for convenience
pub use browse::*;
pub use connection::*;
pub use query::*;
pub use server::*;
