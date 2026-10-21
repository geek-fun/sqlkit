import type { DatabaseType } from '@/store'

import accessLogo from '@/assets/images/database-icons/access-logo.svg'
import alloydbLogo from '@/assets/images/database-icons/alloydb-logo.svg'
import bigqueryLogo from '@/assets/images/database-icons/bigquery-logo.svg'
import cassandraLogo from '@/assets/images/database-icons/cassandra-logo.svg'
import clickhouseLogo from '@/assets/images/database-icons/clickhouse-logo.svg'
import cloudsqlpgLogo from '@/assets/images/database-icons/cloudsqlpg-logo.svg'
import cockroachdbLogo from '@/assets/images/database-icons/cockroachdb-logo.svg'
import cratedbLogo from '@/assets/images/database-icons/cratedb-logo.svg'
import databendLogo from '@/assets/images/database-icons/databend-logo.svg'
import databricksLogo from '@/assets/images/database-icons/databricks-logo.svg'
import db2Logo from '@/assets/images/database-icons/db2-logo.svg'
import derbyLogo from '@/assets/images/database-icons/derby-logo.svg'
import damengLogo from '@/assets/images/database-icons/dm8-logo.svg'
import dorisLogo from '@/assets/images/database-icons/doris-logo.svg'
import duckdbLogo from '@/assets/images/database-icons/duckdb-logo.svg'
import enterprisedbLogo from '@/assets/images/database-icons/enterprisedb-logo.svg'
import exasolLogo from '@/assets/images/database-icons/exasol-logo.svg'
import firebirdLogo from '@/assets/images/database-icons/firebird-logo.svg'
import gaussdbLogo from '@/assets/images/database-icons/gaussdb-logo.svg'
import gbaseLogo from '@/assets/images/database-icons/gbase-logo.svg'
import goldendbLogo from '@/assets/images/database-icons/goldendb-logo.svg'
import greenplumLogo from '@/assets/images/database-icons/greenplum-logo.svg'
import h2Logo from '@/assets/images/database-icons/h2-logo.svg'
import hanaLogo from '@/assets/images/database-icons/hana-logo.svg'
import highgoLogo from '@/assets/images/database-icons/highgo-logo.svg'
import hiveLogo from '@/assets/images/database-icons/hive-logo.svg'
import informixLogo from '@/assets/images/database-icons/informix-logo.svg'
import irisLogo from '@/assets/images/database-icons/iris-logo.svg'
import kingbaseesLogo from '@/assets/images/database-icons/kingbasees-logo.svg'
import kylinLogo from '@/assets/images/database-icons/kylin-logo.svg'
import manticoreLogo from '@/assets/images/database-icons/manticore-logo.svg'
import mariadbLogo from '@/assets/images/database-icons/mariadb-logo.svg'
import materializeLogo from '@/assets/images/database-icons/materialize-logo.svg'
import mysqlLogo from '@/assets/images/database-icons/mysql-logo.svg'
import oceanbaseLogo from '@/assets/images/database-icons/oceanbase-logo.svg'
import opengaussLogo from '@/assets/images/database-icons/opengauss-logo.svg'
import oracleLogo from '@/assets/images/database-icons/oracle-logo.svg'
import polardbLogo from '@/assets/images/database-icons/polardb-logo.svg'
import postgresqlLogo from '@/assets/images/database-icons/postgresql-logo.svg'
import prestoLogo from '@/assets/images/database-icons/presto-logo.svg'
import questdbLogo from '@/assets/images/database-icons/questdb-logo.svg'
import redshiftLogo from '@/assets/images/database-icons/redshift-logo.svg'
import rqliteLogo from '@/assets/images/database-icons/rqlite-logo.svg'
import selectdbLogo from '@/assets/images/database-icons/selectdb-logo.svg'
import singlestoreLogo from '@/assets/images/database-icons/singlestorememsql-logo.svg'
import snowflakeLogo from '@/assets/images/database-icons/snowflake-logo.svg'
import sqlcipherLogo from '@/assets/images/database-icons/sqlcipher-logo.svg'
import sqliteLogo from '@/assets/images/database-icons/sqlite-logo.svg'
import sqlserverLogo from '@/assets/images/database-icons/sqlserver-logo.svg'
import starrocksLogo from '@/assets/images/database-icons/starrocks-logo.svg'
import tdengineLogo from '@/assets/images/database-icons/tdengine-logo.svg'
import tdsqlLogo from '@/assets/images/database-icons/tdsql-logo.svg'
import teradataLogo from '@/assets/images/database-icons/teradata-logo.svg'
import tidbLogo from '@/assets/images/database-icons/tidb-logo.svg'
import timescaledbLogo from '@/assets/images/database-icons/timescaledb-logo.svg'
import trinoLogo from '@/assets/images/database-icons/trino-logo.svg'
import tursoLogo from '@/assets/images/database-icons/turso-logo.svg'
import uxdbLogo from '@/assets/images/database-icons/uxdb-logo.svg'
import vastbaseLogo from '@/assets/images/database-icons/vastbase-logo.svg'
import verticaLogo from '@/assets/images/database-icons/vertica-logo.svg'
import xugudbLogo from '@/assets/images/database-icons/xugudb-logo.svg'
import yashandbLogo from '@/assets/images/database-icons/yashandb-logo.svg'
import yugabytedbLogo from '@/assets/images/database-icons/yugabytedb-logo.svg'

