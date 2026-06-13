use serde_json::{json, Value};
use tauri::AppHandle;
use tauri::Manager;

use super::registry;

#[tauri::command]
pub async fn invoke_capability(
    name: String,
    args: Value,
    connection_id: Option<String>,
    app: AppHandle,
) -> Result<String, String> {
    let config = match connection_id {
        Some(ref id) => Some(resolve_connection_config(&app, id).await?),
        None => None,
    };
    registry::invoke_capability_inner(&name, args, config).await
}

#[tauri::command]
pub async fn get_available_tools(source_kinds: Option<Vec<String>>) -> Result<String, String> {
    let reg = registry::registry();
    let db_types = source_kinds.unwrap_or_default();
    let caps = reg.matching_sources(&db_types);

    let openai_tools: Vec<Value> = caps.iter().map(|c| to_openai_tool(c)).collect();
    let metadata: serde_json::Map<String, Value> = caps
        .iter()
        .map(|cap| (cap.name.to_string(), to_metadata(cap)))
        .collect();

    let result = json!({
        "tools": openai_tools,
        "metadata": metadata,
    });

    serde_json::to_string(&result).map_err(|e| e.to_string())
}

fn to_openai_tool(cap: &super::Capability) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": cap.name,
            "description": cap.description,
            "parameters": cap.input_schema.clone()
        }
    })
}

fn to_metadata(cap: &super::Capability) -> Value {
    json!({
        "riskLevel": cap.risk_level,
        "requiredPermission": cap.required_permission
    })
}

async fn resolve_connection_config(app: &AppHandle, connection_id: &str) -> Result<Value, String> {
    let state: tauri::State<'_, crate::state::AppState> = app.state();
    let conns = state.connections.lock().await;
    let _active = conns
        .get(connection_id)
        .ok_or_else(|| format!("Connection not found: {}", connection_id))?;

    Ok(json!({ "connectionId": connection_id }))
}
