pub mod api_response;
pub mod database;
pub mod ssh;
pub mod transfer;

// Application state management
pub mod state;

// Connection lifecycle management
pub mod connection;

// Tauri command handlers
pub mod commands;

// Menu setup
pub mod menu;

// Agent / AI modules
pub mod agent;
pub mod agent_adapters;
pub mod capabilities;
pub mod common;
pub mod db;

use std::sync::Arc;
use std::sync::OnceLock;
use tauri::AppHandle;

/// Global AppHandle, set once during app setup. Allows capability handlers
/// and other background code to access the Tauri application handle.
pub static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();

/// Global ConnectionGuardian, set once during app setup. Used by query execution
/// for health checks and by capability handlers for connection quality warnings.
pub static GUARDIAN: OnceLock<Arc<crate::connection::guardian::ConnectionGuardian>> = OnceLock::new();

use agent::executor::SqlKitToolExecutor;
use agent::query_history::{add_query_history_entry, load_query_history};
use agent::session_store::{
    clear_agent_session_messages, clear_session_confirmation_rules, create_agent_session,
    delete_agent_session, delete_attached_source, delete_confirmation_rule, load_agent_sessions,
    load_attached_sources, load_confirmation_rules, load_session_messages,
    migrate_session_metadata, save_attached_source, save_confirmation_rule, update_session_meta,
    update_session_status,
};
use agent_adapters::{
    cancel_agent_loop, compact_agent_session, confirm_tool_call, get_agent_context_usage,
    get_all_tools, get_tool_full_result, list_llm_models, run_agent_loop, run_agent_step,
    validate_llm_config,
};
use capabilities::commands::{get_available_tools, invoke_capability};
use data_studio_agent as lib;
use data_studio_agent::storage as storage;

#[derive(Clone, serde::Serialize)]
struct AuthPayload {
    token: String,
    username: String,
    email: String,
}

