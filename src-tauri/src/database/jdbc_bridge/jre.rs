//! Managed JRE detection, download, and lifecycle.
//!
//! SQLKit bundles a minimal JRE 21 built with `jlink` for each supported
//! platform. This module handles detecting Java (managed → `JAVA_HOME` → `PATH`),
//! downloading/extracting the managed JRE, and cleaning it up.

use crate::database::error::{DbError, DbResult};
use std::path::PathBuf;

/// Subdirectory under user home for the managed JRE.
const JRE_BASE_DIR: &str = ".sqlkit/jre";

/// Version of the bundled JRE.
#[allow(dead_code)]
const MANAGED_JRE_VERSION: &str = "21";

/// Java executable path relative to JRE root (platform-aware).
const JAVA_EXE: &str = if cfg!(target_os = "windows") {
    "bin/java.exe"
} else {
    "bin/java"
};

/// Download URL base for releases.
const BRIDGE_RELEASE_URL: &str =
    "https://github.com/geek-fun/sqlkit/releases/latest/download";

// ── helpers ────────────────────────────────────────────────

/// User home directory.
fn home_dir() -> PathBuf {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string())
        .into()
}

// ── path helpers ──────────────────────────────────────────

/// Managed JRE base directory (`~/.sqlkit/jre`).
pub fn jre_base_dir() -> PathBuf {
    home_dir().join(JRE_BASE_DIR)
}

/// Path to the `java` binary inside the managed JRE.
pub fn managed_jre_java_path() -> PathBuf {
    jre_base_dir().join(JAVA_EXE)
}

/// Whether the managed JRE is already installed.
pub fn is_managed_jre_installed() -> bool {
    managed_jre_java_path().exists()
}

/// Platform-specific JRE archive filename.
pub fn jre_archive_name() -> &'static str {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        "jre-macos-aarch64.tar.gz"
    }
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    {
        "jre-macos-x64.tar.gz"
    }
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        "jre-linux-x64.tar.gz"
    }
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    {
        "jre-linux-aarch64.tar.gz"
    }
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        "jre-windows-x64.zip"
    }
    #[cfg(not(any(
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "aarch64"),
        all(target_os = "windows", target_arch = "x86_64"),
    )))]
    {
        ""
    }
}

// ── detection ─────────────────────────────────────────────

/// Detects a working Java executable.
///
/// Priority: managed JRE → `JAVA_HOME` → `PATH`
pub struct JreDetector;

impl JreDetector {
    /// Return the best available Java executable.
    pub fn detect() -> Option<PathBuf> {
        // 1. Managed (bundled) JRE
        let managed = managed_jre_java_path();
        if Self::is_valid_java(&managed) {
            return Some(managed);
        }

        // 2. System Java
        Self::detect_system_java()
    }

    /// Check whether `path` points to an existing file.
    pub fn is_valid_java(path: &PathBuf) -> bool {
        path.exists()
    }

    /// Probe `JAVA_HOME` then `PATH` for a `java` executable.
    pub fn detect_system_java() -> Option<PathBuf> {
        // Check JAVA_HOME
        if let Ok(java_home) = std::env::var("JAVA_HOME") {
            let java = PathBuf::from(java_home).join(JAVA_EXE);
            if java.exists() {
                return Some(java);
            }
        }

        // Check PATH
        if let Ok(path_var) = std::env::var("PATH") {
            for dir in std::env::split_paths(&path_var) {
                let java = dir.join("java");
                if java.exists() {
                    return Some(java);
                }
            }
        }

        None
    }
}

// ── download / remove ─────────────────────────────────────

