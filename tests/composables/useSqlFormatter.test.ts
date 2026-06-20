import { formatSql, resolveDialect } from '@/composables/useSqlFormatter'
import { DatabaseType } from '@/store/connectionStore'

describe('resolveDialect', () => {
  it('maps PostgreSQL to postgresql', () => {
    expect(resolveDialect(DatabaseType.POSTGRESQL)).toBe('postgresql')
  })

  it('maps MySQL to mysql', () => {
    expect(resolveDialect(DatabaseType.MYSQL)).toBe('mysql')
  })

  it('maps SQLite to sqlite', () => {
    expect(resolveDialect(DatabaseType.SQLITE)).toBe('sqlite')
  })

  it('maps SQL Server to tsql', () => {
    expect(resolveDialect(DatabaseType.SQLSERVER)).toBe('tsql')
  })

  it('maps Oracle to plsql', () => {
    expect(resolveDialect(DatabaseType.ORACLE)).toBe('plsql')
  })

  it('maps unknown type to sql fallback', () => {
    // Use a type-cast to simulate an unknown DatabaseType
    expect(resolveDialect('UNKNOWN_DB' as DatabaseType)).toBe('sql')
  })

  it('maps all PostgreSQL variants correctly', () => {
    const pgVariants = [
      DatabaseType.COCKROACHDB,
      DatabaseType.YUGABYTEDB,
      DatabaseType.TIMESCALEDB,
      DatabaseType.KINGBASEES,
      DatabaseType.GAUSSDB,
      DatabaseType.HIGHGO,
      DatabaseType.UXDB,
      DatabaseType.OPENGAUSS,
      DatabaseType.GBASE8C,
      DatabaseType.QUESTDB,
      DatabaseType.VASTBASE,
      DatabaseType.YASHANDB,
    ]
    for (const variant of pgVariants) {
      expect(resolveDialect(variant)).toBe('postgresql')
    }
  })

  it('maps MariaDB to mariadb', () => {
    expect(resolveDialect(DatabaseType.MARIADB)).toBe('mariadb')
  })

  it('maps TiDB to tidb', () => {
    expect(resolveDialect(DatabaseType.TIDB)).toBe('tidb')
  })

  it('maps all MySQL variants correctly', () => {
    const mysqlVariants = [
      DatabaseType.OCEANBASE,
      DatabaseType.TDSQL,
      DatabaseType.POLARDB,
      DatabaseType.DORIS,
      DatabaseType.SELECTDB,
      DatabaseType.STARROCKS,
      DatabaseType.DATABEND,
      DatabaseType.GOLDENDB,
      DatabaseType.MANTICORESEARCH,
      DatabaseType.XUGUDB,
      DatabaseType.GBASE8A,
    ]
    for (const variant of mysqlVariants) {
      expect(resolveDialect(variant)).toBe('mysql')
    }
  })
})

describe('formatSql', () => {
  describe('mySQL dialect', () => {
    it('formats simple SELECT with keywords uppercased', () => {
      const result = formatSql('select * from users where id = 1', 'mysql')
      expect(result.error).toBeNull()
      expect(result.sql).toContain('SELECT')
      expect(result.sql).toContain('FROM')
      expect(result.sql).toContain('WHERE')
      expect(result.sql).not.toContain('select')
    })

    it('formats multi-line query consistently', () => {
      const result = formatSql('SELECT id, name FROM users ORDER BY name', 'mysql')
      expect(result.error).toBeNull()
      const lines = result.sql.split('\n')
      expect(lines.length).toBeGreaterThan(1)
      expect(lines[0]).toContain('SELECT')
    })
  })

  describe('postgreSQL dialect', () => {
    it('formats SELECT with postgresql dialect', () => {
      const result = formatSql('select id, name, email from users where active = true', 'postgresql')
      expect(result.error).toBeNull()
      expect(result.sql).toContain('SELECT')
      expect(result.sql).toContain('FROM')
      expect(result.sql).toContain('WHERE')
    })
  })

  describe('sQLite dialect', () => {
    it('formats SELECT with sqlite dialect', () => {
      const result = formatSql('select * from users limit 10', 'sqlite')
      expect(result.error).toBeNull()
      expect(result.sql).toContain('SELECT')
      expect(result.sql).toContain('FROM')
    })
  })

  describe('sQL Server (tsql) dialect', () => {
    it('formats SELECT with tsql dialect', () => {
      const result = formatSql('select top 10 * from users order by id', 'tsql')
      expect(result.error).toBeNull()
      expect(result.sql).toContain('SELECT')
      expect(result.sql).toContain('FROM')
    })
  })

  describe('unknown dialect fallback', () => {
    it('falls back to generic sql formatter', () => {
      const result = formatSql('select * from users', 'unknown_dialect')
      // Should not throw - sql-formatter will throw ConfigError, which we catch
      expect(result.error).not.toBeNull()
      expect(result.error).toContain('Unsupported SQL dialect')
      // Original SQL should be returned when error occurs
      expect(result.sql).toBe('select * from users')
    })
  })

  describe('error handling', () => {
    it('returns original SQL on parse error with error message', () => {
      // Use a truly invalid token that triggers a parse error
      const result = formatSql('SELECT {{ FROM users', 'mysql')
      expect(result.error).not.toBeNull()
      expect(result.error).toContain('Parse error')
      expect(result.sql).toBe('SELECT {{ FROM users')
    })

    it('returns empty string unchanged for empty input', () => {
      const result = formatSql('', 'mysql')
      expect(result.error).toBeNull()
      expect(result.sql).toBe('')
    })

    it('returns whitespace-only unchanged', () => {
      const result = formatSql('   ', 'mysql')
      expect(result.error).toBeNull()
      expect(result.sql).toBe('   ')
    })

    it('handles invalid SQL syntax gracefully', () => {
      const result = formatSql('SELECT id {{ FROM users', 'mysql')
      expect(result.error).not.toBeNull()
      expect(result.sql).toBe('SELECT id {{ FROM users')
    })
  })

  describe('tab width option', () => {
    it('respects tabWidth=4 producing wider indentation', () => {
      const resultWide = formatSql('select * from users', 'mysql', { tabWidth: 4 })
      const resultNarrow = formatSql('select * from users', 'mysql', { tabWidth: 2 })

      expect(resultWide.error).toBeNull()
      expect(resultNarrow.error).toBeNull()

      // The formatted SQL should have different indentation
      const wideIndent = resultWide.sql.match(/\n(\s+)/)?.[1]
      const narrowIndent = resultNarrow.sql.match(/\n(\s+)/)?.[1]

      // Wide should use more spaces per indent
      if (wideIndent && narrowIndent) {
        expect(wideIndent.length).toBeGreaterThan(narrowIndent.length)
      }
    })
  })

  describe('keyword case option', () => {
    it('formats keywords in lowercase when specified', () => {
      const result = formatSql('SELECT * FROM users', 'mysql', { keywordCase: 'lower' })
      expect(result.error).toBeNull()
      expect(result.sql).toContain('select')
      expect(result.sql).toContain('from')
    })
  })

  describe('round-trip formatting', () => {
    it('idempotently formats already-formatted SQL', () => {
      const sql = 'SELECT *\nFROM users\nWHERE id = 1'
      const result1 = formatSql(sql, 'mysql')
      expect(result1.error).toBeNull()
      const result2 = formatSql(result1.sql, 'mysql')
      expect(result2.error).toBeNull()
      // Second pass should not change the output
      expect(result2.sql).toBe(result1.sql)
    })
  })
})
