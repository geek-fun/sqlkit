//! Database connection strategy routing.
//!
//! This module provides protocol alias mapping and connection strategy
//! resolution for the multi-database architecture.

use crate::database::config::DatabaseType;

/// Core database types that have native adapter implementations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreDatabaseType {
    PostgreSQL,
    MySQL,
    SqlServer,
    SQLite,
    DuckDb,
    ClickHouse,
    Oracle,
    DB2,
    H2,
    Snowflake,
    DM8Oracle,
    Trino,
    Presto,
}

/// Connection strategy for a database type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionStrategy {
    /// Route to a native adapter via CoreDatabaseType.
    Native(CoreDatabaseType),
    /// Route to JDBC bridge adapter (Java subprocess).
    JdbcBridge,
    /// Route to HTTP SQL bridge adapter.
    Http,
}

/// Map a [`DatabaseType`] to its effective connection strategy.
///
/// Protocol-compatible databases (e.g., CockroachDB → PostgreSQL wire)
/// are mapped to their native adapter's [`CoreDatabaseType`] so that
/// they reuse existing adapter code.
pub fn resolve_effective_type(db: DatabaseType) -> ConnectionStrategy {
    use DatabaseType::*;
    match db {
        // Native PG adapter
        PostgreSQL => ConnectionStrategy::Native(CoreDatabaseType::PostgreSQL),
        // PG wire protocol compat
        CockroachDB | Redshift | YugabyteDB | TimescaleDB | KingbaseES | GaussDB | HighGo
        | UXDB | OpenGauss | GBase8c | QuestDB | Vastbase | YashanDB => ConnectionStrategy::Native(CoreDatabaseType::PostgreSQL),

        // Native MySQL adapter
        MySQL => ConnectionStrategy::Native(CoreDatabaseType::MySQL),
        // MySQL wire protocol compat
        MariaDB | TiDB | OceanBase | TDSQL | PolarDB | DM8 | Doris | SelectDB | StarRocks
        | Databend | GoldenDB | ManticoreSearch => {
            ConnectionStrategy::Native(CoreDatabaseType::MySQL)
        }

        // Other native adapters
        SqlServer => ConnectionStrategy::Native(CoreDatabaseType::SqlServer),
        SQLite => ConnectionStrategy::Native(CoreDatabaseType::SQLite),
        DuckDb => ConnectionStrategy::Native(CoreDatabaseType::DuckDb),
        ClickHouse => ConnectionStrategy::Native(CoreDatabaseType::ClickHouse),

        // JDBC bridge (Java subprocess)
        Oracle => {
            #[cfg(feature = "oracle")]
            { ConnectionStrategy::Native(CoreDatabaseType::Oracle) }
            #[cfg(not(feature = "oracle"))]
            { ConnectionStrategy::JdbcBridge }
        },
        DB2 => ConnectionStrategy::JdbcBridge,
        H2 => ConnectionStrategy::JdbcBridge,
        Snowflake => ConnectionStrategy::JdbcBridge,
        DM8Oracle => ConnectionStrategy::JdbcBridge,
        XuguDB => ConnectionStrategy::JdbcBridge,
        GBase8a => ConnectionStrategy::JdbcBridge,
        Derby => ConnectionStrategy::JdbcBridge,

        // HTTP SQL bridge
        Trino | Presto => ConnectionStrategy::Http,
    }
}

/// Check whether a given database type should be treated as a MySQL-family
/// database (uses MySQLAdapter).
pub fn is_mysql_family(db: DatabaseType) -> bool {
    matches!(
        resolve_effective_type(db),
        ConnectionStrategy::Native(CoreDatabaseType::MySQL)
    )
}

/// Check whether a given database type should be treated as a PG-family
/// database (uses PostgresAdapter).
pub fn is_pg_family(db: DatabaseType) -> bool {
    matches!(
        resolve_effective_type(db),
        ConnectionStrategy::Native(CoreDatabaseType::PostgreSQL)
    )
}

