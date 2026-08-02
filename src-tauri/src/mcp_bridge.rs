use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::Mutex;

use axum::extract::State;
use axum::routing::{get, post};
use axum::Json;
use data_studio_agent::capabilities::permissions::McpPolicy;
use data_studio_agent::capabilities::registry;
use data_studio_agent::capabilities::types::Capability;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::{AppHandle, Manager};
use tauri_plugin_store::StoreExt;
use tokio::net::TcpListener;
use tokio::sync::oneshot;

// ---------------------------------------------------------------------------
// Managed state (Tauri)
// ---------------------------------------------------------------------------

pub struct McpServerHandle {
    pub shutdown_tx: Mutex<Option<oneshot::Sender<()>>>,
    pub server_task: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl McpServerHandle {
    pub fn new() -> Self {
        Self {
            shutdown_tx: Mutex::new(None),
            server_task: Mutex::new(None),
        }
    }
}

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default = "default_auto_start")]
    pub auto_start: bool,
    #[serde(default)]
    pub policy: McpPolicy,
}

fn default_auto_start() -> bool {
    true
}

impl McpConfig {
    pub fn load(app_data_dir: &Path) -> Self {
        let path = app_data_dir.join("mcp-config.json");
        match std::fs::read_to_string(&path) {
            Ok(s) => match serde_json::from_str(&s) {
                Ok(cfg) => cfg,
                Err(e) => {
                    log::warn!(
                        "Failed to parse mcp-config.json (corrupt?): {}. Using defaults.",
                        e
                    );
                    McpConfig::default()
                }
            },
            Err(_) => McpConfig::default(),
        }
    }

    pub fn save(&self, app_data_dir: &Path) -> Result<(), String> {
        let path = app_data_dir.join("mcp-config.json");
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(&path, &json).map_err(|e| format!("Failed to write mcp-config.json: {}", e))
    }
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            port: None,
            auto_start: true,
            policy: McpPolicy::default(),
        }
    }
}

// ---------------------------------------------------------------------------
// Request / response types
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct InvokeRequest {
    name: String,
    args: Value,
    connection_id: Option<String>,
}

#[derive(Serialize)]
pub struct InvokeResponse {
    status: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

impl InvokeResponse {
    fn ok(data: Value) -> Self {
        Self {
            status: 200,
            data: Some(data),
            message: None,
        }
    }

