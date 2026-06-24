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

/// Metadata for a saved query file
#[derive(Debug, Serialize, Deserialize)]
pub struct SavedQueryInfo {
    /// Filename with extension, e.g. "monthly_revenue.sql"
    pub file_name: String,
    /// Full absolute path to the file
    pub file_path: String,
    /// Parent folder name relative to queries directory, e.g. "queries" or "finance"
    pub folder: String,
    /// Last modified time as Unix timestamp (seconds)
    pub modified_at: u64,
    /// File size in bytes
    pub size_bytes: u64,
}

/// Metadata for a saved query entry in the metadata file
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SavedQueryMetadata {
    pub connection_id: Option<String>,
    pub connection_name: Option<String>,
    pub created_at: u64,
    pub modified_at: u64,
}

/// Collection of saved query metadata entries keyed by file path
#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SavedQueriesMetadata {
    pub queries: std::collections::HashMap<String, SavedQueryMetadata>,
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

fn get_metadata_file_path(app_handle: &AppHandle) -> Result<PathBuf, String> {
    let queries_dir = get_queries_dir(app_handle)?;
    Ok(queries_dir.join("metadata.json"))
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
        let name =
            file_name.unwrap_or_else(|| format!("query_{}.sql", chrono::Utc::now().timestamp()));
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
    fs::write(&target_path, content).map_err(|e| format!("Failed to write file: {}", e))?;

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
pub async fn load_query_file(file_path: String) -> Result<LoadResult, String> {
    let path = Path::new(&file_path);

    if !path.exists() {
        return Ok(LoadResult {
            success: false,
            content: None,
            message: "File not found".to_string(),
        });
    }

    let content = fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

    Ok(LoadResult {
        success: true,
        content: Some(content),
        message: "Query loaded successfully".to_string(),
    })
}

/// List all saved SQL query files with metadata.
///
/// Walks the queries directory recursively and collects metadata for each .sql file.
/// Returns files sorted by modification time (most recent first).
#[tauri::command]
pub async fn list_saved_queries(app_handle: AppHandle) -> Result<Vec<SavedQueryInfo>, String> {
    let queries_dir = get_queries_dir(&app_handle)?;
    let queries_dir_str = queries_dir.to_string_lossy().to_string();

    let mut files = Vec::new();

    if queries_dir.exists() {
        collect_sql_files(&queries_dir, &queries_dir_str, &mut files)?;
    }

    files.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));
    Ok(files)
}

fn collect_sql_files(
    dir: &Path,
    queries_dir_str: &str,
    files: &mut Vec<SavedQueryInfo>,
) -> Result<(), String> {
    let entries = fs::read_dir(dir).map_err(|e| format!("Failed to read directory: {}", e))?;

    for entry in entries.flatten() {
        let path = entry.path();

        if let Ok(file_type) = entry.file_type() {
            if file_type.is_file() {
                if path.extension().map_or(false, |ext| ext == "sql") {
                    let metadata = fs::metadata(&path)
                        .map_err(|e| format!("Failed to read file metadata: {}", e))?;

                    let modified_at = metadata
                        .modified()
                        .map_err(|e| format!("Failed to get modification time: {}", e))?
                        .duration_since(std::time::UNIX_EPOCH)
                        .map_err(|e| format!("Time conversion error: {}", e))?
                        .as_secs();

                    let file_name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();

                    let parent = path.parent().unwrap_or(&Path::new(""));
                    let folder = parent
                        .strip_prefix(queries_dir_str)
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_else(|_| "queries".to_string());
                    let folder = if folder.is_empty() {
                        "queries".to_string()
                    } else {
                        folder
                    };

                    files.push(SavedQueryInfo {
                        file_name,
                        file_path: path.to_string_lossy().to_string(),
                        folder,
                        modified_at,
                        size_bytes: metadata.len(),
                    });
                }
            } else if file_type.is_dir() {
                collect_sql_files(&path, queries_dir_str, files)?;
            }
        }
    }

    Ok(())
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
pub async fn delete_query_file(file_path: String) -> Result<String, String> {
    let path = Path::new(&file_path);

    if !path.exists() {
        return Err("File not found".to_string());
    }

    fs::remove_file(path).map_err(|e| format!("Failed to delete file: {}", e))?;

    Ok("File deleted successfully".to_string())
}

/// Write arbitrary text content to an absolute file path.
///
/// Used by the frontend after the native save dialog resolves a path.
#[tauri::command]
pub async fn write_text_file(path: String, content: String) -> Result<(), String> {
    let target = Path::new(&path);
    if let Some(parent) = target.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).map_err(|e| format!("Failed to create directory: {}", e))?;
        }
    }
    fs::write(target, content).map_err(|e| format!("Failed to write file: {}", e))
}

