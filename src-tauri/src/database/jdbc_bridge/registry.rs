//! JDBC driver registry — parses the embedded `drivers.toml` at compile time.
//!
//! Provides:
//! - [`DriverRegistry`] — singleton loaded via `OnceLock` + `include_str!`
//! - [`resolve_maven_url`] — constructs a Maven Central download URL
//! - [`build_jdbc_url`] — substitutes `{host}`, `{port}`, `{database}` in a template

use serde::Deserialize;
use std::sync::OnceLock;

use crate::database::config::DatabaseType;

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

/// Top-level registry, mirroring `drivers.toml`.
#[derive(Debug, Clone, Deserialize)]
pub struct DriverRegistry {
    /// Map of database key → driver config (e.g. `"oracle"`, `"db2"`).
    pub databases: std::collections::HashMap<String, DatabaseDriverConfig>,
}

/// Configuration for a single database type's JDBC driver.
#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseDriverConfig {
    /// Human-readable display name (e.g. `"Oracle Database"`).
    pub name: String,
    /// Fully qualified JDBC driver class name.
    pub class_name: String,
    /// Maven group ID (e.g. `"com.oracle.database.jdbc"`).
    pub maven_group: String,
    /// Maven artifact ID (e.g. `"ojdbc11"`).
    pub maven_artifact: String,
    /// JDBC URL template with `{host}`, `{port}`, `{database}` placeholders.
    pub jdbc_url_template: String,
    /// Default port for this database type.
    #[serde(default)]
    pub default_port: Option<u16>,
    /// Minimum JRE version required.
    #[serde(default)]
    pub min_jre_version: Option<String>,
    /// Error-message substrings that signal a version mismatch.
    #[serde(default)]
    pub version_error_signatures: Vec<String>,
    /// Ordered fallback chain of driver versions (tried from first to last).
    pub versions: Vec<DriverVersion>,
}

/// A single driver version entry in the fallback chain.
#[derive(Debug, Clone, Deserialize)]
pub struct DriverVersion {
    /// Driver version string (e.g. `"21.15.0.0"`).
    pub version: String,
    /// Minimum database version this driver supports.
    #[serde(default)]
    pub min_db_version: Option<String>,
    /// User-facing label for UI selection.
    pub label: String,
    /// SHA-256 checksum of the JAR (empty string = not verified).
    #[serde(default)]
    pub jar_sha256: String,
    /// Per-version override of the Maven group (if different from parent).
    #[serde(default)]
    pub maven_group_override: Option<String>,
    /// Per-version override of the Maven artifact (if different from parent).
    #[serde(default)]
    pub maven_artifact_override: Option<String>,
    /// Additional error signatures specific to this version.
    #[serde(default)]
    pub version_error_signatures: Vec<String>,
    /// Maven classifier (e.g. `"standalone"` for hive-jdbc-{version}-standalone.jar).
    /// When set, the download URL becomes: {artifact}-{version}-{classifier}.jar
    #[serde(default)]
    pub maven_classifier: Option<String>,
}

// ---------------------------------------------------------------------------
// Singleton
// ---------------------------------------------------------------------------

static DRIVER_REGISTRY: OnceLock<DriverRegistry> = OnceLock::new();

impl DriverRegistry {
    /// Load (or retrieve) the compiled-in driver registry.
    ///
    /// The TOML is embedded via `include_str!("drivers.toml")` and parsed once.
    pub fn load() -> &'static Self {
        DRIVER_REGISTRY.get_or_init(|| {
            let toml_str = include_str!("drivers.toml");
            toml::from_str(toml_str).expect("Failed to parse embedded drivers.toml")
        })
    }

    /// Return the TOML key for a database type, or `None` if it has no JDBC
    /// bridge registry entry.
    pub fn registry_key(db_type: DatabaseType) -> Option<&'static str> {
        db_type_to_registry_key(db_type)
    }

    /// Get the full driver-version fallback chain for a given database type.
    pub fn get_driver_chain(&self, db_type: DatabaseType) -> Option<&[DriverVersion]> {
        let key = Self::registry_key(db_type)?;
        self.databases.get(key).map(|cfg| cfg.versions.as_slice())
    }

    /// Get the full driver configuration for a given database type.
    pub fn get_config(&self, db_type: DatabaseType) -> Option<&DatabaseDriverConfig> {
        let key = Self::registry_key(db_type)?;
        self.databases.get(key)
    }
}

