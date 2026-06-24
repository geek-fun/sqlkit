import { DatabaseType } from '@/store'

export type DbCapabilities = {
  schemas: boolean
  procedures: boolean
  functions: boolean
  views: boolean
  materializedViews: boolean
  backup: boolean
  export: boolean
  newDatabase: boolean
  newSchema: boolean
  dropDatabase: boolean
}

const ALL: DbCapabilities = {
  schemas: true,
  procedures: true,
  functions: true,
  views: true,
  materializedViews: true,
  backup: true,
  export: true,
  newDatabase: true,
  newSchema: true,
  dropDatabase: true,
}

const NONE: DbCapabilities = {
  schemas: false,
  procedures: false,
  functions: false,
  views: false,
  materializedViews: false,
  backup: false,
  export: false,
  newDatabase: false,
  newSchema: false,
  dropDatabase: false,
}

const PG_COMPAT: DbCapabilities = { ...ALL, backup: false }

const MYSQL_COMPAT: DbCapabilities = { ...ALL, schemas: false, materializedViews: false, backup: false, newSchema: false, newDatabase: true, dropDatabase: true }

const SQLITE_LIKE: DbCapabilities = { ...NONE, views: true, export: true }

const CAPABILITY_MAP: Partial<Record<DatabaseType, DbCapabilities>> = {
  [DatabaseType.POSTGRESQL]: PG_COMPAT,
  [DatabaseType.COCKROACHDB]: PG_COMPAT,
  [DatabaseType.REDSHIFT]: PG_COMPAT,
  [DatabaseType.YUGABYTEDB]: PG_COMPAT,
  [DatabaseType.TIMESCALEDB]: PG_COMPAT,
  [DatabaseType.KINGBASEES]: PG_COMPAT,
  [DatabaseType.GAUSSDB]: PG_COMPAT,
  [DatabaseType.HIGHGO]: PG_COMPAT,
  [DatabaseType.UXDB]: PG_COMPAT,
  [DatabaseType.OPENGAUSS]: PG_COMPAT,
  [DatabaseType.GBASE8C]: PG_COMPAT,
  [DatabaseType.QUESTDB]: PG_COMPAT,
  [DatabaseType.VASTBASE]: PG_COMPAT,
  [DatabaseType.YASHANDB]: PG_COMPAT,
  [DatabaseType.GREENPLUM]: PG_COMPAT,
  [DatabaseType.ENTERPRISEDB]: PG_COMPAT,
  [DatabaseType.CRATEDB]: PG_COMPAT,
  [DatabaseType.MATERIALIZE]: { ...PG_COMPAT, materializedViews: true },
  [DatabaseType.ALLOYDB]: PG_COMPAT,
  [DatabaseType.CLOUDSQLPG]: PG_COMPAT,
  [DatabaseType.FUJITSUPG]: PG_COMPAT,

  [DatabaseType.MYSQL]: MYSQL_COMPAT,
  [DatabaseType.MARIADB]: MYSQL_COMPAT,
  [DatabaseType.TIDB]: MYSQL_COMPAT,
  [DatabaseType.OCEANBASE]: MYSQL_COMPAT,
  [DatabaseType.TDSQL]: MYSQL_COMPAT,
  [DatabaseType.POLARDB]: MYSQL_COMPAT,
  [DatabaseType.DORIS]: { ...MYSQL_COMPAT, procedures: false },
  [DatabaseType.SELECTDB]: { ...MYSQL_COMPAT, procedures: false },
  [DatabaseType.STARROCKS]: { ...MYSQL_COMPAT, procedures: false },
  [DatabaseType.DATABEND]: { ...MYSQL_COMPAT, procedures: false },
  [DatabaseType.GOLDENDB]: MYSQL_COMPAT,
  [DatabaseType.MANTICORESEARCH]: { ...MYSQL_COMPAT, procedures: false, functions: false },
  [DatabaseType.SINGLESTOREMEMSQL]: MYSQL_COMPAT,
  [DatabaseType.CLOUDSQLMYSQL]: MYSQL_COMPAT,

  [DatabaseType.SQLITE]: SQLITE_LIKE,
  [DatabaseType.DUCKDB]: { ...NONE, views: true, export: true, materializedViews: true },
  [DatabaseType.RQLITE]: SQLITE_LIKE,
  [DatabaseType.TURSO]: SQLITE_LIKE,

  [DatabaseType.SQLSERVER]: { ...ALL, materializedViews: false },
  [DatabaseType.ORACLE]: { ...ALL, materializedViews: true },
  [DatabaseType.DAMENG]: { ...ALL, materializedViews: false },
  [DatabaseType.OCEANBASE_ORACLE]: { ...ALL, materializedViews: false },
  [DatabaseType.XUGUDB]: { ...ALL, materializedViews: false },
  [DatabaseType.GBASE8A]: { ...ALL, materializedViews: false },

  [DatabaseType.CLICKHOUSE]: { ...ALL, procedures: false, functions: false, materializedViews: true, schemas: false, newSchema: false },
  [DatabaseType.DB2]: { ...ALL, materializedViews: false },
  [DatabaseType.H2]: { ...ALL, materializedViews: false },
  [DatabaseType.SNOWFLAKE]: ALL,
  [DatabaseType.TRINO]: { ...ALL, newDatabase: false, newSchema: false, dropDatabase: false, procedures: false },
  [DatabaseType.PRESTO]: { ...ALL, newDatabase: false, newSchema: false, dropDatabase: false, procedures: false },
  [DatabaseType.DERBY]: { ...ALL, materializedViews: false },
  [DatabaseType.HIVE]: { ...ALL, newSchema: false, procedures: false, functions: false },
  [DatabaseType.DATABRICKS]: { ...ALL, newDatabase: false, newSchema: false, dropDatabase: false, procedures: false },
  [DatabaseType.HANA]: { ...ALL, materializedViews: false },
  [DatabaseType.TERADATA]: { ...ALL, materializedViews: false },
  [DatabaseType.VERTICA]: ALL,
  [DatabaseType.EXASOL]: { ...ALL, materializedViews: false },
  [DatabaseType.BIGQUERY]: { ...ALL, newDatabase: false, dropDatabase: false, materializedViews: true },
  [DatabaseType.INFORMIX]: { ...ALL, materializedViews: false },
  [DatabaseType.KYLIN]: { ...ALL, newDatabase: false, newSchema: false, dropDatabase: false },
  [DatabaseType.CASSANDRA]: { ...NONE, export: true, views: false },
  [DatabaseType.IRIS]: { ...ALL, materializedViews: false },
  [DatabaseType.ACCESS]: { ...NONE, export: true, views: true },
  [DatabaseType.FIREBIRD]: { ...ALL, materializedViews: false },
  [DatabaseType.TDENGINE]: { ...NONE, export: true },
}

const DEFAULT_CAPS: DbCapabilities = { ...ALL }

export function getDbCapabilities(dbType: DatabaseType): DbCapabilities {
  return CAPABILITY_MAP[dbType] ?? DEFAULT_CAPS
}
