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
  POSTGRESQL: { icon: postgresqlLogo, color: 'bg-blue-100 dark:bg-blue-900/30' },
  MYSQL: { icon: mysqlLogo, color: 'bg-orange-100 dark:bg-orange-900/30' },
  MARIADB: { icon: mariadbLogo, color: 'bg-purple-100 dark:bg-purple-900/30' },
  SQLITE: { icon: sqliteLogo, color: 'bg-green-100 dark:bg-green-900/30' },
  SQLSERVER: { icon: sqlserverLogo, color: 'bg-red-100 dark:bg-red-900/30' },
  DUCKDB: { icon: duckdbLogo, color: 'bg-yellow-100 dark:bg-yellow-900/30' },
  CLICKHOUSE: { icon: clickhouseLogo, color: 'bg-red-100 dark:bg-red-900/30' },
  COCKROACHDB: { icon: cockroachdbLogo, color: 'bg-blue-100 dark:bg-blue-900/30' },
  REDSHIFT: { icon: redshiftLogo, color: 'bg-blue-100 dark:bg-blue-900/30' },
  YUGABYTEDB: { icon: yugabytedbLogo, color: 'bg-orange-100 dark:bg-orange-900/30' },
  TIMESCALEDB: { icon: timescaledbLogo, color: 'bg-blue-100 dark:bg-blue-900/30' },
  KINGBASEES: { icon: kingbaseesLogo, color: 'bg-purple-100 dark:bg-purple-900/30' },
  GAUSSDB: { icon: gaussdbLogo, color: 'bg-blue-100 dark:bg-blue-900/30' },
  HIGHGO: { icon: highgoLogo, color: 'bg-blue-100 dark:bg-blue-900/30' },
  UXDB: { icon: uxdbLogo, color: 'bg-blue-100 dark:bg-blue-900/30' },
  OPENGAUSS: { icon: opengaussLogo, color: 'bg-blue-100 dark:bg-blue-900/30' },
  GBASE8C: { icon: gbaseLogo, color: 'bg-blue-100 dark:bg-blue-900/30' },
  QUESTDB: { icon: questdbLogo, color: 'bg-green-100 dark:bg-green-900/30' },
  VASTBASE: { icon: vastbaseLogo, color: 'bg-blue-100 dark:bg-blue-900/30' },
  YASHANDB: { icon: yashandbLogo, color: 'bg-blue-100 dark:bg-blue-900/30' },
  TIDB: { icon: tidbLogo, color: 'bg-red-100 dark:bg-red-900/30' },
  OCEANBASE: { icon: oceanbaseLogo, color: 'bg-blue-100 dark:bg-blue-900/30' },
  TDSQL: { icon: tdsqlLogo, color: 'bg-blue-100 dark:bg-blue-900/30' },
  POLARDB: { icon: polardbLogo, color: 'bg-blue-100 dark:bg-blue-900/30' },
  DAMENG: { icon: damengLogo, color: 'bg-blue-100 dark:bg-blue-900/30' },
  DORIS: { icon: dorisLogo, color: 'bg-orange-100 dark:bg-orange-900/30' },
  SELECTDB: { icon: selectdbLogo, color: 'bg-cyan-100 dark:bg-cyan-900/30' },
  STARROCKS: { icon: starrocksLogo, color: 'bg-yellow-100 dark:bg-yellow-900/30' },
  DATABEND: { icon: databendLogo, color: 'bg-orange-100 dark:bg-orange-900/30' },
  GOLDENDB: { icon: goldendbLogo, color: 'bg-yellow-100 dark:bg-yellow-900/30' },
  MANTICORESEARCH: { icon: manticoreLogo, color: 'bg-amber-100 dark:bg-amber-900/30' },
  ORACLE: { icon: oracleLogo, color: 'bg-red-100 dark:bg-red-900/30' },
  DB2: { icon: db2Logo, color: 'bg-blue-100 dark:bg-blue-900/30' },
  H2: { icon: h2Logo, color: 'bg-green-100 dark:bg-green-900/30' },
  SNOWFLAKE: { icon: snowflakeLogo, color: 'bg-blue-100 dark:bg-blue-900/30' },
  XUGUDB: { icon: xugudbLogo, color: 'bg-blue-100 dark:bg-blue-900/30' },
  GBASE8A: { icon: gbaseLogo, color: 'bg-blue-100 dark:bg-blue-900/30' },
  TRINO: { icon: trinoLogo, color: 'bg-blue-100 dark:bg-blue-900/30' },
  PRESTO: { icon: prestoLogo, color: 'bg-blue-100 dark:bg-blue-900/30' },
  DERBY: { icon: derbyLogo, color: 'bg-blue-100 dark:bg-blue-900/30' },
  HIVE: { icon: hiveLogo, color: 'bg-orange-100 dark:bg-orange-900/30' },
  DATABRICKS: { icon: databricksLogo, color: 'bg-blue-100 dark:bg-blue-900/30' },
  HANA: { icon: hanaLogo, color: 'bg-teal-100 dark:bg-teal-900/30' },
  TERADATA: { icon: teradataLogo, color: 'bg-red-100 dark:bg-red-900/30' },
  VERTICA: { icon: verticaLogo, color: 'bg-green-100 dark:bg-green-900/30' },
  EXASOL: { icon: exasolLogo, color: 'bg-amber-100 dark:bg-amber-900/30' },
  BIGQUERY: { icon: bigqueryLogo, color: 'bg-blue-100 dark:bg-blue-900/30' },
  INFORMIX: { icon: informixLogo, color: 'bg-blue-100 dark:bg-blue-900/30' },
  KYLIN: { icon: kylinLogo, color: 'bg-blue-100 dark:bg-blue-900/30' },
  CASSANDRA: { icon: cassandraLogo, color: 'bg-gray-100 dark:bg-gray-800/30' },
  IRIS: { icon: irisLogo, color: 'bg-teal-100 dark:bg-teal-900/30' },
  ACCESS: { icon: accessLogo, color: 'bg-red-100 dark:bg-red-900/30' },
  FIREBIRD: { icon: firebirdLogo, color: 'bg-purple-100 dark:bg-purple-900/30' },
  RQLITE: { icon: rqliteLogo, color: 'bg-green-100 dark:bg-green-900/30' },
  TURSO: { icon: tursoLogo, color: 'bg-cyan-100 dark:bg-cyan-900/30' },
  TDENGINE: { icon: tdengineLogo, color: 'bg-amber-100 dark:bg-amber-900/30' },
  ALLOYDB: { icon: alloydbLogo, color: 'bg-blue-100 dark:bg-blue-900/30' },
  CRATEDB: { icon: cratedbLogo, color: 'bg-blue-100 dark:bg-blue-900/30' },
  CLOUDSQLPG: { icon: cloudsqlpgLogo, color: 'bg-blue-100 dark:bg-blue-900/30' },
  ENTERPRISEDB: { icon: enterprisedbLogo, color: 'bg-blue-100 dark:bg-blue-900/30' },
  GREENPLUM: { icon: greenplumLogo, color: 'bg-blue-100 dark:bg-blue-900/30' },
  MATERIALIZE: { icon: materializeLogo, color: 'bg-blue-100 dark:bg-blue-900/30' },
  FUJITSUPG: { icon: postgresqlLogo, color: 'bg-blue-100 dark:bg-blue-900/30' },
  SINGLESTOREMEMSQL: { icon: singlestoreLogo, color: 'bg-orange-100 dark:bg-orange-900/30' },
  CLOUDSQLMYSQL: { icon: mysqlLogo, color: 'bg-orange-100 dark:bg-orange-900/30' },
}

export function useDatabaseIcon() {
  const getDatabaseIcon = (type: DatabaseType): string => {
    return databaseIcons[type]?.icon ?? sqliteLogo
  }

  const getDatabaseColor = (type: DatabaseType): string => {
    return databaseIcons[type]?.color ?? 'bg-gray-100 dark:bg-gray-900/30'
  }

  const getDatabaseConfig = (type: DatabaseType): DatabaseIconConfig => {
    return databaseIcons[type] ?? { icon: sqliteLogo, color: 'bg-gray-100 dark:bg-gray-900/30' }
  }

  return {
    getDatabaseIcon,
    getDatabaseColor,
    getDatabaseConfig,
  }
}