/// Download and extract the managed JRE for the current platform.
///
/// The JRE is a minimal image built with `jlink` (only `java.base` + `java.sql`),
/// compressed as `.tar.gz` (macOS / Linux) or `.zip` (Windows).
pub async fn download_managed_jre() -> DbResult<()> {
    let archive_name = jre_archive_name();
    if archive_name.is_empty() {
        return Err(DbError::Connection(
            "No bundled JRE available for this platform".to_string(),
        ));
    }

    let parent = jre_base_dir()
        .parent()
        .expect("jre_base_dir has a parent")
        .to_path_buf();
    tokio::fs::create_dir_all(&parent)
        .await
        .map_err(|e| DbError::Connection(format!("Failed to create JRE parent dir: {}", e)))?;

    let url = format!("{}/jre/{}", BRIDGE_RELEASE_URL, archive_name);
    let response = reqwest::get(&url)
        .await
        .map_err(|e| DbError::Connection(format!("Failed to download JRE: {}", e)))?;

    let bytes = response
        .bytes()
        .await
        .map_err(|e| DbError::Connection(format!("Failed to read JRE download: {}", e)))?;

    let tmp_path = parent.join(format!("{}.tmp", archive_name));
    tokio::fs::write(&tmp_path, &bytes)
        .await
        .map_err(|e| DbError::Connection(format!("Failed to write JRE archive: {}", e)))?;

    let jre_path = jre_base_dir();
    if jre_path.exists() {
        tokio::fs::remove_dir_all(&jre_path)
            .await
            .map_err(|e| DbError::Connection(format!("Failed to remove old JRE: {}", e)))?;
    }

    let extract_result = tokio::task::spawn_blocking(move || -> Result<(), String> {
        let file = std::fs::File::open(&tmp_path)
            .map_err(|e| format!("Failed to open archive: {}", e))?;

        if archive_name.ends_with(".tar.gz") {
            let decoder = flate2::read::GzDecoder::new(file);
            let mut archive = tar::Archive::new(decoder);
            archive
                .unpack(&parent)
                .map_err(|e| format!("Failed to extract JRE: {}", e))?;
        }

        for entry in std::fs::read_dir(&parent)
            .map_err(|e| format!("Failed to list extracted files: {}", e))?
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        {
            let bin_java = entry.path().join("bin").join(if cfg!(target_os = "windows") {
                "java.exe"
            } else {
                "java"
            });
            if bin_java.exists() {
                let extracted_path = entry.path();
                let target_path = parent.join("jre");
                std::fs::rename(&extracted_path, &target_path)
                    .map_err(|e| format!("Failed to rename JRE directory: {}", e))?;
                break;
            }
        }

        let _ = std::fs::remove_file(&tmp_path);
        Ok(())
    })
    .await
    .map_err(|e| DbError::Connection(format!("JRE extraction panicked: {}", e)))?;

    extract_result.map_err(|e| DbError::Connection(format!("JRE extraction failed: {}", e)))?;

    Ok(())
}

/// Remove the managed JRE from disk.
pub fn remove_managed_jre() -> DbResult<()> {
    let path = jre_base_dir();
    if path.exists() {
        std::fs::remove_dir_all(&path)
            .map_err(|e| DbError::Connection(format!("Failed to remove JRE: {}", e)))?;
    }
    Ok(())
}

// ── tests ─────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_jre_base_dir() {
        let dir = jre_base_dir();
        assert!(dir.is_absolute() || dir.starts_with("."));
        assert!(dir.to_string_lossy().contains(".sqlkit/jre"));
    }

    #[test]
    fn test_managed_jre_java_path() {
        let path = managed_jre_java_path();
        let s = path.to_string_lossy();
        assert!(s.contains(".sqlkit/jre"));
        assert!(s.contains("java"));
    }

    #[test]
    fn test_jre_archive_name_macos_aarch64() {
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        assert_eq!(jre_archive_name(), "jre-macos-aarch64.tar.gz");

        #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
        let _ = jre_archive_name(); // no-op: test only applies on macOS ARM
    }

    #[test]
    fn test_jre_archive_name_non_empty_for_known_platform() {
        let name = jre_archive_name();
        // On known platforms the name is non-empty; on unknown platforms
        // it's empty by design. Both are valid — we just verify format when set.
        if !name.is_empty() {
            assert!(name.starts_with("jre-"));
            assert!(
                name.ends_with(".tar.gz") || name.ends_with(".zip"),
                "Expected .tar.gz or .zip, got: {}",
                name
            );
        }
    }

    #[test]
    fn test_is_valid_java() {
        // Non-existent path is invalid
        assert!(!JreDetector::is_valid_java(&PathBuf::from("/nonexistent/java")));

        // The current executable should exist
        assert!(
            JreDetector::is_valid_java(&std::env::current_exe().unwrap_or_else(|_| PathBuf::from("/"))),
            "current_exe() should be valid"
        );
    }
}
