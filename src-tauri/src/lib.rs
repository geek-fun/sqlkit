pub mod database;
pub mod transfer;
pub mod api_response;

// Application state management
pub mod state;

// Tauri command handlers
pub mod commands;

// Menu setup
pub mod menu;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

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
    use state::AppState;

    let app_state = AppState::new();
    let store = commands::store::Store::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_os::init())
        .manage(app_state)
        .manage(store.clone())
        .setup(move |app| {
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                store.set_app_handle(handle).await;
            });
            menu::create_menu(app)?;

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
            greet,
            // Store management (internal use)
            commands::store_get,
            commands::store_set,
            commands::store_delete,
            commands::store_clear,
            // Connection management
            commands::save_connection,
            commands::list_connections,
            commands::delete_connection,
            commands::test_connection,
            // Connection lifecycle commands
            commands::connect_server,
            commands::disconnect_server,
            commands::get_connection_status,
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
            commands::update_table_row,
            commands::delete_table_row,
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
