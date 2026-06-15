use crate::database::strategy::{resolve_effective_type, ConnectionStrategy, CoreDatabaseType};
use crate::database::{
    clickhouse::ClickHouseAdapter, config::ConnectionConfig, duckdb::DuckDbAdapter,
    http_sql::HttpSqlAdapter, jdbc_bridge::JdbcBridgeAdapter, ConnectionStatus, DatabaseAdapter,
};
use crate::database::{
    mysql::MySQLAdapter, postgres::PostgresAdapter, sqlite::SQLiteAdapter,
    sqlserver::SqlServerAdapter,
};
use crate::state::ActiveConnection;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Create and connect a database adapter based on database type string.
pub async fn create_and_connect_adapter(
    db_type: &str,
    conn_config: ConnectionConfig,
) -> Result<ActiveConnection, String> {
    // Normalize the db_type string to a DatabaseType enum
    let db_type = db_type_to_enum(db_type)?;
    let strategy = resolve_effective_type(db_type);

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
                CoreDatabaseType::DuckDb => {
                    let mut adapter = DuckDbAdapter::new(conn_config);
                    adapter.connect().await.map_err(|e| e.to_string())?;
                    Ok(ActiveConnection::DuckDb(Arc::new(Mutex::new(adapter))))
                }
                CoreDatabaseType::ClickHouse => {
                    let mut adapter = ClickHouseAdapter::new(conn_config);
                    adapter.connect().await.map_err(|e| e.to_string())?;
                    Ok(ActiveConnection::ClickHouse(Arc::new(Mutex::new(adapter))))
                }
                CoreDatabaseType::Oracle => {
                    #[cfg(feature = "oracle")]
                    {
                        let mut adapter = crate::database::OracleAdapter::new(conn_config);
                        adapter.connect().await.map_err(|e| e.to_string())?;
                        return Ok(ActiveConnection::Oracle(Arc::new(Mutex::new(adapter))));
                    }
                    #[cfg(not(feature = "oracle"))]
                Err("Oracle support requires the 'oracle' feature: cargo build --features oracle".to_string())
                }
                _ => Err(format!("Native adapter not yet implemented for {:?}", core)),
            }
        }
        ConnectionStrategy::JdbcBridge => {
            let mut adapter = JdbcBridgeAdapter::new(conn_config);
            adapter.connect().await.map_err(|e| e.to_string())?;
            Ok(ActiveConnection::JdbcBridge(Arc::new(Mutex::new(adapter))))
        }
        ConnectionStrategy::Http => {
            let mut adapter = HttpSqlAdapter::new(conn_config);
            adapter.connect().await.map_err(|e| e.to_string())?;
            Ok(ActiveConnection::HttpSql(Arc::new(Mutex::new(adapter))))
        }
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
            CoreDatabaseType::DuckDb => {
                let mut adapter = DuckDbAdapter::new(conn_config);
                adapter.connect().await.map_err(|e| e.to_string())?;
                adapter.test_connection().await.map_err(|e| e.to_string())
            }
            CoreDatabaseType::ClickHouse => {
                let mut adapter = ClickHouseAdapter::new(conn_config);
                adapter.connect().await.map_err(|e| e.to_string())?;
                adapter.test_connection().await.map_err(|e| e.to_string())
            }
            CoreDatabaseType::Oracle => {
                #[cfg(feature = "oracle")]
                {
                    let mut adapter = crate::database::OracleAdapter::new(conn_config);
                    adapter.connect().await.map_err(|e| e.to_string())?;
                    return adapter.test_connection().await.map_err(|e| e.to_string());
                }
                #[cfg(not(feature = "oracle"))]
                Err("Oracle support requires the 'oracle' feature".to_string())
            }
            _ => Err("Native adapter not yet implemented".into()),
        },
        ConnectionStrategy::JdbcBridge => {
            let mut adapter = JdbcBridgeAdapter::new(conn_config);
            adapter.connect().await.map_err(|e| e.to_string())?;
            adapter.test_connection().await.map_err(|e| e.to_string())
        }
        ConnectionStrategy::Http => {
            let mut adapter = HttpSqlAdapter::new(conn_config);
            adapter.connect().await.map_err(|e| e.to_string())?;
            adapter.test_connection().await.map_err(|e| e.to_string())
        }
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
        "oracle" => Ok(DatabaseType::Oracle),
        "db2" => Ok(DatabaseType::DB2),
        "h2" => Ok(DatabaseType::H2),
        "snowflake" => Ok(DatabaseType::Snowflake),
        "trino" => Ok(DatabaseType::Trino),
        "presto" => Ok(DatabaseType::Presto),
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
        "derby" => Ok(DatabaseType::Derby),
        _ => Err(format!("Unsupported database type: {}", db_type)),
    }
}
