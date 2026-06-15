use serde::Serialize;

/// Events emitted during the JDBC auto-detect connection flow.
/// These are sent from the backend to the frontend via Tauri events.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case", tag = "step")]
pub enum ConnectionProgress {
    /// Downloading the managed JRE.
    #[serde(rename_all = "snake_case")]
    JreDownload { downloaded: u64, total: u64 },
    /// Extracting the JRE archive.
    JreExtract,
    /// Downloading a specific JDBC driver JAR.
    #[serde(rename_all = "snake_case")]
    DriverDownload {
        db_type: String,
        driver_version: String,
        downloaded: u64,
        total: u64,
    },
    /// Attempting connection with a specific driver.
    #[serde(rename_all = "snake_case")]
    Connecting {
        db_type: String,
        driver_version: String,
        attempt: u32,
        total_attempts: u32,
    },
    /// Version incompatibility detected — falling back to next driver.
    #[serde(rename_all = "snake_case")]
    VersionFallback {
        db_type: String,
        from_driver: String,
        to_driver: String,
        error: String,
    },
    /// Successfully connected.
    #[serde(rename_all = "snake_case")]
    Connected {
        db_type: String,
        driver_version: String,
    },
    /// Fatal error — abort.
    #[serde(rename_all = "snake_case")]
    Error { error: String },
}
