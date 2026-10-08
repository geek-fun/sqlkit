//! SSH host-key trust store — TOFU (Trust On First Use).
//!
//! First connection to a host pins its key fingerprint; later connections
//! must match or the handshake is rejected (possible MITM). `verify_host_key`
//! off in the tunnel config bypasses the store entirely (legacy lenient mode).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

const STORE_FILE: &str = "ssh_known_hosts.json";

static STORE_PATH: OnceLock<PathBuf> = OnceLock::new();

/// Called once during app setup with the app data directory.
pub fn init_store_dir(app_data_dir: &std::path::Path) {
    let _ = STORE_PATH.set(app_data_dir.join(STORE_FILE));
}

fn store_path() -> PathBuf {
    STORE_PATH
        .get()
        .cloned()
        .unwrap_or_else(|| std::env::temp_dir().join(STORE_FILE))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct KnownHostEntry {
    fingerprint: String,
    added_at: u64,
}

fn load() -> HashMap<String, String> {
    let path = store_path();
    let Ok(raw) = std::fs::read_to_string(&path) else {
        return HashMap::new();
    };
    serde_json::from_str::<Vec<(String, String, u64)>>(&raw)
        .map(|entries| entries.into_iter().map(|(h, f, _)| (h, f)).collect())
        .unwrap_or_else(|_| {
            serde_json::from_str::<HashMap<String, KnownHostEntry>>(&raw)
                .map(|map| map.into_iter().map(|(h, e)| (h, e.fingerprint)).collect())
                .unwrap_or_default()
        })
}

fn save(hosts: &HashMap<String, String>) {
    let path = store_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let entries: Vec<(String, String, u64)> = hosts
        .iter()
        .map(|(host, fingerprint)| (host.clone(), fingerprint.clone(), now))
        .collect();
    if let Ok(raw) = serde_json::to_string_pretty(&entries) {
        let _ = std::fs::write(&path, raw);
    }
}

/// TOFU check: accept and pin a first-seen host key, require a match afterwards.
/// Returns `Err` with a user-facing message on fingerprint mismatch.
pub fn verify_or_pin(host: &str, port: u16, fingerprint: &str) -> Result<(), String> {
    let key = format!("{}:{}", host, port);
    let mut hosts = load();
    match hosts.get(&key) {
        Some(pinned) if pinned != fingerprint => Err(format!(
            "Host key for {}:{} changed (possible man-in-the-middle). \
             Pinned: {}. Received: {}. If this change is expected, \
             remove the saved host key and reconnect.",
            host, port, pinned, fingerprint
        )),
        Some(_) => Ok(()),
        None => {
            hosts.insert(key, fingerprint.to_string());
            save(&hosts);
            Ok(())
        }
    }
}

/// Remove the pinned key for a host (used when the user accepts a changed key).
pub fn unpin(host: &str, port: u16) {
    let key = format!("{}:{}", host, port);
    let mut hosts = load();
    if hosts.remove(&key).is_some() {
        save(&hosts);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store(tag: &str) {
        let dir = std::env::temp_dir().join(format!("sqlkit-known-hosts-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let _ = STORE_PATH.set(dir.join(STORE_FILE));
    }

    #[test]
    fn pins_first_seen_key_and_accepts_match() {
        temp_store("pin");
        verify_or_pin("bastion", 22, "SHA256:abc").unwrap();
        verify_or_pin("bastion", 22, "SHA256:abc").unwrap();
    }

    #[test]
    fn rejects_mismatched_key() {
        temp_store("mismatch");
        verify_or_pin("bastion", 22, "SHA256:abc").unwrap();
        let err = verify_or_pin("bastion", 22, "SHA256:evil").unwrap_err();
        assert!(err.contains("changed"));
        assert!(err.contains("SHA256:abc"));
    }

    #[test]
    fn keys_are_isolated_per_host_port() {
        temp_store("isolation");
        verify_or_pin("bastion", 22, "SHA256:abc").unwrap();
        verify_or_pin("other", 22, "SHA256:xyz").unwrap();
        verify_or_pin("bastion", 2222, "SHA256:xyz2").unwrap();
        verify_or_pin("bastion", 22, "SHA256:abc").unwrap();
    }

    #[test]
    fn pins_survive_a_store_reload() {
        temp_store("reload");
        verify_or_pin("bastion", 22, "SHA256:abc").unwrap();
        // a fresh load must still see the pinned key (file-backed)
        let err = verify_or_pin("bastion", 22, "SHA256:evil").unwrap_err();
        assert!(err.contains("changed"));
    }
}
