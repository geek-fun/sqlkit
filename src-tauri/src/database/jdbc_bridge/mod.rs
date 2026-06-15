//! JDBC bridge module.
//!
//! Provides database connectivity through a Java subprocess that
//! uses JDBC drivers. Communication is via JSON-RPC over stdin/stdout.
//!
//! # Architecture
//!
//! ```text
//! Rust (JdbcBridgeAdapter) ←→ stdin/stdout JSON-RPC ←→ Java process (HikariCP + JDBC)
//! ```
//!
//! # Prerequisites
//!
//! 1. Java Runtime (JRE 17+) installed on the system
//! 2. `jdbc-bridge.jar` downloaded (via `download_bridge_plugin()`)
//! 3. JDBC driver JARs for target databases (downloaded automatically)

pub mod adapter;
pub mod download;
pub mod error_classifier;
pub mod fallback;
pub mod jre;
pub mod launcher;
pub mod pool;
pub mod progress;
pub mod protocol;
pub mod registry;

pub use adapter::JdbcBridgeAdapter;
pub use launcher::JdbcBridgeLauncher;
pub use pool::{JdbcBridgeConnection, JdbcBridgePool};
pub use protocol::{JdbcMethod, JdbcRequest, JdbcResponse};
