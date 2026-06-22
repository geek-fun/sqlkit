//! Unified download event protocol.
//!
//! Provides typed download progress events for JRE, Bridge JAR, and JDBC driver
//! downloads. Helpers emit Tauri events via the global [`crate::APP_HANDLE`].
//!
//! # Event shape
//!
//! All events use the `"download-progress"` event name with a JSON payload
//! tagged by `"phase"`:
//!
//! - `{"phase":"progress","id":"...","kind":"jre","downloaded":N,"total":M}`
//! - `{"phase":"complete","id":"...","kind":"bridge"}`
//! - `{"phase":"error","id":"...","kind":"driver","error":"..."}`

use serde::Serialize;
use tauri::Emitter;

/// Named Tauri event constant for all download progress events.
pub const DOWNLOAD_EVENT: &str = "download-progress";

/// The kind of download operation.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DownloadKind {
    /// Managed JRE download (Eclipse Temurin).
    Jre,
    /// JDBC bridge fat JAR download.
    Bridge,
    /// JDBC driver JAR download from Maven Central.
    Driver,
}

/// A typed download event tagged by phase.
///
/// Serialized as a flat JSON object with a `"phase"` discriminator field.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "phase", rename_all = "snake_case")]
pub enum DownloadEvent {
    /// Download is in progress with byte-level progress.
    Progress {
        id: String,
        kind: DownloadKind,
        downloaded: u64,
        total: u64,
    },
    /// Download completed successfully.
    Complete {
        id: String,
        kind: DownloadKind,
    },
    /// Download failed with an error message.
    Error {
        id: String,
        kind: DownloadKind,
        error: String,
    },
}

/// Emit a [`DownloadEvent::Progress`] via the global app handle.
///
/// Returns immediately and silently if [`crate::APP_HANDLE`] has not been set yet.
pub fn emit_progress(id: &str, kind: DownloadKind, downloaded: u64, total: u64) {
    let event = DownloadEvent::Progress {
        id: id.to_string(),
        kind,
        downloaded,
        total,
    };
    if let Some(handle) = crate::APP_HANDLE.get() {
        let _ = handle.emit(DOWNLOAD_EVENT, &event);
    }
}

/// Emit a [`DownloadEvent::Complete`] via the global app handle.
///
/// Returns immediately and silently if [`crate::APP_HANDLE`] has not been set yet.
pub fn emit_complete(id: &str, kind: DownloadKind) {
    let event = DownloadEvent::Complete {
        id: id.to_string(),
        kind,
    };
    if let Some(handle) = crate::APP_HANDLE.get() {
        let _ = handle.emit(DOWNLOAD_EVENT, &event);
    }
}

/// Emit a [`DownloadEvent::Error`] via the global app handle.
///
/// Accepts any type that implements `Into<String>` for the error message
/// (e.g., `&str`, `String`, `Box<dyn Error>`).
///
/// Returns immediately and silently if [`crate::APP_HANDLE`] has not been set yet.
pub fn emit_error(id: &str, kind: DownloadKind, error: impl Into<String>) {
    let event = DownloadEvent::Error {
        id: id.to_string(),
        kind,
        error: error.into(),
    };
    if let Some(handle) = crate::APP_HANDLE.get() {
        let _ = handle.emit(DOWNLOAD_EVENT, &event);
    }
}
