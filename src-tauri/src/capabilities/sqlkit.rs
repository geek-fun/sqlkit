use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{json, Value};

use super::registry::CapabilityRegistry;
use super::types::{Capability, CapabilityHandler, RiskLevel, SourceKind};

struct ListConnectionsHandler;
struct GetStoreValueHandler;

#[async_trait]
impl CapabilityHandler for ListConnectionsHandler {
    async fn handle(&self, _args: &Value, _connection_config: Option<&Value>) -> Result<String, String> {
        // The frontend provides the connections list; this is a placeholder
        // that tells the LLM what connections are available to attach.
        Ok(json!({
            "message": "Use the Data Studio UI to attach a data source. Once attached, the session will have database connections available."
        }).to_string())
    }
}

#[async_trait]
impl CapabilityHandler for GetStoreValueHandler {
    async fn handle(&self, args: &Value, _connection_config: Option<&Value>) -> Result<String, String> {
        let key = args
            .get("key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing 'key' argument".to_string())?;
        Ok(json!({ "key": key, "value": null }).to_string())
    }
}

pub fn register_all(reg: &mut CapabilityRegistry) {
    reg.register(Capability {
        name: "sqlkit__list_connections",
        description: "List all available database connections that can be attached as data sources for the agent session.",
        handler: Arc::new(ListConnectionsHandler),
        input_schema: json!({
            "type": "object",
            "properties": {},
            "required": []
        }),
        risk_level: RiskLevel::Safe,
        required_permission: "read",
        source_kind: SourceKind::SqlKit,
        tags: &["agent"],
    });

    reg.register(Capability {
        name: "sqlkit__get_store_value",
        description: "Get a value from the persistent key-value store.",
        handler: Arc::new(GetStoreValueHandler),
        input_schema: json!({
            "type": "object",
            "properties": {
                "key": {
                    "type": "string",
                    "description": "The key to look up"
                }
            },
            "required": ["key"]
        }),
        risk_level: RiskLevel::Safe,
        required_permission: "read",
        source_kind: SourceKind::SqlKit,
        tags: &["agent"],
    });
}
