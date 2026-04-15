import { parseSqlStatements } from '@/composables/useSqlStatements'

describe('parseSqlStatements', () => {
  describe('simple statements', () => {
    it('parses a single SELECT without semicolon', () => {
      const sql = 'SELECT * FROM users'
      const result = parseSqlStatements(sql)
      expect(result).toHaveLength(1)
      expect(result[0].statement).toBe('SELECT * FROM users')
    })

    it('parses a single SELECT with semicolon', () => {
      const sql = 'SELECT * FROM users;'
      const result = parseSqlStatements(sql)
      expect(result).toHaveLength(1)
      expect(result[0].statement).toBe('SELECT * FROM users')
    })

    it('parses two statements separated by semicolon', () => {
      const sql = 'SELECT * FROM users;\nDELETE FROM logs;'
      const result = parseSqlStatements(sql)
      expect(result).toHaveLength(2)
      expect(result[0].statement).toBe('SELECT * FROM users')
      expect(result[1].statement).toBe('DELETE FROM logs')
    })

    it('parses multi-line statement', () => {
      const sql = 'SELECT *\nFROM users\nWHERE id = 1;'
      const result = parseSqlStatements(sql)
      expect(result).toHaveLength(1)
      expect(result[0].statement).toBe('SELECT *\nFROM users\nWHERE id = 1')
    })

    it('ignores blank lines and comment lines', () => {
      const sql = '\n-- comment\nSELECT * FROM users;'
      const result = parseSqlStatements(sql)
      expect(result).toHaveLength(1)
      expect(result[0].statement).toContain('SELECT * FROM users')
    })
  })

  describe('cte (common table expression) statements', () => {
    it('treats a single CTE as one statement', () => {
      const sql = [
        'WITH ActiveUsers AS (',
        '  SELECT Id, FirstName FROM Users WHERE IsActive = 1',
        ')',
        'SELECT * FROM ActiveUsers;',
      ].join('\n')

      const result = parseSqlStatements(sql)
      expect(result).toHaveLength(1)
      expect(result[0].statement).toContain('WITH ActiveUsers AS')
      expect(result[0].statement).toContain('SELECT * FROM ActiveUsers')
    })

    it('treats a CTE with INNER JOIN as one statement', () => {
      const sql = [
        'WITH ActiveUsers AS (',
        '  SELECT Id, FirstName, LastName FROM Users WHERE IsActive = 1',
        ')',
        'SELECT au.FirstName, au.LastName, o.ProductName',
        'FROM ActiveUsers au',
        'INNER JOIN Orders o ON au.Id = o.UserId;',
      ].join('\n')

      const result = parseSqlStatements(sql)
      expect(result).toHaveLength(1)
      expect(result[0].statement).toContain('WITH ActiveUsers AS')
      expect(result[0].statement).toContain('INNER JOIN Orders')
    })

    it('treats multiple CTEs (comma-separated) as one statement', () => {
      const sql = [
        'WITH',
        '  cte1 AS (SELECT 1 AS n),',
        '  cte2 AS (SELECT 2 AS n)',
        'SELECT * FROM cte1 UNION ALL SELECT * FROM cte2;',
      ].join('\n')

      const result = parseSqlStatements(sql)
      expect(result).toHaveLength(1)
      expect(result[0].statement).toContain('WITH')
      expect(result[0].statement).toContain('cte1')
      expect(result[0].statement).toContain('cte2')
    })

    it('correctly separates a CTE from a following plain SELECT', () => {
      const sql = [
        'WITH ActiveUsers AS (',
        '  SELECT Id FROM Users WHERE IsActive = 1',
        ')',
        'SELECT * FROM ActiveUsers;',
        'SELECT * FROM Orders;',
      ].join('\n')

      const result = parseSqlStatements(sql)
      expect(result).toHaveLength(2)
      expect(result[0].statement).toContain('WITH ActiveUsers AS')
      expect(result[1].statement).toBe('SELECT * FROM Orders')
    })

    it('records correct line positions for a CTE statement', () => {
      const sql = [
        'WITH cte AS (',
        '  SELECT 1',
        ')',
        'SELECT * FROM cte;',
      ].join('\n')

      const result = parseSqlStatements(sql)
      expect(result).toHaveLength(1)
      expect(result[0].position.startLineNumber).toBe(1)
      expect(result[0].position.endLineNumber).toBe(4)
    })
  })

  describe('nested parens (subqueries)', () => {
    it('does not split on SELECT inside a subquery', () => {
      const sql = [
        'SELECT * FROM (',
        '  SELECT id FROM users WHERE active = 1',
        ') sub;',
      ].join('\n')

      const result = parseSqlStatements(sql)
      expect(result).toHaveLength(1)
      expect(result[0].statement).toContain('SELECT * FROM')
      expect(result[0].statement).toContain('SELECT id FROM users')
    })
  })
})
