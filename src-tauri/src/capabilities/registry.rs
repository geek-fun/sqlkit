use std::collections::HashMap;
use std::sync::OnceLock;

use serde_json::Value;

use super::types::Capability;

static REGISTRY: OnceLock<CapabilityRegistry> = OnceLock::new();

pub fn registry() -> &'static CapabilityRegistry {
    REGISTRY
        .get()
        .expect("CapabilityRegistry not initialized — call init_registry() on startup")
}

pub struct CapabilityRegistry {
    capabilities: HashMap<&'static str, Capability>,
}

impl CapabilityRegistry {
    pub fn new() -> Self {
        Self {
            capabilities: HashMap::new(),
        }
    }

    pub fn register(&mut self, capability: Capability) {
        let name = capability.name;
        if self.capabilities.contains_key(name) {
            panic!("Duplicate capability registration: {}", name);
        }
        self.capabilities.insert(name, capability);
    }

    pub fn get(&self, name: &str) -> Option<&Capability> {
        self.capabilities.get(name)
    }

    pub fn matching_sources(&self, db_types: &[String]) -> Vec<&Capability> {
        self.capabilities
            .values()
            .filter(|cap| {
                if !cap.tags.contains(&"agent") {
                    return false;
                }
                if cap.source_kind == super::types::SourceKind::SqlKit {
                    return true;
                }
                // SQL database tools are always available for a SQL client
                if cap.source_kind == super::types::SourceKind::SqlDatabase {
                    return true;
                }
                db_types
                    .iter()
                    .any(|dt| cap.source_kind.matches_db_type(dt))
            })
            .collect()
    }

    pub fn agent_tools(&self) -> Vec<&Capability> {
        self.capabilities
            .values()
            .filter(|cap| cap.tags.contains(&"agent"))
            .collect()
    }
}

pub fn init_registry() {
    let mut reg = CapabilityRegistry::new();

    crate::capabilities::sqlkit::register_all(&mut reg);
    // Register SQL tools once — the per-DB modules all share the same tool names,
    // so we register only a single set that matches any SQL database type.
    crate::capabilities::sql::register_sql_tools(&mut reg);

    REGISTRY.set(reg).ok();
}

pub async fn invoke_capability_inner(
    name: &str,
    args: Value,
    connection_config: Option<Value>,
) -> Result<String, String> {
    let cap = registry()
        .get(name)
        .ok_or_else(|| format!("Unknown capability: {}", name))?;

    let config_ref = connection_config.as_ref();
    cap.handler.handle(&args, config_ref).await
}
