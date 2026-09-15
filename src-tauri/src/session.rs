//! Refresh-token session (geekfun#59 §2.2 / cloud-sync P0).
//!
//! The refresh token is an opaque 30-day bearer credential rotated on every
//! use; the raw value lives only on this machine — macOS Keychain /
//! Windows Credential Manager / Linux Secret Service via `keyring`, with a
//! 0600-style app-data file fallback for machines without a keyring
//! service. Presenting a rotated token server-side is treated as a leak and
//! revokes the device scope, so rotation failures must never keep the old
//! token: `rotate_session` persists the successor before returning.

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, State};

use crate::device_activation::DeviceIdentityState;
use crate::device_identity;

const CONSOLE_PROD_URL: &str = "https://console-geekfun.wentsen.com";
const CONSOLE_DEV_URL: &str = "http://localhost:5174";
const HTTP_TIMEOUT_SECS: u64 = 10;

const KEYCHAIN_SERVICE: &str = "geekfun/sqlkit/refresh-token";
const KEYCHAIN_USER: &str = "default";
const FALLBACK_FILE: &str = "device-refresh-token";

pub fn api_base_url() -> &'static str {
    if cfg!(debug_assertions) {
        CONSOLE_DEV_URL
    } else {
        CONSOLE_PROD_URL
    }
}

pub struct SessionState {
    pub app_data_dir: PathBuf,
}

fn keyring_entry() -> Result<keyring::Entry, String> {
    keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_USER)
        .map_err(|e| format!("keyring entry unavailable: {e}"))
}

pub fn persist_token(state: &SessionState, token: &str) {
    let stored = keyring_entry().and_then(|entry| {
        entry
            .set_password(token)
            .map_err(|e| format!("keychain write failed: {e}"))
    });
    if let Err(err) = stored {
        log::warn!("refresh token keychain storage unavailable ({err}) — using file fallback");
        if let Err(e) = std::fs::write(state.app_data_dir.join(FALLBACK_FILE), token) {
            log::error!("failed to persist refresh token: {e}");
        }
    }
}

pub fn load_token(state: &SessionState) -> Option<String> {
    if let Ok(entry) = keyring_entry() {
        if let Ok(token) = entry.get_password() {
            if !token.is_empty() {
                return Some(token);
            }
        }
    }
    std::fs::read_to_string(state.app_data_dir.join(FALLBACK_FILE))
        .ok()
        .map(|raw| raw.trim().to_string())
        .filter(|raw| !raw.is_empty())
}

pub fn clear_token(state: &SessionState) {
    if let Ok(entry) = keyring_entry() {
        let _ = entry.delete_credential();
    }
    let _ = std::fs::remove_file(state.app_data_dir.join(FALLBACK_FILE));
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshedSession {
    #[serde(rename = "access_token")]
    pub access_token: String,
    #[serde(rename = "refresh_token")]
    pub refresh_token: String,
}

/// Exchange the stored lease for a fresh pair. The persisted refresh token
/// is replaced with the successor (single-use semantics) before returning.
pub async fn rotate_session(
    state: &SessionState,
    device_payload: &device_identity::DevicePayload,
) -> Result<RefreshedSession, String> {
    let Some(refresh_token) = load_token(state) else {
        return Err("no stored refresh token".to_string());
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(HTTP_TIMEOUT_SECS))
        .build()
        .map_err(|e| format!("failed to build http client: {e}"))?;
    let response = client
        .post(format!("{}/api/v1/auth/refresh", api_base_url()))
        .json(&json!({ "refresh_token": refresh_token, "device": device_payload }))
        .send()
        .await
        .map_err(|e| format!("network error: {e}"))?;

    let raw: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("invalid refresh payload: {e}"))?;
    let code = raw.get("code").and_then(|v| v.as_u64()).unwrap_or(0);
    if code != 2000 {
        // Rejected (expired / revoked / reuse) — the stored lease is dead.
        clear_token(state);
        let message = raw
            .get("messages")
            .and_then(|m| m.get(0))
            .and_then(|m| m.as_str())
            .unwrap_or("session refresh rejected")
            .to_string();
        return Err(message);
    }

    let data = raw.get("data").cloned().unwrap_or(json!({}));
    let session: RefreshedSession = serde_json::from_value(data)
        .map_err(|e| format!("invalid refresh result: {e}"))?;
    persist_token(state, &session.refresh_token);
    Ok(session)
}

/// Fire the refresh loop explicitly; broadcasts the new access token so the
/// frontend can update its stored session.
#[tauri::command]
pub async fn refresh_session(
    app: AppHandle,
    session: State<'_, SessionState>,
    identity: State<'_, DeviceIdentityState>,
) -> Result<RefreshedSession, String> {
    let result = rotate_session(&session, &identity.payload()).await;
    if let Ok(refreshed) = &result {
        let _ = app.emit("session-refreshed", refreshed.access_token.clone());
    }
    result
}

/// Logout: the lease must not outlive the account on this machine.
#[tauri::command]
pub fn clear_session(session: State<'_, SessionState>) {
    clear_token(&session);
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestState(PathBuf);

    impl TestState {
        fn new() -> Self {
            let dir = std::env::temp_dir().join(format!(
                "dockit-session-test-{}",
                uuid::Uuid::new_v4().simple()
            ));
            std::fs::create_dir_all(&dir).expect("temp dir");
            TestState(dir)
        }
    }

    impl Drop for TestState {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn file_fallback_roundtrips_and_clears() {
        let state = TestState::new();
        let session_state = SessionState {
            app_data_dir: state.0.clone(),
        };

        assert_eq!(load_token(&session_state), None);

        std::fs::write(state.0.join(FALLBACK_FILE), "raw-lease-token\n").unwrap();
        assert_eq!(load_token(&session_state).as_deref(), Some("raw-lease-token"));

        clear_token(&session_state);
        assert_eq!(load_token(&session_state), None);
    }

    #[test]
    fn refreshed_session_deserializes_the_envelope_shape() {
        // the backend keeps access_token-style snake_case for token fields
        let raw = json!({
            "access_token": "jwt-value",
            "refresh_token": "opaque-lease",
        });
        let session: RefreshedSession = serde_json::from_value(raw).expect("session");
        assert_eq!(session.access_token, "jwt-value");
        assert_eq!(session.refresh_token, "opaque-lease");
    }
}
