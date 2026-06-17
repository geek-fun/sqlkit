use crate::database::config::DatabaseType;
use crate::database::rqlite::RqliteAdapter;
use crate::database::strategy::{resolve_effective_type, ConnectionStrategy, CoreDatabaseType};
use crate::database::turso::TursoAdapter;
use crate::database::jdbc_bridge;
use crate::database::{
    clickhouse::ClickHouseAdapter, config::ConnectionConfig, http_sql::HttpSqlAdapter,
    jdbc_bridge::JdbcBridgeAdapter, ConnectionStatus, DatabaseAdapter,
};
use crate::database::{
    mysql::MySQLAdapter, postgres::PostgresAdapter, sqlite::SQLiteAdapter,
    sqlserver::SqlServerAdapter,
};
use crate::ssh::TunnelManager;
use crate::ssh::start_transport_layers;
use crate::state::ActiveConnection;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Create and connect a database adapter based on database type string.
pub async fn create_and_connect_adapter(
    db_type: &str,
    conn_config: ConnectionConfig,
) -> Result<ActiveConnection, String> {
    // Normalize the db_type string to a DatabaseType enum
    let db_type_enum = db_type_to_enum(db_type)?;
    let strategy = resolve_effective_type(db_type_enum);

    match strategy {
        ConnectionStrategy::Native(core) => {
            match core {
                CoreDatabaseType::PostgreSQL => {
                    let mut adapter = PostgresAdapter::new(conn_config);
                    adapter.connect().await.map_err(|e| e.to_string())?;
                    Ok(ActiveConnection::Postgres(Arc::new(Mutex::new(adapter))))
                }
                CoreDatabaseType::MySQL => {
                    let mut adapter = MySQLAdapter::new(conn_config);
                    adapter.connect().await.map_err(|e| e.to_string())?;
                    Ok(ActiveConnection::MySQL(Arc::new(Mutex::new(adapter))))
                }
                CoreDatabaseType::SqlServer => {
                    let mut adapter = SqlServerAdapter::new(conn_config);
                    adapter.connect().await.map_err(|e| e.to_string())?;
                    Ok(ActiveConnection::SQLServer(Arc::new(Mutex::new(adapter))))
                }
                CoreDatabaseType::SQLite => {
                    let mut adapter = SQLiteAdapter::new(conn_config);
                    adapter.connect().await.map_err(|e| e.to_string())?;
                    Ok(ActiveConnection::SQLite(Arc::new(Mutex::new(adapter))))
                }
                CoreDatabaseType::ClickHouse => {
                    let mut adapter = ClickHouseAdapter::new(conn_config);
                    adapter.connect().await.map_err(|e| e.to_string())?;
                    Ok(ActiveConnection::ClickHouse(Arc::new(Mutex::new(adapter))))
                }
                _ => Err(format!("Native adapter not yet implemented for {:?}", core)),
            }
        }
        ConnectionStrategy::JdbcBridge => {
            if !is_jdbc_needed() {
                return Err(
                    "JDBC connections are disabled. Enable them in Settings → JRE & Drivers."
                        .to_string(),
                );
            }
            let mut adapter = JdbcBridgeAdapter::new(conn_config);
            adapter.connect().await.map_err(|e| e.to_string())?;
            Ok(ActiveConnection::JdbcBridge(Arc::new(Mutex::new(adapter))))
        }
        ConnectionStrategy::Http => match db_type_enum {
            DatabaseType::RQLite => {
                let mut adapter = RqliteAdapter::new(conn_config);
                adapter.connect().await.map_err(|e| e.to_string())?;
                Ok(ActiveConnection::Rqlite(Arc::new(Mutex::new(adapter))))
            }
            DatabaseType::Turso => {
                let mut adapter = TursoAdapter::new(conn_config);
                adapter.connect().await.map_err(|e| e.to_string())?;
                Ok(ActiveConnection::Turso(Arc::new(Mutex::new(adapter))))
            }
            _ => {
                let mut adapter = HttpSqlAdapter::new(conn_config);
                adapter.connect().await.map_err(|e| e.to_string())?;
                Ok(ActiveConnection::HttpSql(Arc::new(Mutex::new(adapter))))
            }
        },
    }
}

