/**
 * Dialect profiles + formatter→monaco dialect mapping.
 *
 * Single source of truth for SQL keywords/types/functions (migrated from
 * useMonacoEditor.ts) and for converging per-dialect differences as DATA
 * (quote char, schema qualification) instead of code branches.
 *
 * Pure module — no monaco/tauri imports (jest node-testable).
 */

import type { DialectProfile, SQLDialect } from './types'
import { GRAMMAR_DIALECTS } from './types'

// ── SQL keywords / types / functions (migrated verbatim from useMonacoEditor.ts) ──

export const SQL_KEYWORDS: string[] = [
  'SELECT',
  'FROM',
  'WHERE',
  'INSERT',
  'UPDATE',
  'DELETE',
  'CREATE',
  'ALTER',
  'DROP',
  'TABLE',
  'INDEX',
  'VIEW',
  'DATABASE',
  'SCHEMA',
  'JOIN',
  'INNER',
  'LEFT',
  'RIGHT',
  'OUTER',
  'ON',
  'AS',
  'AND',
  'OR',
  'NOT',
  'NULL',
  'IS',
  'IN',
  'BETWEEN',
  'LIKE',
  'ORDER',
  'BY',
  'GROUP',
  'HAVING',
  'LIMIT',
  'OFFSET',
  'UNION',
  'DISTINCT',
  'COUNT',
  'SUM',
  'AVG',
  'MAX',
  'MIN',
  'CAST',
  'CASE',
  'WHEN',
  'THEN',
  'ELSE',
  'END',
  'PRIMARY',
  'KEY',
  'FOREIGN',
  'REFERENCES',
  'CONSTRAINT',
  'UNIQUE',
  'CHECK',
  'DEFAULT',
  'AUTO_INCREMENT',
  'CASCADE',
  'SET',
  'VALUES',
  'INTO',
  'BEGIN',
  'COMMIT',
  'ROLLBACK',
  'TRANSACTION',
  'SAVEPOINT',
  'TRUNCATE',
  'GRANT',
  'REVOKE',
  'WITH',
  'RECURSIVE',
  'WINDOW',
  'PARTITION',
  'OVER',
  'ROW_NUMBER',
  'RANK',
  'DENSE_RANK',
]

export const SQL_TYPES: string[] = [
  'INT',
  'INTEGER',
  'BIGINT',
  'SMALLINT',
  'TINYINT',
  'DECIMAL',
  'NUMERIC',
  'FLOAT',
  'REAL',
  'DOUBLE',
  'CHAR',
  'VARCHAR',
  'TEXT',
  'NCHAR',
  'NVARCHAR',
  'NTEXT',
  'DATE',
  'TIME',
  'DATETIME',
  'TIMESTAMP',
  'YEAR',
  'BOOLEAN',
  'BOOL',
  'BINARY',
  'VARBINARY',
  'BLOB',
  'CLOB',
  'JSON',
  'UUID',
  'SERIAL',
  'BIGSERIAL',
]

export const SQL_FUNCTIONS: string[] = [
  'CONCAT',
  'SUBSTRING',
  'UPPER',
  'LOWER',
  'TRIM',
  'LTRIM',
  'RTRIM',
  'LENGTH',
  'REPLACE',
  'COALESCE',
  'NULLIF',
  'IFNULL',
  'NOW',
  'CURRENT_DATE',
  'CURRENT_TIME',
  'CURRENT_TIMESTAMP',
  'DATE_ADD',
  'DATE_SUB',
  'DATEDIFF',
  'EXTRACT',
  'TO_CHAR',
  'TO_DATE',
  'TO_NUMBER',
  'ROUND',
  'CEIL',
  'FLOOR',
  'ABS',
  'SIGN',
  'MOD',
  'POWER',
  'SQRT',
  'EXP',
  'LN',
  'LOG',
]

export const NO_PAREN_FUNCTIONS: string[] = [
  'NOW',
  'CURRENT_DATE',
  'CURRENT_TIME',
  'CURRENT_TIMESTAMP',
]

// ── Dialect profiles ──

const BASE_PROFILE = {
  keywords: SQL_KEYWORDS,
  functions: SQL_FUNCTIONS,
  types: SQL_TYPES,
  noParenFunctions: NO_PAREN_FUNCTIONS,
}

/** Full profiles for dialects with monaco grammar; others reuse the 'sql' profile. */
export const DIALECT_PROFILES: Record<SQLDialect, DialectProfile> = {
  sql: {
    id: 'sql',
    quoteChar: '"',
    supportsSchemaQualification: true,
    ...BASE_PROFILE,
  },
  pgsql: {
    id: 'pgsql',
    quoteChar: '"',
    supportsSchemaQualification: true,
    ...BASE_PROFILE,
  },
  mysql: {
    id: 'mysql',
    quoteChar: '`',
    supportsSchemaQualification: true,
    ...BASE_PROFILE,
  },
  // mssql/plsql/sqlite: completion registered for these ids (future grammar),
  // but they reuse the generic profile until real dialect data is added.
  mssql: {
    id: 'mssql',
    quoteChar: '"',
    supportsSchemaQualification: true,
    ...BASE_PROFILE,
  },
  plsql: {
    id: 'plsql',
    quoteChar: '"',
    supportsSchemaQualification: true,
    ...BASE_PROFILE,
  },
  sqlite: {
    id: 'sqlite',
    quoteChar: '"',
    supportsSchemaQualification: false,
    ...BASE_PROFILE,
  },
}

export function getDialectProfile(id: SQLDialect): DialectProfile {
  return DIALECT_PROFILES[id]
}

/** True when the dialect id has a monaco grammar contribution (highlighting). */
export function hasGrammar(id: SQLDialect): boolean {
  return GRAMMAR_DIALECTS.includes(id)
}

// ── formatter dialect → monaco dialect mapping ──
// resolveDialect() (useSqlFormatter) returns sql-formatter ids; the editor needs
// monaco language ids. Unmapped ids fall back to 'sql' (generic).

const FORMATTER_TO_MONACO: Record<string, SQLDialect> = {
  // PostgreSQL family
  postgresql: 'pgsql',
  redshift: 'pgsql',
  // MySQL family
  mysql: 'mysql',
  mariadb: 'mysql',
  tidb: 'mysql',
  // SQL Server family
  tsql: 'mssql',
  // Oracle family
  plsql: 'plsql',
  oracle: 'plsql',
  // SQLite
  sqlite: 'sqlite',
  // Everything else → generic 'sql' (trino, hive, spark, bigquery, db2,
  // duckdb, clickhouse, snowflake, hana, teradata, exasol, ...)
}

export function resolveMonacoDialect(formatterId: string): SQLDialect {
  return FORMATTER_TO_MONACO[formatterId] ?? 'sql'
}
