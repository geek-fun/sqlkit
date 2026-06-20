import type { SqlLanguage } from 'sql-formatter'
import { format } from 'sql-formatter'
import { DatabaseType } from '@/store/connectionStore'

export type FormatOptions = {
  tabWidth?: number
  expressionWidth?: number
  keywordCase?: 'upper' | 'lower'
}

export type FormatResult = {
  sql: string
  error: string | null
}

/**
 * Mapping from DatabaseType enum to sql-formatter dialect strings.
 * PostgreSQL variants map to 'postgresql', MySQL variants to 'mysql', etc.
 */
const dialectMap: Record<string, string> = {
  [DatabaseType.POSTGRESQL]: 'postgresql',
  [DatabaseType.COCKROACHDB]: 'postgresql',
  [DatabaseType.REDSHIFT]: 'redshift',
  [DatabaseType.YUGABYTEDB]: 'postgresql',
  [DatabaseType.TIMESCALEDB]: 'postgresql',
  [DatabaseType.KINGBASEES]: 'postgresql',
  [DatabaseType.GAUSSDB]: 'postgresql',
  [DatabaseType.HIGHGO]: 'postgresql',
  [DatabaseType.UXDB]: 'postgresql',
  [DatabaseType.OPENGAUSS]: 'postgresql',
  [DatabaseType.GBASE8C]: 'postgresql',
  [DatabaseType.QUESTDB]: 'postgresql',
  [DatabaseType.VASTBASE]: 'postgresql',
  [DatabaseType.YASHANDB]: 'postgresql',

  [DatabaseType.MYSQL]: 'mysql',
  [DatabaseType.MARIADB]: 'mariadb',
  [DatabaseType.TIDB]: 'tidb',
  [DatabaseType.OCEANBASE]: 'mysql',
  [DatabaseType.TDSQL]: 'mysql',
  [DatabaseType.POLARDB]: 'mysql',
  [DatabaseType.DORIS]: 'mysql',
  [DatabaseType.SELECTDB]: 'mysql',
  [DatabaseType.STARROCKS]: 'mysql',
  [DatabaseType.DATABEND]: 'mysql',
  [DatabaseType.GOLDENDB]: 'mysql',
  [DatabaseType.MANTICORESEARCH]: 'mysql',

  [DatabaseType.SQLITE]: 'sqlite',

  [DatabaseType.SQLSERVER]: 'tsql',

  [DatabaseType.DUCKDB]: 'duckdb',

  [DatabaseType.CLICKHOUSE]: 'clickhouse',

  [DatabaseType.ORACLE]: 'plsql',
  [DatabaseType.DAMENG]: 'plsql',

  [DatabaseType.DB2]: 'db2',

  [DatabaseType.H2]: 'sql',
  [DatabaseType.DERBY]: 'sql',

  [DatabaseType.SNOWFLAKE]: 'snowflake',

  [DatabaseType.XUGUDB]: 'mysql',
  [DatabaseType.GBASE8A]: 'mysql',

  [DatabaseType.TRINO]: 'trino',
  [DatabaseType.PRESTO]: 'trino',

  [DatabaseType.HIVE]: 'hive',

  [DatabaseType.DATABRICKS]: 'spark',

  [DatabaseType.HANA]: 'tsql',
  [DatabaseType.TERADATA]: 'tsql',
  [DatabaseType.VERTICA]: 'postgresql',
  [DatabaseType.EXASOL]: 'tsql',

  [DatabaseType.BIGQUERY]: 'bigquery',

  [DatabaseType.INFORMIX]: 'sql',
  [DatabaseType.KYLIN]: 'sql',
  [DatabaseType.CASSANDRA]: 'sql',
  [DatabaseType.IRIS]: 'sql',
  [DatabaseType.ACCESS]: 'sql',
}

/**
 * Resolve a sql-formatter dialect string from a DatabaseType.
 * Falls back to 'sql' (generic SQL) for unknown types.
 */
export function resolveDialect(dbType: DatabaseType): string {
  return dialectMap[dbType] ?? 'sql'
}

/**
 * Format a SQL string using sql-formatter with the given options.
 *
 * @param sql - The SQL string to format
 * @param dialect - The sql-formatter dialect string (use resolveDialect to get this)
 * @param options - Formatting options
 * @returns A FormatResult containing the formatted SQL or an error message
 */
export function formatSql(sql: string, dialect: string, options: FormatOptions = {}): FormatResult {
  if (!sql || !sql.trim()) {
    return { sql, error: null }
  }

  try {
    const formatted = format(sql, {
      language: dialect as SqlLanguage,
      tabWidth: options.tabWidth ?? 2,
      keywordCase: options.keywordCase ?? 'upper',
      expressionWidth: options.expressionWidth ?? 120,
    })
    return { sql: formatted, error: null }
  }
  catch (err) {
    const message = err instanceof Error ? err.message : String(err)
    return { sql, error: message }
  }
}

/**
 * Vue composable for SQL formatting.
 * Returns the formatSql function and dialect resolution helper.
 */
export function useSqlFormatter() {
  return { formatSql, resolveDialect }
}
