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

const databaseIcons: Record<DatabaseType, DatabaseIconConfig> = {
  POSTGRESQL: {
    icon: postgresqlLogo,
    color: 'bg-blue-100 dark:bg-blue-900/30',
  },
  MYSQL: {
    icon: mysqlLogo,
    color: 'bg-orange-100 dark:bg-orange-900/30',
  },
  MARIADB: {
    icon: mariadbLogo,
    color: 'bg-purple-100 dark:bg-purple-900/30',
  },
  SQLITE: {
    icon: sqliteLogo,
    color: 'bg-green-100 dark:bg-green-900/30',
  },
  SQLSERVER: {
    icon: mssqlLogo,
    color: 'bg-red-100 dark:bg-red-900/30',
  },
}

export function useDatabaseIcon() {
  const getDatabaseIcon = (type: DatabaseType): string => {
    return databaseIcons[type]?.icon ?? postgresqlLogo
  }

  const getDatabaseColor = (type: DatabaseType): string => {
    return databaseIcons[type]?.color ?? 'bg-gray-100 dark:bg-gray-900/30'
  }

  const getDatabaseConfig = (type: DatabaseType): DatabaseIconConfig => {
    return databaseIcons[type] ?? databaseIcons.POSTGRESQL
  }

  return {
    getDatabaseIcon,
    getDatabaseColor,
    getDatabaseConfig,
  }
}
