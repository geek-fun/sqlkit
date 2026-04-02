//! Store management commands.
//!
//! This module provides Tauri commands for managing persistent key-value storage.

use serde_json::Value;
use std::sync::Arc;
use tauri::State;
use tauri_plugin_store::StoreExt;
use tokio::sync::Mutex;

/// Store wrapper with persistence
#[derive(Clone)]
pub struct Store {
    pub app_handle: Arc<Mutex<Option<tauri::AppHandle>>>,
}

impl Default for Store {
    fn default() -> Self {
        Self::new()
    }
}

impl Store {
    pub fn new() -> Self {
        Self {
            app_handle: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn set_app_handle(&self, handle: tauri::AppHandle) {
        let mut app = self.app_handle.lock().await;
        *app = Some(handle);
    }

    pub async fn get_store(&self) -> Result<Arc<tauri_plugin_store::Store<tauri::Wry>>, String> {
        let app = self.app_handle.lock().await;
        let handle = app.as_ref().ok_or("App handle not initialized")?;

        handle
            .store(".store.dat")
            .map_err(|e| format!("Failed to access store: {}", e))
    }
}

/// Get a value from the store.
///
/// # Arguments
///
/// * `key` - The key to retrieve
/// * `state` - Store state
///
/// # Returns
///
/// The value from the store or null if not found.
#[tauri::command]
pub async fn store_get(key: String, state: State<'_, Store>) -> Result<Option<Value>, String> {
    let store = state.get_store().await?;
    Ok(store
        .get(key)
        .and_then(|v| v.as_object().cloned())
        .map(Value::Object))
}

/// Set a value in the store.
///
/// # Arguments
///
/// * `key` - The key to set
/// * `value` - The value to store
/// * `state` - Store state
#[tauri::command]
pub async fn store_set(key: String, value: Value, state: State<'_, Store>) -> Result<(), String> {
    let store = state.get_store().await?;
    store.set(key, value);
    store
        .save()
        .map_err(|e| format!("Failed to save store: {}", e))?;
    Ok(())
}

/// Delete a value from the store.
///
/// # Arguments
///
/// * `key` - The key to delete
/// * `state` - Store state
#[tauri::command]
pub async fn store_delete(key: String, state: State<'_, Store>) -> Result<(), String> {
    let store = state.get_store().await?;
    store.delete(key);
    store
        .save()
        .map_err(|e| format!("Failed to save store: {}", e))?;
    Ok(())
}

/// Clear all values from the store.
///
/// # Arguments
///
/// * `state` - Store state
#[tauri::command]
pub async fn store_clear(state: State<'_, Store>) -> Result<(), String> {
    let store = state.get_store().await?;
    store.clear();
    store
        .save()
        .map_err(|e| format!("Failed to save store: {}", e))?;
    Ok(())
}
