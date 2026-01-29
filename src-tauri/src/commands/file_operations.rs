//! File operations commands.
//!
//! This module provides Tauri commands for saving and loading SQL query files.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

/// Result for save operations
#[derive(Debug, Serialize, Deserialize)]
pub struct SaveResult {
    pub success: bool,
    pub file_path: Option<String>,
    pub message: String,
}

/// Result for load operations
#[derive(Debug, Serialize, Deserialize)]
pub struct LoadResult {
    pub success: bool,
    pub content: Option<String>,
    pub message: String,
}

/// Get the queries directory path, creating it if necessary
fn get_queries_dir(app_handle: &AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    
    let queries_dir = app_data_dir.join("queries");
    
    // Create directory if it doesn't exist
    if !queries_dir.exists() {
        fs::create_dir_all(&queries_dir)
            .map_err(|e| format!("Failed to create queries directory: {}", e))?;
    }
    
    Ok(queries_dir)
}

/// Save a SQL query to a file.
///
/// If file_path is provided, saves to that path.
/// Otherwise, creates a new file in the queries directory.
///
/// # Arguments
///
/// * `content` - The SQL content to save
/// * `file_path` - Optional path to save to
/// * `file_name` - Optional filename (used if no file_path provided)
/// * `app_handle` - Tauri app handle
///
/// # Returns
///
/// SaveResult with the saved file path
#[tauri::command]
pub async fn save_query_file(
    content: String,
    file_path: Option<String>,
    file_name: Option<String>,
    app_handle: AppHandle,
) -> Result<SaveResult, String> {
    let target_path = if let Some(path) = file_path {
        PathBuf::from(path)
    } else {
        let queries_dir = get_queries_dir(&app_handle)?;
        let name = file_name.unwrap_or_else(|| {
            format!("query_{}.sql", chrono::Utc::now().timestamp())
        });
        queries_dir.join(name)
    };
    
    // Ensure parent directory exists
    if let Some(parent) = target_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create parent directory: {}", e))?;
        }
    }
    
    // Write file
    fs::write(&target_path, content)
        .map_err(|e| format!("Failed to write file: {}", e))?;
    
    Ok(SaveResult {
        success: true,
        file_path: Some(target_path.to_string_lossy().to_string()),
        message: "Query saved successfully".to_string(),
    })
}

/// Load a SQL query from a file.
///
/// # Arguments
///
/// * `file_path` - Path to the file to load
///
/// # Returns
///
/// LoadResult with the file content
#[tauri::command]
pub async fn load_query_file(
    file_path: String,
) -> Result<LoadResult, String> {
    let path = Path::new(&file_path);
    
    if !path.exists() {
        return Ok(LoadResult {
            success: false,
            content: None,
            message: "File not found".to_string(),
        });
    }
    
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read file: {}", e))?;
    
    Ok(LoadResult {
        success: true,
        content: Some(content),
        message: "Query loaded successfully".to_string(),
    })
}

/// List all saved SQL query files.
///
/// # Arguments
///
/// * `app_handle` - Tauri app handle
///
/// # Returns
///
/// List of file paths
#[tauri::command]
pub async fn list_saved_queries(
    app_handle: AppHandle,
) -> Result<Vec<String>, String> {
    let queries_dir = get_queries_dir(&app_handle)?;
    
    let mut files = Vec::new();
    
    if queries_dir.exists() {
        let entries = fs::read_dir(&queries_dir)
            .map_err(|e| format!("Failed to read queries directory: {}", e))?;
        
        for entry in entries.flatten() {
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_file() {
                    if let Some(ext) = entry.path().extension() {
                        if ext == "sql" {
                            files.push(entry.path().to_string_lossy().to_string());
                        }
                    }
                }
            }
        }
    }
    
    files.sort();
    Ok(files)
}

/// Delete a saved SQL query file.
///
/// # Arguments
///
/// * `file_path` - Path to the file to delete
///
/// # Returns
///
/// Success message
#[tauri::command]
pub async fn delete_query_file(
    file_path: String,
) -> Result<String, String> {
    let path = Path::new(&file_path);
    
    if !path.exists() {
        return Err("File not found".to_string());
    }
    
    fs::remove_file(path)
        .map_err(|e| format!("Failed to delete file: {}", e))?;
    
    Ok("File deleted successfully".to_string())
}
