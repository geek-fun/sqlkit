use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::Mutex;

use crate::ai_permissions::{
    load_permission_policy, read_permission_config, AiPolicy, AI_POLICY_LOCK,
};
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::Json;
use data_studio_agent::capabilities::registry;
use data_studio_agent::capabilities::types::{Capability, SourceKind};
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
    pub policy: AiPolicy,
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
        let temporary = app_data_dir.join(format!("mcp-config-{}.tmp", uuid::Uuid::new_v4()));
        std::fs::write(&temporary, &json)
            .map_err(|e| format!("Failed to write MCP config: {e}"))?;
        std::fs::rename(&temporary, &path).map_err(|e| {
            let _ = std::fs::remove_file(&temporary);
            format!("Failed to save MCP config: {e}")
        })
    }
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            port: None,
            auto_start: true,
            policy: AiPolicy::default(),
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
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

fn tools_payload(policy: &AiPolicy) -> Value {
    let reg = registry::registry();
    let caps = reg.agent_tools();
    let notice = policy.policy_notice();

    let tools: Vec<Value> = caps
        .iter()
        .filter(|cap| policy.advertises(cap))
        .map(|cap| {
            // Neon-style policy notice: tells the client (LLM) which
            // capability classes are gated and how to lift the gate,
            // without exposing the gated tools themselves.
            let description = match &notice {
                Some(n) => format!("{}\n\n{}", cap.description, n),
                None => cap.description.to_string(),
            };
            json!({
                "name": cap.name,
                "description": description,
                "inputSchema": cap.input_schema,
                "metadata": to_metadata(cap),
            })
        })
        .collect();

    json!({
        "tools": tools,
    })
}

fn entitlement_denied_for(session_confirmed: bool, local_entitled: bool) -> bool {
    !(session_confirmed && local_entitled)
}

fn entitlement_denied(state: &BridgeState) -> Option<String> {
    let entitlement: tauri::State<'_, crate::entitlement::EntitlementState> = state.handle.state();
    if entitlement_denied_for(
        entitlement.session_confirmed(),
        entitlement.local_entitled(),
    ) {
        return Some("MCP Server requires an active Ultimate subscription".to_string());
    }
    None
}

async fn handle_tools(State(state): State<Arc<BridgeState>>) -> Response {
    if let Some(reason) = entitlement_denied(&state) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({
                "error_type": crate::entitlement::ENTITLEMENT_ERROR_TYPE,
                "message": reason,
            })),
        )
            .into_response();
    }
    let policy = match load_permission_policy(&state.handle) {
        Ok(policy) => policy,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"message": e})),
            )
                .into_response()
        }
    };
    let mut result = tools_payload(&policy);
    result["connections"] = list_allowed_connections(&policy);
    Json(result).into_response()
}

/// List safe connection metadata and agent context from the store, without credentials.
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
        .map(|arr| arr.iter().map(safe_connection_summary).collect())
        .unwrap_or_default();

    json!(safe_list)
}

fn safe_connection_summary(connection: &Value) -> Value {
    let mut summary = json!({
        "id": connection.get("id"),
        "name": connection.get("name"),
        "type": connection.get("db_type"),
        "discover": discover_tool_for(&db_type_str(connection)),
    });
    if let Some(prompt) = connection.get("prompt").filter(|value| !value.is_null()) {
        summary["prompt"] = prompt.clone();
    }
    summary
}

/// The SQL discovery entry point for a connection type: list tables first,
/// then drill into a table with describe_table or the full schema.
fn discover_tool_for(_db_type: &str) -> &'static str {
    "sqlkit__list_tables"
}