    fn error(status: u16, message: String) -> Self {
        Self {
            status,
            data: None,
            message: Some(message),
        }
    }
}

// ---------------------------------------------------------------------------
// Axum application state
// ---------------------------------------------------------------------------

struct BridgeState {
    handle: AppHandle,
    app_name: &'static str,
    app_data_dir: PathBuf,
    policy: McpPolicy,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

fn tools_payload(policy: &McpPolicy) -> Value {
    let reg = registry::registry();
    let caps = reg.agent_tools();

    let tools: Vec<Value> = caps
        .iter()
        .filter(|cap| check_policy(cap, policy, None).is_ok())
        .map(|cap| {
            json!({
                "name": cap.name,
                "description": cap.description,
                "inputSchema": cap.input_schema,
                "metadata": to_metadata(cap),
            })
        })
        .collect();

    json!({
        "tools": tools,
    })
}

async fn handle_tools(State(state): State<Arc<BridgeState>>) -> Json<Value> {
    let mut result = tools_payload(&state.policy);
    result["connections"] = list_connections();
    Json(result)
}

/// List saved connections from the store — id/name/db_type only, no credentials.
fn list_connections() -> Value {
    let handle = match crate::APP_HANDLE.get() {
        Some(h) => h,
        None => return json!([]),
    };
    let store = match handle.store(".store.dat") {
        Ok(s) => s,
        Err(_) => return json!([]),
    };

    let connections = store.get("connections").unwrap_or(json!([]));
    let safe_list: Vec<Value> = connections
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|c| {
                    json!({
                        "id": c.get("id"),
                        "name": c.get("name"),
                        "type": c.get("db_type"),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    json!(safe_list)
}

/// Gate a capability against the MCP permission policy.
/// /tools advertises connection-agnostically (None), so visibility reflects
/// the global mode + confirm_destructive gate, not per-connection overrides.
fn check_policy(
    cap: &Capability,
    policy: &McpPolicy,
    connection_id: Option<&str>,
) -> Result<(), String> {
    if policy.allows(cap.risk_level, connection_id) {
        return Ok(());
    }
    let risk = format!("{:?}", cap.risk_level).to_lowercase();
    Err(format!(
        "Capability '{}' ({}) blocked by MCP policy (mode={:?}, confirm_destructive={})",
        cap.name, risk, policy.mode, policy.confirm_destructive
    ))
}

async fn handle_invoke(
    State(state): State<Arc<BridgeState>>,
    Json(payload): Json<InvokeRequest>,
) -> Json<InvokeResponse> {
    Json(invoke_with_policy(&state.policy, payload).await)
}

// Split from handle_invoke so tests can run it without a Tauri runtime —
// BridgeState holds a Wry AppHandle, which tauri::test mocks cannot provide.
async fn invoke_with_policy(policy: &McpPolicy, payload: InvokeRequest) -> InvokeResponse {
    let cap = match registry::registry().get(&payload.name) {
        Some(c) => c,
        None => return InvokeResponse::error(404, format!("Unknown capability: {}", payload.name)),
    };

    if let Err(msg) = check_policy(cap, policy, payload.connection_id.as_deref()) {
        return InvokeResponse::error(403, msg);
    }

    let config = match payload.connection_id {
        Some(ref id) => match resolve_connection(id).await {
            Ok(cfg) => Some(cfg),
            Err(e) => return InvokeResponse::error(400, e),
        },
        None => None,
    };

    match registry::invoke_capability_inner(&payload.name, payload.args, config).await {
        Ok(data) => match serde_json::from_str::<Value>(&data) {
            Ok(parsed) => InvokeResponse::ok(parsed),
            Err(_) => InvokeResponse::ok(json!({"result": data})),
        },
        Err(msg) => InvokeResponse::error(400, msg),
    }
}

async fn handle_health(State(state): State<Arc<BridgeState>>) -> Json<Value> {
    Json(json!({
        "status": "ok",
        "app": state.app_name,
        "version": state.handle.package_info().version.to_string(),
        "port": get_actual_port(&state.app_data_dir).unwrap_or(0),
    }))
}

// ---------------------------------------------------------------------------
// Bridge startup
// ---------------------------------------------------------------------------

fn get_default_port() -> u16 {
    9121
}

fn get_actual_port(app_data_dir: &Path) -> Option<u16> {
    let path = app_data_dir.join("mcp-port");
    let port = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| s.trim().parse::<u16>().ok())?;

    let addr = std::net::SocketAddrV4::new(std::net::Ipv4Addr::LOCALHOST, port);
    if std::net::TcpStream::connect_timeout(
        &std::net::SocketAddr::V4(addr),
        std::time::Duration::from_millis(200),
    )
    .is_ok()
    {
        Some(port)
    } else {
        let _ = std::fs::remove_file(&path);
        None
    }
}

async fn write_port_file(app_data_dir: &Path, port: u16) -> Result<(), String> {
    let path = app_data_dir.join("mcp-port");
    tokio::fs::create_dir_all(app_data_dir)
        .await
        .map_err(|e| format!("Failed to create app data dir: {}", e))?;
    tokio::fs::write(&path, port.to_string())
        .await
        .map_err(|e| format!("Failed to write port file: {}", e))?;
    Ok(())
}

async fn remove_port_file(app_data_dir: &Path) {
    let path = app_data_dir.join("mcp-port");
    let _ = tokio::fs::remove_file(path).await;
}

/// Shut down the bridge if it's running.
/// Returns the JoinHandle for the old server task, if any, so the caller can
/// await its full completion (including port file cleanup) before starting a new one.
async fn send_shutdown(handle: &AppHandle) -> Option<tokio::task::JoinHandle<()>> {
    let server_handle: tauri::State<'_, McpServerHandle> = handle.state();
    let old_tx = {
        let mut tx = server_handle.shutdown_tx.lock().unwrap();
        tx.take()
    };
    if let Some(sender) = old_tx {
        let _ = sender.send(());
    }
    let mut task = server_handle.server_task.lock().unwrap();
    task.take()
}

pub async fn start(
    handle: AppHandle,
    app_data_dir: PathBuf,
    preferred_port: u16,
    shutdown_rx: oneshot::Receiver<()>,
) -> Result<u16, String> {
    // Try preferred port first, fall back to random
    let policy = McpConfig::load(&app_data_dir).policy;
    let port = match TcpListener::bind(format!("127.0.0.1:{}", preferred_port)).await {
        Ok(listener) => {
            let state = Arc::new(BridgeState {
                handle: handle.clone(),
                app_name: "sqlkit",
                app_data_dir: app_data_dir.clone(),
                policy: policy.clone(),
            });

            let app = axum::Router::new()
                .route("/tools", post(handle_tools))
                .route("/invoke", post(handle_invoke))
                .route("/health", get(handle_health))
                .with_state(state);

            write_port_file(&app_data_dir, preferred_port).await?;

            let data_dir = app_data_dir.clone();
            let join_handle = tokio::spawn(async move {
                log::info!("MCP bridge listening on 127.0.0.1:{}", preferred_port);
                axum::serve(listener, app)
                    .with_graceful_shutdown(async {
                        shutdown_rx.await.ok();
                        log::info!("MCP bridge shutting down");
                    })
                    .await
                    .ok();
                let _ = remove_port_file(&data_dir).await;
            });
            {
                let mcp_handle: tauri::State<'_, McpServerHandle> = handle.state();
                let mut task = mcp_handle.server_task.lock().unwrap();
                *task = Some(join_handle);
            }

            Ok(preferred_port)
        }
        Err(_) => {
            log::warn!(
                "MCP bridge port {} is in use, picking random port",
                preferred_port
            );
            let random_port =
                portpicker::pick_unused_port().ok_or("no port available on localhost")?;
            let listener = TcpListener::bind(format!("127.0.0.1:{}", random_port))
                .await
                .map_err(|e| format!("Failed to bind bridge: {}", e))?;

            let state = Arc::new(BridgeState {
                handle: handle.clone(),
                app_name: "sqlkit",
                app_data_dir: app_data_dir.clone(),
                policy: policy.clone(),
            });

            let app = axum::Router::new()
                .route("/tools", post(handle_tools))
                .route("/invoke", post(handle_invoke))
                .route("/health", get(handle_health))
                .with_state(state);

            write_port_file(&app_data_dir, random_port).await?;

            let data_dir = app_data_dir.clone();
            let join_handle = tokio::spawn(async move {
                log::info!("MCP bridge listening on 127.0.0.1:{}", random_port);
                axum::serve(listener, app)
                    .with_graceful_shutdown(async {
                        shutdown_rx.await.ok();
                        log::info!("MCP bridge shutting down");
                    })
                    .await
                    .ok();
                let _ = remove_port_file(&data_dir).await;
            });
            {
                let mcp_handle: tauri::State<'_, McpServerHandle> = handle.state();
                let mut task = mcp_handle.server_task.lock().unwrap();
                *task = Some(join_handle);
            }

            Ok(random_port)
        }
    };

    port
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn to_metadata(cap: &Capability) -> Value {
    json!({
        "riskLevel": cap.risk_level,
        "requiredPermission": cap.required_permission
    })
}

async fn resolve_connection(connection_id: &str) -> Result<Value, String> {
    let handle = crate::APP_HANDLE
        .get()
        .ok_or_else(|| "AppHandle not initialized".to_string())?;
    use tauri::State;
    let state: State<'_, crate::state::AppState> = handle.state();
    let conns = state.connections.read().await;
    let _active = conns
        .get(connection_id)
        .ok_or_else(|| format!("Connection not found: {}", connection_id))?;
    Ok(json!({ "connectionId": connection_id }))
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn get_mcp_status(app: AppHandle) -> Result<String, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {}", e))?;

    let config = McpConfig::load(&app_data_dir);
    let running_port = get_actual_port(&app_data_dir);

    let status = json!({
        "running": running_port.is_some(),
        "port": running_port,
        "configuredPort": config.port,
        "autoStart": config.auto_start,
        "policy": serde_json::to_value(&config.policy).map_err(|e| e.to_string())?,
    });

    serde_json::to_string(&status).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_mcp_config(
    port: Option<u16>,
    auto_start: bool,
    policy: Option<McpPolicy>,
    app: AppHandle,
) -> Result<String, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {}", e))?
        .to_path_buf();

    // Load first so a None policy from older clients keeps the stored policy.
    let mut config = McpConfig::load(&app_data_dir);
    config.port = port;
    config.auto_start = auto_start;
    if let Some(p) = policy {
        config.policy = p;
    }
    config.save(&app_data_dir)?;

    // Always shut down the current server first and await its full exit
    // (including port file cleanup) to prevent the new server's port file
    // from being deleted by stale cleanup.
    let old_task = send_shutdown(&app).await;
    if let Some(h) = old_task {
        let _ = h.await;
    }

    if auto_start {
        let (new_shutdown_tx, new_shutdown_rx) = oneshot::channel();
        {
            let server_handle: tauri::State<'_, McpServerHandle> = app.state();
            let mut tx = server_handle.shutdown_tx.lock().unwrap();
            *tx = Some(new_shutdown_tx);
        }

        let preferred = port.unwrap_or(get_default_port());
        start(app.clone(), app_data_dir, preferred, new_shutdown_rx).await?;
    }

    Ok(serde_json::to_string(&json!({"status": "ok"})).map_err(|e| e.to_string())?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use data_studio_agent::capabilities::permissions::McpPermissionMode;
    use data_studio_agent::capabilities::types::RiskLevel;

    fn init_registry_for_tests() {
        // OnceLock set-once: subsequent calls are no-ops, safe to call in every test
        data_studio_agent::capabilities::registry::init_registry(&[
            crate::capabilities::sqlkit::register_all,
            crate::capabilities::sql::register_sql_tools,
        ]);
    }

    #[test]
    fn test_default_port_is_9121() {
        assert_eq!(get_default_port(), 9121);
    }

    #[test]
    fn test_mcp_config_default() {
        let cfg = McpConfig::default();
        assert_eq!(cfg.port, None);
        assert!(cfg.auto_start);
        assert_eq!(cfg.policy, McpPolicy::default());
    }

    #[test]
    fn test_mcp_config_save_and_load_roundtrip() {
        let dir = std::env::temp_dir().join(format!(
            "sqlkit-mcp-test-{}-config-roundtrip",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let cfg = McpConfig {
            port: Some(9444),
            auto_start: false,
            policy: McpPolicy {
                mode: McpPermissionMode::DataReadWrite,
                confirm_destructive: false,
                ..McpPolicy::default()
            },
        };
        cfg.save(&dir).unwrap();

        let loaded = McpConfig::load(&dir);
        assert_eq!(loaded.port, Some(9444));
        assert!(!loaded.auto_start);
        assert_eq!(loaded.policy, cfg.policy);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_mcp_config_load_corrupt_file_uses_default() {
        let dir = std::env::temp_dir().join(format!(
            "sqlkit-mcp-test-{}-config-corrupt",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("mcp-config.json"), "{ not valid json").unwrap();

        let cfg = McpConfig::load(&dir);
        assert_eq!(cfg.port, None);
        assert!(cfg.auto_start);
        assert_eq!(cfg.policy, McpPolicy::default());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_list_connections_empty_without_app_handle() {
        assert_eq!(list_connections(), json!([]));
    }

    #[test]
    fn test_invoke_response_ok_serialization() {
        let resp = InvokeResponse::ok(json!({"rows": 1}));
        let v = serde_json::to_value(&resp).unwrap();
        assert_eq!(v["status"], 200);
        assert_eq!(v["data"]["rows"], 1);
        assert!(v.get("message").is_none());
    }

    #[test]
    fn test_invoke_response_error_serialization() {
        let resp = InvokeResponse::error(403, "forbidden".into());
        let v = serde_json::to_value(&resp).unwrap();
        assert_eq!(v["status"], 403);
        assert_eq!(v["message"], "forbidden");
        assert!(v.get("data").is_none());
    }

    #[test]
    fn test_to_metadata_shape() {
        init_registry_for_tests();
        let reg = registry::registry();
        let caps = reg.agent_tools();
        assert!(!caps.is_empty(), "registry should have agent tools");
        let cap = &caps[0];

        let meta = to_metadata(cap);
        assert_eq!(
            meta["riskLevel"],
            serde_json::to_value(cap.risk_level).unwrap(),
            "riskLevel should use the lowercase serde name"
        );
        assert_eq!(meta["requiredPermission"], cap.required_permission);
    }

    #[test]
    fn test_tools_payload_is_flat_with_metadata() {
        init_registry_for_tests();
        let v = tools_payload(&McpPolicy::default());
        let tools = v["tools"].as_array().unwrap();
        assert!(!tools.is_empty());
        for t in tools {
            assert!(t["name"].is_string());
            assert!(t["description"].is_string());
            assert!(t["inputSchema"].is_object());
            assert!(t["metadata"]["riskLevel"].is_string());
            assert!(t["metadata"]["requiredPermission"].is_string());
            assert!(t.get("type").is_none(), "flat shape has no openai wrapper");
            assert!(
                t.get("function").is_none(),
                "flat shape has no openai wrapper"
            );
        }
    }

    #[test]
    fn test_tools_payload_filters_by_policy() {
        init_registry_for_tests();
        let reg = registry::registry();
        let caps = reg.agent_tools();
        if caps.iter().all(|c| matches!(c.risk_level, RiskLevel::Safe)) {
            return;
        }

        let v = tools_payload(&McpPolicy::default());
        let names: std::collections::HashSet<&str> = v["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap())
            .collect();
        for cap in caps.iter() {
            let exposed = names.contains(cap.name);
            assert_eq!(
                exposed,
                matches!(cap.risk_level, RiskLevel::Safe),
                "capability '{}' exposure should follow the ReadOnly default policy",
                cap.name
            );
        }

        let full = McpPolicy {
            mode: McpPermissionMode::FullAccess,
            ..McpPolicy::default()
        };
        let v_full = tools_payload(&full);
        assert_eq!(v_full["tools"].as_array().unwrap().len(), caps.len());
    }

    #[test]
    fn test_check_policy_message_and_gate() {
        init_registry_for_tests();
        let caps = registry::registry().agent_tools();
        if caps.iter().all(|c| matches!(c.risk_level, RiskLevel::Safe)) {
            return;
        }

        let risky = caps
            .iter()
            .find(|c| !matches!(c.risk_level, RiskLevel::Safe))
            .unwrap();

        let err = check_policy(risky, &McpPolicy::default(), None).unwrap_err();
        assert!(err.contains("blocked by MCP policy"));
        assert!(err.contains("mode="));
        assert!(err.contains("confirm_destructive="));

        let full_no_confirm = McpPolicy {
            mode: McpPermissionMode::FullAccess,
            confirm_destructive: false,
            ..McpPolicy::default()
        };
        if let Some(destructive) = caps
            .iter()
            .find(|c| matches!(c.risk_level, RiskLevel::Destructive))
        {
            assert!(check_policy(destructive, &full_no_confirm, None).is_err());
        }
    }

    #[test]
    fn test_handle_invoke_unknown_capability_returns_404() {
        init_registry_for_tests();
        let req = InvokeRequest {
            name: "definitely__not_a_real_capability".into(),
            args: json!({}),
            connection_id: None,
        };

        let rt = tokio::runtime::Runtime::new().unwrap();
        let resp = rt.block_on(invoke_with_policy(&McpPolicy::default(), req));

        assert_eq!(resp.status, 404);
        assert!(resp.message.unwrap().contains("Unknown capability"));
    }

    #[test]
    fn test_handle_invoke_rejects_elevated_and_destructive() {
        init_registry_for_tests();
        let tools = registry::registry().agent_tools();
        // Concurrent tests may initialize the global registry (OnceLock) with
        // test-only Safe capabilities; only assert when the full app registry is present.
        let Some(risky) = tools
            .iter()
            .find(|c| !matches!(c.risk_level, RiskLevel::Safe))
        else {
            return;
        };

        let req = InvokeRequest {
            name: risky.name.to_string(),
            args: json!({}),
            connection_id: None,
        };

        let rt = tokio::runtime::Runtime::new().unwrap();
        let resp = rt.block_on(invoke_with_policy(&McpPolicy::default(), req));

        assert_eq!(resp.status, 403);
        assert!(resp.message.unwrap().contains("blocked by MCP policy"));
    }

    #[test]
    fn test_handle_invoke_full_access_allows_risky_but_fails_invoke() {
        init_registry_for_tests();
        let tools = registry::registry().agent_tools();
        let Some(risky) = tools
            .iter()
            .find(|c| !matches!(c.risk_level, RiskLevel::Safe))
        else {
            return;
        };

        let req = InvokeRequest {
            name: risky.name.to_string(),
            args: json!({}),
            connection_id: None,
        };

        let full = McpPolicy {
            mode: McpPermissionMode::FullAccess,
            ..McpPolicy::default()
        };
        let rt = tokio::runtime::Runtime::new().unwrap();
        let resp = rt.block_on(invoke_with_policy(&full, req));

        // Policy passes; with no connection config the capability itself fails,
        // proving execution reached the invoke path.
        assert_eq!(resp.status, 400);
    }
}