/// Test connection for a given database configuration.
pub async fn test_connection(
    db_type: &str,
    conn_config: ConnectionConfig,
) -> Result<ConnectionStatus, String> {
    let dt = db_type_to_enum(db_type)?;
    let strategy = resolve_effective_type(dt);

    match strategy {
        ConnectionStrategy::Native(core) => match core {
            CoreDatabaseType::PostgreSQL => {
                let mut adapter = PostgresAdapter::new(conn_config);
                adapter.connect().await.map_err(|e| e.to_string())?;
                adapter.test_connection().await.map_err(|e| e.to_string())
            }
            CoreDatabaseType::MySQL => {
                let mut adapter = MySQLAdapter::new(conn_config);
                adapter.connect().await.map_err(|e| e.to_string())?;
                adapter.test_connection().await.map_err(|e| e.to_string())
            }
            CoreDatabaseType::SqlServer => {
                let mut adapter = SqlServerAdapter::new(conn_config);
                adapter.connect().await.map_err(|e| e.to_string())?;
                adapter.test_connection().await.map_err(|e| e.to_string())
            }
            CoreDatabaseType::SQLite => {
                let mut adapter = SQLiteAdapter::new(conn_config);
                adapter.connect().await.map_err(|e| e.to_string())?;
                adapter.test_connection().await.map_err(|e| e.to_string())
            }
            CoreDatabaseType::ClickHouse => {
                let mut adapter = ClickHouseAdapter::new(conn_config);
                adapter.connect().await.map_err(|e| e.to_string())?;
                adapter.test_connection().await.map_err(|e| e.to_string())
            }
            _ => Err("Native adapter not yet implemented".into()),
        },
        ConnectionStrategy::JdbcBridge => {
            if !is_jdbc_needed() {
                return Err(
                    "JDBC connections are disabled. Enable them in Settings → JRE & Drivers."
                        .to_string(),
                );
            }
            let mut adapter = JdbcBridgeAdapter::new(conn_config);
            adapter.connect().await.map_err(|e| e.to_string())?;
            adapter.test_connection().await.map_err(|e| e.to_string())
        }
        ConnectionStrategy::Http => match dt {
            DatabaseType::RQLite => {
                let mut adapter = RqliteAdapter::new(conn_config);
                adapter.connect().await.map_err(|e| e.to_string())?;
                adapter.test_connection().await.map_err(|e| e.to_string())
            }
            DatabaseType::Turso => {
                let mut adapter = TursoAdapter::new(conn_config);
                adapter.connect().await.map_err(|e| e.to_string())?;
                adapter.test_connection().await.map_err(|e| e.to_string())
            }
            _ => {
                let mut adapter = HttpSqlAdapter::new(conn_config);
                adapter.connect().await.map_err(|e| e.to_string())?;
                adapter.test_connection().await.map_err(|e| e.to_string())
            }
        },
    }
}

