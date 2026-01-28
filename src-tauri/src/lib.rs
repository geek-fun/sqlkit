// Database adapter module
pub mod database;

// API response types
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    use state::AppState;

    let app_state = AppState::new();
    let store = commands::store::Store::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .manage(app_state)
        .manage(store.clone())
        .setup(move |app| {
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                store.set_app_handle(handle).await;
            });
            menu::create_menu(app)?;
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
            commands::get_table_info,
            commands::get_table_data,
            // File operations commands
            commands::save_query_file,
            commands::load_query_file,
            commands::list_saved_queries,
            commands::delete_query_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