fn parse_auth_from_url(url: &str) -> Option<AuthPayload> {
    let url = url::Url::parse(url).ok()?;
    if url.scheme() != "sqlkit" || url.host_str() != Some("auth") {
        return None;
    }
    let params: std::collections::HashMap<_, _> = url.query_pairs().collect();
    let token = params.get("token")?.to_string();
    let username = params.get("username")?.to_string();
    let email = params.get("email")?.to_string();
    Some(AuthPayload {
        token,
        username,
        email,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    use crate::connection::guardian::ConnectionGuardian;
    use state::AppState;

    let app_state = Arc::new(AppState::new());
    let store = commands::store::Store::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_os::init())
        .manage(app_state.clone())
        .manage(store.clone())
        .setup(move |app| {
            let handle = app.handle().clone();

            // Store AppHandle globally for capability handlers and agent
            let _ = APP_HANDLE.set(handle.clone());

            // Disable native decorations so data-tauri-drag-region works for
            // the custom header. On macOS with titleBarStyle:Overlay, this
            // removes the native titlebar making the drag-region active.
            #[cfg(target_os = "macos")]
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_decorations(false);
            }

            // Start connection guardian for health monitoring + auto-reconnect
            let guardian = ConnectionGuardian::new(app_state.clone());
            tauri::async_runtime::spawn(guardian.run());

            tauri::async_runtime::spawn(async move {
                store.set_app_handle(handle.clone()).await;
            });
            menu::create_menu(app)?;

            // Initialize the capability registry
            capabilities::registry::init_registry();

            // Initialize agent SQLite database
            use tauri::Manager;
            let app_data_dir = app
                .path()
                .app_data_dir()
                .map_err(|e| format!("Failed to resolve app data dir: {}", e))?;
            let db_path = app_data_dir.join("agent.sqlite");
            let agent_db = storage::db::open(&db_path)?;
            storage::db::migrate(&agent_db)?;
            {
                let conn = agent_db.0.lock().map_err(|e| e.to_string())?;
                recover_stuck_sessions_inner(&conn)?;
            }
            app.manage(agent_db);

            use std::collections::HashMap;
            use std::sync::{Arc, Mutex};
            let confirm_map: lib::traits::ConfirmMap = Arc::new(Mutex::new(HashMap::new()));
            let cancel_map: lib::traits::CancelMap = Arc::new(Mutex::new(HashMap::new()));
            app.manage(confirm_map);
            app.manage(cancel_map);
            let executor: Arc<dyn lib::ToolExecutor> = Arc::new(SqlKitToolExecutor);
            app.manage(executor);

            use tauri::{Emitter, Listener};

            // Handle deep links received while the app is already running
            let app_handle = app.handle().clone();
            app.listen("deep-link://new-url", move |event: tauri::Event| {
                if let Ok(urls) = serde_json::from_str::<Vec<String>>(event.payload()) {
                    for url in &urls {
                        if let Some(payload) = parse_auth_from_url(url) {
                            let _ = app_handle.emit("sqlkit://auth", payload.clone());
                        }
                    }
                }
            });

            // Handle deep links passed at launch (cold start)
            use tauri_plugin_deep_link::DeepLinkExt;
            if let Ok(Some(urls)) = app.deep_link().get_current() {
                let app_handle = app.handle().clone();
                for url in &urls {
                    if let Some(payload) = parse_auth_from_url(url.as_str()) {
                        let _ = app_handle.emit("sqlkit://auth", payload);
                    }
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Agent commands
            invoke_capability,
            get_available_tools,
            run_agent_step,
            validate_llm_config,
            list_llm_models,
            get_all_tools,
            run_agent_loop,
            cancel_agent_loop,
            compact_agent_session,
            get_agent_context_usage,
            confirm_tool_call,
            get_tool_full_result,
            // Session management
            load_agent_sessions,
            create_agent_session,
            update_session_status,
            update_session_meta,
            delete_agent_session,
            clear_agent_session_messages,
            load_session_messages,
            // Confirmation rules
            load_confirmation_rules,
            save_confirmation_rule,
            delete_confirmation_rule,
            clear_session_confirmation_rules,
            // Attached sources
            load_attached_sources,
            save_attached_source,
            delete_attached_source,
            migrate_session_metadata,
            // Query history
            load_query_history,
            add_query_history_entry,
            // Store management (internal use)
            commands::store_get,
            commands::store_set,
            commands::store_delete,
            commands::store_clear,
            // Connection management
            // JDBC driver management commands
            commands::check_jre_status,
            commands::download_jre,
            commands::remove_jre,
            commands::check_jre_update,
            commands::check_bridge_status,
            commands::download_bridge_jar,
            commands::remove_bridge_jar,
            commands::list_drivers,
            commands::download_driver,
            commands::remove_driver,
            commands::list_tns_aliases,
            commands::download_jdbc_driver_direct,
            // Connection management
            commands::save_connection,
            commands::list_connections,
            commands::delete_connection,
            commands::test_connection,
            // Connection lifecycle commands
            commands::connect_server,
            commands::disconnect_server,
            commands::get_connection_status,
            commands::get_connection_quality,
            // Query execution commands
            commands::execute_query,
            commands::cancel_query,
            commands::explain_query,
            // Database browsing commands
            commands::list_databases,
            commands::list_schemas,
            commands::list_tables,
            commands::list_columns,
            commands::get_table_info,
            commands::get_table_data,
            commands::get_table_count,
            commands::get_foreign_keys,
            commands::update_table_row,
            commands::delete_table_row,
            // Schema object browsing commands
            commands::list_views,
            commands::list_procedures,
            commands::list_functions,
            commands::list_triggers,
            commands::list_indexes,
            commands::list_foreign_keys,
            commands::get_object_ddl,
            commands::drop_object,
            commands::rename_object,
            commands::build_table_search_filter,
            // File operations commands
            commands::save_query_file,
            commands::load_query_file,
            commands::list_saved_queries,
            commands::delete_query_file,
            commands::write_text_file,
            commands::preview_export_data,
            commands::execute_export_data,
            commands::detect_file_format,
            commands::preview_import_data,
            commands::execute_import_data,
            commands::preview_migration_data,
            commands::execute_migration_data,
            commands::auto_map_migration_columns,
            commands::generate_ddl_for_objects,
            commands::execute_sql_content,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn recover_stuck_sessions_inner(conn: &rusqlite::Connection) -> Result<(), String> {
    conn.execute(
        "UPDATE agent_sessions SET status = 'idle' WHERE status IN ('running', 'waiting_confirmation')",
        [],
    )
    .map_err(|e| format!("Failed to recover sessions: {}", e))?;
    Ok(())
}
