//! Database connection strategy routing.
//!
//! This module provides protocol alias mapping and connection strategy
//! resolution for the multi-database architecture.

use crate::database::config::DatabaseType;

/// Core database types that have native adapter implementations.
/// Non-core databases route through JDBC bridge or HTTP bridge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreDatabaseType {
    PostgreSQL,
    MySQL,
    SqlServer,
    SQLite,
    ClickHouse,
    DB2,
    H2,
    Snowflake,
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
        //         Native PG adapter
        PostgreSQL => ConnectionStrategy::Native(CoreDatabaseType::PostgreSQL),
        // PG wire protocol compat
        CockroachDB | Redshift | YugabyteDB | TimescaleDB | GaussDB | HighGo | UXDB | OpenGauss
        | GBase8c | QuestDB | Vastbase | Greenplum | EnterpriseDB | CrateDB | Materialize
        | AlloyDB | CloudSQLPG | FujitsuPG => {
            ConnectionStrategy::Native(CoreDatabaseType::PostgreSQL)
        }

        // Native MySQL adapter
        MySQL => ConnectionStrategy::Native(CoreDatabaseType::MySQL),
        // MySQL wire protocol compat
        MariaDB | TiDB | OceanBase | TDSQL | PolarDB | Doris | SelectDB | StarRocks | Databend
        | GoldenDB | ManticoreSearch | SingleStoreMemSQL | CloudSQLMySQL => {
            ConnectionStrategy::Native(CoreDatabaseType::MySQL)
        }

        // Other native adapters
        SqlServer => ConnectionStrategy::Native(CoreDatabaseType::SqlServer),
        SQLite => ConnectionStrategy::Native(CoreDatabaseType::SQLite),
        ClickHouse => ConnectionStrategy::Native(CoreDatabaseType::ClickHouse),

        // JDBC bridge (Java subprocess)
        // DuckDB, Oracle, Firebird moved from native to JDBC bridge
        // for binary size reduction (DuckDB) and simplified maintenance (Oracle, Firebird)
        DuckDb | Oracle | Firebird => ConnectionStrategy::JdbcBridge,
        DB2 => ConnectionStrategy::JdbcBridge,
        H2 => ConnectionStrategy::JdbcBridge,
        Snowflake => ConnectionStrategy::JdbcBridge,
        TDengine => ConnectionStrategy::JdbcBridge,
        Dameng => ConnectionStrategy::JdbcBridge,
        XuguDB => ConnectionStrategy::JdbcBridge,
        GBase8a => ConnectionStrategy::JdbcBridge,
        Derby => ConnectionStrategy::JdbcBridge,
        Hive => ConnectionStrategy::JdbcBridge,
        Databricks => ConnectionStrategy::JdbcBridge,
        Hana => ConnectionStrategy::JdbcBridge,
        Teradata => ConnectionStrategy::JdbcBridge,
        Vertica => ConnectionStrategy::JdbcBridge,
        Exasol => ConnectionStrategy::JdbcBridge,
        BigQuery => ConnectionStrategy::JdbcBridge,
        Informix => ConnectionStrategy::JdbcBridge,
        Kylin => ConnectionStrategy::JdbcBridge,
        Cassandra => ConnectionStrategy::JdbcBridge,
        Iris => ConnectionStrategy::JdbcBridge,
        Access => ConnectionStrategy::JdbcBridge,
        YashanDB => ConnectionStrategy::JdbcBridge,
        KingbaseES => ConnectionStrategy::JdbcBridge,
        OceanbaseOracle => ConnectionStrategy::JdbcBridge,

        // HTTP SQL bridge
        Trino | Presto => ConnectionStrategy::Http,
        RQLite | Turso => ConnectionStrategy::Http,
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
        PostgreSQL | CockroachDB | Redshift | YugabyteDB | TimescaleDB | GaussDB | HighGo
        | UXDB | OpenGauss | GBase8c | Vastbase | Greenplum | EnterpriseDB | CrateDB
        | Materialize | AlloyDB | CloudSQLPG | FujitsuPG => Some(5432),
        QuestDB => Some(8812),
        YashanDB => Some(1688),
        KingbaseES => Some(54321),
        OceanbaseOracle => Some(2881),
        MySQL | MariaDB | TiDB | OceanBase | TDSQL | PolarDB | GoldenDB | SingleStoreMemSQL
        | CloudSQLMySQL => Some(3306),
        Doris | SelectDB | StarRocks => Some(9030),
        Databend => Some(3307),
        ManticoreSearch => Some(9306),
        SqlServer => Some(1433),
        SQLite => None,
        DuckDb => None,
        ClickHouse => Some(8123),
        Firebird => Some(3050),
        Oracle => Some(1521),
        DB2 => Some(50000),
        H2 => Some(9092),
        Snowflake => Some(443),
        TDengine => Some(6030),
        Dameng => Some(5236),
        XuguDB => Some(5138),
        GBase8a => Some(5258),
        Derby => Some(1527),
        Hive => Some(10000),
        Databricks => Some(443),
        Hana => Some(30015),
        Teradata => Some(1025),
        Vertica => Some(5433),
        Exasol => Some(8563),
        BigQuery => Some(443),
        Informix => Some(9088),
        Kylin => Some(7070),
        Cassandra => Some(9042),
        Iris => Some(1972),
        Access => Some(0),
        Trino => Some(8080),
        Presto => Some(8080),
        RQLite => Some(4001),
        Turso => Some(443),
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
            DatabaseType::GaussDB,
            DatabaseType::HighGo,
            DatabaseType::QuestDB,
            DatabaseType::Vastbase,
            DatabaseType::Greenplum,
            DatabaseType::EnterpriseDB,
            DatabaseType::CrateDB,
            DatabaseType::Materialize,
            DatabaseType::AlloyDB,
            DatabaseType::CloudSQLPG,
            DatabaseType::FujitsuPG,
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
            DatabaseType::SingleStoreMemSQL,
            DatabaseType::CloudSQLMySQL,
        ] {
            assert!(
                is_mysql_family(db),
                "{:?} should route to MySQL adapter",
                db
            );
        }
    }

    #[test]
    fn test_duckdb_routes_to_jdbc() {
        assert_eq!(
            resolve_effective_type(DatabaseType::DuckDb),
            ConnectionStrategy::JdbcBridge,
            "DuckDB should route to JDBC bridge"
        );
    }

    #[test]
    fn test_firebird_routes_to_jdbc() {
        assert_eq!(
            resolve_effective_type(DatabaseType::Firebird),
            ConnectionStrategy::JdbcBridge,
            "Firebird should route to JDBC bridge"
        );
    }

    #[test]
    fn test_oracle_routes_to_jdbc() {
        assert_eq!(
            resolve_effective_type(DatabaseType::Oracle),
            ConnectionStrategy::JdbcBridge,
            "Oracle should route to JDBC bridge"
        );
    }

    #[test]
    fn test_jdbc_bridge_types() {
        for db in [
            DatabaseType::DuckDb,
            DatabaseType::Firebird,
            DatabaseType::Oracle,
            DatabaseType::DB2,
            DatabaseType::H2,
            DatabaseType::Snowflake,
            DatabaseType::TDengine,
            DatabaseType::Dameng,
            DatabaseType::XuguDB,
            DatabaseType::GBase8a,
            DatabaseType::Derby,
            DatabaseType::Hive,
            DatabaseType::Databricks,
            DatabaseType::Hana,
            DatabaseType::Teradata,
            DatabaseType::Vertica,
            DatabaseType::Exasol,
            DatabaseType::BigQuery,
            DatabaseType::Informix,
            DatabaseType::Kylin,
            DatabaseType::Cassandra,
            DatabaseType::Iris,
            DatabaseType::Access,
            DatabaseType::YashanDB,
            DatabaseType::KingbaseES,
            DatabaseType::OceanbaseOracle,
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
        for db in [
            DatabaseType::Trino,
            DatabaseType::Presto,
            DatabaseType::RQLite,
            DatabaseType::Turso,
        ] {
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
        assert_eq!(default_port(DatabaseType::Dameng), Some(5236));
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
        assert_eq!(default_port(DatabaseType::KingbaseES), Some(54321));
        assert_eq!(default_port(DatabaseType::OceanbaseOracle), Some(2881));
        assert_eq!(default_port(DatabaseType::Hive), Some(10000));
        assert_eq!(default_port(DatabaseType::Databricks), Some(443));
        assert_eq!(default_port(DatabaseType::Hana), Some(30015));
        assert_eq!(default_port(DatabaseType::Teradata), Some(1025));
        assert_eq!(default_port(DatabaseType::Vertica), Some(5433));
        assert_eq!(default_port(DatabaseType::Exasol), Some(8563));
        assert_eq!(default_port(DatabaseType::BigQuery), Some(443));
        assert_eq!(default_port(DatabaseType::Informix), Some(9088));
        assert_eq!(default_port(DatabaseType::Kylin), Some(7070));
        assert_eq!(default_port(DatabaseType::Cassandra), Some(9042));
        assert_eq!(default_port(DatabaseType::Iris), Some(1972));
        assert_eq!(default_port(DatabaseType::Access), Some(0));
        assert_eq!(default_port(DatabaseType::DuckDb), None);
        assert_eq!(default_port(DatabaseType::Greenplum), Some(5432));
        assert_eq!(default_port(DatabaseType::EnterpriseDB), Some(5432));
        assert_eq!(default_port(DatabaseType::CrateDB), Some(5432));
        assert_eq!(default_port(DatabaseType::SingleStoreMemSQL), Some(3306));
        assert_eq!(default_port(DatabaseType::CloudSQLMySQL), Some(3306));
        assert_eq!(default_port(DatabaseType::RQLite), Some(4001));
        assert_eq!(default_port(DatabaseType::Turso), Some(443));
        assert_eq!(default_port(DatabaseType::TDengine), Some(6030));
    }
}
