/**
 * @jest-environment node
 */
import {
  getDialectProfile,
  hasGrammar,
  NO_PAREN_FUNCTIONS,
  resolveMonacoDialect,
  SQL_FUNCTIONS,
  SQL_KEYWORDS,
  SQL_TYPES,
} from '@/composables/sqlCompletion/dialects'

describe('resolveMonacoDialect — formatter→monaco map', () => {
  it('maps the five known families', () => {
    expect(resolveMonacoDialect('postgresql')).toBe('pgsql')
    expect(resolveMonacoDialect('mysql')).toBe('mysql')
    expect(resolveMonacoDialect('tsql')).toBe('mssql')
    expect(resolveMonacoDialect('plsql')).toBe('plsql')
    expect(resolveMonacoDialect('sqlite')).toBe('sqlite')
  })

  it('maps mysql-family formatter ids to mysql', () => {
    expect(resolveMonacoDialect('mariadb')).toBe('mysql')
    expect(resolveMonacoDialect('tidb')).toBe('mysql')
  })

  it('maps postgresql-family ids to pgsql', () => {
    expect(resolveMonacoDialect('redshift')).toBe('pgsql')
  })

  it('falls back to sql for unmapped formatter ids', () => {
    for (const id of ['trino', 'snowflake', 'duckdb', 'clickhouse', 'hive', 'spark', 'bigquery', 'db2', 'hana', 'teradata', 'exasol', 'bogus', '']) {
      expect(resolveMonacoDialect(id)).toBe('sql')
    }
  })
})

describe('dialect profiles', () => {
  it('exposes a profile for every SQLDialect id', () => {
    for (const id of ['sql', 'mysql', 'pgsql', 'mssql', 'plsql', 'sqlite'] as const) {
      expect(getDialectProfile(id).id).toBe(id)
    }
  })

  it('mysql uses backtick quote char, others use double quote', () => {
    expect(getDialectProfile('mysql').quoteChar).toBe('`')
    expect(getDialectProfile('pgsql').quoteChar).toBe('"')
    expect(getDialectProfile('sql').quoteChar).toBe('"')
  })

  it('sqlite does not support schema qualification, pgsql does', () => {
    expect(getDialectProfile('sqlite').supportsSchemaQualification).toBe(false)
    expect(getDialectProfile('pgsql').supportsSchemaQualification).toBe(true)
  })

  it('hasGrammar is true only for dialects with monaco grammar contributions', () => {
    expect(hasGrammar('sql')).toBe(true)
    expect(hasGrammar('mysql')).toBe(true)
    expect(hasGrammar('pgsql')).toBe(true)
    expect(hasGrammar('mssql')).toBe(false)
    expect(hasGrammar('plsql')).toBe(false)
    expect(hasGrammar('sqlite')).toBe(false)
  })
})

describe('migrated keyword/type/function lists (single source of truth)', () => {
  it('sql profile keyword list is non-empty and matches SQL_KEYWORDS', () => {
    expect(getDialectProfile('sql').keywords).toBe(SQL_KEYWORDS)
    expect(SQL_KEYWORDS.length).toBe(77)
    expect(SQL_KEYWORDS).toContain('SELECT')
    expect(SQL_KEYWORDS).toContain('DENSE_RANK')
  })

  it('types and functions lists migrated fully', () => {
    expect(SQL_TYPES).toContain('BIGSERIAL')
    expect(SQL_FUNCTIONS).toContain('LOG')
    expect(SQL_FUNCTIONS.length).toBe(34)
  })

  it('noParenFunctions is exactly the four no-paren functions', () => {
    expect(NO_PAREN_FUNCTIONS).toEqual(['NOW', 'CURRENT_DATE', 'CURRENT_TIME', 'CURRENT_TIMESTAMP'])
    expect(getDialectProfile('sql').noParenFunctions).toBe(NO_PAREN_FUNCTIONS)
  })
})
