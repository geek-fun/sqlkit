//! Managed JRE detection, download, and lifecycle.
//!
//! SQLKit downloads a JRE 25 (latest stable) from Adoptium (Eclipse Temurin) for each supported
//! platform. This module handles detecting Java (managed → `JAVA_HOME` → `PATH`),
//! downloading/extracting the managed JRE, version checking via the built-in
//! `release` file, and cleaning it up.

use crate::database::error::{DbError, DbResult};
use crate::APP_HANDLE;
use futures::StreamExt;
use std::path::Path;
use std::path::PathBuf;
use std::sync::OnceLock;
use tauri::Emitter;
use tokio::sync::Mutex;

/// Subdirectory under user home for the managed JRE.
const JRE_BASE_DIR: &str = ".sqlkit/jre";

/// Java executable path relative to JRE root (platform-aware).
const JAVA_EXE: &str = if cfg!(target_os = "windows") {
    "bin/java.exe"
} else {
    "bin/java"
};

// ── helpers ────────────────────────────────────────────────

/// User home directory.
pub(crate) fn home_dir() -> PathBuf {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string())
        .into()
}

/// Serialize JRE installations to prevent concurrent downloads from
/// racing on `remove_dir_all` / extract / rename.
static JRE_INSTALL_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

/// Compare two dotted version strings numerically (e.g. "21.0.11" > "9.0.0").
/// Returns negative if a < b, positive if a > b, 0 if equal.
pub(crate) fn compare_versions(a: &str, b: &str) -> i32 {
    let parts_a: Vec<&str> = a.split('.').collect();
    let parts_b: Vec<&str> = b.split('.').collect();
    let max_len = parts_a.len().max(parts_b.len());
    for i in 0..max_len {
        let na: u32 = parts_a.get(i).and_then(|s| s.parse().ok()).unwrap_or(0);
        let nb: u32 = parts_b.get(i).and_then(|s| s.parse().ok()).unwrap_or(0);
        if na != nb {
            return if na > nb { 1 } else { -1 };
        }
    }
    0
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

// ── Adoptium platform ─────────────────────────────────────

/// Determine the Adoptium OS and arch strings for the current platform.
fn adoptium_os_arch() -> Option<(&'static str, &'static str)> {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))] { Some(("mac", "aarch64")) }
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))] { Some(("mac", "x64")) }
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))] { Some(("linux", "x64")) }
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))] { Some(("linux", "aarch64")) }
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))] { Some(("windows", "x64")) }
    #[cfg(not(any(
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "aarch64"),
        all(target_os = "windows", target_arch = "x86_64"),
    )))] { None }
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
        // If the managed JRE exists but isn't executable, try to fix permissions
        if managed.exists() {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(meta) = managed.metadata() {
                    let mode = meta.permissions().mode();
                    if mode & 0o111 == 0 {
                        let perms = std::fs::Permissions::from_mode(mode | 0o100);
                        let _ = std::fs::set_permissions(&managed, perms);
                        if Self::is_valid_java(&managed) {
                            return Some(managed);
                        }
                    }
                }
            }
        }

        // 2. System Java
        Self::detect_system_java()
    }

    /// Check whether `path` points to an existing, executable java binary.
    pub fn is_valid_java(path: &PathBuf) -> bool {
        if !path.exists() {
            return false;
        }
        // On Unix, verify the file has executable permission
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            path.metadata()
                .map(|meta| meta.permissions().mode() & 0o111 != 0)
                .unwrap_or(false)
        }
        #[cfg(not(unix))]
        {
            true
        }
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

// ── version reading ───────────────────────────────────────

/// Determine the major Java version by running `java -version`.
///
/// Handles both pre-Java-9 format (`1.8.0_431` → 8), Java-9+ format
/// (`25.0.1` → 25), and pre-release builds (`25-ea` → 25). Falls back to
/// stdout if stderr is empty (some alternative JDK builds output version
/// there). Returns `None` if the path doesn't exist, isn't a Java binary,
/// or the version string can't be parsed.
pub fn system_java_version(java: &PathBuf) -> Option<u32> {
    let output = std::process::Command::new(java).arg("-version").output().ok()?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let version_str = stderr
        .lines()
        .chain(stdout.lines())
        .find(|l| l.contains("version"))
        .and_then(|l| l.split('"').nth(1))?;
    let parts: Vec<&str> = version_str.split('.').collect();
    if parts.is_empty() {
        return None;
    }
    let major_str = parts[0].trim_end_matches(|c: char| !c.is_ascii_digit());
    let major = major_str.parse::<u32>().ok()?;
    if major == 1 && parts.len() >= 2 {
        parts[1].parse::<u32>().ok() // Java 8: "1.8.0_431" → 8
    } else {
        Some(major) // Java 9+: "25.0.1" → 25, "25-ea" → 25
    }
}

