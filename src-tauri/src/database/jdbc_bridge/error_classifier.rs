//! Error classification for JDBC connection errors.
//!
//! Maps raw JDBC error messages to structured categories so that the
//! application can react appropriately (e.g., retry with a different
//! driver version on `VersionIncompatible`, prompt for credentials on
//! `Authentication`).

use crate::database::config::DatabaseType;

/// Classification of a database connection error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorCategory {
    /// Driver version is incompatible with the database server version.
    /// Suggests trying an older/newer JDBC driver JAR.
    VersionIncompatible,
    /// Authentication failure — wrong credentials, user not found, etc.
    Authentication,
    /// Network-level connectivity issue (refused, unreachable, reset, etc.).
    Network,
    /// Connection or socket read timed out.
    Timeout,
    /// Unclassified / unknown error.
    Unknown,
}

/// Classify a JDBC connection error message into an [`ErrorCategory`].
///
/// Classification priority:
/// 1. **Version incompatibility** — matched against `version_patterns`
///    (from the driver registry in `drivers.toml`).
/// 2. **Authentication** — keywords such as "authentication failed",
///    "ORA-01017", "Access denied", etc.
/// 3. **Timeout** — keywords such as "timed out", "timeout".
/// 4. **Network** — keywords such as "connection refused", "host unknown",
///    "Connection reset", etc.
/// 5. **Unknown** — fallback when no pattern matches.
///
/// All matching is **case-insensitive** via `to_lowercase()`.
///
/// # Arguments
///
/// * `db_type` — The target database type (used for context, currently
///   informational but reserved for future type-specific overrides).
/// * `error_message` — The raw error string from the JDBC driver / Java
///   subprocess.
/// * `version_patterns` — Version-incompatibility signatures from the
///   driver registry for this database type (see `drivers.toml`).
pub fn classify_connection_error(
    _db_type: DatabaseType,
    error_message: &str,
    version_patterns: &[String],
) -> ErrorCategory {
    let msg = error_message.trim();
    if msg.is_empty() {
        return ErrorCategory::Unknown;
    }
    let msg_lower = msg.to_lowercase();

    // 1. Version incompatibility patterns (from drivers.toml)
    for pattern in version_patterns {
        if msg_lower.contains(&pattern.to_lowercase()) {
            return ErrorCategory::VersionIncompatible;
        }
    }

    // 2. Authentication keywords
    let auth_keywords: &[&str] = &[
        "authentication failed",
        "invalid username/password",
        "login denied",
        "ora-01017",
        "sql30082n",
        "wrong user name or password",
        "access denied",
        "password incorrect",
    ];
    for keyword in auth_keywords {
        if msg_lower.contains(keyword) {
            return ErrorCategory::Authentication;
        }
    }

    // 3. Timeout keywords (checked before general network keywords
    //    so that "timed out" maps to Timeout, not Network).
    if msg_lower.contains("timed out") || msg_lower.contains("time out") {
        return ErrorCategory::Timeout;
    }

    // 4. Network / connectivity keywords
    let network_keywords: &[&str] = &[
        "connection refused",
        "connection reset",
        "unreachable",
        "host unknown",
        "unknown host",
        "io error",
        "network adapter",
    ];
    for keyword in network_keywords {
        if msg_lower.contains(keyword) {
            return ErrorCategory::Network;
        }
    }

    // 5. Timeout (broad match after specific network checks)
    if msg_lower.contains("timeout") {
        return ErrorCategory::Timeout;
    }

    // 6. Fallback
    ErrorCategory::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Version incompatibility ──

    #[test]
    fn test_classify_oracle_version_error() {
        let patterns = vec!["ORA-28040".into(), "ORA-03134".into()];
        let result = classify_connection_error(
            DatabaseType::Oracle,
            "ORA-28040: No matching authentication protocol",
            &patterns,
        );
        assert_eq!(result, ErrorCategory::VersionIncompatible);
    }

    #[test]
    fn test_classify_oracle_version_alt() {
        let patterns = vec!["ORA-28040".into(), "ORA-03134".into()];
        let result = classify_connection_error(
            DatabaseType::Oracle,
            "ORA-03134: Connections to this server version are no longer supported",
            &patterns,
        );
        assert_eq!(result, ErrorCategory::VersionIncompatible);
    }

    #[test]
    fn test_classify_version_no_match() {
        let patterns = vec!["ORA-28040".into()];
        let result =
            classify_connection_error(DatabaseType::Oracle, "Something else entirely", &patterns);
        assert_ne!(result, ErrorCategory::VersionIncompatible);
    }

    #[test]
    fn test_classify_empty_patterns() {
        let result = classify_connection_error(DatabaseType::Oracle, "ORA-28040: some error", &[]);
        assert_ne!(result, ErrorCategory::VersionIncompatible);
    }

    #[test]
    fn test_classify_oracle_auth() {
        let result = classify_connection_error(
            DatabaseType::Oracle,
            "ORA-01017: invalid username/password; logon denied",
            &[],
        );
        assert_eq!(result, ErrorCategory::Authentication);
    }

    #[test]
    fn test_classify_mysql_auth() {
        let result = classify_connection_error(
            DatabaseType::MySQL,
            "Access denied for user 'root'@'localhost'",
            &[],
        );
        assert_eq!(result, ErrorCategory::Authentication);
    }

    #[test]
    fn test_classify_db2_auth() {
        let result = classify_connection_error(
            DatabaseType::DB2,
            "SQL30082N  Security processing failed with reason",
            &[],
        );
        assert_eq!(result, ErrorCategory::Authentication);
    }

    #[test]
    fn test_classify_generic_auth() {
        let result =
            classify_connection_error(DatabaseType::H2, "wrong user name or password", &[]);
        assert_eq!(result, ErrorCategory::Authentication);
    }

    #[test]
    fn test_classify_connection_refused() {
        let result = classify_connection_error(
            DatabaseType::Oracle,
            "Connection refused: connect to 192.168.1.1:1521",
            &[],
        );
        assert_eq!(result, ErrorCategory::Network);
    }

    #[test]
    fn test_classify_host_unknown() {
        let result =
            classify_connection_error(DatabaseType::MySQL, "Unknown host 'db.example.com'", &[]);
        assert_eq!(result, ErrorCategory::Network);
    }

    #[test]
    fn test_classify_io_error() {
        let result = classify_connection_error(
            DatabaseType::Oracle,
            "IO Error: The Network Adapter could not establish the connection",
            &[],
        );
        assert_eq!(result, ErrorCategory::Network);
    }

    #[test]
    fn test_classify_timeout() {
        let result = classify_connection_error(
            DatabaseType::PostgreSQL,
            "Connection timed out: connect to localhost:5432",
            &[],
        );
        assert_eq!(result, ErrorCategory::Timeout);
    }

    #[test]
    fn test_classify_connect_timeout() {
        let result =
            classify_connection_error(DatabaseType::Oracle, "connect timed out after 30000ms", &[]);
        assert_eq!(result, ErrorCategory::Timeout);
    }

    #[test]
    fn test_classify_timeout_broad() {
        let result =
            classify_connection_error(DatabaseType::Oracle, "timeout occurred during auth", &[]);
        assert_eq!(result, ErrorCategory::Timeout);
    }

    #[test]
    fn test_classify_unknown() {
        let result =
            classify_connection_error(DatabaseType::Oracle, "Something went horribly wrong", &[]);
        assert_eq!(result, ErrorCategory::Unknown);
    }

    #[test]
    fn test_classify_empty_message() {
        let result = classify_connection_error(DatabaseType::Oracle, "", &[]);
        assert_eq!(result, ErrorCategory::Unknown);
    }

    #[test]
    fn test_classify_unrecognized_db_type() {
        let result = classify_connection_error(
            DatabaseType::Snowflake,
            "Access denied for user 'readonly'@'host'",
            &[],
        );
        assert_eq!(result, ErrorCategory::Authentication);
    }
}