/// Get the default port for a database type, if known.
pub fn default_port(db: DatabaseType) -> Option<u16> {
    use DatabaseType::*;
    match db {
        PostgreSQL | CockroachDB | Redshift | YugabyteDB | TimescaleDB | KingbaseES | GaussDB
        | HighGo | UXDB | OpenGauss | GBase8c | Vastbase => Some(5432),
        QuestDB => Some(8812),
        YashanDB => Some(1688),
        MySQL | MariaDB | TiDB | OceanBase | TDSQL | PolarDB | DM8 | GoldenDB => Some(3306),
        Doris | SelectDB | StarRocks => Some(9030),
        Databend => Some(3307),
        ManticoreSearch => Some(9306),
        SqlServer => Some(1433),
        SQLite => None,
        DuckDb => None,
        ClickHouse => Some(8123),
        Oracle => Some(1521),
        DB2 => Some(50000),
        H2 => Some(9092),
        Snowflake => Some(443),
        DM8Oracle => Some(5236),
        XuguDB => Some(5138),
        GBase8a => Some(5258),
        Derby => Some(1527),
        Trino => Some(8080),
        Presto => Some(8080),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pg_family_routes_to_postgres() {
        for db in [
            DatabaseType::PostgreSQL,
            DatabaseType::CockroachDB,
            DatabaseType::Redshift,
            DatabaseType::KingbaseES,
            DatabaseType::GaussDB,
            DatabaseType::HighGo,
            DatabaseType::QuestDB,
            DatabaseType::Vastbase,
            DatabaseType::YashanDB,
        ] {
            assert!(
                is_pg_family(db),
                "{:?} should route to PostgreSQL adapter",
                db
            );
        }
    }

    #[test]
    fn test_mysql_family_routes_to_mysql() {
        for db in [
            DatabaseType::MySQL,
            DatabaseType::MariaDB,
            DatabaseType::TiDB,
            DatabaseType::OceanBase,
            DatabaseType::TDSQL,
            DatabaseType::PolarDB,
            DatabaseType::Doris,
            DatabaseType::SelectDB,
            DatabaseType::StarRocks,
            DatabaseType::Databend,
            DatabaseType::GoldenDB,
            DatabaseType::ManticoreSearch,
        ] {
            assert!(
                is_mysql_family(db),
                "{:?} should route to MySQL adapter",
                db
            );
        }
    }

    #[test]
    fn test_oracle_routes_to_native() {
        assert_eq!(
            resolve_effective_type(DatabaseType::Oracle),
            ConnectionStrategy::Native(CoreDatabaseType::Oracle),
            "Oracle should route to native Oracle adapter"
        );
    }

    #[test]
    fn test_jdbc_bridge_types() {
        for db in [
            DatabaseType::DB2,
            DatabaseType::H2,
            DatabaseType::Snowflake,
            DatabaseType::DM8Oracle,
            DatabaseType::XuguDB,
            DatabaseType::GBase8a,
        ] {
            assert_eq!(
                resolve_effective_type(db),
                ConnectionStrategy::JdbcBridge,
                "{:?} should be JDBC bridge",
                db
            );
        }
    }

    #[test]
    fn test_http_types() {
        for db in [DatabaseType::Trino, DatabaseType::Presto] {
            assert_eq!(
                resolve_effective_type(db),
                ConnectionStrategy::Http,
                "{:?} should be HTTP bridge",
                db
            );
        }
    }

    #[test]
    fn test_default_ports() {
        assert_eq!(default_port(DatabaseType::PostgreSQL), Some(5432));
        assert_eq!(default_port(DatabaseType::MySQL), Some(3306));
        assert_eq!(default_port(DatabaseType::SqlServer), Some(1433));
        assert_eq!(default_port(DatabaseType::DM8Oracle), Some(5236));
        assert_eq!(default_port(DatabaseType::Oracle), Some(1521));
        assert_eq!(default_port(DatabaseType::Doris), Some(9030));
        assert_eq!(default_port(DatabaseType::SelectDB), Some(9030));
        assert_eq!(default_port(DatabaseType::StarRocks), Some(9030));
        assert_eq!(default_port(DatabaseType::Databend), Some(3307));
        assert_eq!(default_port(DatabaseType::GoldenDB), Some(3306));
        assert_eq!(default_port(DatabaseType::ManticoreSearch), Some(9306));
        assert_eq!(default_port(DatabaseType::QuestDB), Some(8812));
        assert_eq!(default_port(DatabaseType::Vastbase), Some(5432));
        assert_eq!(default_port(DatabaseType::YashanDB), Some(1688));
    }
}
