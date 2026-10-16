//! Tauri commands for SSH profile management.

use crate::ssh::config::{SshProfile, SshTunnelConfig};
use crate::ssh::ssh_config::read_ssh_config;
use crate::ssh::TunnelManager;
use serde::Serialize;
use tauri::{AppHandle, Manager};
use tauri_plugin_store::StoreExt;
use uuid::Uuid;

/// Remove the pinned host key for a host — the recovery path when a server's
/// key legitimately changed and TOFU verification started rejecting it.
#[tauri::command]
pub fn unpin_ssh_host(host: String, port: u16) {
    crate::ssh::known_hosts::unpin(&host, port);
}

/// List all saved SSH profiles.
#[tauri::command]
pub async fn list_ssh_profiles(app: AppHandle) -> Result<Vec<SshProfile>, String> {
    let store = app
        .store(".store.dat")
        .map_err(|e| format!("Failed to open store: {}", e))?;

    let profiles: Vec<SshProfile> = store
        .get("sshProfiles")
        .and_then(|v| v.as_array().cloned())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| serde_json::from_value(v.clone()).ok())
                .collect()
        })
        .unwrap_or_default();

    Ok(profiles)
}

/// Save (create or update) an SSH profile.
/// Returns the profile ID.
#[tauri::command]
pub async fn save_ssh_profile(app: AppHandle, profile: SshProfile) -> Result<SshProfile, String> {
    crate::entitlement::ensure_local_ultimate(
        &app.state::<crate::entitlement::EntitlementState>(),
        "SSH tunnel",
    )?;
    let store = app
        .store(".store.dat")
        .map_err(|e| format!("Failed to open store: {}", e))?;

    let mut profile = profile;
    if profile.id.is_empty() {
        profile.id = Uuid::new_v4().to_string();
    }

    let mut profiles: Vec<SshProfile> = store
        .get("sshProfiles")
        .and_then(|v| v.as_array().cloned())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| serde_json::from_value(v.clone()).ok())
                .collect()
        })
        .unwrap_or_default();

    // Update or insert
    if let Some(pos) = profiles.iter().position(|p| p.id == profile.id) {
        profiles[pos] = profile.clone();
    } else {
        profiles.push(profile.clone());
    }

    store.set(
        "sshProfiles".to_string(),
        serde_json::to_value(&profiles).map_err(|e| e.to_string())?,
    );
    store
        .save()
        .map_err(|e| format!("Failed to save store: {}", e))?;

    Ok(profile)
}

/// Names of the connections whose SSH chain still lists `profile_id`.
fn profile_users(connections: &[serde_json::Value], profile_id: &str) -> Vec<String> {
    connections
        .iter()
        .filter(|connection| {
            connection
                .get("sshTunnel")
                .and_then(|tunnel| tunnel.get("profileIds"))
                .and_then(|ids| ids.as_array())
                .is_some_and(|ids| ids.iter().any(|id| id.as_str() == Some(profile_id)))
        })
        .filter_map(|connection| {
            connection
                .get("name")
                .and_then(|name| name.as_str())
                .map(str::to_string)
        })
        .collect()
}

/// Delete-time guard: the connections that still reference `profile_id`.
fn connections_using_profile(
    store: &tauri_plugin_store::Store<tauri::Wry>,
    profile_id: &str,
) -> Option<Vec<String>> {
    let connections = store
        .get("connections")
        .and_then(|value| value.as_array().cloned())
        .unwrap_or_default();
    let users = profile_users(&connections, profile_id);
    if users.is_empty() {
        None
    } else {
        Some(users)
    }
}

/// Delete an SSH profile by ID. Refused while any connection still references
/// it — the callers detach it there first, so an in-use tunnel is never
/// silently broken.
#[tauri::command]
pub async fn delete_ssh_profile(app: AppHandle, profile_id: String) -> Result<(), String> {
    crate::entitlement::ensure_local_ultimate(
        &app.state::<crate::entitlement::EntitlementState>(),
        "SSH tunnel",
    )?;
    let store = app
        .store(".store.dat")
        .map_err(|e| format!("Failed to open store: {}", e))?;

    if let Some(users) = connections_using_profile(&store, &profile_id) {
        return Err(format!(
            "SSH profile is still used by: {}",
            users.join(", ")
        ));
    }

    let mut profiles: Vec<SshProfile> = store
        .get("sshProfiles")
        .and_then(|v| v.as_array().cloned())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| serde_json::from_value(v.clone()).ok())
                .collect()
        })
        .unwrap_or_default();

    profiles.retain(|p| p.id != profile_id);

    store.set(
        "sshProfiles".to_string(),
        serde_json::to_value(&profiles).map_err(|e| e.to_string())?,
    );
    store
        .save()
        .map_err(|e| format!("Failed to save store: {}", e))?;

    Ok(())
}

/// Test an SSH connection by creating an ephemeral tunnel. `message` is a
/// developer-facing English diagnostic; the success actors come through the
/// structured fields so each locale renders its own wording.
#[derive(Serialize, Debug)]
pub struct TestSshResult {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
}

#[tauri::command]
pub async fn test_ssh_connection(
    config: SshTunnelConfig,
    remote_host: String,
    remote_port: u16,
) -> TestSshResult {
    if let Err(raw) = crate::entitlement::ensure_local_ultimate_global("SSH tunnel") {
        return TestSshResult {
            success: false,
            message: raw,
            user: None,
            host: None,
            port: None,
        };
    }
    let tunnels = TunnelManager::new();
    let connection_key = format!("test-{}", Uuid::new_v4());

    match tunnels
        .start_tunnel(&connection_key, &config, &remote_host, remote_port, false)
        .await
    {
        Ok(_port) => {
            tunnels.stop_tunnel(&connection_key).await;
            TestSshResult {
                success: true,
                message: format!(
                    "SSH connection to {}@{}:{} successful",
                    config.username, config.host, config.port
                ),
                user: Some(config.username.clone()),
                host: Some(config.host.clone()),
                port: Some(config.port),
            }
        }
        Err(e) => TestSshResult {
            success: false,
            message: e,
            user: None,
            host: None,
            port: None,
        },
    }
}

/// List hosts from ~/.ssh/config.
#[tauri::command]
pub async fn list_ssh_config_hosts() -> Result<Vec<crate::ssh::config::SshConfigHostEntry>, String>
{
    read_ssh_config()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn profile_users_lists_only_the_connections_that_reference_it() {
        let connections = vec![
            json!({ "name": "prod", "sshTunnel": { "profileIds": ["p1", "p2"] } }),
            json!({ "name": "dev", "sshTunnel": { "profileIds": ["p2"] } }),
            json!({ "name": "plain", "sshTunnel": null }),
            json!({ "name": "other", "sshTunnel": { "profileIds": [] } }),
        ];

        assert_eq!(profile_users(&connections, "p1"), vec!["prod".to_string()]);
        assert_eq!(
            profile_users(&connections, "p2"),
            vec!["prod".to_string(), "dev".to_string()]
        );
        assert!(profile_users(&connections, "unused").is_empty());
    }
}