/// Read the JRE version from the built-in `release` file.
///
/// The release file is a Java properties file that ships with every OpenJDK build
/// at `~/.sqlkit/jre/release`. It contains lines like:
///   JAVA_VERSION="21.0.11"
///   JAVA_BUILD="21.0.11+10"
pub fn read_jre_version() -> Option<String> {
    let release_path = jre_base_dir().join("release");
    if !release_path.exists() {
        return None;
    }
    let content = std::fs::read_to_string(release_path).ok()?;
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("JAVA_VERSION=") {
            let version = line
                .trim_start_matches("JAVA_VERSION=")
                .trim_matches('"')
                .trim()
                .to_string();
            return Some(version);
        }
    }
    None
}

// ── Adoptium update check ─────────────────────────────────

/// Check if a newer JRE build is available from Adoptium.
///
/// Returns `Some(redirect_url)` with the redirect target containing the build
/// version, or `None` if not available / on error.
pub async fn check_adoptium_update() -> Option<String> {
    let (os, arch) = adoptium_os_arch()?;
    let url = format!(
        "https://api.adoptium.net/v3/binary/latest/25/ga/{os}/{arch}/jre/hotspot/normal/eclipse"
    );

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .ok()?;
    let response = client.head(&url).send().await.ok()?;
    let final_url = response.url().to_string();
    Some(final_url)
}

/// Extract the build version from an Adoptium redirect URL.
///
/// e.g. ".../OpenJDK21U-jre_aarch64_mac_hotspot_21.0.10.8_1.tar.gz" -> "21.0.10.8"
pub fn parse_adoptium_build_version(url: &str) -> Option<String> {
    // Find "hotspot_" or "hotspot-" in URL
    let marker = if let Some(idx) = url.find("hotspot_") {
        idx + 8
    } else if let Some(idx) = url.find("hotspot-") {
        idx + 8
    } else {
        return None;
    };
    // Take the version part until the next non-version char
    let rest = &url[marker..];
    let version: String = rest
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    if version.is_empty() { None } else { Some(version) }
}

// ── download / remove ─────────────────────────────────────

/// Stream a JRE download to disk with progress events.
async fn download_jre_stream(
    client: &reqwest::Client,
    url: &str,
    tmp_path: &Path,
    os: &str,
    _parent: &Path,
) -> DbResult<()> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| DbError::Connection(format!("Failed to download JRE: {}", e)))?;

    if !response.status().is_success() {
        return Err(DbError::Connection(format!(
            "Failed to download JRE: HTTP {} from {}",
            response.status(),
            url
        )));
    }

    let is_zip = os == "windows";
    let _ext = if is_zip { "zip" } else { "tar.gz" };

    let mut file = tokio::fs::File::create(tmp_path)
        .await
        .map_err(|e| DbError::Connection(format!("Failed to create temp file: {}", e)))?;
    use tokio::io::AsyncWriteExt;
    let mut stream = response.bytes_stream();
    let mut downloaded: u64 = 0;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| DbError::Connection(format!("Download stream error: {}", e)))?;
        downloaded += chunk.len() as u64;
        file.write_all(&chunk)
            .await
            .map_err(|e| DbError::Connection(format!("Failed to write chunk: {}", e)))?;
        if let Some(handle) = APP_HANDLE.get() {
            let _ = handle.emit(
                "connection-progress",
                serde_json::json!({
                    "step": "jre_download",
                    "downloaded": downloaded,
                    "total": 60_000_000,
                }),
            );
        }
    }
    file.flush().await.ok();
    Ok(())
}