/// Map a db_type string to a DatabaseType enum.
fn db_type_to_enum(db_type: &str) -> Result<crate::database::DatabaseType, String> {
    use crate::database::DatabaseType;
    match db_type.to_lowercase().as_str() {
        "postgresql" | "postgres" => Ok(DatabaseType::PostgreSQL),
        "mysql" => Ok(DatabaseType::MySQL),
        "sqlserver" | "mssql" => Ok(DatabaseType::SqlServer),
        "sqlite" => Ok(DatabaseType::SQLite),
        "duckdb" | "duck_db" | "duck" => Ok(DatabaseType::DuckDb),
        "clickhouse" => Ok(DatabaseType::ClickHouse),
        "firebird" => Ok(DatabaseType::Firebird),
        "oracle" => Ok(DatabaseType::Oracle),
        "db2" => Ok(DatabaseType::DB2),
        "h2" => Ok(DatabaseType::H2),
        "snowflake" => Ok(DatabaseType::Snowflake),
        "tdengine" | "td" => Ok(DatabaseType::TDengine),
        "trino" => Ok(DatabaseType::Trino),
        "presto" => Ok(DatabaseType::Presto),
        "rqlite" => Ok(DatabaseType::RQLite),
        "turso" | "libsql" => Ok(DatabaseType::Turso),
        "cockroachdb" | "cockroach" => Ok(DatabaseType::CockroachDB),
        "redshift" => Ok(DatabaseType::Redshift),
        "mariadb" => Ok(DatabaseType::MariaDB),
        "tidb" => Ok(DatabaseType::TiDB),
        "oceanbase" => Ok(DatabaseType::OceanBase),
        "tdsql" => Ok(DatabaseType::TDSQL),
        "polardb" => Ok(DatabaseType::PolarDB),
        "dm8" | "dm" => Ok(DatabaseType::DM8),
        "dm8_oracle" | "dm8oracle" => Ok(DatabaseType::DM8Oracle),
        "kingbasees" | "kingbase" => Ok(DatabaseType::KingbaseES),
        "gaussdb" | "gauss" => Ok(DatabaseType::GaussDB),
        "highgo" => Ok(DatabaseType::HighGo),
        "uxdb" => Ok(DatabaseType::UXDB),
        "opengauss" => Ok(DatabaseType::OpenGauss),
        "gbase8c" | "gbase_8c" => Ok(DatabaseType::GBase8c),
        "xugudb" | "xugu" => Ok(DatabaseType::XuguDB),
        "gbase8a" | "gbase_8a" => Ok(DatabaseType::GBase8a),
        "doris" => Ok(DatabaseType::Doris),
        "selectdb" => Ok(DatabaseType::SelectDB),
        "starrocks" => Ok(DatabaseType::StarRocks),
        "databend" => Ok(DatabaseType::Databend),
        "goldendb" => Ok(DatabaseType::GoldenDB),
        "manticore" | "manticore_search" => Ok(DatabaseType::ManticoreSearch),
        "questdb" => Ok(DatabaseType::QuestDB),
        "vastbase" => Ok(DatabaseType::Vastbase),
        "yashandb" => Ok(DatabaseType::YashanDB),
        "greenplum" | "cloudberry" | "greengage" => Ok(DatabaseType::Greenplum),
        "edb" | "enterprisedb" => Ok(DatabaseType::EnterpriseDB),
        "cratedb" | "crate" => Ok(DatabaseType::CrateDB),
        "materialize" => Ok(DatabaseType::Materialize),
        "alloydb" | "google_alloydb" => Ok(DatabaseType::AlloyDB),
        "cloudsqlpg" | "cloud_sql_pg" => Ok(DatabaseType::CloudSQLPG),
        "fujitsupg" | "fujitsu_pg" => Ok(DatabaseType::FujitsuPG),
        "singlestore" | "memsql" | "single_store" => Ok(DatabaseType::SingleStoreMemSQL),
        "cloudsqlmysql" | "cloud_sql_mysql" => Ok(DatabaseType::CloudSQLMySQL),
        "derby" => Ok(DatabaseType::Derby),
        "hive" => Ok(DatabaseType::Hive),
        "databricks" => Ok(DatabaseType::Databricks),
        "hana" | "sap_hana" => Ok(DatabaseType::Hana),
        "teradata" => Ok(DatabaseType::Teradata),
        "vertica" => Ok(DatabaseType::Vertica),
        "exasol" => Ok(DatabaseType::Exasol),
        "bigquery" | "google_bigquery" => Ok(DatabaseType::BigQuery),
        "informix" => Ok(DatabaseType::Informix),
        "kylin" => Ok(DatabaseType::Kylin),
        "cassandra" => Ok(DatabaseType::Cassandra),
        "iris" | "intersystems_iris" => Ok(DatabaseType::Iris),
        "access" | "ms_access" | "microsoft_access" => Ok(DatabaseType::Access),
        _ => Err(format!("Unsupported database type: {}", db_type)),
    }
}

/// Returns true for database types that operate on local files rather than network connections.
/// SSH tunneling is not applicable for these types.
fn is_file_based_db(db_type: &crate::database::DatabaseType) -> bool {
    matches!(
        db_type,
        crate::database::DatabaseType::SQLite | crate::database::DatabaseType::DuckDb
    )
}

/// Resolve the effective host and port for a connection, accounting for SSH tunnels.
/// If transport layers are configured, starts the tunnel and returns `127.0.0.1:local_port`.
/// Otherwise returns the original `(host, port)` for direct connection.
pub async fn connection_host_port(
    connection_id: &str,
    config: &ConnectionConfig,
    tunnels: &TunnelManager,
) -> Result<(String, u16), String> {
    if config.transport_layers.is_empty() {
        return Ok((config.host.clone(), config.port));
    }
    if is_file_based_db(&config.db_type) {
        return Ok((config.host.clone(), config.port));
    }

    let layers: Vec<crate::ssh::config::TransportLayerConfig> = config
        .transport_layers
        .iter()
        .filter(|layer| layer.enabled())
        .cloned()
        .collect();

    if layers.is_empty() {
        return Ok((config.host.clone(), config.port));
    }

    match start_transport_layers(connection_id, &layers, &config.host, config.port, tunnels).await? {
        Some(local_port) => Ok(("127.0.0.1".to_string(), local_port)),
        None => Ok((config.host.clone(), config.port)),
    }
}

/// Check whether JDBC connections are allowed.
/// Returns false when the `~/.sqlkit/.jdbc_not_needed` marker file exists.
fn is_jdbc_needed() -> bool {
    let marker = jdbc_bridge::jre::home_dir().join(".sqlkit").join(".jdbc_not_needed");
    !marker.exists()
}