type DatabaseIconConfig = {
  icon: string
  color: string
}

const databaseIcons: Record<DatabaseType, DatabaseIconConfig> = {
  POSTGRESQL: { icon: postgresqlLogo, color: 'bg-muted' },
  MYSQL: { icon: mysqlLogo, color: 'bg-muted' },
  MARIADB: { icon: mariadbLogo, color: 'bg-muted' },
  SQLITE: { icon: sqliteLogo, color: 'bg-muted' },
  SQLCIPHER: { icon: sqlcipherLogo, color: 'bg-muted' },
  SQLSERVER: { icon: sqlserverLogo, color: 'bg-muted' },
  DUCKDB: { icon: duckdbLogo, color: 'bg-muted' },
  CLICKHOUSE: { icon: clickhouseLogo, color: 'bg-muted' },
  COCKROACHDB: { icon: cockroachdbLogo, color: 'bg-muted' },
  REDSHIFT: { icon: redshiftLogo, color: 'bg-muted' },
  YUGABYTEDB: { icon: yugabytedbLogo, color: 'bg-muted' },
  TIMESCALEDB: { icon: timescaledbLogo, color: 'bg-muted' },
  KINGBASEES: { icon: kingbaseesLogo, color: 'bg-muted' },
  GAUSSDB: { icon: gaussdbLogo, color: 'bg-muted' },
  HIGHGO: { icon: highgoLogo, color: 'bg-muted' },
  UXDB: { icon: uxdbLogo, color: 'bg-muted' },
  OPENGAUSS: { icon: opengaussLogo, color: 'bg-muted' },
  GBASE8C: { icon: gbaseLogo, color: 'bg-muted' },
  QUESTDB: { icon: questdbLogo, color: 'bg-muted' },
  VASTBASE: { icon: vastbaseLogo, color: 'bg-muted' },
  YASHANDB: { icon: yashandbLogo, color: 'bg-muted' },
  TIDB: { icon: tidbLogo, color: 'bg-muted' },
  OCEANBASE: { icon: oceanbaseLogo, color: 'bg-muted' },
  OCEANBASE_ORACLE: { icon: oceanbaseLogo, color: 'bg-muted' },
  TDSQL: { icon: tdsqlLogo, color: 'bg-muted' },
  POLARDB: { icon: polardbLogo, color: 'bg-muted' },
  DAMENG: { icon: damengLogo, color: 'bg-muted' },
  DORIS: { icon: dorisLogo, color: 'bg-muted' },
  SELECTDB: { icon: selectdbLogo, color: 'bg-muted' },
  STARROCKS: { icon: starrocksLogo, color: 'bg-muted' },
  DATABEND: { icon: databendLogo, color: 'bg-muted' },
  GOLDENDB: { icon: goldendbLogo, color: 'bg-muted' },
  MANTICORESEARCH: { icon: manticoreLogo, color: 'bg-muted' },
  ORACLE: { icon: oracleLogo, color: 'bg-muted' },
  DB2: { icon: db2Logo, color: 'bg-muted' },
  H2: { icon: h2Logo, color: 'bg-muted' },
  SNOWFLAKE: { icon: snowflakeLogo, color: 'bg-muted' },
  XUGUDB: { icon: xugudbLogo, color: 'bg-muted' },
  GBASE8A: { icon: gbaseLogo, color: 'bg-muted' },
  TRINO: { icon: trinoLogo, color: 'bg-muted' },
  PRESTO: { icon: prestoLogo, color: 'bg-muted' },
  DERBY: { icon: derbyLogo, color: 'bg-muted' },
  HIVE: { icon: hiveLogo, color: 'bg-muted' },
  DATABRICKS: { icon: databricksLogo, color: 'bg-muted' },
  HANA: { icon: hanaLogo, color: 'bg-muted' },
  TERADATA: { icon: teradataLogo, color: 'bg-muted' },
  VERTICA: { icon: verticaLogo, color: 'bg-muted' },
  EXASOL: { icon: exasolLogo, color: 'bg-muted' },
  BIGQUERY: { icon: bigqueryLogo, color: 'bg-muted' },
  INFORMIX: { icon: informixLogo, color: 'bg-muted' },
  KYLIN: { icon: kylinLogo, color: 'bg-muted' },
  CASSANDRA: { icon: cassandraLogo, color: 'bg-muted' },
  IRIS: { icon: irisLogo, color: 'bg-muted' },
  ACCESS: { icon: accessLogo, color: 'bg-muted' },
  FIREBIRD: { icon: firebirdLogo, color: 'bg-muted' },
  RQLITE: { icon: rqliteLogo, color: 'bg-muted' },
  TURSO: { icon: tursoLogo, color: 'bg-muted' },
  TDENGINE: { icon: tdengineLogo, color: 'bg-muted' },
  ALLOYDB: { icon: alloydbLogo, color: 'bg-muted' },
  CRATEDB: { icon: cratedbLogo, color: 'bg-muted' },
  CLOUDSQLPG: { icon: cloudsqlpgLogo, color: 'bg-muted' },
  ENTERPRISEDB: { icon: enterprisedbLogo, color: 'bg-muted' },
  GREENPLUM: { icon: greenplumLogo, color: 'bg-muted' },
  MATERIALIZE: { icon: materializeLogo, color: 'bg-muted' },
  FUJITSUPG: { icon: postgresqlLogo, color: 'bg-muted' },
  SINGLESTOREMEMSQL: { icon: singlestoreLogo, color: 'bg-muted' },
  CLOUDSQLMYSQL: { icon: mysqlLogo, color: 'bg-muted' },
}

export function useDatabaseIcon() {
  const getDatabaseIcon = (type: DatabaseType): string => {
    return databaseIcons[type]?.icon ?? sqliteLogo
  }

  const getDatabaseColor = (type: DatabaseType): string => {
    return databaseIcons[type]?.color ?? 'bg-muted'
  }

  const getDatabaseConfig = (type: DatabaseType): DatabaseIconConfig => {
    return databaseIcons[type] ?? { icon: sqliteLogo, color: 'bg-muted' }
  }

  return {
    getDatabaseIcon,
    getDatabaseColor,
    getDatabaseConfig,
  }
}