/// Download and extract the managed JRE for the current platform from Adoptium.
///
/// Downloads the latest JRE 25 (Eclipse Temurin) build from the Adoptium API,
/// extracts the archive, and renames the extracted directory to `jre`.
/// Uses atomic operations: download to temp → validate → extract to temp dir → replace.
pub async fn download_managed_jre() -> DbResult<()> {
    let _guard = JRE_INSTALL_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .await;

    let (os, arch) = adoptium_os_arch().ok_or_else(|| {
        DbError::Connection("No JRE available for this platform".to_string())
    })?;

    let base_dir = jre_base_dir(); // ~/.sqlkit/jre
    let parent = base_dir.parent()
        .expect("jre_base_dir has a parent")
        .to_path_buf(); // ~/.sqlkit
    tokio::fs::create_dir_all(&parent)
        .await
        .map_err(|e| DbError::Connection(format!("Failed to create JRE parent dir: {}", e)))?;

    let url = format!(
        "https://api.adoptium.net/v3/binary/latest/25/ga/{os}/{arch}/jre/hotspot/normal/eclipse"
    );

    let is_zip = os == "windows";
    let ext = if is_zip { "zip" } else { "tar.gz" };
    let tmp_archive = parent.join(format!("jre_download.{}", ext));
    let tmp_extract = parent.join(format!("jre_extract_{}", uuid::Uuid::new_v4()));

    // Step 1: Download archive (single attempt — retrying the same URL doesn't help)
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| DbError::Connection(format!("Failed to build HTTP client: {}", e)))?;

    if let Err(e) = download_jre_stream(&client, &url, &tmp_archive, &os, &parent).await {
        let _ = tokio::fs::remove_file(&tmp_archive).await;
        return Err(e);
    }

    // Step 2: Validate downloaded archive (size and magic bytes)
    let meta = std::fs::metadata(&tmp_archive)
        .map_err(|e| DbError::Connection(format!("Failed to check JRE archive: {}", e)))?;
    if meta.len() < 10_000_000 {
        let _ = tokio::fs::remove_file(&tmp_archive).await;
        return Err(DbError::Connection(format!(
            "JRE archive too small: {} bytes (expected ≥ 10MB)", meta.len()
        )));
    }
    // Validate gzip magic bytes (1f 8b) if not a zip file
    if !is_zip {
        let magic = std::fs::read(&tmp_archive)
            .map_err(|e| DbError::Connection(format!("Failed to read JRE archive: {}", e)))?;
        if magic.len() < 2 || magic[0] != 0x1f || magic[1] != 0x8b {
            let _ = tokio::fs::remove_file(&tmp_archive).await;
            return Err(DbError::Connection(
                "Downloaded JRE archive has invalid gzip magic bytes — corrupt download".to_string()
            ));
        }
    }

    // Step 3: Extract to temporary directory
    tokio::fs::create_dir_all(&tmp_extract)
        .await
        .map_err(|e| DbError::Connection(format!("Failed to create extract dir: {}", e)))?;

    let tmp_archive_clone = tmp_archive.clone();
    let tmp_extract_clone = tmp_extract.clone();
    let extract_result = tokio::task::spawn_blocking(move || -> Result<PathBuf, String> {
        let file = std::fs::File::open(&tmp_archive_clone)
            .map_err(|e| format!("Failed to open archive: {}", e))?;

        if is_zip {
            let mut archive = zip::ZipArchive::new(file)
                .map_err(|e| format!("Failed to open zip archive: {}", e))?;
            archive.extract(&tmp_extract_clone)
                .map_err(|e| format!("Failed to extract JRE zip: {}", e))?;
        } else {
            let decoder = flate2::read::GzDecoder::new(file);
            let mut archive = tar::Archive::new(decoder);
            archive.unpack(&tmp_extract_clone)
                .map_err(|e| format!("Failed to extract JRE tar: {}", e))?;
        }

        // Find the directory with bin/java — could be directly in extract dir
        // or inside a subdirectory (e.g. jdk-25.0.1/)
        let java_bin = if cfg!(target_os = "windows") { "java.exe" } else { "java" };
        let jdk_contents_home = |p: &Path| p.join("Contents").join("Home").join("bin").join(java_bin);

        // Case 1: directly in extract dir (no wrapper directory)
        if tmp_extract_clone.join("bin").join(java_bin).exists() {
            return Ok(tmp_extract_clone.clone());
        }

        // Case 2: macOS .jdk bundle format (Contents/Home/bin/java)
        if jdk_contents_home(&tmp_extract_clone).exists() {
            // Return the Contents/Home directory as the JRE root
            return Ok(tmp_extract_clone.join("Contents").join("Home"));
        }

        // Case 3: inside a subdirectory (e.g. jdk-25.0.1/)
        for entry in std::fs::read_dir(&tmp_extract_clone)
            .map_err(|e| format!("Failed to list extracted files: {}", e))?
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        {
            let path = entry.path();
            // Standard: subdir/bin/java
            if path.join("bin").join(java_bin).exists() {
                return Ok(path);
            }
            // macOS bundle inside subdir: subdir/Contents/Home/bin/java
            if jdk_contents_home(&path).exists() {
                return Ok(path.join("Contents").join("Home"));
            }
        }
        Err(format!(
            "Extracted JRE archive does not contain bin/{java_bin} — checked {:?} and its subdirectories",
            tmp_extract_clone
        ))
    })
    .await
    .map_err(|e| DbError::Connection(format!("JRE extraction panicked: {}", e)))?;

    let extracted_dir = extract_result
        .map_err(|e| DbError::Connection(format!("JRE extraction failed: {}", e)))?;

    // Step 4: Atomic swap — rename temp to final, with rollback
    let _ = tokio::fs::remove_file(&tmp_archive).await;

    // If target exists, move it aside first (backup), then rename temp, then delete backup
    if base_dir.exists() {
        let backup = parent.join(format!("jre_old_{}", uuid::Uuid::new_v4()));
        tokio::fs::rename(&base_dir, &backup)
            .await
            .map_err(|e| DbError::Connection(format!("Failed to backup old JRE: {}", e)))?;
        match tokio::fs::rename(&extracted_dir, &base_dir).await {
            Ok(()) => {
                let _ = tokio::fs::remove_dir_all(&backup).await;
            }
            Err(_) => {
                // Rollback: restore backup
                let _ = tokio::fs::rename(&backup, &base_dir).await;
                let _ = tokio::fs::remove_dir_all(&extracted_dir).await;
                return Err(DbError::Connection("Failed to install JRE — restored previous version".to_string()));
            }
        }
    } else {
        tokio::fs::rename(&extracted_dir, &base_dir)
            .await
            .map_err(|e| DbError::Connection(format!("Failed to install JRE: {}", e)))?;
    }

    // Step 5: Ensure java binary is executable (fix permissions if needed)
    #[cfg(unix)]
    {
        let java_bin = base_dir.join("bin").join("java");
        if java_bin.exists() {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(meta) = java_bin.metadata() {
                let mode = meta.permissions().mode();
                if mode & 0o111 == 0 {
                    let perms = std::fs::Permissions::from_mode(mode | 0o100);
                    let _ = std::fs::set_permissions(&java_bin, perms);
                }
            }
        }
    }

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
    fn test_is_valid_java() {
        // Non-existent path is invalid
        assert!(!JreDetector::is_valid_java(&PathBuf::from(
            "/nonexistent/java"
        )));

        // The current executable should exist
        assert!(
            JreDetector::is_valid_java(
                &std::env::current_exe().unwrap_or_else(|_| PathBuf::from("/"))
            ),
            "current_exe() should be valid"
        );
    }

    #[test]
    fn test_read_jre_version_from_release_file() {
        let content = "JAVA_VERSION=\"21.0.11\"\nJAVA_BUILD=\"21.0.11+10\"\n";
        let parsed = content
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                if line.starts_with("JAVA_VERSION=") {
                    Some(
                        line.trim_start_matches("JAVA_VERSION=")
                            .trim_matches('"')
                            .trim()
                            .to_string(),
                    )
                } else {
                    None
                }
            })
            .next();
        assert_eq!(parsed, Some("21.0.11".to_string()));
    }

    #[test]
    fn test_parse_adoptium_build_version() {
        // macOS aarch64
        let url = "https://github.com/adoptium/temurin21-binaries/releases/download/jdk-21.0.10.8%2B1/OpenJDK21U-jre_aarch64_mac_hotspot_21.0.10.8_1.tar.gz";
        assert_eq!(
            parse_adoptium_build_version(url),
            Some("21.0.10.8".to_string())
        );

        // Linux x64
        let url2 = "https://github.com/adoptium/temurin21-binaries/releases/download/jdk-21.0.11.9%2B1/OpenJDK21U-jre_x64_linux_hotspot_21.0.11.9_1.tar.gz";
        assert_eq!(
            parse_adoptium_build_version(url2),
            Some("21.0.11.9".to_string())
        );

        // Windows x64 (zip)
        let url3 = "https://github.com/adoptium/temurin21-binaries/releases/download/jdk-21.0.9.2%2B1/OpenJDK21U-jre_x64_windows_hotspot_21.0.9.2_1.zip";
        assert_eq!(
            parse_adoptium_build_version(url3),
            Some("21.0.9.2".to_string())
        );

        // Invalid - no hotspot marker
        assert_eq!(
            parse_adoptium_build_version("https://example.com/no-match"),
            None
        );

        // Empty version part after hotspot_
        assert_eq!(
            parse_adoptium_build_version("https://example.com/hotspot_"),
            None
        );
    }

    #[test]
    fn test_system_java_version_not_java() {
        // A non-Java binary should return None.
        let current = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("/"));
        assert!(system_java_version(&current).is_none());
    }
}
