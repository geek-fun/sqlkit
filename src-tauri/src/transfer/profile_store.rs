use crate::transfer::TransferProfile;
use std::fs;
use std::path::PathBuf;
use tauri::path::BaseDirectory;
use tauri::{AppHandle, Manager};

const TRANSFER_PROFILES_FILE_NAME: &str = "transfer_profiles.json";

fn get_profiles_path(app: &AppHandle) -> Result<PathBuf, String> {
    let app_config_dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("Failed to resolve app config directory: {}", e))?;

    if !app_config_dir.exists() {
        fs::create_dir_all(&app_config_dir)
            .map_err(|e| format!("Failed to create app config directory: {}", e))?;
    }

    app.path()
        .resolve(TRANSFER_PROFILES_FILE_NAME, BaseDirectory::AppConfig)
        .map_err(|e| format!("Failed to resolve transfer profiles path: {}", e))
}

pub fn load_profiles(app: &AppHandle) -> Result<Vec<TransferProfile>, String> {
    let path = get_profiles_path(app)?;
    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read transfer profiles: {}", e))?;

    if content.trim().is_empty() {
        return Ok(Vec::new());
    }

    serde_json::from_str::<Vec<TransferProfile>>(&content)
        .map_err(|e| format!("Failed to parse transfer profiles: {}", e))
}

pub fn save_profiles(app: &AppHandle, profiles: &[TransferProfile]) -> Result<(), String> {
    let path = get_profiles_path(app)?;
    let payload = serde_json::to_string_pretty(profiles)
        .map_err(|e| format!("Failed to serialize transfer profiles: {}", e))?;

    fs::write(&path, payload).map_err(|e| format!("Failed to write transfer profiles: {}", e))
}
