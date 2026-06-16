//! Query execution plan types.
//!
//! This module defines the structures returned by the `explain_query` command
//! for visual query execution plan display.

use serde::{Deserialize, Serialize};

/// Result of an EXPLAIN query execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplainResult {
    /// Database type (e.g., "postgresql", "mysql", "sqlite", "sqlserver").
    pub database_type: String,
    /// Raw EXPLAIN output text or JSON string.
    pub raw: String,
    /// Format of the raw output: "json" or "text".
    pub format: String,
    /// Whether EXPLAIN ANALYZE was used.
    pub analyze: bool,
}