fn db_type_str(c: &serde_json::Value) -> String {
    c.get("db_type")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

/// Saved connections restricted to the policy allowlist, so MCP clients only
/// learn about connections they are permitted to reach.
fn list_allowed_connections(policy: &AiPolicy) -> Value {
    if policy.permission_defaults.is_some() {
        return filter_ai_connections(list_connections(), policy);
    }
    if policy.allowed_connection_ids.is_empty() {
        return list_connections();
    }
    filter_connections_by_allowlist(list_connections(), policy)
}

fn filter_connections_by_allowlist(connections: Value, policy: &AiPolicy) -> Value {
    let filtered: Vec<Value> = connections
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter(|c| {
                    c.get("id")
                        .and_then(|v| v.as_str())
                        .is_some_and(|id| policy.allowed_connection_ids.iter().any(|a| a == id))
                })
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    json!(filtered)
}

/// Gate a capability against the MCP permission policy.
/// /tools advertises connection-agnostically (None), so visibility reflects
/// the global mode + confirm_destructive gate, not per-connection overrides.
fn check_policy(
    cap: &Capability,
    policy: &AiPolicy,
    connection_id: Option<&str>,
) -> Result<(), String> {
    if policy.allows(cap.risk_level, connection_id) {
        return Ok(());
    }
    let risk = format!("{:?}", cap.risk_level).to_lowercase();
    // Actionable guidance so the agent can relay "how to enable" to the user
    let reason = policy
        .deny_reason(cap.risk_level, connection_id)
        .unwrap_or_else(|| "blocked by MCP policy".to_string());
    Err(format!(
        "Capability '{}' ({}) blocked by MCP policy: {}",
        cap.name, risk, reason
    ))
}

/// Deny reason for a connection-less invocation when an allowlist is set.
/// AppLocal capabilities need no connection, so they stay exempt.
fn allowlist_missing_connection_reason(
    cap: &Capability,
    policy: &AiPolicy,
    connection_id: Option<&str>,
) -> Option<String> {
    if connection_id.is_none()
        && policy.permission_defaults.is_none()
        && !policy.allowed_connection_ids.is_empty()
        && !matches!(cap.source_kind, SourceKind::AppLocal)
    {
        return Some(
            "connection allowlist is set — supply a connection_id from the allowlist in Settings → MCP Bridge".into(),
        );
    }
    None
}

async fn handle_invoke(
    State(state): State<Arc<BridgeState>>,
    Json(payload): Json<InvokeRequest>,
) -> Json<InvokeResponse> {
    if let Some(reason) = entitlement_denied(&state) {
        return Json(InvokeResponse::error(403, reason));
    }
    let policy = match load_permission_policy(&state.handle) {
        Ok(policy) => policy,
        Err(e) => return Json(InvokeResponse::error(500, e)),
    };
    Json(invoke_with_policy(&policy, payload).await)
}

// Split from handle_invoke so tests can run it without a Tauri runtime —
// BridgeState holds a Wry AppHandle, which tauri::test mocks cannot provide.
async fn invoke_with_policy(policy: &AiPolicy, payload: InvokeRequest) -> InvokeResponse {
    if let (Some(envelope), Some(argument)) = (
        payload.connection_id.as_deref(),
        payload.args.get("connection_id").and_then(Value::as_str),
    ) {
        if envelope != argument {
            return InvokeResponse::error(400, "Conflicting connection identifiers".into());
        }
    }
    let payload = InvokeRequest {
        connection_id: payload.connection_id.or_else(|| {
            payload
                .args
                .get("connection_id")
                .and_then(Value::as_str)
                .map(str::to_owned)
        }),
        ..payload
    };

    let cap = match registry::registry().get(&payload.name) {
        Some(c) => c,
        None => return InvokeResponse::error(404, format!("Unknown capability: {}", payload.name)),
    };

    // Allowlist set but no connection specified: a DB capability would fall
    // back to the default connection, bypassing the allowlist. Mirrors the
    // guard in the built-in agent loop (agent_adapters.rs).
    if let Some(reason) =
        allowlist_missing_connection_reason(cap, policy, payload.connection_id.as_deref())
    {
        return InvokeResponse::error(
            403,
            format!(
                "Capability '{}' blocked by MCP policy: {}",
                cap.name, reason
            ),
        );
    }

    if let Err(msg) = if policy.permission_defaults.is_some() {
        policy
            .authorize(cap, &payload.args, payload.connection_id.as_deref(), None)
            .map(|_| ())
    } else {
        check_policy(cap, policy, payload.connection_id.as_deref())
    } {
        return InvokeResponse::error(403, msg);
    }

    let config = payload
        .connection_id
        .as_ref()
        .map(|id| json!({"connectionId": id}))
        .unwrap_or(Value::Null);
    match crate::ai_permissions::invoke_ai(&payload.name, &payload.args, &config, None).await {
        Ok(data) => match serde_json::from_str::<Value>(&data) {
            Ok(parsed) => InvokeResponse::ok(parsed),
            Err(_) => InvokeResponse::ok(json!({"result": data})),
        },
        Err(data_studio_agent::tool_executor::ToolExecutionError::Denied(msg)) => {
            InvokeResponse::error(403, msg)
        }
        Err(error) => InvokeResponse::error(400, error.to_string()),
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

/// The bridge only runs for a session confirmed in this app run.
pub fn sync_with_entitlement(handle: &AppHandle) {
    use tauri::Manager;

    let entitled = {
        let entitlement: tauri::State<'_, crate::entitlement::EntitlementState> = handle.state();
        entitlement.session_confirmed() && entitlement.local_entitled()
    };
    if entitled {
        auto_start_if_entitled(handle);
        return;
    }
    let handle = handle.clone();
    tauri::async_runtime::spawn(async move {
        shutdown_if_running(&handle).await;
    });
}

fn auto_start_if_entitled(handle: &AppHandle) {
    use tauri::Manager;

    let Ok(app_data_dir) = handle.path().app_data_dir() else {
        return;
    };
    let config = McpConfig::load(&app_data_dir);
    if !config.auto_start {
        return;
    }
    {
        let entitlement: tauri::State<'_, crate::entitlement::EntitlementState> = handle.state();
        if !entitlement.session_confirmed() || !entitlement.local_entitled() {
            return;
        }
    }

    // Claiming the slot is what keeps concurrent triggers from double-starting.
    let shutdown_rx = {
        let server_handle: tauri::State<'_, McpServerHandle> = handle.state();
        let mut tx = server_handle.shutdown_tx.lock().unwrap();
        if tx.is_some() {
            return;
        }
        let (sender, receiver) = tokio::sync::oneshot::channel();
        *tx = Some(sender);
        receiver
    };

    let bridge_handle = handle.clone();
    let retry_handle = handle.clone();
    let preferred = config.port.unwrap_or(get_default_port());
    tauri::async_runtime::spawn(async move {
        if let Err(e) = start(bridge_handle, app_data_dir, preferred, shutdown_rx).await {
            log::error!("MCP bridge failed to start: {}", e);
            let _ = send_shutdown(&retry_handle).await;
        }
    });
}

async fn shutdown_if_running(handle: &AppHandle) {
    if let Some(task) = send_shutdown(handle).await {
        let _ = task.await;
    }
}

pub async fn start(
    handle: AppHandle,
    app_data_dir: PathBuf,
    preferred_port: u16,
    shutdown_rx: oneshot::Receiver<()>,
) -> Result<u16, String> {
    // Try preferred port first, fall back to random
    let port = match TcpListener::bind(format!("127.0.0.1:{}", preferred_port)).await {
        Ok(listener) => {
            let state = Arc::new(BridgeState {
                handle: handle.clone(),
                app_name: "sqlkit",
                app_data_dir: app_data_dir.clone(),
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

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn get_mcp_status(app: AppHandle) -> Result<String, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {}", e))?;

    let config = read_permission_config(&app_data_dir)?;
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
    restart: Option<bool>,
    policy: Option<AiPolicy>,
    app: AppHandle,
) -> Result<String, String> {
    use tauri::Manager;
    let entitlement: tauri::State<'_, crate::entitlement::EntitlementState> = app.state();
    crate::entitlement::ensure_local_ultimate(&entitlement, "MCP Server")?;
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {}", e))?
        .to_path_buf();

    {
        let _guard = AI_POLICY_LOCK.lock().map_err(|e| e.to_string())?;
        // Load first so a None policy from older clients keeps the stored policy.
        let mut config = read_permission_config(&app_data_dir)?;
        config.port = port;
        config.auto_start = auto_start;
        if let Some(p) = policy {
            config.policy = p;
        }
        config.save(&app_data_dir)?;
    }

    // Explicit restart starts now without persisting a change to auto_start.
    let should_start = auto_start || restart.unwrap_or(false);
    if should_start {
        if !entitlement.session_confirmed() {
            return Err(crate::entitlement::entitlement_required_error("MCP Server"));
        }
        // Shut down the current server first and await its full exit (including
        // port file cleanup) to prevent the new server's port file from being
        // deleted by stale cleanup.
        let old_task = send_shutdown(&app).await;
        if let Some(h) = old_task {
            let _ = h.await;
        }

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

/// Persist the MCP policy and hot-swap it into the running bridge without a
/// restart, so permission changes never interrupt in-flight LLM requests.
#[tauri::command]
pub async fn save_mcp_policy(policy: AiPolicy, app: AppHandle) -> Result<String, String> {
    use tauri::Manager;
    crate::entitlement::ensure_local_ultimate(
        &app.state::<crate::entitlement::EntitlementState>(),
        "MCP Server",
    )?;
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {}", e))?
        .to_path_buf();

    {
        let _guard = AI_POLICY_LOCK.lock().map_err(|e| e.to_string())?;
        let mut config = read_permission_config(&app_data_dir)?;
        config.policy = policy.clone();
        config.save(&app_data_dir)?;
    }

    serde_json::to_string(&json!({"status": "ok"})).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai_permissions::McpPermissionMode;
    use data_studio_agent::capabilities::types::RiskLevel;

    fn init_registry_for_tests() {
        // OnceLock set-once: subsequent calls are no-ops, safe to call in every test
        data_studio_agent::capabilities::registry::init_registry(&[
            crate::capabilities::sqlkit::register_all,
            crate::capabilities::sql::register_sql_tools,
            crate::capabilities::sql_write::register_write_tools,
            crate::capabilities::dba::register_dba_tools,
        ]);
    }

    #[test]
    fn test_default_port_is_9121() {
        assert_eq!(get_default_port(), 9121);
    }

    #[test]
    fn test_requests_are_denied_unless_a_session_and_a_plan_are_both_present() {
        assert!(!entitlement_denied_for(true, true));
        assert!(entitlement_denied_for(true, false));
        assert!(entitlement_denied_for(false, true));
        assert!(entitlement_denied_for(false, false));
    }

    #[test]
    fn test_mcp_config_default() {
        let cfg = McpConfig::default();
        assert_eq!(cfg.port, None);
        assert!(cfg.auto_start);
        assert_eq!(cfg.policy, AiPolicy::default());
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
            policy: AiPolicy {
                mode: McpPermissionMode::DataReadWrite,
                confirm_destructive: false,
                ..AiPolicy::default()
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
        assert_eq!(cfg.policy, AiPolicy::default());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_list_connections_empty_without_app_handle() {
        assert_eq!(list_connections(), json!([]));
    }

    #[test]
    fn test_safe_connection_summary_includes_prompt_but_excludes_credentials() {
        let summary = safe_connection_summary(&json!({
            "id": "connection-1",
            "name": "analytics",
            "db_type": "PostgreSQL",
            "prompt": "Revenue is stored in cents.",
            "password": "secret",
            "apiKey": "secret-key"
        }));

        assert_eq!(
            summary,
            json!({
                "id": "connection-1",
                "name": "analytics",
                "type": "PostgreSQL",
                "discover": "sqlkit__list_tables",
                "prompt": "Revenue is stored in cents."
            })
        );
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
        let v = tools_payload(&AiPolicy::default());
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

        let v = tools_payload(&AiPolicy::default());
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
                matches!(cap.risk_level, RiskLevel::Safe | RiskLevel::Elevated),
                "capability '{}' exposure should follow the DataReadWrite default policy",
                cap.name
            );
        }

        let full = AiPolicy {
            mode: McpPermissionMode::FullAccess,
            ..AiPolicy::default()
        };
        let v_full = tools_payload(&full);
        assert_eq!(v_full["tools"].as_array().unwrap().len(), caps.len());
    }

    #[test]
    fn test_tools_payload_appends_policy_notice_when_gated() {
        init_registry_for_tests();
        // Default DataReadWrite gates Destructive → notice appended
        let v = tools_payload(&AiPolicy::default());
        let tools = v["tools"].as_array().unwrap();
        assert!(!tools.is_empty());
        for t in tools {
            let desc = t["description"].as_str().unwrap();
            assert!(
                desc.contains("MCP policy notice"),
                "surviving tool descriptions must carry the policy notice"
            );
            assert!(desc.contains("destructive operations"));
        }
        // FullAccess + confirm on → no gate → no notice
        let full = AiPolicy {
            mode: McpPermissionMode::FullAccess,
            ..AiPolicy::default()
        };
        let v_full = tools_payload(&full);
        for t in v_full["tools"].as_array().unwrap() {
            let desc = t["description"].as_str().unwrap();
            assert!(!desc.contains("MCP policy notice"));
        }
    }

    #[test]
    fn test_check_policy_message_and_gate() {
        init_registry_for_tests();
        let caps = registry::registry().agent_tools();
        if caps.iter().all(|c| matches!(c.risk_level, RiskLevel::Safe)) {
            return;
        }

        // Default (DataReadWrite) rejects Destructive, allows Elevated
        let Some(risky) = caps
            .iter()
            .find(|c| matches!(c.risk_level, RiskLevel::Destructive))
        else {
            return;
        };

        let err = check_policy(risky, &AiPolicy::default(), None).unwrap_err();
        assert!(err.contains("blocked by MCP policy"));
        // Error carries actionable guidance for the agent to relay
        assert!(err.contains("requires FullAccess permission mode"));

        let full_no_confirm = AiPolicy {
            mode: McpPermissionMode::FullAccess,
            confirm_destructive: false,
            ..AiPolicy::default()
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
        let resp = rt.block_on(invoke_with_policy(&AiPolicy::default(), req));

        assert_eq!(resp.status, 404);
        assert!(resp.message.unwrap().contains("Unknown capability"));
    }

    #[test]
    fn test_allowlist_missing_connection_reason() {
        init_registry_for_tests();
        let tools = registry::registry().agent_tools();
        let Some(db_cap) = tools
            .iter()
            .find(|c| !matches!(c.source_kind, SourceKind::AppLocal))
        else {
            return;
        };
        let Some(app_local_cap) = tools
            .iter()
            .find(|c| matches!(c.source_kind, SourceKind::AppLocal))
        else {
            return;
        };

        let allowlist = AiPolicy {
            allowed_connection_ids: vec!["conn-1".into()],
            ..AiPolicy::default()
        };
        // DB capability without a connection_id → denied by allowlist
        assert!(allowlist_missing_connection_reason(db_cap, &allowlist, None).is_some());
        assert!(allowlist_missing_connection_reason(db_cap, &allowlist, Some("conn-1")).is_none());
        // Empty allowlist → no restriction
        assert!(allowlist_missing_connection_reason(db_cap, &AiPolicy::default(), None).is_none());
        // AppLocal capabilities never need a connection
        assert!(allowlist_missing_connection_reason(app_local_cap, &allowlist, None).is_none());
    }

    #[test]
    fn test_handle_invoke_denies_missing_connection_when_allowlist_set() {
        init_registry_for_tests();
        let tools = registry::registry().agent_tools();
        let Some(db_cap) = tools
            .iter()
            .find(|c| !matches!(c.source_kind, SourceKind::AppLocal))
        else {
            return;
        };

        let req = InvokeRequest {
            name: db_cap.name.to_string(),
            args: json!({}),
            connection_id: None,
        };
        let allowlist = AiPolicy {
            mode: McpPermissionMode::FullAccess,
            allowed_connection_ids: vec!["conn-1".into()],
            ..AiPolicy::default()
        };

        let rt = tokio::runtime::Runtime::new().unwrap();
        let resp = rt.block_on(invoke_with_policy(&allowlist, req));

        assert_eq!(resp.status, 403);
        assert!(resp.message.unwrap().contains("allowlist"));
    }

    #[test]
    fn test_filter_connections_by_allowlist() {
        let conns = json!([
            {"id": "conn-1", "name": "A", "type": "POSTGRESQL"},
            {"id": "conn-2", "name": "B", "type": "MYSQL"},
            {"id": "conn-3", "name": "C", "type": "SQLITE"},
        ]);

        // Empty allowlist matches nothing (the caller short-circuits on it)
        let none = filter_connections_by_allowlist(conns.clone(), &AiPolicy::default());
        assert_eq!(none.as_array().unwrap().len(), 0);

        // Non-empty allowlist → only listed connections survive
        let allowlist = AiPolicy {
            allowed_connection_ids: vec!["conn-2".into()],
            ..AiPolicy::default()
        };
        let filtered = filter_connections_by_allowlist(conns, &allowlist);
        let arr = filtered.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["id"], "conn-2");
    }

    #[test]
    fn test_handle_invoke_rejects_destructive_by_default() {
        init_registry_for_tests();
        let tools = registry::registry().agent_tools();
        // Concurrent tests may initialize the global registry (OnceLock) with
        // test-only Safe capabilities; only assert when the full app registry is present.
        let Some(risky) = tools
            .iter()
            .find(|c| matches!(c.risk_level, RiskLevel::Destructive))
        else {
            return;
        };

        let req = InvokeRequest {
            name: risky.name.to_string(),
            args: json!({}),
            connection_id: None,
        };

        let rt = tokio::runtime::Runtime::new().unwrap();
        let resp = rt.block_on(invoke_with_policy(&AiPolicy::default(), req));

        assert_eq!(resp.status, 403);
        assert!(resp.message.unwrap().contains("blocked by MCP policy"));
    }

    #[test]
    fn test_handle_invoke_without_host_fails_closed() {
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

        let full = AiPolicy {
            mode: McpPermissionMode::FullAccess,
            ..AiPolicy::default()
        };
        let rt = tokio::runtime::Runtime::new().unwrap();
        let resp = rt.block_on(invoke_with_policy(&full, req));

        assert_eq!(resp.status, 403);
        assert!(resp.message.unwrap().contains("App not initialized"));
    }

    #[test]
    fn test_all_sql_capabilities_are_agent_tagged() {
        init_registry_for_tests();
        let reg = data_studio_agent::capabilities::registry::registry();
        let all_tools = reg.agent_tools();
        let sql_tools: Vec<_> = all_tools
            .iter()
            .filter(|c| c.name.starts_with("sqlkit__"))
            .collect();
        assert!(
            sql_tools.len() >= 25,
            "expected at least 25 sqlkit capabilities tagged for agent, got {}",
            sql_tools.len()
        );
        for name in &[
            "sqlkit__list_sessions",
            "sqlkit__kill_session",
            "sqlkit__get_slow_queries",
            "sqlkit__grant_privilege",
            "sqlkit__revoke_privilege",
        ] {
            assert!(
                sql_tools.iter().any(|c| c.name == *name),
                "missing expected DBA capability: {}",
                name
            );
        }
    }
    #[test]
    fn ai_permission_roundtrip_preserves_partial_decisions_and_service_settings() {
        use crate::ai_permissions::PermissionRules;
        let dir = std::env::temp_dir().join(format!("sqlkit-ai-chain-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut policy = AiPolicy {
            permission_defaults: Some(PermissionRules::explicit(true, false, false, false, false)),
            ..Default::default()
        };
        policy.connection_permissions.insert(
            "42".into(),
            PermissionRules {
                update: Some(true),
                delete: Some(false),
                ..Default::default()
            },
        );
        let config = McpConfig {
            port: Some(9432),
            auto_start: false,
            policy: policy.clone(),
        };
        config.save(&dir).unwrap();
        let loaded = read_permission_config(&dir).unwrap();
        assert_eq!(loaded.policy, policy);
        assert_eq!(loaded.port, Some(9432));
        assert!(!loaded.auto_start);
        assert_eq!(loaded.policy.effective_permissions("42").update, Some(true));
        assert_eq!(
            loaded.policy.effective_permissions("42").delete,
            Some(false)
        );
        std::fs::write(dir.join("mcp-config.json"), "invalid").unwrap();
        assert!(read_permission_config(&dir).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn ai_connection_discovery_filters_numeric_and_string_ids_by_effective_permissions() {
        use crate::ai_permissions::PermissionRules;
        let mut policy = AiPolicy {
            permission_defaults: Some(PermissionRules::explicit(false, false, false, false, false)),
            ..Default::default()
        };
        policy.connection_permissions.insert(
            "42".into(),
            PermissionRules {
                read: Some(true),
                ..Default::default()
            },
        );
        let filtered =
            filter_ai_connections(json!([{"id":42},{"id":"other"},{"id":null}]), &policy);
        assert_eq!(filtered, json!([{"id":42}]));
        policy.connection_permissions.insert(
            "42".into(),
            PermissionRules {
                read: Some(false),
                ..Default::default()
            },
        );
        assert_eq!(filter_ai_connections(filtered, &policy), json!([]));
    }
}

fn filter_ai_connections(connections: Value, policy: &AiPolicy) -> Value {
    json!(connections
        .as_array()
        .into_iter()
        .flatten()
        .filter(|connection| {
            let id = match &connection["id"] {
                Value::String(id) => id.clone(),
                Value::Number(id) => id.to_string(),
                _ => return false,
            };
            policy.effective_permissions(&id).any_allowed()
        })
        .cloned()
        .collect::<Vec<_>>())
}
