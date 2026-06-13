use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RiskLevel {
    Safe,
    Elevated,
    Destructive,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceKind {
    Database(&'static str),
    /// Matches any SQL database type (PostgreSQL, MySQL, SQL Server, SQLite)
    SqlDatabase,
    SqlKit,
}

impl SourceKind {
    pub fn matches_db_type(&self, db_type: &str) -> bool {
        match self {
            SourceKind::Database(t) => t.eq_ignore_ascii_case(db_type),
            SourceKind::SqlDatabase => matches!(
                db_type.to_uppercase().as_str(),
                "POSTGRESQL" | "MYSQL" | "SQLSERVER" | "SQLITE"
            ),
            _ => false,
        }
    }
}

#[async_trait]
pub trait CapabilityHandler: Send + Sync {
    async fn handle(
        &self,
        args: &Value,
        connection_config: Option<&Value>,
    ) -> Result<String, String>;
}

pub struct Capability {
    pub name: &'static str,
    pub description: &'static str,
    pub handler: Arc<dyn CapabilityHandler>,
    pub input_schema: Value,
    pub risk_level: RiskLevel,
    pub required_permission: &'static str,
    pub source_kind: SourceKind,
    pub tags: &'static [&'static str],
}
