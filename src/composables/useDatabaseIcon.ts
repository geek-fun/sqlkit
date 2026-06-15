import type { DatabaseType } from '@/store'

import clickhouseLogo from '@/assets/images/database-icons/clickhouse-logo.svg'
import cockroachdbLogo from '@/assets/images/database-icons/cockroachdb-logo.svg'
import db2Logo from '@/assets/images/database-icons/db2-logo.svg'
import dm8Logo from '@/assets/images/database-icons/dm8-logo.svg'
import dm8oracleLogo from '@/assets/images/database-icons/dm8oracle-logo.svg'
import duckdbLogo from '@/assets/images/database-icons/duckdb-logo.svg'
import gaussdbLogo from '@/assets/images/database-icons/gaussdb-logo.svg'
import gbaseLogo from '@/assets/images/database-icons/gbase-logo.svg'
import h2Logo from '@/assets/images/database-icons/h2-logo.svg'
import highgoLogo from '@/assets/images/database-icons/highgo-logo.svg'
import kingbaseesLogo from '@/assets/images/database-icons/kingbasees-logo.svg'
import mariadbLogo from '@/assets/images/database-icons/mariadb-logo.svg'
import mysqlLogo from '@/assets/images/database-icons/mysql-logo.svg'
import oceanbaseLogo from '@/assets/images/database-icons/oceanbase-logo.svg'
import opengaussLogo from '@/assets/images/database-icons/opengauss-logo.svg'
import oracleLogo from '@/assets/images/database-icons/oracle-logo.svg'
import polardbLogo from '@/assets/images/database-icons/polardb-logo.svg'
import postgresqlLogo from '@/assets/images/database-icons/postgresql-logo.svg'
import prestoLogo from '@/assets/images/database-icons/presto-logo.svg'
import redshiftLogo from '@/assets/images/database-icons/redshift-logo.svg'
import snowflakeLogo from '@/assets/images/database-icons/snowflake-logo.svg'
import sqliteLogo from '@/assets/images/database-icons/sqlite-logo.svg'
import sqlserverLogo from '@/assets/images/database-icons/sqlserver-logo.svg'
import tdsqlLogo from '@/assets/images/database-icons/tdsql-logo.svg'
import tidbLogo from '@/assets/images/database-icons/tidb-logo.svg'
import timescaledbLogo from '@/assets/images/database-icons/timescaledb-logo.svg'
import trinoLogo from '@/assets/images/database-icons/trino-logo.svg'
import uxdbLogo from '@/assets/images/database-icons/uxdb-logo.svg'
import xugudbLogo from '@/assets/images/database-icons/xugudb-logo.svg'
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
  TIDB: { icon: tidbLogo, color: 'bg-red-100 dark:bg-red-900/30' },
  OCEANBASE: { icon: oceanbaseLogo, color: 'bg-blue-100 dark:bg-blue-900/30' },
  TDSQL: { icon: tdsqlLogo, color: 'bg-blue-100 dark:bg-blue-900/30' },
  POLARDB: { icon: polardbLogo, color: 'bg-blue-100 dark:bg-blue-900/30' },
  DM8: { icon: dm8Logo, color: 'bg-blue-100 dark:bg-blue-900/30' },
  ORACLE: { icon: oracleLogo, color: 'bg-red-100 dark:bg-red-900/30' },
  DB2: { icon: db2Logo, color: 'bg-blue-100 dark:bg-blue-900/30' },
  H2: { icon: h2Logo, color: 'bg-green-100 dark:bg-green-900/30' },
  SNOWFLAKE: { icon: snowflakeLogo, color: 'bg-blue-100 dark:bg-blue-900/30' },
  DM8ORACLE: { icon: dm8oracleLogo, color: 'bg-blue-100 dark:bg-blue-900/30' },
  XUGUDB: { icon: xugudbLogo, color: 'bg-blue-100 dark:bg-blue-900/30' },
  GBASE8A: { icon: gbaseLogo, color: 'bg-blue-100 dark:bg-blue-900/30' },
  TRINO: { icon: trinoLogo, color: 'bg-blue-100 dark:bg-blue-900/30' },
  PRESTO: { icon: prestoLogo, color: 'bg-blue-100 dark:bg-blue-900/30' },
  DERBY: { icon: sqliteLogo, color: 'bg-blue-100 dark:bg-blue-900/30' },
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