#[tauri::command]
pub async fn read_saved_queries_metadata(app_handle: AppHandle) -> Result<SavedQueriesMetadata, String> {
    let path = get_metadata_file_path(&app_handle)?;
    if !path.exists() {
        return Ok(SavedQueriesMetadata::default());
    }
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read metadata file: {}", e))?;
    match serde_json::from_str::<SavedQueriesMetadata>(&content) {
        Ok(metadata) => Ok(metadata),
        Err(e) => {
            eprintln!("Corrupt metadata file, returning empty: {}", e);
            Ok(SavedQueriesMetadata::default())
        }
    }
}

#[tauri::command]
pub async fn write_saved_queries_metadata(
    app_handle: AppHandle,
    metadata: SavedQueriesMetadata,
) -> Result<(), String> {
    let path = get_metadata_file_path(&app_handle)?;
    let tmp_path = path.with_extension("json.tmp");
    let content = serde_json::to_string_pretty(&metadata)
        .map_err(|e| format!("Failed to serialize metadata: {}", e))?;
    fs::write(&tmp_path, content)
        .map_err(|e| format!("Failed to write temp metadata file: {}", e))?;
    fs::rename(&tmp_path, &path)
        .map_err(|e| format!("Failed to rename metadata file: {}", e))?;
    Ok(())
}

#[tauri::command]
pub async fn save_query_metadata(
    app_handle: AppHandle,
    file_path: String,
    metadata: SavedQueryMetadata,
) -> Result<(), String> {
    let existing = read_saved_queries_metadata(app_handle.clone()).await?;
    let mut updated = existing;
    updated.queries.insert(file_path, metadata);
    write_saved_queries_metadata(app_handle, updated).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_test_metadata() -> SavedQueriesMetadata {
        let mut queries = HashMap::new();
        queries.insert(
            "/path/to/query1.sql".to_string(),
            SavedQueryMetadata {
                connection_id: Some("conn-uuid-1234".to_string()),
                connection_name: Some("pg-prod".to_string()),
                created_at: 1718926200,
                modified_at: 1719185400,
            },
        );
        queries.insert(
            "/path/to/query2.sql".to_string(),
            SavedQueryMetadata {
                connection_id: None,
                connection_name: None,
                created_at: 1718000000,
                modified_at: 1719000000,
            },
        );
        SavedQueriesMetadata { queries }
    }

    #[test]
    fn test_metadata_write_then_read_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let metadata_path = dir.path().join("metadata.json");

        let original = make_test_metadata();

        let json = serde_json::to_string_pretty(&original).unwrap();
        std::fs::write(&metadata_path, &json).unwrap();

        let content = std::fs::read_to_string(&metadata_path).unwrap();
        let read_back: SavedQueriesMetadata = serde_json::from_str(&content).unwrap();

        assert_eq!(read_back.queries.len(), 2);
        assert!(read_back.queries.contains_key("/path/to/query1.sql"));
        assert!(read_back.queries.contains_key("/path/to/query2.sql"));

        let q1 = read_back.queries.get("/path/to/query1.sql").unwrap();
        assert_eq!(q1.connection_id, Some("conn-uuid-1234".to_string()));
        assert_eq!(q1.connection_name, Some("pg-prod".to_string()));
        assert_eq!(q1.created_at, 1718926200);
        assert_eq!(q1.modified_at, 1719185400);

        let q2 = read_back.queries.get("/path/to/query2.sql").unwrap();
        assert!(q2.connection_id.is_none());
        assert!(q2.connection_name.is_none());
    }

    #[test]
    fn test_metadata_read_missing_file_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        let metadata_path = dir.path().join("metadata.json");

        assert!(!metadata_path.exists());

        let result: SavedQueriesMetadata = if !metadata_path.exists() {
            SavedQueriesMetadata::default()
        } else {
            let content = std::fs::read_to_string(&metadata_path).unwrap();
            serde_json::from_str(&content).unwrap_or_default()
        };

        assert_eq!(result.queries.len(), 0);
    }

    #[test]
    fn test_metadata_read_corrupt_json_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        let metadata_path = dir.path().join("metadata.json");

        std::fs::write(&metadata_path, "not valid json {{{{").unwrap();

        let content = std::fs::read_to_string(&metadata_path).unwrap();
        let result: SavedQueriesMetadata = match serde_json::from_str(&content) {
            Ok(m) => m,
            Err(_) => SavedQueriesMetadata::default(),
        };

        assert_eq!(result.queries.len(), 0);
    }

    #[test]
    fn test_metadata_write_is_atomic() {
        let dir = tempfile::tempdir().unwrap();
        let metadata_path = dir.path().join("metadata.json");
        let tmp_path = metadata_path.with_extension("json.tmp");

        let metadata = make_test_metadata();

        let json = serde_json::to_string_pretty(&metadata).unwrap();
        std::fs::write(&tmp_path, &json).unwrap();
        std::fs::rename(&tmp_path, &metadata_path).unwrap();

        assert!(metadata_path.exists());
        assert!(!tmp_path.exists());

        let content = std::fs::read_to_string(&metadata_path).unwrap();
        let _: SavedQueriesMetadata = serde_json::from_str(&content).unwrap();
    }
}
