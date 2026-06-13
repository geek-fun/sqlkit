import type { DatabaseType } from '@/store'

import mariadbLogo from '@/assets/images/mariadb-logo.png'
import mssqlLogo from '@/assets/images/mssql-logo.png'
import mysqlLogo from '@/assets/images/mysql-logo.png'
import postgresqlLogo from '@/assets/images/postgresql-logo.png'
import sqliteLogo from '@/assets/images/sqlite-logo.png'

type DatabaseIconConfig = {
  icon: string
  color: string
}

const PG_ICON = { icon: postgresqlLogo, color: 'bg-blue-100 dark:bg-blue-900/30' }
const MYSQL_ICON = { icon: mysqlLogo, color: 'bg-orange-100 dark:bg-orange-900/30' }
const DEFAULT_ICON = { icon: sqliteLogo, color: 'bg-gray-100 dark:bg-gray-900/30' }

const databaseIcons: Record<DatabaseType, DatabaseIconConfig> = {
  POSTGRESQL: PG_ICON,
  MYSQL: MYSQL_ICON,
  MARIADB: { icon: mariadbLogo, color: 'bg-purple-100 dark:bg-purple-900/30' },
  SQLITE: { icon: sqliteLogo, color: 'bg-green-100 dark:bg-green-900/30' },
  SQLSERVER: { icon: mssqlLogo, color: 'bg-red-100 dark:bg-red-900/30' },
  DUCKDB: { icon: sqliteLogo, color: 'bg-yellow-100 dark:bg-yellow-900/30' },
  CLICKHOUSE: { icon: sqliteLogo, color: 'bg-red-100 dark:bg-red-900/30' },
  COCKROACHDB: PG_ICON,
  REDSHIFT: PG_ICON,
  YUGABYTEDB: PG_ICON,
  TIMESCALEDB: PG_ICON,
  KINGBASEES: PG_ICON,
  GAUSSDB: PG_ICON,
  HIGHGO: PG_ICON,
  UXDB: PG_ICON,
  OPENGAUSS: PG_ICON,
  GBASE8C: PG_ICON,
  TIDB: MYSQL_ICON,
  OCEANBASE: MYSQL_ICON,
  TDSQL: MYSQL_ICON,
  POLARDB: MYSQL_ICON,
  DM8: MYSQL_ICON,
  ORACLE: DEFAULT_ICON,
  DB2: DEFAULT_ICON,
  H2: DEFAULT_ICON,
  SNOWFLAKE: DEFAULT_ICON,
  DM8ORACLE: DEFAULT_ICON,
  XUGUDB: DEFAULT_ICON,
  GBASE8A: DEFAULT_ICON,
  TRINO: DEFAULT_ICON,
  PRESTO: DEFAULT_ICON,
}

export function useDatabaseIcon() {
  const getDatabaseIcon = (type: DatabaseType): string => {
    return databaseIcons[type]?.icon ?? DEFAULT_ICON.icon
  }

  const getDatabaseColor = (type: DatabaseType): string => {
    return databaseIcons[type]?.color ?? DEFAULT_ICON.color
  }

  const getDatabaseConfig = (type: DatabaseType): DatabaseIconConfig => {
    return databaseIcons[type] ?? DEFAULT_ICON
  }

  return {
    getDatabaseIcon,
    getDatabaseColor,
    getDatabaseConfig,
  }
}
