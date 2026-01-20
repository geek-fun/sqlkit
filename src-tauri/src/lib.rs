// Database adapter module
pub mod database;

// Application state management
pub mod state;

// Tauri command handlers
pub mod commands;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    use state::AppState;

    let app_state = AppState::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            greet,
            // Server management commands
            commands::save_server,
            commands::list_servers,
            commands::delete_server,
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