// ---------------------------------------------------------------------------
// Public helper functions
// ---------------------------------------------------------------------------

/// Construct a Maven Central download URL for a driver version.
///
/// Respects per-version `maven_group_override` and `maven_artifact_override`;
/// falls back to the base group/artifact when those are `None`.
pub fn resolve_maven_url(entry: &DriverVersion, base_group: &str, base_artifact: &str) -> String {
    let group = entry.maven_group_override.as_deref().unwrap_or(base_group);
    let artifact = entry
        .maven_artifact_override
        .as_deref()
        .unwrap_or(base_artifact);
    let version = &entry.version;
    let group_path = group.replace('.', "/");
    let classifier_suffix = entry
        .maven_classifier
        .as_ref()
        .map(|c| format!("-{c}"))
        .unwrap_or_default();
    format!(
        "https://repo1.maven.org/maven2/{group_path}/{artifact}/{version}/{artifact}-{version}{classifier_suffix}.jar"
    )
}

/// Substitute `{host}`, `{port}`, and `{database}` placeholders in a JDBC URL
/// template.
///
/// When `database` is `None`, the `{database}` placeholder and any associated
/// parameter prefix (e.g. `;httpPath=`, `/DATABASE=`) are removed from the
/// template to avoid dangling empty parameters.
pub fn build_jdbc_url(
    config: &DatabaseDriverConfig,
    host: &str,
    port: u16,
    database: Option<&str>,
) -> String {
    let url = config
        .jdbc_url_template
        .replace("{host}", host)
        .replace("{port}", &port.to_string());
    match database {
        Some(db) => url.replace("{database}", db),
        None => {
            let url = url.replace("{database}", "");
            // Remove dangling parameter prefixes left after {database} removal
            let url = url
                .replace(";httpPath=", "")
                .replace("/DATABASE=", "")
                .replace(";schema=", "")
                .replace(";ProjectId=", "")
                .replace(":INFORMIXSERVER=", "");
            // Trim trailing separators that may be left behind
            url.trim_end_matches(&[';', '&', '?'][..]).to_string()
        }
    }
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Map a [`DatabaseType`] to its registry-key string in `drivers.toml`.
fn db_type_to_registry_key(db: DatabaseType) -> Option<&'static str> {
    match db {
        DatabaseType::Oracle => Some("oracle"),
        DatabaseType::DB2 => Some("db2"),
        DatabaseType::H2 => Some("h2"),
        DatabaseType::Derby => Some("derby"),
        DatabaseType::Snowflake => Some("snowflake"),
        DatabaseType::DM8Oracle => Some("dm8_oracle"),
        DatabaseType::XuguDB => Some("xugudb"),
        DatabaseType::GBase8a => Some("gbase8a"),
        DatabaseType::Hive => Some("hive"),
        DatabaseType::Databricks => Some("databricks"),
        DatabaseType::Hana => Some("hana"),
        DatabaseType::Teradata => Some("teradata"),
        DatabaseType::Vertica => Some("vertica"),
        DatabaseType::Exasol => Some("exasol"),
        DatabaseType::BigQuery => Some("bigquery"),
        DatabaseType::Informix => Some("informix"),
        DatabaseType::Kylin => Some("kylin"),
        DatabaseType::Cassandra => Some("cassandra"),
        DatabaseType::Iris => Some("iris"),
        DatabaseType::Access => Some("access"),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_loads_from_embedded_toml() {
        let registry = DriverRegistry::load();
        assert!(
            !registry.databases.is_empty(),
            "registry should contain at least one database"
        );
        // Sanity: keys we know exist
        assert!(registry.databases.contains_key("oracle"));
        assert!(registry.databases.contains_key("db2"));
        assert!(registry.databases.contains_key("h2"));
        assert!(registry.databases.contains_key("hive"));
        assert!(registry.databases.contains_key("databricks"));
        assert!(registry.databases.contains_key("hana"));
        assert!(registry.databases.contains_key("teradata"));
        assert!(registry.databases.contains_key("vertica"));
        assert!(registry.databases.contains_key("exasol"));
        assert!(registry.databases.contains_key("bigquery"));
        assert!(registry.databases.contains_key("informix"));
        assert!(registry.databases.contains_key("kylin"));
        assert!(registry.databases.contains_key("cassandra"));
        assert!(registry.databases.contains_key("iris"));
        assert!(registry.databases.contains_key("access"));
    }

    #[test]
    fn test_get_config_oracle_returns_config() {
        let registry = DriverRegistry::load();
        let config = registry.get_config(DatabaseType::Oracle);
        assert!(config.is_some(), "Oracle should have a config");
        assert_eq!(config.unwrap().name, "Oracle Database");
    }

    #[test]
    fn test_get_config_db2_returns_config() {
        let registry = DriverRegistry::load();
        let config = registry.get_config(DatabaseType::DB2);
        assert!(config.is_some(), "DB2 should have a config");
        assert_eq!(config.unwrap().name, "IBM DB2");
    }

    #[test]
    fn test_get_driver_chain_oracle_has_three_versions() {
        let registry = DriverRegistry::load();
        let chain = registry.get_driver_chain(DatabaseType::Oracle);
        assert!(chain.is_some(), "Oracle should have a driver chain");
        assert_eq!(chain.unwrap().len(), 3);
    }

    #[test]
    fn test_get_driver_chain_h2_has_two_versions() {
        let registry = DriverRegistry::load();
        let chain = registry.get_driver_chain(DatabaseType::H2);
        assert!(chain.is_some(), "H2 should have a driver chain");
        assert_eq!(chain.unwrap().len(), 2);
    }

    #[test]
    fn test_get_driver_chain_unknown_returns_none() {
        let registry = DriverRegistry::load();
        let chain = registry.get_driver_chain(DatabaseType::PostgreSQL);
        assert!(chain.is_none(), "PostgreSQL is not a JDBC-bridge database");
        let chain = registry.get_driver_chain(DatabaseType::MySQL);
        assert!(chain.is_none(), "MySQL is not a JDBC-bridge database");
        let chain = registry.get_driver_chain(DatabaseType::SqlServer);
        assert!(chain.is_none(), "SQL Server is not a JDBC-bridge database");
    }

    #[test]
    fn test_resolve_maven_url_with_overrides() {
        let entry = DriverVersion {
            version: "21.15.0.0".into(),
            min_db_version: Some("19c".into()),
            label: "ojdbc11".into(),
            jar_sha256: "".into(),
            maven_group_override: None,
            maven_artifact_override: Some("ojdbc11".into()),
            version_error_signatures: vec![],
        };
        let url = resolve_maven_url(&entry, "com.oracle.database.jdbc", "ojdbc11");
        assert_eq!(
            url,
            "https://repo1.maven.org/maven2/com/oracle/database/jdbc/ojdbc11/21.15.0.0/ojdbc11-21.15.0.0.jar"
        );
    }

    #[test]
    fn test_resolve_maven_url_without_overrides() {
        let entry = DriverVersion {
            version: "2.2.224".into(),
            min_db_version: Some("2.x".into()),
            label: "h2".into(),
            jar_sha256: "".into(),
            maven_group_override: None,
            maven_artifact_override: None,
            version_error_signatures: vec![],
        };
        let url = resolve_maven_url(&entry, "com.h2database", "h2");
        assert_eq!(
            url,
            "https://repo1.maven.org/maven2/com/h2database/h2/2.2.224/h2-2.2.224.jar"
        );
    }

    #[test]
    fn test_build_jdbc_url_simple() {
        let config = DatabaseDriverConfig {
            name: "H2 Database".into(),
            class_name: "org.h2.Driver".into(),
            maven_group: "com.h2database".into(),
            maven_artifact: "h2".into(),
            jdbc_url_template: "jdbc:h2:tcp://{host}:{port}/{database}".into(),
            default_port: Some(9092),
            min_jre_version: Some("11".into()),
            version_error_signatures: vec![],
            versions: vec![],
        };
        let url = build_jdbc_url(&config, "localhost", 9092, Some("testdb"));
        assert_eq!(url, "jdbc:h2:tcp://localhost:9092/testdb");
    }

    #[test]
    fn test_build_jdbc_url_with_database() {
        let config = DatabaseDriverConfig {
            name: "Oracle Database".into(),
            class_name: "oracle.jdbc.OracleDriver".into(),
            maven_group: "com.oracle.database.jdbc".into(),
            maven_artifact: "ojdbc11".into(),
            jdbc_url_template: "jdbc:oracle:thin:@{host}:{port}:{database}".into(),
            default_port: Some(1521),
            min_jre_version: Some("11".into()),
            version_error_signatures: vec![],
            versions: vec![],
        };
        let url = build_jdbc_url(&config, "localhost", 1521, Some("XEPDB1"));
        assert_eq!(url, "jdbc:oracle:thin:@localhost:1521:XEPDB1");
    }

    /// When no database is provided, the `{database}` placeholder is removed.
    #[test]
    fn test_build_jdbc_url_without_database() {
        let config = DatabaseDriverConfig {
            name: "H2 Database".into(),
            class_name: "org.h2.Driver".into(),
            maven_group: "com.h2database".into(),
            maven_artifact: "h2".into(),
            jdbc_url_template: "jdbc:h2:tcp://{host}:{port}/{database}".into(),
            default_port: Some(9092),
            min_jre_version: Some("11".into()),
            version_error_signatures: vec![],
            versions: vec![],
        };
        let url = build_jdbc_url(&config, "localhost", 9092, None);
        assert_eq!(url, "jdbc:h2:tcp://localhost:9092/");
    }

    #[test]
    fn test_build_jdbc_url_no_database_placeholder() {
        // DM8 template has no {database} — should remain unchanged
        let config = DatabaseDriverConfig {
            name: "DM8".into(),
            class_name: "dm.jdbc.driver.DmDriver".into(),
            maven_group: "com.dameng".into(),
            maven_artifact: "DmJdbcDriver".into(),
            jdbc_url_template: "jdbc:dm://{host}:{port}".into(),
            default_port: Some(5236),
            min_jre_version: Some("11".into()),
            version_error_signatures: vec![],
            versions: vec![],
        };
        let url = build_jdbc_url(&config, "10.0.0.1", 5236, None);
        assert_eq!(url, "jdbc:dm://10.0.0.1:5236");
    }

    #[test]
    fn test_build_jdbc_url_dangler_cleanup() {
        // Each tuple: (template, expected result when database is None)
        let cases: Vec<(&str, &str)> = vec![
            // Databricks: ";httpPath=" prefix cleaned up
            (
                "jdbc:databricks://{host}:{port};httpPath={database}",
                "jdbc:databricks://localhost:443",
            ),
            // Teradata: "/DATABASE=" prefix cleaned up
            (
                "jdbc:teradata://{host}/DATABASE={database}",
                "jdbc:teradata://localhost",
            ),
            // Exasol: ";schema=" prefix cleaned up
            (
                "jdbc:exa:{host}:{port};schema={database}",
                "jdbc:exa:localhost:8563",
            ),
            // BigQuery: ";ProjectId=" prefix cleaned up
            (
                "jdbc:bigquery://{host}:{port};ProjectId={database}",
                "jdbc:bigquery://localhost:443",
            ),
            // Informix: ":INFORMIXSERVER=" prefix cleaned up
            (
                "jdbc:informix-sqli://{host}:{port}/{database}:INFORMIXSERVER=myinst",
                "jdbc:informix-sqli://localhost:1526/myinst",
            ),
            // Snowflake: trailing "?" and "&" trimmed after cleanup
            (
                "jdbc:snowflake://{host}.snowflakecomputing.com?warehouse={database}&db={database}",
                "jdbc:snowflake://localhost.snowflakecomputing.com?warehouse=&db=",
            ),
        ];

        for (idx, (template, expected)) in cases.iter().enumerate() {
            let port: u16 = match idx {
                0 => 443,  // Databricks
                1 => 0,    // Teradata (no port in template)
                2 => 8563, // Exasol
                3 => 443,  // BigQuery
                4 => 1526, // Informix
                5 => 0,    // Snowflake (no port in template)
                _ => 0,
            };
            let config = DatabaseDriverConfig {
                name: format!("test-db-{idx}"),
                class_name: "test.Driver".into(),
                maven_group: "test".into(),
                maven_artifact: "test".into(),
                jdbc_url_template: template.to_string(),
                default_port: None,
                min_jre_version: None,
                version_error_signatures: vec![],
                versions: vec![],
            };
            let url = build_jdbc_url(&config, "localhost", port, None);
            assert_eq!(
                url,
                expected.to_string(),
                "case {idx}: template='{template}'"
            );
        }
    }

    #[test]
    fn test_registry_key_oracle_returns_oracle() {
        assert_eq!(
            DriverRegistry::registry_key(DatabaseType::Oracle),
            Some("oracle")
        );
    }

    #[test]
    fn test_registry_key_unknown_returns_none() {
        assert_eq!(DriverRegistry::registry_key(DatabaseType::PostgreSQL), None);
        assert_eq!(DriverRegistry::registry_key(DatabaseType::MySQL), None);
        assert_eq!(DriverRegistry::registry_key(DatabaseType::SqlServer), None);
        assert_eq!(DriverRegistry::registry_key(DatabaseType::SQLite), None);
    }

    #[test]
    fn test_registry_key_new_jdbc_types() {
        assert_eq!(
            DriverRegistry::registry_key(DatabaseType::Hive),
            Some("hive")
        );
        assert_eq!(
            DriverRegistry::registry_key(DatabaseType::Databricks),
            Some("databricks")
        );
        assert_eq!(
            DriverRegistry::registry_key(DatabaseType::Hana),
            Some("hana")
        );
        assert_eq!(
            DriverRegistry::registry_key(DatabaseType::Teradata),
            Some("teradata")
        );
        assert_eq!(
            DriverRegistry::registry_key(DatabaseType::Vertica),
            Some("vertica")
        );
        assert_eq!(
            DriverRegistry::registry_key(DatabaseType::Exasol),
            Some("exasol")
        );
        assert_eq!(
            DriverRegistry::registry_key(DatabaseType::BigQuery),
            Some("bigquery")
        );
        assert_eq!(
            DriverRegistry::registry_key(DatabaseType::Informix),
            Some("informix")
        );
        assert_eq!(
            DriverRegistry::registry_key(DatabaseType::Kylin),
            Some("kylin")
        );
        assert_eq!(
            DriverRegistry::registry_key(DatabaseType::Cassandra),
            Some("cassandra")
        );
        assert_eq!(
            DriverRegistry::registry_key(DatabaseType::Iris),
            Some("iris")
        );
        assert_eq!(
            DriverRegistry::registry_key(DatabaseType::Access),
            Some("access")
        );
    }
}
